# BUILD PROMPT — proximiio.demon

> **How to use this.** Start a fresh Claude Code agent in the **new
> `proximiio.demon` repo**. Copy this whole `docs/proximiio-demon/` folder into that
> repo first, then paste the prompt below. The agent must read `00-README.md` and
> the three design docs (`01`/`02`/`03`) before writing any code — they are the
> binding spec; this prompt is the orchestration on top.

---

## PROMPT (paste into the demon agent)

You are building **proximiio.demon** from scratch: an always-on Rust DevOps
control-plane daemon — the single auditable brain for an entire in-house
infrastructure company. Read `docs/proximiio-demon/00-README.md` and the three
design docs (`01-core-architecture.md`, `02-provisioning-scaling-ops.md`,
`03-security-architecture.md`) in full before writing any code. They are the spec;
follow them. Where they conflict, security (`03`) wins.

### Non-negotiable principles (do not violate, ever)

1. **A brand-new DevOps admin must be able to do everything from inside this one
   tool** — both fleets (FreeBSD core + Linux enterprise per-tenant). Every capable
   action is discoverable and guided; nothing requires SSH-hopping or out-of-band
   scripts. Build the onboarding/runbook surface as a first-class feature, not an
   afterthought.
2. **Residency is a compile-time invariant.** EU and UAE are physically air-gapped
   and NEVER mixed. Model `Residency` as a type parameter (`Store<Eu>` /
   `Store<Uae>`); no API call may reference two regions; a runtime gate backstops the
   type proof. One daemon + DB + backup target per residency group.
3. **`demon-core` is pure (no I/O).** Parsers, reconcile, `authorize()`, audit,
   capacity math are unit-testable with zero side effects. All I/O (SSH, OpenSearch,
   vault, cloud providers, DB) sits behind traits in driver crates.
4. **Every mutation is gated:** `authorize(principal, ActionSpec) -> Capability`
   → plan → typed-confirm → dry-run → apply → `check-*.sh` verify → **hash-chained,
   redacted audit**. Redaction is mandatory: NEVER log a password, TOTP secret,
   signing key, or raw token. Secrets travel via env / in-memory, never argv, never
   logs. The worst ops (CA use, cross-residency, tenant decommission, mass token
   revoke) require **two-person dual-control** the same human cannot self-approve.
5. **demon is a broker, not a vault.** Exactly one host-bound long-lived secret
   (terrapi-vault auth). Everything else is short-TTL, in-memory, zeroized. SSH is
   short-lived cert-based — no static keys checked into the daemon.
6. **WG-only + mTLS.** The daemon listens only on WireGuard; sessions are bound to
   cert thumbprint + WG peer key + OIDC `sub`; destructive/secret actions need a
   touch-per-op step-up (the YubiKey factor is pluggable and OFF until hardware is in
   hand — design the `SecondFactor` trait + `factor_policy` table now, enforce later).

### Reuse, don't reinvent

The mature `proximiio-tui` (in the `proximiio-infra` repo) already has battle-tested:
agentless-SSH-over-WireGuard transport; host-side `check-<area>.sh` read-only
**line-contract** scripts → pure Rust parsers (malformed/empty → `Unknown`, never
panic); the typed-confirm + dry-run + hash-chained redacted audit mutation pipeline;
API clients (`kalista_api.rs`, `vulture_api.rs`, `identity_api.rs`); and the
`coordination/` file-based contract protocol. **Lift these into `demon-core` /
driver crates rather than rewriting.** Ask the infra agent (via `coordination/`) for
any contract you need.

### Architecture (per `01`)

- **Workspace crates:** `demon-core` (pure domain), `demon-store` (SQLite/WAL +
  migrations), `demon-collect` (SSH + line-contract collectors), `demon-workers`
  (supervised tokio pollers/schedulers/analyzer/reconciler), `demon-server` (Axum
  REST + WebSocket), `demon-bin` (the daemon), plus client crates `demon-tui` and
  `demon-sdk`.
- **API:** versioned REST + a WebSocket stream of live events/state. Resource model:
  `hosts`, `services`, `residency-groups`, **`tenants`**, `jobs`, `alerts`, `audit`,
  `identities`, `runbooks`. Use HATEOAS-lite `available_actions[]` so discoverability
  is structural.
- **State:** SQLite (WAL, migrations, retention) for inventory + current health +
  job/action history + audit chain + alert/runbook state. **Time-series metrics &
  logs → per-group OpenSearch**, not SQLite. HA: start single-node + Litestream;
  escalate to rqlite/libSQL standby only if SLA forces it; never multi-master (the
  audit chain must stay linearizable).
- **Reconciler:** detect/propose drift (desired vs observed), but apply ONLY via
  typed-confirm-gated jobs. No silent 3am auto-scaling; an explicit whitelist governs
  any opt-in auto-apply.

### Provisioning / scaling / ops (per `02`)

- **Hybrid orchestration:** a thin per-provider *birth* layer behind a `Provider`
  trait (Terraform-via-`.tf.json` for cloud; mfsBSD/PXE + ZFS golden image for
  FreeBSD metal; bastille/iocage for jails; cloud-init for Linux VMs) feeding the one
  uniform agentless-SSH *convergence* layer where the `check-*.sh` line-contracts ARE
  the config model. No persistent host agent (pull telemetry over SSH; revisit at
  scale).
