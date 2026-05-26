# proximiio.demon — Security Architecture

**Status:** Design (greenfield). **Audience:** build agent + new DevOps admin.
**Classification:** Internal. **Maps to:** OWASP ASVS 4.0 (L3 targets noted inline).

> proximiio.demon is the single most dangerous asset in the company: an always-on,
> network-reachable Rust/Axum daemon that can deploy hosts, route all traffic, manage
> identities, and touch every machine. Compromise = total company compromise. It also
> brokers access to **user location PII** under **EU + UAE residency that is physically
> air-gapped and must NEVER be mixed**. Security is enforced **by design**, not by
> operator discipline. Assume the operator is brand-new and will make mistakes.

---

## 0. Core principles

1. **Deny by default, gate by type.** Every dangerous capability is unreachable unless
   a typed, residency-tagged, role-checked path is constructed. No "page-level" auth.
2. **Hold as few long-lived secrets as possible.** Demon is a *broker*, not a vault.
3. **Tamper-evident everything.** Hash-chained audit extends to the network plane.
4. **Blast-radius minimization.** A stolen session, a popped host, and a malicious
   insider must each be *contained* by an independent control.
5. **Residency is an invariant, not a config flag.** EU and UAE are separate type
   universes, separate processes, separate stores, separate keys.

---

## 1. AuthN — admin plane

### 1.1 Identity broker
Operators authenticate via **terrapi-identity (OIDC, authorization-code + PKCE)**.
demon is a confidential OIDC client; client secret lives in terrapi vault, fetched at
boot, never on disk. demon **does not store passwords**. Some operators federate
through customer SSO (Azure AD) via terrapi-identity — demon only ever sees
terrapi-identity-issued tokens, never the upstream IdP. (ASVS V2, V3.)

### 1.2 Hardware factor — pluggable, enforce-later
**YubiKey 5C NFC is the planned hardware factor but is NOT yet purchased/agreed.**
Therefore design it as a `SecondFactor` trait with implementations:

- `WebAuthnPlatform` (TouchID/Windows Hello) — available day one.
- `WebAuthnRoaming` (FIDO2 security key) — the YubiKey slot.
- `TotpFallback` — bootstrap only, **disabled in prod for senior/break-glass**.

Policy is data, not code: a `factor_policy` table maps
`(role, action_class) -> required_factor_level`. When YubiKeys arrive, flip
`RoamingHardware` to *required* for `Destructive | SecretAccess | CaUse` without a
rebuild. **Touch-per-op**: destructive/secret actions demand a fresh WebAuthn user-
presence assertion (`userVerification=required`) scoped to that action's challenge —
not a cached session bit. Future: FIDO2 `sk-ssh-ed25519` for SSH, YubiKey PIV as a
short-lived SSH CA holder (see §3).

### 1.3 Session model
- Sessions are **server-side**, opaque 256-bit id in `__Host-` cookie:
  `Secure; HttpOnly; SameSite=Strict; Path=/`. No JWT in the browser.
- **Short idle (15 min) + absolute (8 h) TTL.** Step-up assertions expire in 90 s.
- Session bound to: client cert thumbprint (mTLS, §5), WireGuard peer key, and the
  OIDC `sub`. Mismatch ⇒ kill session + audit `session.binding_violation`.
- Re-auth (full OIDC + factor) required to *escalate* role within a session
  (break-glass is never a default-active role). (ASVS V3.3/V3.7.)

---

## 2. AuthZ / RBAC — per-action, residency-scoped

### 2.1 Roles
| Role | Scope | Notes |
|------|-------|-------|
| `read-only` | residency-scoped | no mutations, no secret reads |
| `operator` | one residency group | day-to-day, single-control non-destructive |
| `senior` | one residency group | may *initiate* dual-control ops |
| `per-tenant` | one tenant within a group | scoped to that tenant's hosts/data only |
| `break-glass` | global, time-boxed | dormant; explicit activation, §8.5 |

No operator is `global` except break-glass. EU and UAE operators are **distinct
principals** even if the same human — separate enrollments, separate factors.

### 2.2 Per-action authorization
Authorization is evaluated **per mutation**, building on the existing
`ensure_whitelisted_mutation` + typed-confirm + dry-run guards. Each action declares:

```
ActionSpec {
  id, action_class: {Read|Mutate|Destructive|SecretAccess|CaUse|CrossResidency},
  residency: Residency,          // EU | UAE  (CrossResidency forbidden, see §6)
  tenant_scope: Option<TenantId>,
  dual_control: bool,
  required_factor: FactorLevel,
}
```

A central `authorize(principal, action_spec) -> Decision` is the **only** path to
execute; the worker layer accepts a `Capability` token minted by `authorize`, so an
un-authorized call simply has no token to present (capability pattern). (ASVS V4.)

