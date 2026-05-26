# proximiio.demon — Core Architecture (01)

Status: DRAFT — for build agent. Date: 2026-05-25. Author: backend/architecture.

> **Guiding principle.** A brand-new DevOps admin can do EVERYTHING from inside this one
> tool: no SSH hopping, no ad-hoc scripts, no tribal knowledge. The daemon discovers state,
> proposes actions, and walks the admin through runbooks. It administers **two fleets** through
> one model: (1) FreeBSD **core** infra, (2) N Linux **enterprise per-tenant** installs.

---

## 1. Why a daemon (not just the TUI), and what becomes a thin client

`proximiio-tui` today is a stateless operator console: every poll re-SSHes hosts, parses
`check-<area>.sh` line-contracts, and shows a snapshot that dies when you close the terminal.
That model cannot do *always-on* monitoring, scheduled backups/rotation, bottleneck detection,
or "what changed while I was asleep". A long-lived daemon owns **state over time** and **acts
without a human watching**.

**Demon = the single brain.** It holds inventory, runs pollers/schedulers/reconcilers, keeps
the audit chain, and exposes everything over an HTTP+WS API.

```
              ┌────────────── proximiio.demon (always-on) ──────────────┐
  TUI client ─┤  axum API (REST + WS)   demon-core   workers   store    ├─ SSH/WireGuard ─▶ FreeBSD core
  Web UI    ──┤  authn/z  audit-chain   reconciler   pollers   SQLite   ├─ API clients ───▶ kalista/vulture/identity/vault
  CLI       ──┘                                                          ├─ SSH ───────────▶ N Linux tenant installs
              └─────────────────────────────────────────────────────────┘
```

**Thin clients.** The TUI is refactored so its `app.rs`/`update.rs` render server-pushed state
instead of polling SSH directly. A new web UI consumes the **same** REST+WS API. No business
logic in clients — they render `demon-core` view models and POST intents.

**Reuse / migration (high value, low risk):**
- The host-side `check-<area>.sh` **read-only line-contract** scripts move *as-is* into the daemon's
  SSH layer. Their pure-Rust parsers (malformed/empty → `Unknown`, never panic) migrate verbatim
  into `demon-core`.
- `ssh::ensure_whitelisted_mutation` + typed-confirm + dry-run + **redacted hash-chained audit**
  becomes the daemon's mutation core. Secrets stay in env, never argv/log.
- `kalista_api.rs` / `vulture_api.rs` / `identity_api.rs` become `demon-clients` modules.
- `coordination/` file contracts remain the source of truth for cross-service boundaries.

---

## 2. Cargo workspace layout

```
proximiio-demon/
├─ crates/
│  ├─ demon-core      # domain: types, parsers, reconciler logic, runbook engine, audit chain.
│  │                  #   PURE. no axum, no tokio runtime, no sqlite driver. Fully unit-testable.
│  ├─ demon-store     # persistence: sqlx/SQLite, migrations, repositories, retention. Trait-bounded.
│  ├─ demon-collect   # outward I/O: SSH-over-WireGuard runner, line-contract parsers wiring,
│  │                  #   service API clients (kalista/vulture/identity/vault/opensearch).
│  ├─ demon-workers   # tokio: pollers, schedulers, bottleneck analyzer, event bus, reconcile loop.
│  ├─ demon-server    # axum: routes, WS, authn/z hooks, OpenAPI, view-model mapping.
│  ├─ demon-bin       # binary: wiring, config, graceful shutdown, signal handling.
│  └─ clients/
│     ├─ demon-tui    # refactored ratatui client
│     └─ demon-sdk    # typed Rust API client (shared by TUI + CLI + tests)
```

**Boundary justification.** `demon-core` is pure so domain logic (parser correctness, reconcile
decisions, audit chaining) is tested without DB/network/runtime. `demon-store` and `demon-collect`
sit behind traits so `demon-workers`/`demon-server` depend on **interfaces**, not drivers — enables
fakes in tests and future driver swaps (e.g. libSQL). `demon-sdk` is generated/typed once, so every
client (TUI, web via JSON, CLI) shares request/response types.

