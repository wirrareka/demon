# proximiio.demon — Provisioning, Fleet-Ops, Bottleneck-Analysis & Horizontal-Scaling

Status: DESIGN (greenfield) · Owner: DevOps control-plane · Date: 2026-05-25

## 0. Guiding principle

A brand-new admin does **everything from the demon** — no SSH hopping, no
ad-hoc scripts. Two fleets, one tool, one mental model:

- **Core** — FreeBSD nodes running our in-house stack (vault, kalista, vulture,
  proximiio-sync, terrapi-identity, OpenSearch).
- **Enterprise** — dedicated per-tenant installs on Linux; each tenant is its
  own install/version/lifecycle/backup unit.

Demon = Rust/Axum/SQLite + tokio workers. It reuses the existing control-plane
primitives: **agentless SSH**, host-side **`check-*.sh` line-contract** scripts,
**guarded mutations** (plan → confirm → apply → verify), and **hash-chained
audit**. This doc layers provisioning/scaling on top of those primitives.

---

## 1. Deploy a new server from the tool (end-to-end)

### 1.1 Orchestration mechanism — options evaluated

| Option | Verdict |
|---|---|
| Pure SSH + scripts | Already proven for *mutation*; weak for *birth* of a host (no host to SSH to yet). Keep for day-2, not day-0. |
| cloud-init | Great for Linux cloud VMs; **not** for FreeBSD bare-metal/jails. Vendor-coupled. |
| Terraform | Best-in-class for cloud resource graph + state + drift. Heavy, HCL, poor fit for FreeBSD jails / bare-metal and for per-tenant app lifecycle. |
| In-house declarative provisioner | Full control, matches our line-contract ethos, but reinvents state/drift. |
| Lightweight agent on each host | Solves day-0 callback + continuous telemetry, but adds a binary to secure/upgrade and breaks "agentless" simplicity. |

**RECOMMENDATION — hybrid, layered:**

1. **Birth layer (provider-specific, thin):**
   - **Cloud (AWS/Hetzner/etc.):** wrap **Terraform** *only* for the raw
     resource graph (VPC/subnet/instance/EBS/SG). Demon shells Terraform via a
     `TerraformProvider`, owns the state file in vault-encrypted SQLite blob,
     and reads outputs (IP, instance-id). No HCL hand-editing — demon renders
     `.tf.json` from typed Rust structs.
   - **FreeBSD bare-metal / VM / jail:** **mfsBSD / PXE + a golden ZFS image**
     for metal; `iocage`/`bastille` for jails. Demon drives via a
     `FreeBSDProvider` (IPMI/redfish power-on → PXE → first-boot SSH key inject).
   - **Linux enterprise VM:** **cloud-init** seed (NoCloud datasource) rendering
     the join token + WireGuard pubkey + vault bootstrap URL.

2. **Convergence layer (uniform, agentless):** once the host has an IP + our SSH
   key, **everything else is the existing SSH + `check-*.sh` + guarded-mutation
   path.** This is where idempotency lives: each `apply-*.sh` is
   re-runnable, and a matching `check-*.sh` returns a line-contract the demon
   diffs to decide converged/needs-apply. No separate config-mgmt engine
   (no Ansible) — the line-contract scripts ARE our config model.

**No persistent agent.** Telemetry (§3) is pulled over SSH on a schedule by
demon's tokio workers. We accept slightly higher poll latency for a smaller
attack surface and one-less-thing-to-upgrade. (Revisit if poll cost > ~2 min
fleet-wide.)

### 1.2 Golden templates / images

- **FreeBSD:** versioned ZFS base image (`base-freebsd-14.2-<date>`) with pkg
  repo pinned, auditd/FIM seeds, WireGuard, and demon's `check-*.sh` bundle
  pre-placed. Image id recorded in inventory; drift = image-hash vs running.
- **Linux enterprise:** an immutable base AMI/qcow2 (`base-linux-tenant-<date>`)
  + per-tenant overlay applied at convergence (no per-tenant bake → faster).
- Images are **per-residency-built and stored in the residency's own registry.**
  EU images never touch UAE infra and vice-versa.

### 1.3 Provisioning flow (both fleets)

