//! Guarded-mutation HTTP surface: plan → approve (dual-control) → typed-confirm →
//! apply (step-up gated) → verify, every transition durably audited.
//!
//! The pure pipeline lives in `demon-core::mutation`; execution in
//! `demon-workers::exec`. This module is the in-memory job store + the Axum handlers
//! that drive them, gating each step by capability, dual-control, and the step-up
//! [`FactorPolicy`].

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};

use demon_clients::random_state;
use demon_core::{
    authorize, ActionSpec, Actor, AuditEvent, DualControl, FactorLevel, FactorPolicy,
    GuardedAction, JobState, Outcome, Plan, Principal, Residency, Target,
};
use demon_workers::exec::{execute, SshMutator};

use crate::session::AuthCtx;
use crate::{now_ms, now_rfc3339, AppState, ErrorBody};

/// An in-flight or completed mutation job.
#[derive(Clone)]
pub(crate) struct Job {
    pub(crate) id: String,
    pub(crate) plan: Plan,
    pub(crate) state: JobState,
    dual: Option<DualControl>,
    report: Option<String>,
    /// Step-up factor presented for *this* job (touch-per-op), if any.
    stepped_up: Option<FactorLevel>,
}

/// In-memory job store (poison-safe).
#[derive(Clone, Default)]
pub struct JobStore {
    inner: Arc<Mutex<HashMap<String, Job>>>,
}

impl JobStore {
    /// Empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn put(&self, job: Job) {
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(job.id.clone(), job);
    }

    pub(crate) fn get(&self, id: &str) -> Option<Job> {
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(id)
            .cloned()
    }

    fn list(&self) -> Vec<Job> {
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .values()
            .cloned()
            .collect()
    }

    /// Record a per-job step-up factor (set by the WebAuthn assertion). Returns whether
    /// the job exists.
    pub(crate) fn mark_stepped_up(&self, id: &str, level: FactorLevel) -> bool {
        let mut g = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(job) = g.get_mut(id) {
            job.stepped_up = Some(level);
            true
        } else {
            false
        }
    }
}

// ---- DTOs ------------------------------------------------------------------

pub(crate) fn state_str(s: JobState) -> &'static str {
    match s {
        JobState::Planned => "planned",
        JobState::AwaitingApproval => "awaiting_approval",
        JobState::Confirmed => "confirmed",
        JobState::DryRunOk => "dry_run_ok",
        JobState::Applying => "applying",
        JobState::Verified => "verified",
        JobState::Failed => "failed",
        JobState::RolledBack => "rolled_back",
    }
}

#[derive(Serialize)]
struct JobDto {
    id: String,
    action: String,
    target: String,
    state: &'static str,
    confirm_phrase: String,
    dual_control: bool,
    dual_satisfied: bool,
    report: Option<String>,
}

fn dto(j: &Job) -> JobDto {
    JobDto {
        id: j.id.clone(),
        action: j.plan.action.id().to_owned(),
        target: j.plan.target.clone(),
        state: state_str(j.state),
        confirm_phrase: j.plan.confirm_phrase.clone(),
        dual_control: j.plan.dual_control,
        dual_satisfied: j.dual.as_ref().is_none_or(DualControl::is_satisfied),
        report: j.report.clone(),
    }
}

fn err(code: StatusCode, msg: impl Into<String>) -> Response {
    (code, Json(ErrorBody { error: msg.into() })).into_response()
}

fn host_id(target: &str) -> &str {
    let t = target.strip_prefix("host:").unwrap_or(target);
    t.split('/').next().unwrap_or(t)
}

/// Durable hash-chained audit + best-effort B3 fan-out.
async fn audit<R: Residency>(
    s: &AppState<R>,
    actor_sub: &str,
    action: &str,
    target: &str,
    outcome: Outcome,
) {
    let payload = format!(
        "{{\"outcome\":\"{}\"}}",
        match outcome {
            Outcome::Success => "success",
            Outcome::Failure => "failure",
        }
    );
    if let Err(e) = s
        .store
        .append_audit(actor_sub, action, target, false, &payload, now_ms())
        .await
    {
        tracing::error!(error = %e, "durable audit append failed");
    }
    if let Some(audit) = s.audit.clone() {
        let ev = AuditEvent::control_plane(
            now_rfc3339(),
            s.node.clone(),
            R::REGION,
            Actor::user(actor_sub.to_owned(), None),
            action,
            Target::new("action", Some(target.to_owned())),
            outcome,
        );
        tokio::spawn(async move {
            let _ = audit.ship(&ev).await;
        });
    }
}

