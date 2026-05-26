//! Guided-runbook HTTP surface.
//!
//! `GET /api/v1/runbooks` lists the catalog; `POST /api/v1/runbooks/{id}/runs`
//! instantiates a runbook against a target by planning one guarded [`Job`] per step
//! (each still requires its own confirm/approve/apply — a runbook is a guided
//! checklist, not a bypass); `GET /api/v1/runbooks/runs/{run_id}` reports progress
//! derived from the underlying job states.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};

use demon_clients::random_state;
use demon_core::{JobState, Residency, RunStatus, RunbookId};

use crate::jobs::{plan_job, state_str};
use crate::session::AuthCtx;
use crate::{now_ms, AppState, ErrorBody};

/// A started runbook (its ordered planned job ids).
#[derive(Clone)]
struct RunRecord {
    runbook: RunbookId,
    target: String,
    job_ids: Vec<String>,
}

/// In-memory runbook-run store (poison-safe).
#[derive(Clone, Default)]
pub struct RunbookStore {
    inner: Arc<Mutex<HashMap<String, RunRecord>>>,
}

impl RunbookStore {
    /// Empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn put(&self, id: String, rec: RunRecord) {
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(id, rec);
    }

    fn get(&self, id: &str) -> Option<RunRecord> {
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(id)
            .cloned()
    }
}

fn err(code: StatusCode, msg: impl Into<String>) -> Response {
    (code, Json(ErrorBody { error: msg.into() })).into_response()
}

fn run_status_str(s: RunStatus) -> &'static str {
    match s {
        RunStatus::Pending => "pending",
        RunStatus::InProgress => "in_progress",
        RunStatus::Completed => "completed",
        RunStatus::Failed => "failed",
    }
}

// ---- catalog ---------------------------------------------------------------

#[derive(Serialize)]
struct CatalogStep {
    action: String,
    description: String,
}

#[derive(Serialize)]
struct CatalogEntry {
    id: String,
    title: String,
    steps: Vec<CatalogStep>,
}

/// `GET /api/v1/runbooks` — the runbook catalog.
pub(crate) async fn catalog<R: Residency>(State(_s): State<AppState<R>>) -> Response {
    let entries: Vec<CatalogEntry> = RunbookId::ALL
        .into_iter()
        .map(|rb| CatalogEntry {
            id: rb.id().to_owned(),
            title: rb.title().to_owned(),
            steps: rb
                .steps("<target>")
                .into_iter()
                .map(|st| CatalogStep {
                    action: st.action.id().to_owned(),
                    description: st.description,
                })
                .collect(),
        })
        .collect();
    Json(serde_json::json!({ "data": entries })).into_response()
}

// ---- start + progress ------------------------------------------------------

#[derive(Deserialize)]
pub(crate) struct StartReq {
    target: String,
}

#[derive(Serialize)]
struct RunStepDto {
    job_id: String,
    action: String,
    description: String,
    confirm_phrase: String,
    dual_control: bool,
    state: &'static str,
}

#[derive(Serialize)]
struct RunDto {
    run_id: String,
    runbook: String,
    target: String,
    status: &'static str,
    current: usize,
    total: usize,
    steps: Vec<RunStepDto>,
}

/// `POST /api/v1/runbooks/{id}/runs` — plan every step into a guarded job.
pub(crate) async fn start<R: Residency>(
    State(s): State<AppState<R>>,
    Extension(ctx): Extension<AuthCtx>,
    Path(id): Path<String>,
    Json(req): Json<StartReq>,
) -> Response {
    let Some(rb) = RunbookId::from_id(&id) else {
        return err(StatusCode::NOT_FOUND, format!("unknown runbook {id:?}"));
    };
    let steps = rb.steps(&req.target);
    // Plan (authorize) every step up front; a single forbidden step fails the whole run.
    let mut planned = Vec::new();
    for st in &steps {
        match plan_job(&s, &ctx.principal, st.action.id(), &st.target) {
            Ok(job) => planned.push((job, st.description.clone())),
            Err((code, msg)) => return err(code, msg),
        }
    }
    let run_id = random_state();
    s.runbooks.put(
        run_id.clone(),
        RunRecord {
            runbook: rb,
            target: req.target.clone(),
            job_ids: planned.iter().map(|(j, _)| j.id.clone()).collect(),
        },
    );
    // Durable audit of the runbook start (per-step actions are audited on apply).
    let _ = s
        .store
        .append_audit(
            &ctx.principal.sub,
            "runbook.start",
            &req.target,
            false,
            &format!("{{\"runbook\":\"{}\"}}", rb.id()),
            now_ms(),
        )
        .await;

    let step_dtos = planned
        .iter()
        .map(|(j, desc)| RunStepDto {
            job_id: j.id.clone(),
            action: j.plan.action.id().to_owned(),
            description: desc.clone(),
            confirm_phrase: j.plan.confirm_phrase.clone(),
            dual_control: j.plan.dual_control,
            state: state_str(j.state),
        })
        .collect();
    (
        StatusCode::CREATED,
        Json(RunDto {
            run_id,
            runbook: rb.id().to_owned(),
            target: req.target,
            status: "pending",
            current: 0,
            total: steps.len(),
            steps: step_dtos,
        }),
    )
        .into_response()
}

/// `GET /api/v1/runbooks/runs/{run_id}` — progress derived from the step jobs.
pub(crate) async fn get_run<R: Residency>(
    State(s): State<AppState<R>>,
    Path(run_id): Path<String>,
) -> Response {
    let Some(rec) = s.runbooks.get(&run_id) else {
        return err(StatusCode::NOT_FOUND, "no such runbook run");
    };
    let mut run = demon_core::RunbookRun::new(rec.runbook, &rec.target);
    let descriptions = rec.runbook.steps(&rec.target);
    let mut steps = Vec::new();
    let mut advancing = true;
    for (idx, jid) in rec.job_ids.iter().enumerate() {
        let job = s.jobs.get(jid);
        let state = job.as_ref().map_or("missing", |j| state_str(j.state));
        if advancing {
            match job.as_ref().map(|j| j.state) {
                Some(JobState::Verified) => run.advance(true),
                Some(JobState::Failed | JobState::RolledBack) => {
                    run.advance(false);
                    advancing = false;
                }
                _ => advancing = false, // current step still in progress
            }
        }
        steps.push(RunStepDto {
            job_id: jid.clone(),
            action: descriptions
                .get(idx)
                .map_or_else(String::new, |st| st.action.id().to_owned()),
            description: descriptions
                .get(idx)
                .map_or_else(String::new, |st| st.description.clone()),
            confirm_phrase: job
                .as_ref()
                .map_or_else(String::new, |j| j.plan.confirm_phrase.clone()),
            dual_control: job.as_ref().is_some_and(|j| j.plan.dual_control),
            state,
        });
    }
    Json(RunDto {
        run_id,
        runbook: rec.runbook.id().to_owned(),
        target: rec.target,
        status: run_status_str(run.status),
        current: run.current,
        total: run.total_steps,
        steps,
    })
    .into_response()
}
