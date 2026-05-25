# Design: Traffic inspector, request traps, and rate-limit monitoring

> Status: **proposal / awaiting decisions**. Once the pivotal choices
> (§7) are locked, this becomes one or more ADRs under
> `docs/architecture/`. Scope is large (4 features × backend+frontend);
> build it in the phases of §8, not all at once.

## 0. What the operator asked for

1. **Full inspect view** on a request (headers, query params, body, …)
   and likewise on the response — reachable from *Hosts → Recent traffic*.
2. **Pause button** + retain the **last 1000 requests**.
3. **Request trap**: match live requests by criteria (URL, method, auth
   header, query param, …), capture matching request+response, and
   **auto-delete after 1h** — debug only.
4. **Rate-limiter monitoring** over **at least the last 30 days**, plus
   **alerting when the rate limiter trips often for a particular IP**.
5. **More detailed audit history** — capture *what specifically changed*
   (field-level before/after diff), not just the action.

## 1. Verified current state (what we build on)

| Thing | Today | File |
|---|---|---|
| `AccessLogEntry` | **metadata only** — `at, host, method, path(+query), status, duration_ms, remote_ip, upstream, phases, upgraded, grpc`. **No headers, no body.** | `kalista-config/src/access.rs` |
| `AccessLogBus` | `tokio::sync::broadcast`, **pure fan-out, zero retention**. One instance, cloned writer (data plane) + reader (`/ws/logs`). Slow consumer → `Lagged(n)`. | `kalista-config/src/access.rs` |
| `logging()` callback | builds the entry from `Session`/`Ctx`. Request headers, query, and response status **are reachable here**; bodies are **not** read. | `kalista-data-plane/src/proxy.rs` |
| Body filters | `request_body_filter` (size-gates → 413) and `response_body_filter` (buffers into `ctx.cache_capture` **only when cacheable**) both exist. Bodies otherwise **stream, never buffered**. | `proxy.rs` |
| Rate limiting | `pingora_limits::Rate` (count-min sketch) in `Arc<DashMap<HostId, Rate>>`, evaluated in `request_filter`, 429 + `Retry-After`. Emits `kalista_proxy_requests_rate_limited_total{host,tenant,route}`. | `proxy.rs` |
| **Rate-limit stats** | `RateLimitStatsRegistry` **already exists**: per-host ring (256 hits, 60 s window), `histogram()` + `top_ips()`, written by data plane on 429, read by host-detail sparkline. **No persistence, no alerting.** | `kalista-config/src/ratelimit_stats.rs` |
| Cross-plane shared mutable state | **`KeyStoreCache`** = sibling `Arc<…DashMap…>` next to `LiveConfig` (NOT in the ArcSwap snapshot); entry CRUD bypasses snapshot rebuild; built in `main.rs`, cloned into both planes. | `kalista-config/src/keystore_cache.rs` |
| Scheduled jobs | `tokio::spawn` interval loops on the admin runtime (backups w/ retention pruning; cluster replica sync). `select! { sleep(interval) … , shutdown … }`. | `backups.rs`, `cluster_sync.rs` |
| Persistence | sqlx + numbered migrations; `Storage::open_temp()` test helper; **no cross-op transaction API**; **no time-series/aggregate table exists**. | `kalista-storage/` |
| Endpoints | `utoipa_axum::OpenApiRouter`, nested under `/api/v1/tenants/{slug}/…`; `TenantScope` extractor + per-handler `min_role_check`. TS client regenerates from `/openapi.json`. | `handlers/tenants/mod.rs` |
| Frontend traffic UI | `LogsPage` (full screen: pause/play already present, status filters, client ring `MAX_BUFFERED=500`, WS reconnect + `Lagged` handling). `HostDetailPage` "Live logs" tab (single-host, `RECENT_LIMIT=15`). | `web/src/pages/LogsPage.tsx`, `HostDetailPage.tsx` |
| Reusable FE blocks | shadcn `Dialog`, `PageEnvelope` (+ contract test), `StatusPill`, pause-toggle, WS-reconnect pattern. **No `Sheet` component yet.** | `web/src/components/` |

