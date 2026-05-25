# proximiio.demon — Master Build Plan & Phasing

> Synthesis plan for building **proximiio.demon**: an always-on Rust/Axum DevOps
> control-plane daemon — the single auditable brain for the EU+UAE in-house infra
> fleet (FreeBSD core + Linux per-tenant). Next-gen of `proximiio-tui`; the TUI, a
> new web UI, and a CLI collapse into thin clients over one daemon API.
>
> Authoritative spec: `../proximiio-demon/*` design docs (copied from
> `proximiio-infra/docs/proximiio-demon/`). Where docs conflict, **security (03) wins.**
> Companion planning docs in this folder:
> - `01-tui-reuse-map.md` — module-by-module lift/adapt/rewrite verdicts
> - `02-auth-secrets-contracts.md` — identity + vault contract answers (Phase 2/4 inputs)
> - `03-ui-design-system.md` — binding UI design system (kalista v0.5 spec)

---

## 0. Scope of this document

This is a **plan only** — no code is produced here. It maps the design docs +
reuse inventory + cross-service contracts + UI spec into an actionable, phased
build with explicit exit criteria, dependencies, and a risk register.

---

## 1. Non-negotiables (carried into every phase)

1. **One tool does everything** — both fleets; onboarding/runbooks are first-class.
2. **Residency is a compile-time invariant** — `Store<Eu>`/`Store<Uae>`, `Capability<R>`;
   no API references two regions; runtime gate backstops the type proof. One daemon +
   DB + backup target per group.
3. **`demon-core` is pure (no I/O)** — parsers/reconcile/`authorize()`/audit/capacity
   are unit-testable; all I/O behind driver traits.
4. **Every mutation gated**: `authorize(principal, ActionSpec) -> Capability` → plan →
   typed-confirm → dry-run → apply → `check-*.sh` verify → hash-chained redacted audit.
   Worst ops (CA use, cross-residency, tenant decommission, mass revoke) need 2-person
   dual-control (no self-approval).
5. **demon is a broker, not a vault** — exactly one host-bound long-lived secret;
   everything else short-TTL, in-memory, zeroized, never logged. **⚠ See §3 blocker.**
6. **WG-only + mTLS** listener; sessions bound to cert thumbprint + WG peer key + OIDC
   `sub`; touch-per-op step-up for destructive/secret actions (`SecondFactor` trait +
   `factor_policy` table now, enforce hardware later).
7. **Every phase ends GREEN**: `cargo build` + `clippy --all-targets` (pedantic) +
   `cargo test`, committed before the next phase begins.

---

## 2. Workspace layout (target)

```
demon-core      pure domain: Residency<R>, ActionSpec/Capability, authorize(),
                check-*.sh parsers, audit chain, reconcile, capacity math   (no I/O)
demon-store     SQLite (WAL) + migrations + retention; Store<Eu>/Store<Uae>
demon-collect   SSH-over-WireGuard transport + line-contract collectors + check-*.sh
demon-clients   kalista/vulture/identity (+ vault?) typed API clients
demon-workers   supervised tokio pollers / schedulers / analyzer / reconciler
demon-server    Axum REST + WebSocket, authn/z, mTLS listener, OpenAPI
demon-bin       the daemon
demon-sdk       single typed client source (web UI consumes OpenAPI-gen types)
demon-tui       thin TUI client (proves the API in Phase 1)
web/            React + Vite + Tailwind + shadcn thin client (Phase 7)
```

---

## 3. ⚠ Critical findings that reshape the plan

### 3.1 BLOCKER — terrapi-vault is not a network secrets broker
`02-auth-secrets-contracts.md` confirms: `terrapi-vault` is an **embedded SQLCipher
at-rest library** (memento/probe), **not** a network service. It provides **no** daemon
auth, **no** SSH CA / signed certs, **no** dynamic short-TTL creds, **no** lease model,
**no** namespaces, **no** audit emission. Demon's entire "broker, not a vault" secrets
story (Phase 2 daemon→vault auth, Phase 4 SSH-cert provisioning) has **no backing
service today**.