```
demon: provision(spec) →
 1. validate spec (role, residency, fleet, size, tenant?)         [guardrail §5]
 2. residency-pin: select provider+region from residency group     [HARD gate]
 3. birth: provider.create(image, size) → host_id, mgmt_ip
 4. first-boot: inject demon SSH key (cloud-init / PXE / iocage)
 5. wireguard-join:
      - demon generates keypair, fetches peer config FROM VAULT
      - assigns IP from residency's WG /24, pushes peer to hub + node
 6. secrets-bootstrap:
      - host presents one-time join token (TTL 10m) → terrapi vault
      - vault issues short-lived AppRole/role creds; demon NEVER holds
        long-lived tenant secrets in SQLite
 7. converge: run apply-*.sh for the host's role bundle (idempotent)
 8. verify: run check-*.sh; diff line-contracts → must be all-green
 9. register: insert into demon inventory (host, role, residency, image,
      wg_ip, fleet, tenant_id?), append hash-chained audit entry
10. enroll telemetry poller
```

Steps 3–9 are a **guarded mutation**: dry-run renders the full plan; admin
confirms; rollback path defined per step (e.g. fail at 7 → provider.destroy +
WG peer revoke + vault token revoke). Idempotent re-run resumes at first
non-converged step.

---

## 2. Per-tenant enterprise lifecycle (Linux)

Each tenant = a row in `tenants` + ≥1 host(s) + a pinned **release manifest**
(`tenant.lock`: stack-component versions, feature flags, residency, sizing).

### 2.1 Provision tenant

`tenant.create(name, residency, plan)` → provisions host(s) per §1, then applies
the tenant overlay: dedicated vault namespace, dedicated kalista vhost + WAF
profile, vulture+sync workers scoped to the tenant DB/schema, identity realm,
**a dedicated OpenSearch index/alias inside the residency's cluster** (small
tenants share a cluster with per-tenant index + RBAC; large tenants get a
dedicated cluster — capacity model §4 decides).

### 2.2 Upgrade

- **Version pinning:** `tenant.lock` is the source of truth; demon never
  upgrades implicitly. `tenant.plan-upgrade(target_release)` diffs locked vs
  target and renders a per-component plan.
- **Strategy:** **blue/green for stateful API tiers** (vulture/identity) —
  stand up green host(s), converge, smoke-check via `check-*.sh`, flip kalista
  upstream, drain blue. **Rolling for stateless workers** (sync). DB migrations
  run expand/contract (backward-compatible) so blue+green coexist.
- Per-tenant canary: upgrade one tenant from a release ring before fleet-wide.

### 2.3 Scale / decommission

- Scale = §4 actions scoped to the tenant's residency + tenant_id.
- Decommission: backup (§6) → confirm → drain → destroy hosts → revoke WG +
  vault namespace → archive audit → mark tenant `retired` (retain audit/backup
  per residency retention policy).

### 2.4 Isolation

Hard: separate WG IPs, vault namespace, OpenSearch RBAC, kalista WAF profile,
DB schema/instance. Cross-tenant data access is structurally impossible, not
policy-gated.

---

## 3. Bottleneck evaluation engine

### 3.1 Signals collected (pulled over SSH, per service)

| Tier | Signals |
|---|---|
| Host | CPU%, load, mem/swap, disk %/inode, IO wait, fs latency |
| OpenSearch | JVM heap %, GC pause, shard count/size, unassigned shards, indexing rate, search p95, rejected-queue, pending tasks |
| kalista (proxy/WAF) | conns, req/s, upstream 5xx, queue/latency p95, WAF block rate |
| vulture / sync | req p95, worker-job backlog (queue depth), error rate, pool saturation |
| DB | active/idle connections, pool wait, replication lag, slow-query rate |
| WireGuard | throughput, retransmits, handshake failures |
| Worker jobs | backlog depth, age of oldest job, throughput |

Stored as compact rolling windows in SQLite (downsampled; raw stays in
OpenSearch). Demon does **not** become a metrics DB — it keeps enough to reason.

### 3.2 Detection — rules + confidence, not ML

A **rule engine** of typed predicates over windows. Each rule emits a
**finding** with a `confidence ∈ {high, medium, low}`:

- *high*: signal over hard threshold AND sustained ≥ quiet-period AND a
  corroborating second signal (e.g. OS heap >85% **and** GC pause >1s **and**
  rejected-queue >0 → "OpenSearch heap-bound", high).
- *medium*: single saturated signal, or threshold crossed but short.
- *low*: trend/forecast only (e.g. disk will fill in <7d).

Cross-tier correlation: latency at kalista + healthy vulture CPU + DB pool-wait
high → root cause ranked to **DB**, not the proxy.

### 3.3 Output

Ranked findings: `{tier, host/tenant, symptom, root-cause-guess, confidence,
evidence[], recommended_action, est_impact, est_cost}`. The recommended_action
links directly to a §4 scaling action or §6 runbook — one click from diagnosis
to (guarded) fix.

---

## 4. Horizontal scaling actions