- **Per-tenant lifecycle:** provision / upgrade / scale / decommission a Linux
  enterprise install with structural isolation (own WG IP, vault namespace,
  OpenSearch RBAC, kalista WAF profile, DB schema). Blue/green for stateful tiers
  (expand/contract migrations), rolling for stateless workers.
- **Bottleneck engine:** rules + confidence tiers with cross-tier correlation;
  per-service signals (CPU/mem/IO/disk, queue depth, p95, OpenSearch heap/shard/
  indexing rate, DB conns, worker backlog, WG throughput). Output = ranked findings,
  each linking one-click to a scaling action or a runbook.
- **Horizontal scaling:** capacity model computes N (never blind +1); modes =
  guided (default) / safe-auto (opt-in) / manual; guardrails = residency hard-gate,
  cost awareness, quiet-period/anti-flap, inverse-plan rollback.

### Security (per `03`)

OIDC AuthN via terrapi-identity + WebAuthn (pluggable YubiKey); capability-based
per-action AuthZ with residency/tenant scoping + dual-control; vault-brokered
short-lived creds + SSH certs; tamper-evident audit shipped per-group to OpenSearch
(`source=control-plane`) with off-box signed chain-head anchoring; cargo-deny/audit +
SBOM in CI. Map to OWASP ASVS 4.0 / NIST 800-63B. Treat the new admin as part of the
threat surface: a dangerous op must be *unreachable* except via a typed, role-checked,
residency-tagged path.

### Coordination protocol (mandatory)

This stack coordinates via **files, not human relay** (see `proximiio-infra/CLAUDE.md`
+ `coordination/`). Before building anything that touches a service boundary:
- read `coordination/CONTRACTS.md` + `coordination/conventions/`
  (jwt-claims, audit-event-schema, residency, ports-env);
- when you need a contract (e.g. the terrapi-identity OIDC **client** registration
  shape, the vault brokered-cred API, OpenSearch index/RBAC for `source=control-plane`),
  drop a note in that service's `coordination/inbox/<service>/` and proceed against
  the documented contract once answered;
- when you define a boundary demon owns, commit it to `coordination/` AND notify the
  affected inbox. Never put secrets or per-tenant data in `coordination/`.

### Build phases (each phase ends GREEN: `cargo build` + `clippy --all-targets`
pedantic + `cargo test`, and is committed before the next)

- **Phase 0 — skeleton:** workspace + crates; `demon-core` domain types incl. the
  `Residency` type param, `ActionSpec`/`Capability`, the audit chain; SQLite store +
  migrations; Axum server scaffolding (health + version); config; structured logging.
- **Phase 1 — read-only observability:** SSH collectors + lift the `check-*.sh`
  parsers; host/service/group/tenant inventory; health snapshots; REST + WS read API;
  ship `demon-tui` as a thin client to prove the API. *New admin can SEE everything.*
- **Phase 2 — auth + audit hardening:** OIDC login via terrapi-identity, sessions,
  capability AuthZ, RBAC + residency scoping, dual-control scaffold, the pluggable
  `SecondFactor`; audit shipping to OpenSearch. WG-only + mTLS listener.
- **Phase 3 — guarded actions + runbooks:** the mutation pipeline (plan/confirm/
  dry-run/apply/verify/audit); first-class guided runbooks (restart, cert rotate,
  backup, drain, pkg-update + checksum refresh); `available_actions[]`.
- **Phase 4 — provisioning:** `Provider` trait + the birth/convergence split; deploy
  a new FreeBSD core node and a new Linux per-tenant install end-to-end; vault
  secret bootstrap; WireGuard join; inventory registration.
- **Phase 5 — bottleneck engine + scaling:** signal collection, the rules/confidence
  analyzer, capacity models, the guarded horizontal-scaling actions (OpenSearch first,
  then kalista/vulture/sync/DB), guardrails + rollback.
- **Phase 6 — per-tenant lifecycle + reconciler:** upgrade/scale/decommission flows,
  blue/green + rolling, the drift reconciler (detect/propose/gated-apply).
- **Phase 7 — web UI** over the same API (Axum-served), and the YubiKey factor flipped
  to required once hardware is agreed.

### Working rules

- Keep `README.md` current as features land. Concise per-feature commits; commit each
  GREEN phase before moving on. Commit messages with apostrophes break heredocs —
  write the message to a temp file and `git commit -F`.
- Parsers never panic (malformed → `Unknown`); workers are supervised + restartable;
  failure is isolated per service. Dry-run everything dangerous first.
- Ask (via `coordination/`) instead of guessing a sibling service's contract.

### Open risks to track (from the design docs)

SQLite writer contention under fan-out; OpenSearch air-gap reachability per group;
daemon-auth bootstrap + break-glass; SSH fan-out concurrency at tenant scale;
reconciler blast radius; Litestream restore drills; contract/SDK version skew;
no-persistent-agent decision revisited at scale. Surface these in `docs/` and resolve
deliberately.

---

*End of prompt.*
