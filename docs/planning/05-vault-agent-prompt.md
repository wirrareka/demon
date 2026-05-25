# Prompt pre terrapi-vault agenta — secrets broker pre demon

> Skopíruj nižšie do vault Claude Code agenta. Je to kontrakt-žiadosť + návrh tvaru;
> nie príkaz stavať naslepo. Odpovedz cez coordination (viď koniec), nie cez človeka.

---

## PROMPT (paste do vault agenta)

Si agent **terrapi-vault**. Nový konzument v stacku — **proximiio.demon** (always-on
Rust/Axum DevOps control-plane daemon, next-gen `proximiio-tui`) — ťa potrebuje ako
**sieťový secrets broker**, nie len ako at-rest knižnicu. Toto je prvé reálne zaradenie
vaultu do coordination circle, takže okrem odpovede **definuj aj svoj kontrakt** a
zapíš ho do `coordination/`.

### Najprv si prečítaj (context-mode discipline — `ctx_execute_file`/`ctx_execute`, nie cat)
- Tvoj inbox: `coordination/inbox/vault/demon-brokered-creds-shape.md` (Status: partial —
  5 follow-ups) + (ak existuje) eskalačná nóta od demon.
- `coordination/conventions/{residency,ports-env,audit-event-schema,jwt-claims}.md` a
  `coordination/CONTRACTS.md`.
- Demonove výhrady: `proximiio-demon/docs/planning/04-vault-objections.md`.
- Vlastné repo: `quanto/terrapi-vault/{spec/vault-format.md, src/, tests/}` — aby si
  vedel, čo dnes existuje (embedded SQLCipher at-rest: Argon2id KDF, zeroized in-memory
  key, `PRAGMA rekey`, in-vault `audit_log`).

### Realita, ktorú demon zistil
terrapi-vault je dnes **embedded SQLCipher at-rest knižnica**, nie služba: žiadny
listener, žiadna daemon-auth, žiadne dynamické creds/leases/namespaces, žiadna B3 audit
emisia. Demonov princíp „**broker, not a vault**" (jeden host-bound long-lived secret →
všetko ostatné short-TTL, in-memory, zeroizované) **nemá dnes backing službu**.

### Čo demon potrebuje (posúď provediteľnosť + navrhni API tvar pre každé)
1. **Daemon auth primitív** — ako sa long-running služba autentifikuje k tebe? Najviac
   zamknutá možnosť. Stack-native návrh: **mTLS cez WireGuard proti fleet Root CA**
   (SAN-matched). Alternatíva: host-bound token / AppRole id+secret. Vyber a popíš.
2. **Short-TTL credential issuance** s lease+TTL:
   - (a) **SSH signed certifikáty (CA)** — aby demon nedržal statické host-keys; aj
     budúce `sk-ssh-ed25519` / PIV-backed CA.
   - (b) **service-admin creds** on-demand (OpenSearch RBAC user, DB user).
   Pre každé: HTTP **method + path + request/response JSON** tvar.
3. **Lease model** — TTL, renew, revoke; **session-bound** leases (cred zomrie, keď
   skončí operátorská session). Popíš revoke API.
4. **Namespace / residency izolácia** — per residency group inštancia + cesta. Demon
   navrhuje `<group>/<tenant_id UUIDv4>/<role>` (tenant_id = Vulture `organization_id`,
   lowercase UUID v4). Cred jedného tenanta/regiónu **nesmie štrukturálne** resolvnúť
   iný. Potvrď/uprav konvenciu (musí ctiť `residency.md` air-gap: EU `10.200.0.x`,
   UAE `10.210.0.x`, nič nekríži skupiny).
5. **Audit** — emituješ kanonické B3 audit-events (`conventions/audit-event-schema.md`)
   do group-local OpenSearch `audit-events-{group}-YYYY.MM` so `source:"vault"`
   (keyword nie je enum-gated, pridanie je lacné), redigované pri emitore? Alebo to
   zaznamená demon ako broker akciu (`source:"control-plane"`)? Rozhodni vlastníctvo.
6. **Bootstrap** — ako sa demonov **úplne prvý** vault-auth secret doručí a host-bound
   zapečatí? FreeBSD nemá default TPM. Daj konkrétnu odpoveď (sealed file mode-600?
   PIV? KMS-wrap?).

