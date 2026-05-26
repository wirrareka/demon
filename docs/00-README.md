# proximiio.demon — design package

> **What this is.** Design home for **proximiio.demon**: an always-on Rust DevOps
> control-plane daemon that becomes the single auditable brain for the whole
> in-house infrastructure company. It is the next-generation evolution of
> `proximiio-tui` (the terminal control plane) — the TUI, a new web UI, and a CLI
> all collapse into thin clients over one daemon API.
>
> **demon is a SEPARATE project/repo.** These docs live here (in `proximiio-infra`)
> because that's where the lineage + reusable code + `coordination/` contracts are.
> When the build starts, copy this `docs/proximiio-demon/` folder into the new
> `proximiio.demon` repo as its `docs/`.

## North star

A **brand-new DevOps admin** can do *everything* they need from inside this one
tool — no SSH hopping, no ad-hoc scripts, no tribal knowledge. Two fleets, one tool:

1. **Core infra** — FreeBSD servers running our in-house services.
2. **Enterprise per-tenant installs** — dedicated per-customer Linux deployments,
   each with its own lifecycle / monitoring / upgrades.

The daemon does: monitoring, actions/mutations, backups, fleet overview, logging,
identity management, **deploying new servers**, **bottleneck evaluation**, and
**guided horizontal scaling** (spawn OpenSearch nodes when OpenSearch is the
bottleneck, etc.) — and, being a daemon, it keeps watching even when no UI is open.

## The stack it controls (all in-house, HW → worker jobs)

| Service | Role |
|---|---|
| **terrapi vault** | secrets manager (brand=terrapi) |
| **kalista** | reverse proxy + WAF (JWT verifier + opaque KeyStore tokens) |
| **vulture** | main API (opaque DB tokens; super-admin cookie+CSRF admin plane) |
| **proximiio-sync** | data sync service |
| **terrapi-identity** | Rust/Axum JWT issuer / IdP / SSO broker (customers may BYO SSO) |
| **OpenSearch** | audit / logs / search — one instance **per residency group** |
| **proximiio-lib** | mobile SDKs |

Hosts: **FreeBSD (core)** + **Linux (enterprise per-tenant)**. Connectivity over
**WireGuard**. **Residency: EU + UAE, physically air-gapped, NEVER mixed.**

## The documents

| Doc | Pillar | Author agent |
|---|---|---|
| [`01-core-architecture.md`](01-core-architecture.md) | Daemon vs TUI, workspace/crates, Axum REST+WS API, SQLite schema, workers, reconciler, multi-tenant, onboarding surface | backend-developer |
| [`02-provisioning-scaling-ops.md`](02-provisioning-scaling-ops.md) | Deploy-a-server, per-tenant lifecycle, bottleneck engine, horizontal scaling, guardrails, runbooks, provider abstraction | devops-engineer |
| [`03-security-architecture.md`](03-security-architecture.md) | AuthN (OIDC + WebAuthn/YubiKey), capability AuthZ + dual-control, vault-brokered creds, audit chain, hardening, type-level residency, threat model | security-engineer |
| [`BUILD-PROMPT.md`](BUILD-PROMPT.md) | **The master prompt to hand a fresh demon Claude Code agent** | synthesis |

## The few decisions that bind all three docs

1. **Daemon owns state-over-time; clients are dumb renderers** of `demon-core` view
   models. Reuse the TUI's `check-*.sh` line-contract parsers, the typed-confirm +
   dry-run + hash-chained redacted audit mutation pipeline, and `coordination/`.
2. **`demon-core` is pure (no I/O)** — parsers / reconcile / authorize / audit are
   unit-testable; drivers (SSH, OpenSearch, vault, providers) sit behind traits.
3. **SQLite (WAL) for current/health state; OpenSearch for time-series** metrics &
   logs. Don't abuse SQLite as a metrics store.
4. **One daemon + DB + backup target per residency group.** EU/UAE share nothing.
   Residency is a **compile-time type invariant** (`Store<Eu>` vs `Store<Uae>`),
   backstopped by a runtime gate. No API accepts two regions.
5. **Provisioning = thin per-provider "birth" + one uniform agentless-SSH
   "convergence" layer.** The `check-*.sh` line-contracts ARE the config model
   (no Ansible). Bottleneck detection = rules + confidence tiers (not ML); scaling
   computes N from a capacity model (never blind +1); every mutation is
   plan → typed-confirm → apply → `check-*.sh` verify → audit, with an inverse-plan
   rollback.
6. **demon is a broker, not a vault** — exactly one host-bound long-lived secret
   (vault auth); everything else is short-TTL, in-memory, zeroized, never logged.
   The only execution path to a dangerous op is a `Capability` minted by a central
   `authorize(principal, ActionSpec)`; the worst ops need two-person dual-control.

## Coordination note

The security doc touches the **audit-event schema** (`source=control-plane`) and a
new **OIDC client contract** with terrapi-identity. Per `CLAUDE.md`, when the build
phase begins these must be committed to `coordination/` and noted in the
`identity/` and `vault/` inboxes — not relayed by hand.
