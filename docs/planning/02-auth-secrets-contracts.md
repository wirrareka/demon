# 02 — Phase 2 Auth + Secrets contracts (what demon can rely on)

> Source of truth: the two coordination inbox answers (2026-05-26):
> `proximiio-infra/coordination/inbox/identity/demon-oidc-client-shape.md` and
> `…/inbox/vault/demon-brokered-creds-shape.md`. Conventions:
> `coordination/conventions/{jwt-claims,residency,ports-env,audit-event-schema}.md`.
> Evidence repos: `quanto/identity`, `quanto/terrapi-vault`. Contract-only — no secrets.

## A. Operator auth (identity as OIDC IdP) — BUILDABLE in Phase 2

What demon's auth layer CAN rely on today:

- **Registration**: one **confidential** OAuth client **per residency group**,
  created via identity's admin API (`POST /admin/api/v1/...`, cookie+CSRF+MFA), against
  the LOCKED per-group issuer:
  - EU: `https://identity-eu.proximi.io/`
  - UAE: `https://identity-uae.proximi.io/`
  (`iss` exact-compare incl. trailing slash; JWKS `…/.well-known/jwks.json`).
- **client_secret**: operator-supplied at create, stored Argon2id-hashed by identity,
  never returned. demon holds it as its host-bound secret in macOS Keychain — never in
  the repo or coordination/.
- **Flow**: Authorization Code + **PKCE S256 (mandatory)**. `redirect_uri` is EXACT
  match (no wildcard) — register demon's exact callback per group.
- **Token-endpoint auth**: `client_secret_basic` (use this). `private_key_jwt`/mTLS are
  NOT supported yet (follow-up).
- **Edge-JWT claims demon can verify/use** (`conventions/jwt-claims.md` + claims.rs):
  `iss, aud, sub, tenant_id, residency_group(eu|uae), scope, roles, iat, nbf, exp, jti`;
  header `alg:ES256, kid, typ:at+jwt`. Map operator → role from `sub`/`roles`
  (`roles` = free-form, informational). **No `email`, no `groups` in the token**
  (`email` via `/userinfo` if needed).
- **Login MFA is owned by identity**: `/authorize` enforces TOTP (mandatory for admin)
  after password (H1 FIXED). demon does NOT build login MFA.
- **residency_group** is a per-instance constant; verify it matches the group demon is
  acting in (mismatch handling mirrors the edge: 403).

### Demon must build itself (not provided by identity)
- **Per-op step-up for destructive ops** (touch-per-op). Identity mints **no `acr`/
  `amr`** claim and exposes no per-operation re-auth (`max_age`/`prompt=login`/step-up
  endpoint). So demon owns its own step-up gate for god-mode/destructive actions
  (later: WebAuthn/FIDO2 YubiKey at the demon layer).

### Open follow-ups for the IDENTITY owner
1. `private_key_jwt` / mTLS client auth — on the roadmap? (demon prefers it over a
   static secret.)
2. Add `acr`/`amr` claims (so demon can assert MFA strength from the token) + WebAuthn/
   FIDO2 as an identity factor — roadmap? Until then login MFA = identity, per-op
   step-up = demon.

## B. Brokered short-lived secrets (vault) — NOT BUILDABLE as assumed

Hard finding: **`terrapi-vault` is an embedded SQLCipher encrypted-at-rest LIBRARY**
(memento notes / probe API client) — Argon2id KDF, zeroized in-memory key, `PRAGMA
rekey`, plaintext salt sidecar, in-vault `audit_log` table. It is **NOT** a
network secrets broker. It has no server/listener, no daemon auth, no SSH CA, no
dynamic service-cred engine, no leases/TTL, no namespaces, and no B3 audit emission.
(`ports-env.md` already marks the vault row "TBD … future KMS — confirm".)

Implication: demon's "broker, not a vault" design (one host-bound vault auth → brokered
short-TTL zeroized creds) **cannot target terrapi-vault as it exists**. Phase 2/4
secrets layer is BLOCKED on a decision.

### Open follow-ups for the VAULT owner (all 5 currently unanswered)
1. Daemon auth primitive (host-bound token / mTLS over WG / AppRole) — most locked-down
   option. Stack-native = fleet Root CA mTLS over WireGuard.
2. Short-TTL issuance API: (a) SSH signed-cert CA; (b) leased service-admin creds
   (OpenSearch RBAC, DB) — publish method+path+req/resp.
3. Lease model: TTL/renew/revoke; session-bound (revoke-on-operator-session-end).
4. Namespace/residency path convention. Proposed: per-group instance +
   `<group>/<tenant_id UUIDv4>/<role>` (tenant_id = Vulture organization_id, lowercase
   UUID v4) so a cred can't resolve another tenant/region. Air-gap per `residency.md`.
5. Audit: does the (future) broker emit canonical B3 audit-events to group-local
   OpenSearch and with what `source` (proposed `vault`), or does demon record the
   broker action and emit `source:"control-plane"`? (The `source` keyword is free-form,
   not enum-gated, so adding `vault` is cheap.)

### Demon decision needed before Phase 2/4 secrets work
Either (a) the vault owner builds a brokering service to the shape above, or (b) demon
targets a different broker (e.g. HashiCorp Vault / SSH-CA service). Until resolved,
demon's secrets layer is a stub. demon's blast-radius story still holds for the auth
side (single host-bound OIDC client secret + short-lived edge JWTs).

## C. Audit (both layers)
demon emits canonical B3 audit-events (`conventions/audit-event-schema.md`) to
group-local OpenSearch `audit-events-{residency_group}-YYYY.MM` with
`source:"control-plane"`, redacted at the emitter (never secrets/tokens/keys),
best-effort/non-blocking.