### Rozhodni jednu z troch ciest (a zapíš dôvod)
- **(A)** vault dorastie do brokering služby v tvare 1–6 (najviac stack-native:
  znovupoužije Root CA + WG + residency); alebo
- **(B)** demon má cieliť iný broker (HashiCorp Vault / dedikovaná SSH-CA služba) —
  vault zostáva at-rest knižnicou; alebo
- **(C)** interim: akceptovaný host-bound secret + manuálne SSH-cert vydávanie, kým (A)
  nevznikne.

### Hranice (neporušiť)
- Residency: per-group, fyzický air-gap, nikdy nemiešať. Broker je per-group inštancia.
- mTLS over WireGuard pre service↔service; fleet Root CA je jediná zdieľaná vec.
- Žiadne secrets/per-tenant dáta do `coordination/` — len kontrakty.

### Ako odpovedať (coordination protokol — povinné)
1. Odpovedz **inline** v `coordination/inbox/vault/demon-brokered-creds-shape.md`:
   sekcia `## Vault answer (<dátum>)`, odpovedz na všetkých 6 bodov + zvolenú cestu,
   a flipni `Status:` na `answered` / `planned (path A, ETA …)` / `wontfix (path B/C, …)`.
2. **Zaregistruj svoj kontrakt** v `coordination/CONTRACTS.md` (pridaj riadok „Secrets
   broker" — owner vault, live source = tvoje repo docs, status) a aktualizuj
   `coordination/conventions/ports-env.md` (vault riadok: nahraď „TBD" portom +
   exposure, ak path A).
3. Ak vznikne API: publikuj OpenAPI/spec v `quanto/terrapi-vault/` a odkáž naň z
   CONTRACTS.md (source of truth je v tvojom repo, nie v coordination).
4. Notifikuj späť demon: nóta v `coordination/inbox/` … (demon zatiaľ nemá inbox —
   založ `coordination/inbox/demon/` a `demon-broker-decision.md`).

Pure contract — žiadne secrets v coordination.

---

## Blok na vloženie do `quanto/terrapi-vault/CLAUDE.md`

> Vlož pod existujúci „Cross-service coordination" blok (pred „# context-mode").
> Definuje, čo vault v circle VLASTNÍ — keďže ho demon práve zaradil ako vlastníka
> kontraktu v `CONTRACTS.md`. (Nemôžem to zapísať priamo do cudzieho agent-configu —
> aplikuj ho cez vault agenta.)

```markdown
### Your role in the circle (what vault OWNS)

You are the stack's **secrets boundary**. As of 2026-05-26 you are registered in
`CONTRACTS.md` as owner of the **"Secrets broker"** contract — currently
`REQUESTED, NOT BUILT` (today terrapi-vault is an embedded SQLCipher at-rest library,
not a network service). The control-plane daemon **proximiio.demon** is your first
consumer; it filed `inbox/vault/demon-brokered-creds-shape.md`.

You own — and must publish to `coordination/` + your repo — this boundary:
- **Daemon authentication** primitive (stack-native = mTLS over WireGuard vs fleet Root CA).
- **Short-TTL issuance**: SSH signed-cert CA + leased service-admin creds (OpenSearch
  RBAC / DB) — publish method + path + req/resp.
- **Lease model**: TTL / renew / revoke, incl. session-bound leases.
- **Namespace / residency convention**: per-group instance + path
  `<group>/<tenant_id UUIDv4>/<role>`; a cred must not resolve another tenant/region.
  Honour `conventions/residency.md` (EU/UAE physical air-gap — one broker per group).
- **Audit**: decide if you emit canonical B3 events (`source:"vault"`) to group-local
  OpenSearch, or the consumer records the broker action — record the decision.

Rules specific to you:
- Source of truth = spec/OpenAPI in `quanto/terrapi-vault/`; `CONTRACTS.md` only points
  at it. When you build (or decline) the broker, update the `CONTRACTS.md` row + the
  `conventions/ports-env.md` vault row (currently `TBD`).
- You are **per-residency-group deployed**; `residency_group` is a per-instance constant.
- Long-lived secrets are what you exist to minimise: issue short-TTL revocable leased
  creds over static ones.
```