### 2.3 Dual-control (two-person rule)
Required for: **CA use, any cross-residency op (normally forbidden — this is the
break-glass-only exception), tenant decommission, mass revoke, factor_policy edits.**
Initiator (`senior`) creates a *pending intent* with a TTL; a **second distinct
principal** approves with their own fresh hardware assertion. Same human cannot fill
both seats (enforced by `sub` + factor credential id). All four events
(intent, approve, execute, expire) are audited.

---

## 3. Secrets — broker, don't hoard

- **terrapi vault is the source of truth.** demon holds exactly **one** long-lived
  secret class: its vault auth (AppRole/role-id + a wrapped secret-id delivered at
  provisioning), ideally bound to the host (TPM/sealed). Everything else is fetched
  short-lived and **kept in memory only**.
- **SSH = certificate-based, never static keys.** vault (or a PIV-backed CA) issues
  **short-TTL (≤5 min) SSH certs** scoped to `principals=<tenant-host>` per job.
  Static `authorized_keys` are forbidden on managed hosts; hosts trust only the CA.
- **Per-host/service creds are brokered just-in-time** for the duration of a worker
  task and zeroized after (`zeroize` crate; `Secret<T>` wrapper that has no `Debug`/
  `Display`/`Serialize`).
- **Workers never log creds.** Reuse the existing rule: secrets via env / fd, never
  argv, never logs. Audit records the *reference* (`vault://path@version`), never the
  value. Redaction list extends the existing NEVER set: password/TOTP/signing-key/raw
  token **+ vault secret-id, SSH cert private half, OIDC client secret, session id.**
- Vault leases are **revoked on session end** so a stolen session cannot outlive its
  brokered creds.

---

## 4. Audit chain — network-grade, tamper-evident

Extend the existing **hash-chained, redacted** audit to a network service:

- Each record: `seq, ts, prev_hash, principal, factor_used, residency, tenant?,
  action_id, action_class, params_redacted, dual_control_ref?, source="control-plane",
  result, sha256(prev_hash || canonical(record))`.
- **What is always recorded:** every login/step-up/factor failure, every mutation
  (intent/approve/execute), every secret broker, every scaling/deploy event, every
  session binding violation, every authz denial.
- **Ship to per-residency-group OpenSearch** (EU index ↔ EU cluster, UAE ↔ UAE; never
  cross-shipped). Local append-only SQLite WAL is the canonical chain; OpenSearch is
  the queryable mirror. A periodic **chain-head anchor** (signed head hash) is written
  to vault + emailed to security, so deletion/truncation is detectable.
- **Retention:** hot 90 d in OpenSearch, cold ≥ 1 y per residency legal req; the hash
  chain itself is immutable for the full retention window. (ASVS V7, V8.)

---

## 5. Network exposure & hardening

- **WireGuard-only.** The Axum listener binds to the **wg interface address only**
  (never 0.0.0.0, never a public IP). No WG tunnel ⇒ no TCP reachability at all.
- **mTLS on top of WG.** Operator client certs (issued by terrapi-identity / internal
  CA) required for the admin API and UI; cert thumbprint binds to the session (§1.3).
  Defense-in-depth: even an intra-WG attacker needs a valid client cert.
- **TLS:** 1.3 only, modern cipher suites, HSTS, OCSP stapling.
- **Axum surface:** strict input validation (typed extractors, reject unknown JSON
  fields), body-size limits, per-principal + per-IP **rate limiting** and quotas on
  destructive endpoints, request timeouts, structured error types that never leak
  internals.
- **Web UI:** `__Host-` cookies, **synchronizer-token CSRF** on all state-changing
  routes (SameSite=Strict is belt-and-suspenders), strict **CORS allowlist** (no
  `*`, credentials only for the known UI origin), strong CSP (`default-src 'self'`,
  no inline script), `X-Frame-Options: DENY`.
- **Supply chain:** `cargo deny` (license + advisory + ban), `cargo audit` in CI gate,
  **SBOM (CycloneDX)** per build, pinned `Cargo.lock`, reproducible builds, vendored/
  mirrored crate registry, signed release artifacts. (ASVS V14, V1.14.)
- **Host hardening:** dedicated minimal host, no other services, FIM (existing auditd
  + integrity monitoring), no inbound except WG, outbound egress allowlist (vault,
  identity, OpenSearch, managed hosts only), full-disk + SQLite encryption at rest,
  run as non-root with tight capabilities, immutable base image, seccomp/jail
  (FreeBSD jail / Linux systemd hardening sandbox).

---

## 6. Residency enforcement — invariant, not convention

**EU and UAE must never mix. Make mixing impossible to *compile*, not just to *do*.**

