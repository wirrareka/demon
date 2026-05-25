# Výhrady demonu voči terrapi-vault (eskalácia pre vault agenta)

> Kontrakt-only. Cieľ: vault owner posúdi, či vault dorastie do brokeru, alebo demon
> cieli iný. Evidencia: `quanto/terrapi-vault/{spec/vault-format.md, src/, tests/}`,
> `coordination/conventions/ports-env.md` (vault = "TBD … future KMS — confirm").

## Jadro problému
Demon je navrhnutý ako **broker, nie vault**: drží **jeden** host-bound long-lived secret
(auth do vaultu) a všetko ostatné brokeruje ako **short-TTL, in-memory, zeroizované**
creds. Tento model predpokladá, že terrapi-vault je **sieťová brokering služba**.
Realita v repo: terrapi-vault je **embedded SQLCipher at-rest knižnica** (memento notes /
probe), nie server. Tým padá celá demonova Phase 2b/4 secrets vrstva.

## Konkrétne výhrady / chýbajúce schopnosti

1. **Žiadny network surface / listener.** Vault je linkovaná knižnica, nie daemon.
   Demon (always-on služba na vlastnom hoste) nemá kam sa pripojiť. → Potrebný server
   s WG-only listenerom (viď `ports-env.md`, riadok vault = TBD).

2. **Žiadna daemon autentifikácia.** Niet host-bound tokenu, mTLS, ani AppRole-style
   id+secret. Demon potrebuje **jeden** najviac zamknutý auth primitív — stack-native by
   bolo **mTLS cez WireGuard proti fleet Root CA**. Toto je základ celého blast-radius
   príbehu.

3. **Žiadne dynamické short-TTL creds.** Vault vie len uložiť/odšifrovať statické
   tajomstvá. Demon potrebuje **on-demand vydávanie** s lease+TTL pre:
   - (a) **SSH signed certifikáty (CA)** — aby demon nikdy nedržal statické host keys;
   - (b) **service-admin creds** (OpenSearch RBAC user, DB user) s expiráciou.
   Bez toho demon buď drží dlhožijúce SSH kľúče (porušuje princíp 5), alebo provisioning
   (Phase 4) nemá ako bezpečne pristúpiť k novým hostom.

4. **Žiadny lease model.** Niet TTL / renew / revoke, ani **session-bound leases**
   (cieľ: creds zomrú, keď skončí operátorská session). Bez revoke-on-session-end sa
   blast radius nedá ohraničiť.

5. **Žiadne namespaces / residency izolácia.** Demon potrebuje, aby cred pre jeden
   tenant/región **štrukturálne nemohol** resolvnúť iný. Návrh konvencie:
   per-group inštancia + cesta `<group>/<tenant_id UUIDv4>/<role>` (tenant_id =
   Vulture organization_id, lowercase UUID v4), v súlade s `residency.md` (EU/UAE
   fyzický air-gap). Dnes neexistuje žiadny path/namespace model.

6. **Žiadna B3 audit emisia.** Vault má len in-vault `audit_log` tabuľku, neemituje
   kanonické B3 audit-events do group-local OpenSearch. Treba dohodnúť: emituje broker
   vlastné eventy (`source:"vault"` — keyword nie je enum-gated, pridanie je lacné),
   alebo demon zaznamená broker akciu ako `source:"control-plane"`?

## Bootstrap riziko (aj keby broker vznikol)
Demonov **úplne prvý** vault auth secret treba doručiť a zapečatiť host-bound spôsobom.
FreeBSD nemá default TPM story. Treba konkrétnu odpoveď na host-binding (TPM? sealed
file mode-600? macOS Keychain len pre control-plane?). Bez toho je "jeden long-lived
secret" len presunutý problém.

## Čo demon žiada rozhodnúť (jedna z troch ciest)
- **(A)** terrapi-vault dorastie do brokering služby v tvare bodov 1–6 vyššie
  (najviac stack-native, znovupoužije Root CA + WG + residency model); alebo
- **(B)** demon cieli iný broker (napr. HashiCorp Vault / dedikovaná SSH-CA služba) —
  vault zostáva len at-rest knižnicou; alebo
- **(C)** interim: akceptovaný host-bound secret + manuálne SSH-cert vydávanie, kým
  broker (A) nevznikne — demon secrets vrstva je dovtedy stub.

## Čo NIE je blokované (aby bola eskalácia fér)
Demonov auth (operátorský login) stojí na identity, nie na vaulte — Phase 0–3 (vrátane
OIDC, RBAC, audit, guarded actions) bežia bez vyriešeného vaultu. Blokované sú len
**Phase 2b (secrets broker)** a časť **Phase 4 (provisioning secret bootstrap +
SSH-cert prístup k novým hostom)**.

> Stav inbox nóty: `coordination/inbox/vault/demon-brokered-creds-shape.md` =
> `partial — 5 follow-ups`. Táto eskalácia rozširuje tých 5 bodov o bootstrap riziko
> a tri cesty rozhodnutia.