**Impact:** Phases 2 and 4 cannot be built against vault as specced. **Action:**
escalate to the vault owner via the answered inbox note — demon needs one of:
(a) vault grows a network broker surface, (b) demon targets a different broker, or
(c) an interim host-bound secret + manual SSH-cert issuance is accepted. Until
resolved, Phase 2 ships **OIDC auth only**; the secrets-broker slice is deferred to a
new **Phase 2b** gated on the vault decision. This is the #1 sequencing risk.

### 3.2 Identity gaps (confirmed contract, with caveats)
CONFIRMED: admin-API client registration; operator-supplied Argon2id-hashed secret;
exact-match `redirect_uris`; mandatory **PKCE-S256**; **Auth-Code + PKCE** flow;
`client_secret_basic`; claims `sub / tenant_id / residency_group / roles / scope`
(**no `email`, no `groups`**); identity owns login-time MFA (TOTP); **one confidential
client per locked per-group issuer** (`identity-eu.proximi.io` / `identity-uae.proximi.io`).
OPEN follow-ups: (1) **no `private_key_jwt`/mTLS** client auth — demon must accept
`client_secret_basic` for now (the one long-lived secret is now the OIDC client secret,
not vault auth — reconcile with §3.1); (2) **no `acr`/`amr` claim and no WebAuthn factor
in identity** → **demon owns its own per-op step-up** (the `SecondFactor` trait is not
optional — it is the only step-up mechanism). AuthZ maps `roles` claim → operator role.

---

## 4. Reuse strategy (from `01-tui-reuse-map.md`)

- **Free wins (LIFT-AS-IS, ~13):** all 8 `check-*.sh` line-contracts + their already-pure
  Rust parsers → `demon-core::contracts`; scripts → `demon-collect`. The audit hash
  chain (`audit.rs`), `redact.rs`, `seq.rs`.
- **Mechanical ADAPT (~17):** `kalista/vulture/identity_api` clients (already split
  pure-models vs transport, tokens via env) → `demon-clients`; `ssh.rs` guards
  (`ensure_read_only`/`ensure_whitelisted_mutation`/dry-run) split into core(logic)+
  collect(I/O); `workflow*.rs` mutation engine; `inventory.rs`, `drift.rs`, `alerts.rs`,
  `backup.rs`, `rotation.rs`, `scrape.rs`.
- **REWRITE (~6):** `residency.rs` (runtime enum → compile-time `Residency<R>` type-param —
  hardest non-negotiable, greenfield); `secrets.rs` (macOS Keychain → vault-brokered
  zeroized `Secret<T>` — **pending §3.1**); `ws.rs` (client-tail → server-push);
  `app.rs`/`update.rs`/`ui/` → thin `demon-tui`.
- **DROP (2):** TUI-only glue with no daemon analogue.

---

## 5. UI design binding (from `03-ui-design-system.md`)

Demon's web UI (and TUI styling where applicable) **must match the kalista v0.5 design
spec**. Copy `kalista/web/src/styles/tokens.css` verbatim (OKLCH vars, `data-theme`/
`data-accent`/`data-density`, default dark/mono/compact); mirror `components.json`
(`new-york`, `baseColor neutral`, `cssVariables true`, lucide) and the `tailwind.config.ts`
var-rewire. Reuse AppShell+Sidebar+Cmd-K, TanStack tables, `useConfirm` dialog, `.pill`/
`.dot` status badges, Toaster, HostConfigDiff. **Demon deltas:** typed-confirm
dual-control modal, dry-run diff view, EU/UAE residency switcher, WS live-state
indicators, canonical `lib/health.ts` (up/degraded/down/unknown/pending), fleet stat tiles.

---

## 6. Phased build

Each phase: **Goal → Key tasks → Reuses → Coordination/deps → Exit (GREEN + demo).**

