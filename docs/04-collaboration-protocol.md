# proximiio.demon — cross-agent collaboration protocol

> **Hand this to the demon agent.** It defines how demon collaborates with the other
> service agents (identity / vault / kalista / vulture) through the shared
> `coordination/` directory — **via files, never by relaying messages through the
> human.** This mirrors the rule in each repo's `CLAUDE.md`.

## The shared coordination directory

It currently lives in the **proximiio-infra** repo at `coordination/` (temporary; will
move to a neutral `quanto/contracts` repo later). The demon repo does **not** own it —
demon reads/writes it in whatever checkout of infra is available, or the agreed shared
location. Structure:

```
coordination/
  CONTRACTS.md            # index of each service's LIVE contract
  conventions/            # cross-cutting agreements — READ THESE FIRST
    jwt-claims.md         #   token claim set (sub, tenant_id, residency_group, roles…)
    audit-event-schema.md #   canonical B3 audit-event shape + `source` values
    residency.md          #   EU/UAE air-gap rules — never mix
    ports-env.md          #   ports / env / issuer URLs (some LOCKED)
  decisions/              # durable, ratified agreements (promoted out of inbox)
  inbox/
    demon/   identity/  vault/  kalista/  vulture/  infra/
```

## demon's standing rules (do these every session)

1. **On start, read `coordination/inbox/demon/`** and resolve/ack each note: answer
   inline, flip its `Status:`, and move any durable agreement into
   `coordination/decisions/`.
2. **Read `coordination/CONTRACTS.md` + everything in `coordination/conventions/`**
   before building anything that crosses a service boundary. These are the source of
   truth — do not re-derive a sibling's contract from memory.
3. **When you need something from a sibling** (a claim, route, port, header, endpoint,
   API shape): drop a note in *their* inbox — `coordination/inbox/<service>/<demon>-<topic>.md`
   — with `Status: open`, then proceed against the documented contract once they answer.
   Do not ask the human to relay it.
4. **When you change a boundary demon owns** (its API surface, an audit `source` value,
   a registered OIDC client, a port): commit it to `coordination/` (update CONTRACTS.md
   / add a decision) **AND** drop a note in each affected service's inbox. Don't only
   say it in chat.
5. **Never put secrets or per-tenant data in `coordination/`.** Contracts only. demon's
   own secrets (e.g. its OIDC `client_secret`) live in its host keychain, never here.

## Note format (every inbox file)

```
# <from> → <to>: <one-line topic>

Context: …
Need: …
Why: …

Status: open            # open | answered | partial | done
```
The answering agent appends an `## Answer (YYYY-MM-DD)` section, cites the evidence
(repo paths it read), and flips `Status:`.

## Answers already waiting for demon (read these first)

Two asks demon raised are **already answered** in the inbox — consume them before
Phase 2:

- **`coordination/inbox/identity/demon-oidc-client-shape.md` — ANSWERED.** Key points:
  operator login = OAuth **Authorization Code + PKCE (S256)**, clients registered via
  identity's **admin API** (cookie+CSRF+MFA), **`client_secret_basic`** token auth
  (`private_key_jwt`/mTLS NOT supported yet — follow-up), claims available =
  `sub, tenant_id, residency_group, roles` (**no `groups`, no `email`** in the edge
  JWT; **no `acr`/`amr`** → **demon owns per-operation touch step-up**; identity owns
  login-time MFA = TOTP today). **One confidential client per residency group** against
  `https://identity-eu.proximi.io/` and `…-uae…` (issuers LOCKED).

- **`coordination/inbox/vault/demon-brokered-creds-shape.md` — PARTIAL / important.**
  **terrapi-vault today is an embedded SQLCipher at-rest *library*, NOT a network
  secrets broker** — it has no daemon auth, no SSH-CA / short-TTL issuance, no leases,
  no namespaces, no OpenSearch audit emission. So **demon's "broker, not a vault"
  secrets layer (Phase 2/4) has no backing service yet** → 5 follow-ups for the vault
  owner, and a **decision is required**: either the vault owner builds a brokering
  service, or demon's secrets layer targets a different broker. **Do not assume
  terrapi-vault provides brokering as-is.** Until then, demon's single host-bound
  secret + SSH (static key, short-lived if/when a CA exists) is the interim.

When demon consumes an answer, append a short ack and flip `Status:` to `done` (or move
the agreement to `decisions/`).

## Who's who (the collaborating agents)

| Service | What it owns | demon's relationship |
|---|---|---|
| **identity** (terrapi-identity) | OIDC IdP / JWT issuer / SSO broker | demon's operator-login IdP |
| **terrapi-vault** | (today) at-rest SQLCipher lib; (future?) secrets broker | demon's secrets backing — **gap, see above** |
| **kalista** | reverse proxy + WAF | demon observes/configures it; scaling target |
| **vulture** | main API (opaque DB tokens) | demon observes/scales; token-revoke surface |
| **infra** (this repo) | host/fleet scripts, the TUI lineage, `coordination/` host | demon reuses its parsers/contracts; lives alongside |

Each of these repos carries the same "Cross-service coordination" hook in its
`CLAUDE.md`, so every agent follows this same file-based protocol.

## ACTION for the demon agent: add the hook to your own CLAUDE.md

The demon repo's `CLAUDE.md` does **not** yet carry the cross-service coordination
hook (the 5 core services + infra already do). Add it so future demon sessions
auto-follow this protocol. Copy the canonical block from
`proximiio-infra/coordination/CLAUDE-snippet.md` (or the "Cross-service coordination"
section of `proximiio-infra/CLAUDE.md`) into the demon repo's `CLAUDE.md`, adjusting
only the relative path to wherever `coordination/` is reachable from the demon repo.
The block must tell every demon session to: read `coordination/inbox/demon/` on start,
read `CONTRACTS.md` + `conventions/`, post asks to a sibling's inbox (never relay via
the human), commit boundary changes + drop an inbox note, and never put secrets /
per-tenant data in `coordination/`.

> Live checkouts (confirmed 2026-05-26): the quanto services run from
> `/Volumes/ExtremeSSD/quanto/{identity,terrapi-vault,kalista}` and vulture from
> `/Volumes/ExtremeSSD/proximiio/vulture-mono` — all four carry the hook. `proximiio-sync`
> is a stack service that still lacks both the hook and an `inbox/sync/` dir (open item).
