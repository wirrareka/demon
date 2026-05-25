# 01 — proximiio-tui → proximiio.demon Reuse Map

Status: planning (read-only research). Date: 2026-05-26.
Source under analysis: `proximiio-infra/tui/proximiio-tui/src/` (60+ files) +
host-side `proximiio-infra/scripts/check-*.sh` and `*/scripts/check-encryption.sh`.

Target crates (per `01-core-architecture.md` §2):
`demon-core` (PURE, no I/O), `demon-store` (SQLite/WAL), `demon-collect`
(SSH-over-WG + line-contract wiring + API clients), `demon-workers` (tokio),
`demon-server` (Axum), `demon-bin`, `clients/demon-tui`, `clients/demon-sdk`.

Non-negotiables touched (`BUILD-PROMPT.md` §"Non-negotiable"):
- **PURITY** — `demon-core` has zero I/O; parsers/reconcile/authorize/audit/capacity testable.
- **RESIDENCY** — `Residency` is a *compile-time type param* (`Store<Eu>/Store<Uae>`), runtime gate backstop.
- **MUTATION GATING** — `authorize → plan → typed-confirm → dry-run → apply → verify → hash-chained redacted audit`.
- **BROKER-NOT-VAULT** — one host-bound long-lived secret; everything else short-TTL, in-memory, zeroized; SSH = short cert.

Verdict legend: **LIFT-AS-IS** | **ADAPT** (split pure↔driver / refactor) | **REWRITE** | **DROP**.

---

## A. The mutation pipeline (highest-value reuse — `BUILD-PROMPT` calls this out by name)