---

## 3. Axum API surface (REST + WebSocket)

Versioned under `/api/v1`. OpenAPI emitted from handlers (utoipa). Auth deferred to security doc
(02): JWT from **terrapi-identity** for humans; mTLS/service token for the TUI/CLI on the WireGuard
net. Every mutation passes the audit + typed-confirm hooks before execution.

**Resource model (REST):**
| Resource | Verbs | Notes |
|---|---|---|
| `/hosts` | GET, GET/:id | FreeBSD core + Linux tenant hosts; `?fleet=core\|tenant`, `?residency=eu\|uae` |
| `/services` | GET, GET/:id | kalista/vulture/identity/vault/sync/opensearch instances |
| `/residency-groups` | GET | `eu`, `uae` — read-only grouping; enforced filter on every query |
| `/tenants` | GET, POST, GET/:id, PATCH, DELETE | enterprise per-tenant installs; lifecycle state machine |
| `/jobs` | GET, POST, GET/:id, DELETE | async actions/mutations; returns job id, stream via WS |
| `/alerts` | GET, POST/:id/ack, POST/:id/resolve | alert state + suggested runbook link |
| `/audit` | GET | read-only, hash-chained, redacted; export to OpenSearch |
| `/identities` | GET, POST… | proxied terrapi-identity admin (users/SSO/tokens) |
| `/runbooks` | GET, GET/:id, POST/:id/run | discoverable guided workflows (see §8) |
| `/reconcile` | GET (diffs), POST (apply) | desired vs observed (see §7) |

**WebSocket** `/api/v1/stream` — server-push topics: `health`, `jobs.<id>`, `alerts`, `audit.tail`,
`reconcile.diffs`. Subscription-based; replaces client polling. Snapshot-on-subscribe + deltas.

**Conventions:** RFC-7807 problem+json errors; cursor pagination on lists; idempotency keys on
job POSTs; every list query is residency-scoped server-side (defense in depth, never client trust).

---

## 4. Persistent state — SQLite schema

SQLite in **WAL** mode (one writer, many readers fits a single daemon). `sqlx` with compile-checked
queries + versioned migrations. Tables (abbrev.):

- `hosts(id, fqdn, fleet, os, residency_group, wg_ip, tenant_id?, enrolled_at, last_seen)`
- `services(id, host_id, kind, version, residency_group, desired_state, observed_state, updated_at)`
- `residency_groups(id, region)` — seed `eu`,`uae`.
- `tenants(id, name, residency_group, lifecycle_state, plan, created_at)` — see §6.
- `health_snapshots(id, target_id, target_kind, area, status, raw_json, observed_at)` — rolling,
  retention-pruned (e.g. 30d). One row per poll per area.
- `metrics(target_id, metric, value, ts)` — **short window only** (see recommendation below).
- `jobs(id, kind, target_id, params_json, state, dry_run, started_at, finished_at, result_json, requested_by)`
- `audit(seq, prev_hash, hash, actor, action, target, dry_run, redacted_payload, ts)` — append-only
  hash chain migrated from the TUI's `audit.rs`/`seq.rs`.
- `alerts(id, target_id, severity, state, opened_at, ack_by, resolved_at, runbook_id?)`
- `runbook_runs(id, runbook_id, state, current_step, context_json, started_by)`
- `desired_state(target_id, key, value, source)` — for reconciler (§7).
- `schema_migrations(version, applied_at)`.

**Metrics: delegate time-series to OpenSearch, NOT SQLite.** SQLite holds *current* + short health
snapshots for the dashboard and recent-trend; long-horizon metrics/logs go to the per-residency
OpenSearch (already the audit/logs/search store). Rationale: SQLite write amplification + retention
churn on high-cardinality metrics is the wrong tool; OpenSearch already exists per residency group
and air-gaps cleanly. Demon queries OpenSearch on demand for deep trends; keeps a small rollup cache.

