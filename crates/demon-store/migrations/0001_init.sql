-- proximiio.demon — initial schema (Phase 0).
-- One database per residency group; this file is region-agnostic (the region is a
-- compile-time Store<R> parameter + the runtime gate, not a column scoping rows).

CREATE TABLE residency_groups (
    id     INTEGER PRIMARY KEY,
    region TEXT NOT NULL UNIQUE CHECK (region IN ('eu', 'uae'))
);

CREATE TABLE hosts (
    id              TEXT PRIMARY KEY,
    fqdn            TEXT NOT NULL,
    fleet           TEXT NOT NULL CHECK (fleet IN ('core', 'tenant')),
    os              TEXT NOT NULL,
    residency_group TEXT NOT NULL,
    wg_ip           TEXT,
    tenant_id       TEXT,
    enrolled_at     INTEGER NOT NULL,
    last_seen       INTEGER
);

CREATE TABLE tenants (
    id              TEXT PRIMARY KEY,           -- = Vulture organization_id (UUID v4)
    name            TEXT NOT NULL,
    residency_group TEXT NOT NULL,
    lifecycle_state TEXT NOT NULL,
    plan            TEXT,
    created_at      INTEGER NOT NULL
);

CREATE TABLE services (
    id              TEXT PRIMARY KEY,
    host_id         TEXT NOT NULL REFERENCES hosts(id),
    kind            TEXT NOT NULL,
    version         TEXT,
    residency_group TEXT NOT NULL,
    desired_state   TEXT,
    observed_state  TEXT,
    updated_at      INTEGER NOT NULL
);

CREATE TABLE health_snapshots (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    target_id   TEXT NOT NULL,
    target_kind TEXT NOT NULL,
    area        TEXT NOT NULL,
    status      TEXT NOT NULL,            -- up | degraded | down | unknown
    raw_json    TEXT NOT NULL,
    observed_at INTEGER NOT NULL
);
CREATE INDEX idx_health_target ON health_snapshots(target_id, observed_at);

CREATE TABLE jobs (
    id           TEXT PRIMARY KEY,
    kind         TEXT NOT NULL,
    target_id    TEXT,
    params_json  TEXT NOT NULL DEFAULT '{}',
    state        TEXT NOT NULL,
    dry_run      INTEGER NOT NULL DEFAULT 0,
    started_at   INTEGER,
    finished_at  INTEGER,
    result_json  TEXT,
    requested_by TEXT NOT NULL
);

-- Append-only hash chain. seq is contiguous from 0; prev_hash links to the prior row.
CREATE TABLE audit (
    seq              INTEGER PRIMARY KEY,
    prev_hash        TEXT NOT NULL,
    hash             TEXT NOT NULL UNIQUE,
    actor            TEXT NOT NULL,
    action           TEXT NOT NULL,
    target           TEXT NOT NULL,
    dry_run          INTEGER NOT NULL,
    redacted_payload TEXT NOT NULL,
    ts               INTEGER NOT NULL
);

CREATE TABLE alerts (
    id         TEXT PRIMARY KEY,
    target_id  TEXT NOT NULL,
    severity   TEXT NOT NULL,
    state      TEXT NOT NULL,
    opened_at  INTEGER NOT NULL,
    ack_by     TEXT,
    resolved_at INTEGER,
    runbook_id TEXT
);

CREATE TABLE runbook_runs (
    id           TEXT PRIMARY KEY,
    runbook_id   TEXT NOT NULL,
    state        TEXT NOT NULL,
    current_step INTEGER NOT NULL DEFAULT 0,
    context_json TEXT NOT NULL DEFAULT '{}',
    started_by   TEXT NOT NULL
);

CREATE TABLE desired_state (
    target_id TEXT NOT NULL,
    key       TEXT NOT NULL,
    value     TEXT NOT NULL,
    source    TEXT NOT NULL,
    PRIMARY KEY (target_id, key)
);

INSERT INTO residency_groups (id, region) VALUES (1, 'eu'), (2, 'uae');