**Hard constraints (from CLAUDE.md / ADRs):**
- The admin runtime must never starve the data plane.
- **No SQLite on any data-plane path** — read `LiveConfig` / in-memory only.
- No avoidable allocations in `request_filter`/`upstream_peer`.
- Live-mutable state that isn't per-request config goes in a **sibling
  `Arc`**, not the `LiveConfig` ArcSwap snapshot.
- OpenAPI is the contract; don't hand-edit the generated TS client.

## 2. Unifying architecture — a tiered capture model

The four asks share one tension: **bodies are the only expensive thing**,
and capturing them for all traffic violates hot-path discipline. We
resolve everything with **capture tiers**, each paying only for what it
needs:

- **Tier 0 — live metadata tail (exists, unchanged).** `AccessLogEntry`
  over `AccessLogBus` → `/ws/logs`. Stays lean so the high-volume browser
  tail never regresses.
- **Tier 1 — last-1000 ring with headers (new, cheap, always-on).** A
  **server-side** ring buffer (in the control plane) of the last 1000
  requests *including request+response headers and query params, no
  body*. Fed by an internal bus subscriber. Survives WS reconnect /
  page reload (today a fresh subscriber sees nothing until new traffic).
  Headers are KB-scale → 1000 entries ≈ a few MB. Backs both the
  "last 1000" requirement and header/query inspect.
- **Tier 2 — full capture incl. body (new, opt-in, gated).** Bodies
  (size-capped, both directions) are captured **only** for requests that
  match an **active trap** (Feature 3) or an explicit per-host "capture
  bodies" debug toggle. Ephemeral, TTL'd. This is the only path that
  buffers bodies, and only for matching traffic.
- **Tier 3 — rate-limit history (new).** In-memory aggregation (extends
  `RateLimitStatsRegistry`) periodically **rolled up to SQLite buckets**
  by an admin-runtime job → 30-day charts + alerting. Data plane only
  ever touches memory.

This keeps the hot path honest: Tier 0/1 add at most cheap header clones
(behind a toggle); bodies and DB writes are confined to opt-in, low-rate,
or admin-runtime paths.

## 3. Feature 1 — Full request/response inspect

**Data plane.** Extend the capture so `logging()` (and, when capturing
bodies, the body filters) can produce a richer record. Two record kinds:
- `AccessLogEntry` (unchanged) → Tier 0 live tail.
- `InspectRecord { meta, req_headers, query, resp_headers, req_body?, resp_body?, body_truncated }` → Tier 1/2.

To avoid bloating the browser tail, the richer record travels a **separate
internal channel** (bounded `mpsc` or a second broadcast) consumed by a
control-plane **capture sink**; the existing `AccessLogBus` is untouched.
Header capture is governed by a cheap flag (default on; per-host or global
toggle to disable). The sink maintains the Tier-1 ring keyed by request id.

**Control plane.**
- `GET /api/v1/tenants/{slug}/traffic/recent?host=&limit=` → backfill the
  last N from the ring (initial page load).
- `GET …/traffic/{request_id}` → full `InspectRecord` for the detail view
  (headers/query always; body only if Tier 2 captured it).

**Frontend.** A request row in *Recent traffic* (LogsPage and the host
"Live logs" tab) becomes clickable → opens an **inspect panel** (new
`Sheet`/side-drawer, or reuse `Dialog`) with tabs: *Overview* (timing
phases, status, upstream), *Request* (method/url/query + headers + body),
*Response* (status + headers + body). Bodies render with content-type
awareness (JSON pretty-print, hexdump for binary, "not captured" hint when
Tier 2 was off).

## 4. Feature 2 — Pause + last 1000