### Phase 0 — Skeleton
- **Goal:** compiling workspace, pure domain core, store, server scaffold.
- **Tasks:** cargo workspace + all crates; `demon-core` domain types incl. the
  `Residency<R>` type-param, `ActionSpec`/`Capability<R>`, the audit-chain type; SQLite
  store + migrations; Axum scaffold (`/health`, `/version`); config loader; structured
  logging; `cargo deny` + `cargo audit` + SBOM in CI; copy design docs into repo `docs/`.
- **Reuses:** audit chain, redact, seq (lift); residency rewrite starts here.
- **Exit:** GREEN; `demon-bin` serves health/version; `Residency<R>` proven by a
  compile-fail test (cross-region call must not type-check).

### Phase 1 — Read-only observability  *(New admin can SEE everything)*
- **Goal:** full inventory + health, read API, thin TUI proves it.
- **Tasks:** SSH-over-WG transport; lift `check-*.sh` parsers; host/service/group/tenant
  inventory; health snapshots; REST + WS **read** API with HATEOAS-lite
  `available_actions[]`; `demon-tui` renders server-pushed state.
- **Reuses:** check-* parsers (lift), ssh.rs (adapt, read-only path), inventory/scrape,
  api clients (read paths), ws.rs (rewrite → server-push).
- **Coordination:** confirm per-group OpenSearch reachability + define
  `coordination/conventions/metrics.md` (rollup/query contract).
- **Exit:** GREEN; daemon polls a real (or mock) host set; TUI shows live fleet health.

### Phase 2 — AuthN + audit hardening  *(OIDC only — see §3.1)*
- **Goal:** operator login + RBAC + audit shipping + hardened listener.
- **Tasks:** OIDC Auth-Code+PKCE-S256 against per-group issuer, `client_secret_basic`,
  one client per residency; session model (cert thumbprint + WG peer + `sub`);
  capability AuthZ from `roles` claim, residency scoping; **`SecondFactor` trait +
  `factor_policy` table** (demon-owned step-up, since identity has no acr/amr);
  dual-control scaffold; audit shipping to per-group OpenSearch (`source=control-plane`);
  WG-only + mTLS listener; CSRF/CORS/CSP/rate-limit hardening.
- **Coordination:** identity note is **answered** — register one client per group;
  record demon's `source=control-plane` audit usage + OIDC client contract back into
  `coordination/`.
- **Exit:** GREEN; operator logs in via identity; every read is role+residency gated and
  audited to OpenSearch.

### Phase 2b — Secrets broker  *(GATED on §3.1 vault decision — may slip)*
- **Goal:** the broker-not-vault secrets layer.
- **Tasks:** daemon→broker auth (one host-bound secret); short-TTL in-memory zeroized
  `Secret<T>`; SSH signed-cert issuance; lease model bound to operator session.
- **Blocker:** no backing service today. **Do not start until the vault owner answers**
  the (answered/partial) inbox note with a concrete broker surface or an accepted interim.

### Phase 3 — Guarded actions + runbooks
- **Goal:** the full mutation pipeline + first-class guided runbooks.
- **Tasks:** plan/confirm/dry-run/apply/`check-*` verify/audit pipeline with inverse-plan
  rollback; touch-per-op step-up on destructive/secret ops; dual-control enforcement;
  runbooks (restart, cert rotate, backup, drain, pkg-update + checksum refresh);
  `available_actions[]` wired to capabilities.
- **Reuses:** ssh mutation guards, workflow engine, audit pipeline (adapt).
- **Exit:** GREEN; an operator runs a guarded restart + a dual-control op end-to-end,
  fully audited; dry-run diffs render.

### Phase 4 — Provisioning
- **Goal:** deploy a new node from the tool, end-to-end.
- **Tasks:** `Provider` trait + birth/convergence split; deploy a FreeBSD core node and a
  Linux per-tenant install; WireGuard join; inventory registration; vault secret
  bootstrap **(depends on Phase 2b)**; Terraform-state decision (SQLite-in-broker vs
  remote backend).
- **Coordination:** OpenSearch index/RBAC for new nodes; per-tenant isolation
  (WG IP, namespace, RBAC, WAF profile, DB schema).