**HA recommendation (tradeoffs):**
- *Phase 1 (ship this):* single daemon + **Litestream** continuous backup of the SQLite file to
  per-residency object storage. RPO seconds, manual/scripted failover. Simplest, no consensus, fits
  "single brain". Strong recommendation to start here.
- *Phase 2 (if uptime SLA demands):* **rqlite** or **libSQL replica** for a warm standby behind a
  WireGuard VIP. Cost: distributed write semantics, more ops. Defer until single-node downtime is
  proven unacceptable.
- Reject multi-master: the daemon is intentionally the *one* serialized actor; concurrent writers
  would break the audit chain's linearizability.
- **Per-residency isolation is mandatory:** run **one demon + one SQLite + one Litestream target per
  residency group**. EU and UAE never share a daemon, DB, or backup target. A "global" read-only
  fan-out view (if ever needed) is a separate aggregator, not a shared store.

---

## 5. Worker / scheduler model

`demon-workers` runs on tokio with a structured supervision tree. Components:

- **Per-service pollers** — one task per (host, area), driven by the migrated `check-*.sh` contracts.
  Staggered intervals, jittered, bounded concurrency via a global `Semaphore` (don't SSH-storm the
  fleet). Each poll → parse → `health_snapshots` + emit `health` event. Parser failure ⇒ `Unknown`,
  task survives.
- **Schedulers** — cron-like (`tokio-cron-scheduler`): backups, secret/key rotation, checksum/FIM
  sweeps, retention pruning, Litestream verification. Each scheduled run is a **job** (auditable,
  dry-runnable).
- **Bottleneck analyzer loop** — periodically reads recent snapshots/metrics, applies rules
  (e.g. OpenSearch heap/queue/latency over threshold ⇒ raise `alert` + suggest the
  `scale-opensearch` runbook with a pre-filled plan). Read-only proposal; never auto-scales without
  the gated apply (§7).
- **Reconcile loop** — see §7.
- **Event bus** — in-process `tokio::sync::broadcast` per topic; the WS layer subscribes and fans out
  to clients. Bounded channels = **backpressure**: slow WS clients are dropped, never block workers.

**Failure isolation & resume.** Each worker is its own supervised task; a panic is caught, logged,
audited, and the task is restarted with backoff — one dead poller never takes the daemon down. Jobs
are persisted with state, so on daemon restart, in-flight jobs are marked `interrupted` and
resumable/retryable rather than silently lost. Graceful shutdown drains the job queue and flushes the
audit chain.

---

## 6. Multi-fleet + multi-tenant model

One inventory, two `fleet` discriminators, with a `tenant_id` foreign key making per-tenant installs
first-class:

- **Core (FreeBSD):** shared infra; `hosts.fleet='core'`, `tenant_id=NULL`. Services are the in-house
  stack instances.
- **Tenant (Linux):** each enterprise customer = a `tenants` row + its own host(s)
  (`fleet='tenant'`, `tenant_id=T`). A tenant has its **own lifecycle** (`provisioning → active →
  upgrading → suspended → decommissioned`), its own service versions, its own backup/upgrade cadence.

**Isolation.** Every query is scoped by `tenant_id` and `residency_group` at the repository layer —
a tenant view can never leak another tenant's hosts/secrets/audit. Mutations targeting a tenant carry
the `tenant_id` in the audit record. Tenant onboarding/upgrade/decommission are **runbooks** (§8), so
a new admin provisions a customer by following the guided flow, not by hand.

**Residency tagging is non-negotiable and orthogonal to fleet:** every host, service, tenant, snapshot,
and audit row carries `residency_group`. Because EU/UAE are physically air-gapped, a single demon
instance only ever sees one group's hosts (per §4). The model supports both regions; deployment keeps
them separate.

---