For each tier, a typed `ScaleAction` with a **capacity model** (target metric +
headroom). Demon computes N from the model, never a blind "+1".

| Tier | Action | Capacity model |
|---|---|---|
| OpenSearch | add **data** nodes (heap-bound/disk-bound) or **coordinating** nodes (search-bound); trigger shard rebalance; raise replica/shard count for new indices | nodes = ceil(active_shards × avg_shard_size / per-node_disk_target) and heap target ≤ 75% |
| kalista | spawn N proxy instances + register in LB/health-check pool | conns / per-instance_conn_budget, +30% headroom |
| vulture/sync | add app-worker hosts; register upstream | backlog drain-time target + p95 SLO |
| DB | add **read replica**; route reads via kalista/app config | read-qps / per-replica_budget; replication-lag ceiling |

### Rebalance specifics (OpenSearch)

After adding nodes: set allocation awareness = residency+host, raise
`cluster.routing.allocation` concurrency briefly, watch unassigned→0 and
relocating→0 before declaring done. Throttle to protect indexing p95.

### Execution modes

- **Guided** (default): demon proposes N + plan; admin confirms each step.
- **Safe-auto** (opt-in per tier): demon executes within preset bounds
  (max nodes, max cost/day) only for *high*-confidence findings, with quiet-period.
- **Manual**: admin specifies N directly.

---

## 5. Scaling guardrails

- **Residency HARD gate:** every spawn inherits the source group's residency;
  the provider selector physically cannot place an EU node in UAE infra or mix
  them in one cluster. Asserted twice (plan + apply) and audited.
- **Dry-run mandatory** for auto; renders resource diff + cost estimate.
- **Cost awareness:** each action carries est. monthly delta; budget ceiling per
  fleet/tenant; breach → block + require explicit override.
- **Anti-flap:** quiet-period (e.g. 15 min) + cooldown after any scale event;
  scale-down requires *longer* sustained low-load than scale-up did high.
- **Quorum:** never scale a stateful cluster below quorum on scale-down; OS
  scale-down drains shards first.
- **Rollback:** every scale action records an inverse plan (destroy added nodes,
  de-register from LB, revoke WG/vault) executable as one guarded mutation.

---

## 6. Runbooks / guided ops (first-class)

Routine day-2 ops are typed, guarded actions — each = plan + confirm + apply +
`check-*.sh` verify + audit. Ships with: restart service, rotate cert (via
vault), run/restore backup, **drain node** (deregister LB → wait connections →
mark cordoned), apply pkg update + **refresh checksums** (pkg, then re-run FIM
baseline + `check-*.sh`, diff, commit new baseline), WG peer rotate, reindex.
New admin picks from a catalog; demon shows blast-radius + dry-run before apply.
Backups are per-tenant, residency-local, encrypted, restore-tested on a schedule.

---

## 7. Provider abstraction

One trait, many backends — birth + power + teardown only; convergence stays
agentless-SSH and is provider-agnostic.

```
trait Provider {
  fn create(spec) -> Host;          // metal/VM/jail/cloud
  fn destroy(host);
  fn power(host, on/off/cycle);     // IPMI/redfish or cloud API
  fn network_attach(host, wg_plan);
  fn capabilities() -> {bare_metal, jails, autoscale, snapshot, cost_api};
}
impls: FreeBSDMetalProvider (IPMI+PXE+ZFS), FreeBSDJailProvider (bastille/iocage),
       LinuxCloudProvider (Terraform .tf.json + cloud-init), AwsProvider, ...
```

Inventory model is provider-neutral; the demon reasons about *roles, residency,
fleet, tenant, capacity* — never about "an EC2 instance".

---

## 8. Open questions / risks (for the build agent)

1. **Terraform state ownership** — store encrypted blob in SQLite vs a real
   remote backend per residency? Lean SQLite-in-vault; validate locking.
2. **Agentless poll cost at fleet scale** — re-evaluate the no-agent stance if
   poll cycle > ~2 min; possible middle ground: a tiny read-only telemetry shim.
3. **OpenSearch shared-vs-dedicated cutover** — moving a growing tenant from a
   shared cluster to dedicated needs a reindex/snapshot-restore migration runbook.
4. **PXE/IPMI access for FreeBSD metal** — requires out-of-band network reachable
   from demon; document the mgmt-LAN trust boundary.
5. **Confidence calibration** — thresholds start hand-tuned; need a feedback loop
   (did the recommended action resolve the finding?) before any safe-auto.
6. **Cost API coverage** — bare-metal has no cloud cost API; need a static
   cost model per metal SKU.
7. **Blue/green DB migration safety** — enforce expand/contract; how to gate a
   migration that isn't backward-compatible.