// ---- handlers --------------------------------------------------------------

/// Authorize + plan an action into a [`Job`], storing it. Shared by the jobs API and
/// the runbook instantiation. Returns `(status, message)` on failure.
pub(crate) fn plan_job<R: Residency>(
    s: &AppState<R>,
    principal: &Principal,
    action_id: &str,
    target: &str,
) -> Result<Job, (StatusCode, String)> {
    let ga = GuardedAction::from_id(action_id).ok_or((
        StatusCode::NOT_FOUND,
        format!("unknown action {action_id:?}"),
    ))?;
    let spec = ActionSpec::new(action_id.to_owned(), target.to_owned(), ga.class());
    let cap = authorize(principal, spec).map_err(|e| (StatusCode::FORBIDDEN, e.to_string()))?;
    let plan = Plan::from_capability(&cap).ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "could not build plan".to_owned(),
    ))?;
    let dual = plan
        .dual_control
        .then(|| DualControl::new(principal.sub.clone(), 1));
    let state = if plan.dual_control {
        JobState::AwaitingApproval
    } else {
        JobState::Planned
    };
    let job = Job {
        id: random_state(),
        plan,
        state,
        dual,
        report: None,
        stepped_up: None,
    };
    s.jobs.put(job.clone());
    Ok(job)
}

#[derive(Deserialize)]
pub(crate) struct CreateReq {
    action: String,
    target: String,
}

/// `POST /api/v1/jobs` — authorize + plan an action.
pub(crate) async fn create<R: Residency>(
    State(s): State<AppState<R>>,
    Extension(ctx): Extension<AuthCtx>,
    Json(req): Json<CreateReq>,
) -> Response {
    let job = match plan_job(&s, &ctx.principal, &req.action, &req.target) {
        Ok(j) => j,
        Err((code, msg)) => return err(code, msg),
    };
    audit(
        &s,
        &ctx.principal.sub,
        "job.plan",
        &job.plan.target,
        Outcome::Success,
    )
    .await;
    (StatusCode::CREATED, Json(dto(&job))).into_response()
}

/// `POST /api/v1/jobs/{id}/approve` — record a dual-control approval.
pub(crate) async fn approve<R: Residency>(
    State(s): State<AppState<R>>,
    Extension(ctx): Extension<AuthCtx>,
    Path(id): Path<String>,
) -> Response {
    let Some(mut job) = s.jobs.get(&id) else {
        return err(StatusCode::NOT_FOUND, "no such job");
    };
    let Some(dc) = job.dual.as_mut() else {
        return err(StatusCode::BAD_REQUEST, "job does not require dual-control");
    };
    if let Err(e) = dc.approve(&ctx.principal.sub) {
        return err(StatusCode::CONFLICT, e.to_string());
    }
    s.jobs.put(job.clone());
    audit(
        &s,
        &ctx.principal.sub,
        "job.approve",
        &job.plan.target,
        Outcome::Success,
    )
    .await;
    Json(dto(&job)).into_response()
}

#[derive(Deserialize)]
pub(crate) struct ConfirmReq {
    typed: String,
}