- **Type-level tags.** `Residency` is an enum and every data/credential/store handle
  is generic over it: `Store<Eu>`, `Store<Uae>`, `Capability<R>`, `VaultClient<R>`,
  `AuditSink<R>`. A function that touches data is `fn op<R: Region>(...)` — there is
  **no API that takes two different `R`s**, so cross-mixing is a type error.
- **Separate processes/datastores per group.** Run demon as **two isolated instances**
  (EU instance, UAE instance) with separate SQLite files, separate vault namespaces,
  separate OpenSearch clusters, separate WG networks. No shared in-memory state.
- The **only** cross-residency code path is the dual-control `CrossResidency` action
  class, which is *default-forbidden* and reachable solely via break-glass — and even
  then it copies *metadata*, never location PII.
- **Hard runtime gate** in addition to the type gate: a residency assertion in
  `authorize()` (defense-in-depth in case `unsafe`/dynamic dispatch erodes the type
  proof).

---

## 7. Multi-tenant isolation

- `TenantId` is carried on every `ActionSpec` and capability; `per-tenant` operators
  are confined by `authorize()` to their tenant's host set + data namespace.
- Each enterprise install has its own SSH CA principal namespace, its own vault path
  prefix, its own OpenSearch index pattern. A cert/token minted for tenant A is
  syntactically unusable against tenant B's hosts.
- No cross-tenant queries in the shared control plane: list/search endpoints are
  always filtered by the principal's tenant scope **server-side** (never trust a
  client-supplied tenant filter). Enumeration (IDOR) tested per ASVS V4.2.

---

## 8. Threat model & break-glass

| # | Scenario | Primary mitigations |
|---|----------|---------------------|
| T1 | **Stolen operator session** | session bound to mTLS thumbprint + WG key + `sub`; touch-per-op step-up for anything dangerous; 15-min idle TTL; vault leases die with session |
| T2 | **Compromised daemon host** | few long-lived secrets (only host-bound vault auth); short-TTL brokered creds; egress allowlist; FIM + tamper-evident audit anchored off-box; per-residency split limits blast radius to one region |
| T3 | **Malicious insider (operator)** | per-action authz (capabilities), dual-control on worst ops, two-person rule can't be self-approved, residency/tenant scoping, immutable audit they cannot edit, anomaly alerts on OpenSearch |
| T4 | **Supply-chain (crate/build)** | cargo deny/audit gates, SBOM, pinned + mirrored deps, signed reproducible builds, minimal dependency surface |
| T5 | **Residency leak** | type-level region tags (compile error), separate processes/stores/keys, runtime assertion backstop |
| T6 | **Identity/SSO compromise (Azure AD)** | demon trusts only terrapi-identity tokens; hardware factor is independent of upstream IdP; break-glass does not depend on SSO |

### 8.5 Break-glass procedure
- `break-glass` role is **dormant**. Activation requires: two distinct seniors,
  hardware assertions from both, a stated reason, and a **time box (≤60 min)**.
- On activation: high-severity audit event + out-of-band alert (email/SMS to
  security), all actions during the window flagged `break_glass=true`, automatic
  expiry + forced re-auth. Mandatory post-incident review of every break-glass window.
- Break-glass credentials/path are **independent of terrapi-identity** so an IdP
  outage or compromise cannot lock out recovery (kept sealed in vault + a sealed
  offline backup).

---

## 9. Open questions / risks for the build agent

1. **Hardware factor timing.** Until YubiKeys are bought, `WebAuthnPlatform` is the
   only real second factor — *acceptable interim* but `senior`/`break-glass` should be
   limited or extra-monitored until roaming hardware is enforced. Confirm interim
   policy with security.
2. **Vault bootstrap secret.** How is demon's initial vault auth delivered and sealed
   (TPM? FreeBSD has no TPM story by default)? Needs a concrete host-binding answer.
3. **Two-instance vs one-binary-two-regions.** Doc assumes two fully separate
   instances; confirm ops are OK running/upgrading two daemons (recommended over a
   single shared-memory binary).
4. **SSH CA ownership.** vault-issued vs PIV-backed CA — depends on YubiKey decision.
5. **OpenSearch chain anchoring cadence + alert routing** — define SLA for detecting a
   broken audit chain.
6. **Coordination:** any change to claims, audit-event schema, or ports must be
   committed to `coordination/` and noted in identity/vault/kalista/vulture inboxes
   (per CLAUDE.md) — this doc touches the audit-event schema and the OIDC client
   contract with terrapi-identity.

---

*References: OWASP ASVS 4.0 (V1 architecture, V2 auth, V3 session, V4 access control,
V7 logging, V8 data protection, V9 comms, V14 config). NIST 800-63B (AAL3 = hardware
+ verifier). Capability-security model. Two-person control (NIST/CNSS).*
