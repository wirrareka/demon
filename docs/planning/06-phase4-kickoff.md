# Phase 4 — Provisioning (kickoff brief)

> Resume brief for Phase 4. Spec: `docs/02-provisioning-scaling-ops.md`. Phases 0–3 are
> complete; 5/6 cores + Phase 5 PromQL feed + Phase 2b vault secrets-session layer shipped.

## Goal (per `02`)
Deploy a new server from the tool, end-to-end, for both fleets:
- a new **FreeBSD core** node and a new **Linux per-tenant** install;
- **`Provider` trait** + the **birth → convergence** split (thin per-provider "birth";
  one uniform agentless-SSH "convergence" — the `check-*.sh` line-contracts ARE the
  config model, no Ansible);
- **WireGuard join** + **inventory registration**; **vault secret bootstrap**.
Every mutation still runs the guarded pipeline (plan→confirm→dual-control→apply→verify→audit)
with an **inverse-plan rollback**.

## Now unblocked (vault engines live)
- `ssh/ca` + `ssh/sign` + `creds` are **live** in the broker (v1 shapes; `demon-clients::vault`
  already typed + wired). SSH-cert brokering replaces static SSH keys.
- **Decision `coordination/decisions/demon-vault-session-model.md`:** two mTLS principals —
  `demon-operator` (login session, shipped) + **`demon-system`** (autonomous). One active
  session per SAN; issuance attaches implicitly; `409 no_active_session` if none.

## Build order (each phase ends GREEN: build + clippy -D warnings + test)
1. **System vault session** — `demon-workers`: a keepalive worker that opens a
   `demon-system` vault session and renews within the 30 m idle (re-open on seal/idle).
   demon-bin: second `VaultClient` from `DEMON_VAULT_SYSTEM_URL` + system mTLS cert
   (`VaultClient::with_client`). Needs the deploy cert; until then config-gated/off.
2. **SSH-cert brokering in the transport** — `demon-collect::SshTransport`: before
   connecting, generate an ephemeral key, `vault.ssh_sign(group, {cert_type:user,
   principals:[ssh_user], ...})`, write key+cert to a temp dir (mode 600, zeroize/drop),
   `ssh -i key -o CertificateFile=cert`. Operator path uses the operator session; poller
   uses the system session. Pure cert-handling unit-tested; live path behind config.
3. **`Provider` trait + birth/convergence** — new `demon-provision` crate (or in
   `demon-workers`): `Provider::birth(spec) -> RawHost` (per-provider: bare-metal IPMI/PXE,
   cloud later) behind a trait; `converge(host)` = the agentless-SSH `check-*.sh` apply loop
   reusing the mutation pipeline. Keep `demon-core` pieces pure (plan/inverse-plan).
4. **WG join + inventory registration** — add the new node to the WireGuard mesh
   (group-scoped), `store.upsert_host`, first health poll.
5. **HTTP + UI** — `POST /api/v1/hosts` (provision a node, guarded + dual-control as a
   `Destructive` class) → job pipeline; a Provision wizard surface later.

## Open coordination to confirm during Phase 4
- Terraform/host state ownership (encrypted SQLite-in-broker vs remote backend) — see
  `02` §8 open risks.
- PXE/IPMI mgmt-LAN trust boundary for FreeBSD metal.
- vault bootstrap of the **first** `demon-system` cert at host bring-up (Root CA signs;
  mode-600 on ZFS-encrypted dataset) — already sketched in the vault Path A answer.

## Entry points (where the code is)
- vault client: `crates/demon-clients/src/vault.rs` (typed, error-mapped).
- SSH transport: `crates/demon-collect/src/lib.rs` (`SshTransport`, `Transport` trait).
- mutation pipeline: `crates/demon-core/src/mutation.rs` + `demon-workers/src/exec.rs`.
- guarded HTTP: `crates/demon-server/src/jobs.rs`; AppState in `demon-server/src/lib.rs`.
- store: `crates/demon-store/src/inventory.rs` (`upsert_host`, etc.).
