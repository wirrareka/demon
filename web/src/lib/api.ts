// Typed client for the demon REST + WS API. Same-origin in dev via the Vite proxy.

export type Region = "eu" | "uae";
export type Fleet = "core" | "tenant";
export type HealthStatus = "up" | "degraded" | "down" | "unknown";

export interface Host {
  id: string;
  fqdn: string;
  fleet: Fleet;
  os: string;
  residency_group: Region;
  wg_ip: string | null;
  tenant_id: string | null;
  enrolled_at: number;
  last_seen: number | null;
}

export interface HealthSnapshot {
  target_id: string;
  target_kind: string;
  area: string;
  status: HealthStatus;
  raw_json: string;
  observed_at: number;
}

export interface Tenant {
  id: string;
  name: string;
  residency_group: Region;
  lifecycle_state: string;
  plan: string | null;
  created_at: number;
}

export interface VersionInfo {
  service: string;
  version: string;
  region: string;
}

interface ListResponse<T> {
  data: T[];
  available_actions: string[];
}

export interface Job {
  id: string;
  action: string;
  target: string;
  state: string;
  confirm_phrase: string;
  dual_control: boolean;
  dual_satisfied: boolean;
  report: string | null;
}

export interface ApplyResult {
  state: string;
  dry_run_output: string | null;
  apply_output: string | null;
  verify_status: string | null;
  error: string | null;
}

export interface RunbookCatalogEntry {
  id: string;
  title: string;
  steps: { action: string; description: string }[];
}

export interface RunStep {
  job_id: string;
  action: string;
  description: string;
  confirm_phrase: string;
  dual_control: boolean;
  state: string;
}

export interface RunbookRun {
  run_id: string;
  runbook: string;
  target: string;
  status: string;
  current: number;
  total: number;
  steps: RunStep[];
}

export interface AuditRecord {
  seq: number;
  prev_hash: string;
  hash: string;
  actor: string;
  action: string;
  target: string;
  dry_run: boolean;
  redacted_payload: string;
  ts: number;
}

export class ApiError extends Error {
  constructor(public status: number, message: string) {
    super(message);
  }
}

async function readError(res: Response): Promise<string> {
  try {
    const body = (await res.json()) as { error?: string };
    if (body.error) return body.error;
  } catch {
    /* not json */
  }
  return `${res.status} ${res.statusText}`;
}

async function getJson<T>(path: string): Promise<T> {
  const res = await fetch(path, { headers: { accept: "application/json" } });
  if (!res.ok) throw new ApiError(res.status, await readError(res));
  return (await res.json()) as T;
}

async function postJson<T>(path: string, body?: unknown): Promise<T> {
  const res = await fetch(path, {
    method: "POST",
    headers: { "content-type": "application/json", accept: "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (!res.ok) throw new ApiError(res.status, await readError(res));
  return (await res.json()) as T;
}

export const api = {
  version: () => getJson<VersionInfo>("/version"),
  hosts: () => getJson<ListResponse<Host>>("/api/v1/hosts").then((r) => r.data),
  hostHealth: (id: string) =>
    getJson<ListResponse<HealthSnapshot>>(`/api/v1/hosts/${id}/health`).then((r) => r.data),
  tenants: () => getJson<ListResponse<Tenant>>("/api/v1/tenants").then((r) => r.data),
  jobs: {
    list: () => getJson<{ data: Job[] }>("/api/v1/jobs").then((r) => r.data),
    create: (action: string, target: string) =>
      postJson<Job>("/api/v1/jobs", { action, target }),
    confirm: (id: string, typed: string) =>
      postJson<Job>(`/api/v1/jobs/${id}/confirm`, { typed }),
    apply: (id: string) => postJson<ApplyResult>(`/api/v1/jobs/${id}/apply`),
  },
  runbooks: {
    catalog: () =>
      getJson<{ data: RunbookCatalogEntry[] }>("/api/v1/runbooks").then((r) => r.data),
    start: (id: string, target: string) =>
      postJson<RunbookRun>(`/api/v1/runbooks/${id}/runs`, { target }),
  },
  audit: {
    list: (limit = 100) =>
      getJson<ListResponse<AuditRecord>>(`/api/v1/audit?limit=${limit}`).then((r) => r.data),
    verify: () => getJson<{ intact: boolean }>("/api/v1/audit/verify"),
  },
};

/** Subscribe to the live health-snapshot WebSocket. Returns the socket. */
export function openStream(onSnapshot: (s: HealthSnapshot) => void): WebSocket {
  const url = `${location.origin.replace(/^http/, "ws")}/api/v1/stream`;
  const ws = new WebSocket(url);
  ws.onmessage = (e) => {
    try {
      onSnapshot(JSON.parse(e.data as string) as HealthSnapshot);
    } catch {
      /* ignore malformed frames */
    }
  };
  return ws;
}