## 7. State reconciliation — desired vs observed

**Recommendation: pragmatic middle, not full k8s.** Pure imperative ("run this action now") loses
intent; full continuous auto-reconciliation is dangerous for stateful infra (you do not want a loop
silently restarting a vault or spawning OpenSearch nodes at 3am).

Adopt **declared desired state + a reconciler that DETECTS drift and PROPOSES, but applies only on
explicit gated approval** (typed-confirm, like today's mutations):

1. Admin (or a runbook) writes `desired_state` (e.g. `opensearch.eu.nodes = 5`,
   `service.kalista.version = X`).
2. Pollers populate `observed_state`.
3. The reconcile loop computes diffs continuously and surfaces them on `/reconcile` + the
   `reconcile.diffs` WS topic ("3 drifts: kalista on host-7 behind by one version").
4. **Apply is a human-gated job** (or, for whitelisted low-risk keys, opt-in auto-apply per tenant).
   Apply runs the same audited/dry-runnable mutation pipeline.

This gives the safety of imperative + the clarity of declarative, and the diff view IS part of "what
is broken and how to fix it".

---

## 8. New-admin onboarding surface

The architecture makes "everything in one tool" structural, not a UX afterthought:

- **Runbooks are first-class data** (`/runbooks`, `runbook_runs`). A runbook = an ordered set of steps
  (check / mutation / decision), each backed by existing contracts and the audited mutation pipeline.
  Examples shipped: `onboard-tenant`, `upgrade-service`, `scale-opensearch`, `rotate-secret`,
  `restore-backup`, `enroll-host`. A new admin runs `provision a customer` by following prompts; the
  engine executes vetted steps, shows dry-runs, and records everything.
- **Discoverable actions.** The API advertises, per resource, the set of *available actions* in its
  current state (HATEOAS-lite: each host/service/tenant response includes `available_actions[]` with
  their input schemas). Clients render buttons/menus from this — no hardcoded action lists, nothing
  hidden in scripts.
- **"What is broken and how to fix it."** Alerts and reconcile diffs each link a suggested
  `runbook_id` with a pre-filled context. The dashboard answers triage directly, then offers the
  one-click guided fix.
- **Inline safety.** Dry-run, typed-confirm, and the redacted audit trail mean a new admin can explore
  and act without fear — and seniors can review exactly what was done.

This is how tribal knowledge gets encoded into the tool instead of living in people's heads.

---

## 9. Open questions / risks for the build agent

1. **SQLite single-writer vs job throughput** — validate WAL write throughput under full-fleet poll +
   scheduler load; if contended, introduce a single serialized writer actor (channel-fed). Confirm
   before Phase 2 HA work.
2. **OpenSearch as metric store across air-gap** — confirm the demon can reach the per-residency
   OpenSearch from within the same WireGuard group only; define rollup/query contract in a new
   `coordination/conventions/metrics.md`.
3. **terrapi-identity dependency for daemon auth** — the daemon needs auth *before* identity may be
   fully up; define a bootstrap/break-glass path (security doc 02).
4. **Tenant-scale SSH fan-out** — N Linux tenants × per-area pollers may exceed safe concurrency;
   tune the global semaphore and consider per-tenant poll-interval tiers.
5. **Reconciler blast radius** — agree the whitelist of auto-appliable `desired_state` keys; default
   everything to human-gated.
6. **Litestream restore drills** — schedule and audit periodic restore tests; an unverified backup is
   not a backup.
7. **Schema/version skew between demon and the services it controls** — version the API clients
   against `coordination/CONTRACTS.md` and fail closed on contract mismatch.
8. **Client/SDK drift** — keep `demon-sdk` the single typed source; web UI consumes OpenAPI-generated
   types to avoid a second hand-written client.

---

*Companion docs to follow: `02-security-authz.md` (auth, RBAC, break-glass, audit export),
`03-runbook-engine.md`, `04-deployment-and-ha.md`.*
