# proximiio.demon — Claude Code notes

Always-on Rust/Axum DevOps control-plane daemon — the single auditable brain for the
EU+UAE in-house fleet (FreeBSD core + Linux per-tenant). Next-gen of `proximiio-tui`.
Spec lives in `docs/` (design package) + `docs/planning/` (build plan, reuse map,
contracts, UI spec). Where docs conflict, **security wins**.

## Cross-service coordination (quanto / proximi.io stack)

You (`demon`) integrate tightly with 4 siblings (identity / vault / kalista / vulture)
+ infra. Coordinate via **files**, not by relaying messages through the human. Shared
coordination dir (temporary): `/Users/wired/proximi-admin/proximiio-infra/coordination/`
(moves to a neutral `quanto/contracts` repo later). Full protocol:
`proximiio-infra/docs/proximiio-demon/04-collaboration-protocol.md`.

- **On start, check your inbox** `…/coordination/inbox/demon/` and resolve/ack each note
  (answer inline + flip `Status:`, or move durable agreements to `…/decisions/`).
- **Read** `…/coordination/CONTRACTS.md` (where each service's LIVE contract is) and
  `…/coordination/conventions/` (jwt-claims, audit-event-schema, residency, ports-env)
  **before** building anything that touches a service boundary.
- **When you need a contract**, drop a note in that service's `…/coordination/inbox/<svc>/`
  and proceed against the documented contract once answered — don't relay via the human.
- **When you change a boundary you own** (claim, route, port, header, endpoint), commit
  it to `…/coordination/` (CONTRACTS.md / conventions) AND drop a note in the affected
  inbox — don't only say it.
- **Never** put secrets or per-tenant data in `coordination/`. Contracts only.

Current cross-service state (see `docs/planning/02-auth-secrets-contracts.md`):
- **identity** = `answered` — Auth-Code + PKCE-S256, `client_secret_basic`, claims
  `sub/tenant_id/residency_group/roles/scope` (no `email`/`groups`/`acr`), one client per
  per-group issuer. **demon owns its own per-op step-up** (identity has no acr/amr/WebAuthn).
- **vault** = `partial` / escalated — terrapi-vault is an embedded SQLCipher at-rest lib,
  **NOT a network broker**; Phase 2b/4 secrets layer is **blocked** pending a path A/B/C
  decision (`inbox/vault/demon-needs-brokering-service.md`).

# context-mode — MANDATORY routing rules

You have context-mode MCP tools available. These rules are NOT optional — they protect your context window from flooding. A single unrouted command can dump 56 KB into context and waste the entire session.

## BLOCKED commands — do NOT attempt these

### curl / wget — BLOCKED
Any Bash command containing `curl` or `wget` is intercepted and replaced with an error message. Do NOT retry.
Instead use:
- `ctx_fetch_and_index(url, source)` to fetch and index web pages
- `ctx_execute(language: "javascript", code: "const r = await fetch(...)")` to run HTTP calls in sandbox

### Inline HTTP — BLOCKED
Any Bash command containing `fetch('http`, `requests.get(`, `requests.post(`, `http.get(`, or `http.request(` is intercepted and replaced with an error message. Do NOT retry with Bash.
Instead use:
- `ctx_execute(language, code)` to run HTTP calls in sandbox — only stdout enters context

### WebFetch — BLOCKED
WebFetch calls are denied entirely. The URL is extracted and you are told to use `ctx_fetch_and_index` instead.
Instead use:
- `ctx_fetch_and_index(url, source)` then `ctx_search(queries)` to query the indexed content

## REDIRECTED tools — use sandbox equivalents

### Bash (>20 lines output)
Bash is ONLY for: `git`, `mkdir`, `rm`, `mv`, `cd`, `ls`, `npm install`, `pip install`, and other short-output commands.
For everything else, use:
- `ctx_batch_execute(commands, queries)` — run multiple commands + search in ONE call
- `ctx_execute(language: "shell", code: "...")` — run in sandbox, only stdout enters context

### Read (for analysis)
If you are reading a file to **Edit** it → Read is correct (Edit needs content in context).
If you are reading to **analyze, explore, or summarize** → use `ctx_execute_file(path, language, code)` instead. Only your printed summary enters context. The raw file content stays in the sandbox.

### Grep (large results)
Grep results can flood context. Use `ctx_execute(language: "shell", code: "grep ...")` to run searches in sandbox. Only your printed summary enters context.

## Tool selection hierarchy

1. **GATHER**: `ctx_batch_execute(commands, queries)` — Primary tool. Runs all commands, auto-indexes output, returns search results. ONE call replaces 30+ individual calls.
2. **FOLLOW-UP**: `ctx_search(queries: ["q1", "q2", ...])` — Query indexed content. Pass ALL questions as array in ONE call.
3. **PROCESSING**: `ctx_execute(language, code)` | `ctx_execute_file(path, language, code)` — Sandbox execution. Only stdout enters context.
4. **WEB**: `ctx_fetch_and_index(url, source)` then `ctx_search(queries)` — Fetch, chunk, index, query. Raw HTML never enters context.
5. **INDEX**: `ctx_index(content, source)` — Store content in FTS5 knowledge base for later search.

## Subagent routing

When spawning subagents (Agent/Task tool), the routing block is automatically injected into their prompt. Bash-type subagents are upgraded to general-purpose so they have access to MCP tools. You do NOT need to manually instruct subagents about context-mode.

## Output constraints

- Keep responses under 500 words.
- Write artifacts (code, configs, PRDs) to FILES — never return them as inline text. Return only: file path + 1-line description.
- When indexing content, use descriptive source labels so others can `ctx_search(source: "label")` later.

## ctx commands

| Command | Action |
|---------|--------|
| `ctx stats` | Call the `ctx_stats` MCP tool and display the full output verbatim |
| `ctx doctor` | Call the `ctx_doctor` MCP tool, run the returned shell command, display as checklist |
| `ctx upgrade` | Call the `ctx_upgrade` MCP tool, run the returned shell command, display as checklist |