| Module | What it does | Target crate | Verdict | Non-negotiable |
|---|---|---|---|---|
| `ssh.rs` (932 L, 34 I/O, 13 tests) | Pure guards (`strip_sudo`, `ensure_read_only`, `ensure_whitelisted_mutation`, `resolve`, `display_cmd`) **+** tokio `Command` exec (`spawn_read_only`, `spawn_mutation`, `spawn_text_fetch`); honours dry-run, exports token via **env not argv**. | **SPLIT**: guards/argv-builders → `demon-core::ssh_guard`; exec/`Identity` → `demon-collect::ssh` | **ADAPT** | MUTATION GATING (guard+dry-run is the gate's first leg), BROKER (env-not-argv secret rule) |
| `audit.rs` (407 L) | Append-only **hash chain**: `prev_hash`, `entry_hash = sha256(prev_hash‖canonical)`, `verify_chain`, legacy-line tolerance. Pure chain math + file append. | chain math → `demon-core::audit`; append/persist → `demon-store` (the `audit(seq,prev_hash,hash,…)` table) | **ADAPT** | MUTATION GATING (the audit leg); RESIDENCY (add `residency`+`tenant` columns per `03` §4) |
| `seq.rs` (163 L, 0 I/O, 3 tests) | Strict await-between-legs sequence scheduler (ordered legs, each unblocks the next). Pure. | `demon-core` (reuse for runbook step ordering) | **LIFT-AS-IS** | — |
| `redact.rs` (148 L, 0 I/O, 5 tests) | Central secret redactor: masks WG keys, PEM blocks, GitHub PAT, S3 keys, `SECRET_KEYS` set. Pure `line()`/`blob()`. | `demon-core::redact` | **LIFT-AS-IS** then **EXTEND** the NEVER set (`03` §3: + vault secret-id, SSH cert priv half, OIDC client secret, session id) | MUTATION GATING, BROKER |
| `workflow.rs` (253 L) | Typed step state-machine: `Step{needs,kind,status}`, runnable only when deps `Done`, `verify` gate hook. Mostly pure (4 I/O = capture plumbing). | engine → `demon-core::runbook`; any spawn → `demon-workers` | **ADAPT** (strip I/O; becomes the `/runbooks` engine, `01` §8) | MUTATION GATING (verify hook = the gated apply) |
| `workflows.rs` (1300 L, 0 I/O, 24 tests) | Declarative YAML loader → typed `workflow` engine (`add-region.yml` etc). Pure parse. | `demon-core::runbook::loader` | **LIFT-AS-IS** (re-point to demon runbook YAMLs: onboard-tenant, scale-opensearch…) | onboarding non-negotiable (#1) |

---

## B. check-*.sh line-contracts + their pure Rust parsers

Scripts are **read-only, stable tab-separated contracts** (verb-prefixed lines,
malformed/empty → graceful). Doc `01` §1 says: scripts move **as-is** into the
SSH layer; parsers migrate **verbatim** into `demon-core`. Confirmed: every parser
below is `io=0` with heavy tests.

| Script (verbs) | TUI parser | Target | Verdict | Notes |
|---|---|---|---|---|
| `check-access.sh` (ACCESS, WHEEL) | `access.rs` (0 I/O, 9 tests) | parser→`demon-core::contracts::access`; script→`demon-collect/contracts/` | **LIFT-AS-IS** | |
| `check-audit.sh` (AUDIT, SECEVENT) | `audit_host.rs` (0 I/O, 9 tests) — host IDS posture (distinct from `audit.rs` chain) | `demon-core::contracts::audit_host` | **LIFT-AS-IS** | rename to avoid clash with audit-chain |
| `check-backup.sh` (BACKUP, STORE) | `backup.rs` (0 I/O, 10 tests) | `demon-core::contracts::backup` | **LIFT-AS-IS** | feeds Litestream-verify scheduler |
| `check-compliance.sh` (COMPLIANCE, CHECK) | `compliance.rs` (4 I/O, 11 tests) — CIS + auditor export | parser→`demon-core`; export-write→`demon-server`/`demon-store` | **ADAPT** | |
| `check-drift.sh` (DIRTY, DRIFT) | `drift.rs` (0 I/O, 10 tests) — git repo-is-truth | `demon-core::contracts::drift` | **LIFT-AS-IS** | distinct from §C reconciler-drift |
| `check-fim.sh` (DRIFT, FIM) | `fim.rs` (0 I/O, 11 tests) | `demon-core::contracts::fim` | **LIFT-AS-IS** | |
| `check-os.sh` (OS) | `os.rs` (0 I/O, 7 tests) — cross-platform OS seam | `demon-core::contracts::os` | **LIFT-AS-IS** | critical for FreeBSD+Linux dual-fleet |
| `check-residency.sh` (RESIDENCY, CROSS, CROSSPEER) | `residency.rs` (0 I/O, 11 tests) — EU/UAE air-gap proof, cross-group-peer = Violation | parser→`demon-core::contracts::residency` | **ADAPT** | **RESIDENCY**: keep runtime `Region` enum from the contract, BUT it must populate the type-param `Residency<R>` invariant + the runtime-gate backstop (`03` §6). This is the verify-side of the invariant. |
| `check-encryption.sh` (per-module, ×6 hosts) | `dataprotect.rs` (0 I/O, 6 tests) | `demon-core::contracts::dataprotect`; scripts→per-module collector dir | **LIFT-AS-IS** | |

---

## C. API clients (`BUILD-PROMPT` §"Reuse, don't reinvent")

All three are **already cleanly split**: pure typed models + request-*builders*
("No ssh, no network, no process spawning" — they hand built argv to
`ssh::spawn_text_fetch`/`spawn_mutation` + redactor). This is exactly the
demon-core↔driver seam.

| Module | What it does | Target | Verdict | Non-negotiable |
|---|---|---|---|---|
| `kalista_api.rs` (3250 L, 66 parse, 46 tests) | Kalista admin-API: typed models + ssh→curl request builders; token via env. | models+builders→`demon-core::clients::kalista`; transport wiring→`demon-collect` | **ADAPT** (mechanical split along the existing seam) | BROKER (env token), MUTATION GATING (POST/PUT/DELETE route via `spawn_mutation`) |
| `vulture_api.rs` (1625 L, 31 parse, 33 tests) | Vulture admin-API + cross-region check; session via env cookie. | same split | **ADAPT** | RESIDENCY (cross-region check), BROKER |
| `identity_api.rs` (871 L, 15 parse, 17 tests) | terrapi-identity (OIDC IdP) client + rotation-posture verdict; session/CSRF via env. | same split | **ADAPT** — becomes the seed for the OIDC **confidential-client** auth in `03` §1; expand to PKCE/session-binding | AuthN (OIDC broker), BROKER |
| `agent_client.rs` (638 L) | Thin client for optional read-only `infra-agent`. | `demon-collect` | **ADAPT / defer** | doc `02` "no persistent agent (pull over SSH)" — keep behind a feature flag; revisit at scale |
| `ws.rs` (495 L, 8 I/O) | Client-side WebSocket tail of `/ws/waf-events` over WG. | **INVERT**: demon is now the *server*; lift framing/topic ideas into `demon-server::stream`, the tail becomes a `demon-tui`/`demon-sdk` consumer | **REWRITE** (role flip: client→server push) | — |

---

## D. Audit shipping / B3 cross-system schema

| Module | What it does | Target | Verdict | Non-negotiable |
|---|---|---|---|---|
| `audit_emit.rs` (610 L, 4 I/O, 11 tests) | Ships TUI operator audit entries to per-group OpenSearch `audit-events-{group}-YYYY.MM` in canonical B3 schema. | emit→`demon-collect`/`demon-workers` (the audit-shipping scheduler); schema struct→`demon-core` | **ADAPT** | MUTATION GATING, RESIDENCY (per-group index, never cross-shipped — `03` §4) |
| `audit_events.rs` (932 L, 0 I/O, 16 tests) | Unified B3 audit read-model + OpenSearch `_search` parser (one parser over all three emitters). | parser→`demon-core`; `_search` client→`demon-collect` | **ADAPT** (split parse vs http) | RESIDENCY |

---

## E. Inventory / posture / ops models

| Module | What it does | Target | Verdict | Non-negotiable |
|---|---|---|---|---|
| `inventory.rs` (439 L, 2 I/O) | TUI-owned `inventory.yml` "which module runs where". | model→`demon-core`; persistence→`demon-store` `hosts`/`services` tables | **ADAPT** (file→SQLite; add `fleet`, `tenant_id`, `residency_group`) | RESIDENCY, multi-fleet (`01` §6) |
| `peers.rs` (264 L, 2 I/O) | Parser for WireGuard `peers.yml` mesh source-of-truth. | `demon-core::contracts::peers` | **ADAPT** (small) | RESIDENCY (cross-group peer detection feeds the air-gap gate) |
| `rotation.rs` (451 L, 2 I/O, 11 tests) | Secret-rotation SLO age tracking (control-plane owns age, not hosts). | logic→`demon-core::capacity`/scheduler model; state→`demon-store` | **ADAPT** | BROKER (rotation cadence), MUTATION GATING (rotate = gated job) |
| `provenance.rs` (449 L, 4 I/O, 12 tests) | Supply-chain SHA256 (signature-ready) artifact gate. | verify math→`demon-core`; fetch→`demon-collect` | **ADAPT** | supply-chain (`03` §5) |
| `secretlint.rs` (244 L, 0 I/O, 8 tests) | Inline secret-scan lint over a conf before render. | `demon-core` | **LIFT-AS-IS** | MUTATION GATING (pre-apply lint), BROKER |
| `secrets.rs` (206 L, 0 I/O, 5 tests) | macOS Keychain-backed secret source, print-once capture. | concept→`demon-core::Secret<T>` (zeroize, no Debug/Display/Serialize per `03` §3) | **REWRITE** | BROKER — daemon must NOT use macOS Keychain; brokers from terrapi-vault, in-memory only |
| `conf.rs` (744 L) | Sourced-shell `KEY="value"` per-host conf editor model. | model→`demon-core`; apply→`demon-collect` mutation | **ADAPT** | MUTATION GATING |
| `alerts.rs` (435 L, 0 I/O, 9 tests) | Alertmanager read/act model. | model→`demon-core`; over time supersede with demon's own `alerts` table + bottleneck analyzer | **ADAPT / partial-DROP** | — (demon owns alerts natively, `01` §5) |
| `scrape.rs` (208 L) | Round-trips `scrape-targets.yml`. | `demon-core` if monitoring retained | **ADAPT / defer** | — |
| `dns.rs` (1309 L) | Route53 model + change-batch + residency guardrail. | model+diff→`demon-core`; apply→`demon-collect`; or move to `Provider` trait | **ADAPT** | RESIDENCY (DNS residency guardrail), MUTATION GATING |
| `compliance.rs`, `dataprotect.rs` | (see §B) | | | |

---

## F. TUI runtime / client-only (do NOT pull into the daemon)

| Module | What it does | Verdict | Notes |
|---|---|---|---|
| `app.rs` (1367 L) | Single owned `App` rebuilt per frame. | **REWRITE** in `clients/demon-tui` | render server-pushed state, not SSH polls (`01` §1) |
| `update.rs` (6696 L) | The single reducer. | **REWRITE** in `clients/demon-tui` | huge; thin-client reducer over WS deltas + POST intents |
| `event.rs` (207 L) | `Msg` plumbing. | **ADAPT** into `demon-tui` | |
| `term.rs` (66 L) | PTY suspend/restore for interactive ssh/console. | **LIFT-AS-IS** into `demon-tui` | client-only |
| `cli.rs` (112 L) | CLI front-end + completions. | **ADAPT** → thin client over `demon-sdk` | |
| `config.rs` (247 L) | `~/.config/proximiio-tui/` paths. | **REWRITE** | daemon config is env/file per `03`; client keeps its own |
| `main.rs` (215 L) | TUI boot. | **DROP** (replaced by `demon-bin` + `demon-tui` main) | |
| `ui/` (dir) | ratatui views. | **REWRITE/port** into `demon-tui` | |
| `modules/*.rs` (autossh, bootstrap, dashboards, kalista, monitoring, nats, opensearch, pki, rethinkdb, runners, vm, vulture, wireguard) | Per-subsystem TUI panes/action wiring. | **ADAPT** — extract any pure action-spec/parsers into `demon-core`; the pane UIs go to `demon-tui`; the *actions* become `available_actions[]` + runbook steps in `demon-server` | onboarding non-negotiable (#1); MUTATION GATING |

---

## G. Gaps — must be NEW in demon (no TUI ancestor)

- **`Residency<R>` type-param machinery** (`Store<Eu>/Store<Uae>`, `Capability<R>`, `VaultClient<R>`, `AuditSink<R>`) — `residency.rs` only gives a runtime enum; the compile-time invariant is greenfield (`03` §6).
- **`authorize(principal, ActionSpec) -> Capability`** capability AuthZ + RBAC roles + dual-control two-person rule (`03` §2).
- **`SecondFactor` trait + `factor_policy` table** (WebAuthn now, YubiKey later) (`03` §1.2).
- **OIDC confidential-client + server-side session binding** (mTLS thumbprint + WG key + sub) — seeded from `identity_api.rs` but mostly new (`03` §1).
- **SQLite/WAL store + migrations + repositories** (`demon-store`) (`01` §4).
- **Axum REST+WS server, RFC-7807, cursor pagination, idempotency, HATEOAS `available_actions[]`** (`demon-server`) (`01` §3).
- **Supervised tokio worker tree, bottleneck analyzer, reconcile loop, event bus** (`demon-workers`) (`01` §5/§7).
- **`Provider` trait birth-layer** (Terraform/.tf.json, mfsBSD/PXE, bastille, cloud-init) (`02`).
- **`demon-sdk`** typed client (`01` §2).

---

## H. Summary verdict counts

- **LIFT-AS-IS:** `seq`, `redact`, `workflows`, `secretlint` + 6 contract parsers (`access`, `audit_host`, `backup`, `drift`, `fim`, `os`) + `dataprotect` + `term`(client) = ~13 modules.
- **ADAPT (split pure↔driver / refactor):** `ssh`, `audit`(chain), `workflow`, all 3 api clients, `audit_emit`, `audit_events`, `inventory`, `peers`, `rotation`, `provenance`, `conf`, `dns`, `compliance`, `residency`, `agent_client`, `modules/*`.
- **REWRITE:** `app`, `update`, `ws`(role-flip), `secrets`(keychain→vault broker), `config`, `ui/`.
- **DROP:** `main` (TUI boot), `alerts` (partial — superseded natively).