- **Pause** already exists client-side in `LogsPage`; extend it to the
  host "Live logs" tab.
- **Last 1000**: bump the client ring 500→1000 **and** add the
  server-side Tier-1 ring (§3) so the 1000 survive reconnect/reload via
  `GET …/traffic/recent`. The ring is a sibling `Arc<Mutex<VecDeque>>` (or
  `ArrayDeque`) in the control plane, capped at 1000 globally (or
  per-host); fed by the capture sink.

## 5. Feature 3 — Request trap

**Definition** (persisted in SQLite so traps survive restart — they're
small):
```
trap(id, tenant_id, host_id?, enabled, name,
     match_method?, match_path_glob?, match_query?  (key[, value]),
     match_header?  (name[, value]; e.g. Authorization),
     capture_req_body, capture_resp_body, max_body_bytes,
     created_at, expires_at)   -- expires_at = created_at + 1h (configurable)
```

**Matching** must happen in the data plane, early (`request_filter`),
because body capture has to begin before the body streams. So compiled
traps live in a **sibling `Arc<DashMap<TrapId, CompiledTrap>>`** (the
`KeyStoreCache` model) shared into the data plane — added/removed without
a `LiveConfig` snapshot rebuild. A request is matched against active
traps; on match, `Ctx` is flagged to buffer req/resp bodies up to
`max_body_bytes`, and the resulting `InspectRecord` (Tier 2) is routed to
the trap's capture store tagged with `trap_id`.

**Captures storage — recommended IN-MEMORY, ephemeral** (see §7-Q2): a
sibling `Arc<DashMap<TrapId, CaptureRing>>`, each ring capped by count
(e.g. 100) and by total bytes; entries carry `captured_at`. **Not written
to SQLite** — keeps debug bodies out of the persistent store and honours
"len na debug". A **cleanup job** (admin runtime, every ~1 min) drops
captures past TTL and disables/removes traps past `expires_at`.

**Endpoints** `…/traps` (CRUD on definitions) + `…/traps/{id}/captures`
(list) + `…/traps/{id}/captures/{cid}` (full inspect, reuses §3 view).
**Frontend**: a *Traps* screen (`PageEnvelope`) — define/arm traps, live
count of captures, click a capture → the §3 inspect panel.

## 6. Feature 4 — Rate-limit monitoring (30d) + alerting

