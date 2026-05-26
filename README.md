# proximiio.demon

An always-on **Rust/Axum DevOps control-plane daemon** — the single auditable brain for
the EU+UAE in-house infrastructure fleet (FreeBSD core + Linux per-tenant). Next-gen of
`proximiio-tui`: a TUI, a web UI, and a CLI all collapse into thin clients over one
daemon API.

> Design docs live in [`docs/`](docs/) (architecture, provisioning, security,
> collaboration protocol) and [`docs/planning/`](docs/planning/) (build plan, reuse map,
> contracts, UI spec). The web UI follows the kalista v0.5 design system in
> [`docs/design/`](docs/design/). Where docs conflict, **security wins**.

## Non-negotiables

- **Residency is a compile-time invariant** — `Store<Eu>` / `Store<Uae>` are distinct
  types; no API references two regions; a runtime gate + token-residency check backstop it.
- **`demon-core` is pure** (no I/O) — parsers, `authorize()`, audit chain, capacity math,
  reconcile are deterministic + unit-tested; all I/O sits behind driver-crate traits.
- **Every mutation is gated**: `authorize → plan → typed-confirm → (dual-control) →
  step-up → dry-run → apply → check-*.sh verify → hash-chained redacted audit`. The worst
  classes (destructive / CA / cross-residency) require two-person dual-control.
- **Broker, not a vault** — one host-bound secret; everything else short-TTL/zeroized.
- **WG-only + mTLS**, fail-closed API, touch-per-op WebAuthn step-up.

## Workspace

| Crate | Role |
|---|---|
| `demon-core` | Pure domain: residency, authz/capabilities, audit chain, `check-*` parsers, mutation pipeline, runbooks, bottleneck analyzer, reconciler |
| `demon-store` | SQLite (WAL) `Store<R>` + migrations; durable hash-chained audit |
| `demon-collect` | SSH-over-WireGuard transport + line-contract collectors + `CheckArea` |
| `demon-workers` | Supervised tokio poller + mutation executor |
| `demon-clients` | identity (OIDC), OpenSearch (audit), Prometheus (metrics) clients |
| `demon-server` | Axum REST + WS, OIDC login, sessions, RBAC, mTLS, WebAuthn step-up, jobs/runbooks/audit API |
| `demon-bin` | The daemon (`demon`) |
| `demon-sdk` / `demon-tui` | Typed client + thin terminal client |
| `web/` | React + Vite + Tailwind + shadcn UI (Fleet / Jobs / Runbooks / Audit / Tenants) |

## Quick start (local dev)

```bash
# build
cargo build -p demon-bin -p demon-tui

# seed a demo fleet + run the daemon (dev mode bypasses auth — DEV ONLY)
DEMON_RESIDENCY=eu DEMON_DB_PATH=/tmp/demon.db ./target/debug/demon seed-demo
DEMON_RESIDENCY=eu DEMON_DEV_NO_AUTH=1 DEMON_BIND=127.0.0.1:8787 \
  DEMON_DB_PATH=/tmp/demon.db ./target/debug/demon

# TUI thin client
DEMON_API=http://127.0.0.1:8787 ./target/debug/demon-tui

# web UI (proxies the API to the daemon)
DEMON_DEV_API=http://127.0.0.1:8787 pnpm --dir web dev   # http://localhost:5173
```

Health/version are public; everything under `/api/v1` is fail-closed (401) unless an
operator is logged in via OIDC — or `DEMON_DEV_NO_AUTH=1` is set for local trials.

## Configuration (env)

| Var | Default | Purpose |
|---|---|---|
| `DEMON_RESIDENCY` | — (required) | `eu` or `uae` |
| `DEMON_BIND` | `127.0.0.1:8787` | listen addr (WG interface in prod) |
| `DEMON_DB_PATH` | `demon-<region>.db` | SQLite path |
| `DEMON_SSH_USER` / `DEMON_POLL_SECS` | `ops` / `60` | collector SSH user / poll interval |
| `DEMON_OIDC_ISSUER` / `_CLIENT_ID` / `_CLIENT_SECRET` / `_REDIRECT_URI` | — | terrapi-identity OIDC client |
| `DEMON_TLS_CERT` / `_KEY` / `_CLIENT_CA` | — | mTLS (all three ⇒ mTLS listener) |
| `DEMON_OPENSEARCH_URL` | — | B3 audit fan-out target (group-local) |
| `DEMON_PROMETHEUS_URL` | — | bottleneck load feed (group-local) |
| `DEMON_WEBAUTHN_RP_ID` / `_ORIGIN` | — | WebAuthn step-up relying party |
| `DEMON_NODE` | `demon-<region>` | audit `node` field (prefer host fqdn) |
| `DEMON_DEV_NO_AUTH` | unset | **DEV ONLY** — bypass the auth gate |

## CI

`.github/workflows/ci.yml` gates every push: `cargo fmt --check`, `clippy --all-targets
-D warnings` (pedantic), `cargo test`, `cargo-deny` (advisories/licenses/bans/sources),
and the web typecheck + build.

## Cross-service coordination

demon integrates with identity / vault / kalista / vulture / infra via the file-based
`coordination/` protocol (see `docs/04-collaboration-protocol.md`). It reads
`coordination/inbox/demon/` on start and routes contract asks through neighbours' inboxes.