- **Exit:** GREEN; one new node of each fleet born + converged + registered + healthy.

### Phase 5 — Bottleneck engine + scaling
- **Goal:** detect bottlenecks, compute scale, act with guardrails.
- **Tasks:** signal collection; rules+confidence analyzer (no ML); capacity models
  (compute N, never blind +1); guarded horizontal scaling (OpenSearch first, then
  kalista/vulture/sync/DB); guardrails + rollback.
- **Exit:** GREEN; a synthetic OpenSearch bottleneck → recommendation → guarded
  scale-out with rollback path.

### Phase 6 — Per-tenant lifecycle + reconciler
- **Goal:** upgrade/scale/decommission + drift reconciliation.
- **Tasks:** blue/green + rolling upgrade (expand/contract migrations); scale/decommission
  flows; drift reconciler (detect/propose/**human-gated apply**, explicit auto-apply
  whitelist defaulting to empty).
- **Exit:** GREEN; tenant upgrade + a reconciler-proposed-then-approved drift fix.

### Phase 7 — Web UI + hardware factor
- **Goal:** React/Vite/Tailwind/shadcn UI over the same API; flip YubiKey to required.
- **Tasks:** scaffold `web/` per `03-ui-design-system.md` (kalista tokens + shadcn);
  consume OpenAPI-generated `demon-sdk` types; build fleet/host/jobs/alerts/audit/
  runbooks/tenants screens + demon-delta components (typed-confirm dual-control modal,
  dry-run diff, residency switcher, WS live indicators); Axum-served; flip
  `factor_policy` `RoamingHardware` → required once YubiKeys agreed.
- **Exit:** GREEN; web UI achieves feature parity with TUI for read + guarded actions.

---

## 7. Risk register (track continuously)

| # | Risk | Phase | Mitigation |
|---|------|-------|------------|
| R1 | **Vault is not a broker** (§3.1) | 2b/4 | Escalate; defer 2b; interim host-bound secret |
| R2 | No `private_key_jwt`/mTLS at identity | 2 | Accept `client_secret_basic`; revisit |
| R3 | No acr/amr/WebAuthn at identity | 2/3 | demon owns `SecondFactor` step-up |
| R4 | `Residency<R>` compile-time proof is greenfield | 0 | Build first; compile-fail tests |
| R5 | SQLite single-writer vs job throughput | 0/2 | Validate WAL; serialized writer actor if contended |
| R6 | Tenant-scale SSH fan-out concurrency | 1/4 | Global semaphore + per-tenant poll tiers |
| R7 | Reconciler blast radius | 6 | Auto-apply whitelist defaults empty; human-gated |
| R8 | Litestream restore drills unverified | 0/4 | Scheduled + audited restore tests |
| R9 | Contract/version skew vs services | all | Version clients vs CONTRACTS.md; fail closed |
| R10 | Client/SDK drift | 7 | `demon-sdk` single source; OpenAPI-gen web types |

---

## 8. Coordination state & next actions

- **identity/demon-oidc-client-shape.md** → `answered (pending demon alignment) — 1 follow-up`
  (private_key_jwt/mTLS roadmap). Demon to register one client per group in Phase 2.
- **vault/demon-brokered-creds-shape.md** → `partial — 5 follow-ups` (**blocker R1**);
  awaiting vault-owner decision before Phase 2b.
- **To commit to `coordination/` when built:** demon's `source=control-plane` audit usage,
  the OIDC client contract alignment, and `conventions/metrics.md` (OpenSearch rollup/query).
- Never put secrets / per-tenant data in `coordination/`.

---

## 9. Recommended sequencing

`Phase 0 → 1 → 2 (OIDC) → 3` is fully unblocked and delivers the highest value
(see-everything + guarded-actions). **Resolve R1 (vault) in parallel during Phases 0–2**
so Phase 2b/4 are not on the critical path. Defer Phase 7 web UI until the API surface
stabilizes after Phase 3 (TUI is the cheaper Phase-1 proof). Hardware factor (YubiKey)
stays pluggable-off until procurement is agreed.