/// `POST /api/v1/jobs/{id}/confirm` — typed-confirm (and require approvals if dual).
pub(crate) async fn confirm<R: Residency>(
    State(s): State<AppState<R>>,
    Extension(ctx): Extension<AuthCtx>,
    Path(id): Path<String>,
    Json(req): Json<ConfirmReq>,
) -> Response {
    let Some(mut job) = s.jobs.get(&id) else {
        return err(StatusCode::NOT_FOUND, "no such job");
    };
    if !matches!(job.state, JobState::Planned | JobState::AwaitingApproval) {
        return err(StatusCode::CONFLICT, "job is not awaiting confirmation");
    }
    if !job.plan.confirm_matches(&req.typed) {
        return err(StatusCode::BAD_REQUEST, "confirm phrase does not match");
    }
    if job.plan.dual_control && !job.dual.as_ref().is_some_and(DualControl::is_satisfied) {
        return err(StatusCode::CONFLICT, "dual-control not satisfied");
    }
    job.state = JobState::Confirmed;
    s.jobs.put(job.clone());
    audit(
        &s,
        &ctx.principal.sub,
        "job.confirm",
        &job.plan.target,
        Outcome::Success,
    )
    .await;
    Json(dto(&job)).into_response()
}

#[derive(Serialize)]
struct ApplyDto {
    state: &'static str,
    dry_run_output: Option<String>,
    apply_output: Option<String>,
    verify_status: Option<String>,
    error: Option<String>,
}

/// `POST /api/v1/jobs/{id}/apply` — step-up gated execute (dry-run → apply → verify).
pub(crate) async fn apply<R: Residency>(
    State(s): State<AppState<R>>,
    Extension(ctx): Extension<AuthCtx>,
    Path(id): Path<String>,
) -> Response {
    let Some(mut job) = s.jobs.get(&id) else {
        return err(StatusCode::NOT_FOUND, "no such job");
    };
    if job.state != JobState::Confirmed {
        return err(StatusCode::CONFLICT, "job must be confirmed before apply");
    }
    // Step-up: destructive/secret/CA classes need a fresh per-op factor. A WebAuthn
    // assertion on this job (touch-per-op) takes precedence over the session factor.
    let policy = FactorPolicy::default();
    let factor = job.stepped_up.unwrap_or(ctx.factor);
    if !policy.satisfied(job.plan.action.class(), factor) {
        return err(
            StatusCode::FORBIDDEN,
            format!(
                "step-up required for a {:?} action (have {:?}, need {:?}) — POST /api/v1/jobs/{}/stepup/start",
                job.plan.action.class(),
                factor,
                policy.required(job.plan.action.class()),
                job.id,
            ),
        );
    }
    // Resolve the target host.
    let host = match s.store.get_host(host_id(&job.plan.target)).await {
        Ok(Some(h)) => h,
        Ok(None) => return err(StatusCode::NOT_FOUND, "target host not in inventory"),
        Err(e) => return err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    let addr = host.wg_ip.clone().unwrap_or_else(|| host.fqdn.clone());
    let service_mgr = if host.os.contains("freebsd") {
        "rc"
    } else {
        "systemd"
    };
    let mutator = SshMutator::new(s.transport.clone(), addr.clone(), service_mgr);

    let report = execute(&job.plan, &mutator, &s.transport, &addr).await;
    job.state = report.final_state;
    job.report.clone_from(&report.error);
    s.jobs.put(job.clone());

    let outcome = if report.final_state == JobState::Verified {
        Outcome::Success
    } else {
        Outcome::Failure
    };
    audit(
        &s,
        &ctx.principal.sub,
        job.plan.action.id(),
        &job.plan.target,
        outcome,
    )
    .await;

    let dto = ApplyDto {
        state: state_str(report.final_state),
        dry_run_output: report.dry_run_output,
        apply_output: report.apply_output,
        verify_status: report.verify.map(|v| v.status.as_str().to_owned()),
        error: report.error,
    };
    Json(dto).into_response()
}

/// `GET /api/v1/jobs` — list jobs.
pub(crate) async fn list<R: Residency>(State(s): State<AppState<R>>) -> Response {
    let jobs: Vec<JobDto> = s.jobs.list().iter().map(dto).collect();
    Json(serde_json::json!({ "data": jobs })).into_response()
}

/// `GET /api/v1/jobs/{id}` — one job.
pub(crate) async fn get<R: Residency>(
    State(s): State<AppState<R>>,
    Path(id): Path<String>,
) -> Response {
    match s.jobs.get(&id) {
        Some(j) => Json(dto(&j)).into_response(),
        None => err(StatusCode::NOT_FOUND, "no such job"),
    }
}