**Extend `RateLimitStatsRegistry`** (don't replace): keep the 60 s
sparkline; add a rollup path.
- Data plane keeps calling `record(host_id, ip)` on 429 (unchanged, hot
  path stays one mutex push).
- Admin-runtime **rollup job** (every 1 min): drains/aggregates hits into
  **pre-aggregated SQLite buckets** (recommended over raw events — see
  §7-Q3) — `rate_limit_bucket(host_id, tenant_id, bucket_start_hour,
  client_ip, hits)`, keeping **top-N IPs per bucket** to cap cardinality
  under attack. A **retention job** deletes buckets older than 30 days.
- **Endpoints**: `…/rate-limit/timeseries?host=&from=&to=&bucket=hour`
  and `…/rate-limit/top-ips?window=`.
- **Frontend**: a rate-limit monitoring view (per host + tenant-wide):
  30-day 429 time series + top offending IPs table.

**Alerting** reuses the existing alerts subsystem
(`handlers/tenants/alerts`, `AlertDialog`). New rule type **"IP rate-limit
threshold"**: fires when an IP accumulates `> N` rate-limit hits within
window `W` (e.g. >500 / 10 min). Evaluated by the rollup job (it already
holds the aggregated counts) and delivered through the existing alert
channel — no new delivery plumbing.

## 6b. Feature 5 — Detailed audit history (what changed)

**Current state.** `audit_log` already stores a `payload` JSON with
`{before, after}`, and `audit.rs` exposes `record_create` (after only),
`record_delete`, and `record_update` (before+after). But `record_update`
is called **once** in the whole codebase (16× create, 9× delete, 1×
update) — so edits currently land as opaque "updated" rows with no diff.
`AuditPage.tsx` renders action/target only; it shows **no field-level
diff**. A diff renderer already exists for hosts
(`web/src/lib/diff/hostConfig.ts` + `components/diff/HostConfigDiff.tsx`).

**Backend.** Route every *mutation* handler through `record_update` (or
`record_create`/`record_delete`) with the real `before`/`after` DTO
snapshots — especially the edit paths (host, upstream, keystore entry,
cert, route, user, alert, …). Capture `before` by reading the entity
*before* the write. Stays best-effort (a failed audit insert never breaks
the request). No schema change — the `payload` column already holds it.

**Frontend.** `AuditPage` row → expandable **field-level diff** computed
from `payload.before` vs `payload.after`: changed keys with old→new,
additions, removals; redact sensitive values (tokens, secrets). Generalize
the existing host diff renderer into a payload-agnostic key/value diff
component reused across all target kinds.

This is small, backend-light, hot-path-free, and independently
shippable — a good early win (see P0 in §8).

## 7. Pivotal decisions — LOCKED (operator, 2026-05-23)

- **Q1 — Body capture: option A.** Bodies buffered **only for
  trap-matched requests** (+ optional per-host debug toggle),
  size-capped. The last-1000 ring carries headers+query, **no body**.
  Hot path stays clean.
- **Q2 — Trap captures: IN-MEMORY, ephemeral.** Captures live in a
  sibling `Arc<DashMap<TrapId, CaptureRing>>` with a 1h TTL; lost on
  restart; never written to SQLite (no debug-body DB bloat). **Trap
  definitions are still persisted** in SQLite so an armed trap survives
  a restart.
- **Q3 — Rate-limit history: pre-aggregated buckets, `10-minute`
  granularity** (operator chose 10 min over hourly for finer
  resolution / better alert sensitivity). Schema therefore keys on a
  `bucket_start_10min` boundary; keep **top-N IPs per bucket**; 30-day
  retention. Rollup job runs every ~1 min and accumulates into the
  current 10-min bucket (upsert).
- **Q4 — Build order: P1 → P2 → P3 → P4** (see §8). Value first,
  hot-path-touching traps last.

## 8. Suggested phasing (each independently shippable)

0. **P0 — Detailed audit diff (F5)**: route mutations through
   `record_update` with before/after; expandable field-level diff in
   `AuditPage`. Small, hot-path-free, no migration — good warm-up win.
1. **P1 — Tier-1 ring + inspect (headers/query)**: server ring, `recent`
   + `{id}` endpoints, inspect panel, pause on host tab, bump to 1000.
   *Delivers F1 (minus body) + F2.*
2. **P2 — Rate-limit 30-day monitoring**: rollup job, 10-min buckets
   table, timeseries/top-ips endpoints, monitoring view. *Delivers F4
   charts.*
3. **P3 — Alerting on frequent IP**: new alert rule type wired into the
   rollup. *Completes F4.*
4. **P4 — Request traps + body capture (Tier 2)**: trap CRUD + sibling
   cache + data-plane matching + body buffering + capture store + cleanup
   job + Traps screen. *Delivers F3 + bodies for F1.* (Largest; last
   because it touches the hot path most.)

Operator chose P1-first (§7-Q4); P0 (audit) added later as a small
parallel win — confirm whether to pull P0 ahead of P1.

## 9. Open risks

- Header capture cost at very high RPS — keep behind a toggle; benchmark
  against the ≤110% nginx p99 gate.
- Body buffering reuses the cache-capture machinery but must respect
  `max_body_bytes` and never block streaming on non-matching requests.
- Per-IP cardinality in rate-limit buckets under a botnet — top-N cap +
  an "(other)" rollup bucket.
- Auth-header values in captures are sensitive — redact/secure by
  default, gate the inspect view behind an elevated role.
