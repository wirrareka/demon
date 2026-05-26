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

export class ApiError extends Error {
  constructor(public status: number, message: string) {
    super(message);
  }
}

async function getJson<T>(path: string): Promise<T> {
  const res = await fetch(path, { headers: { accept: "application/json" } });
  if (!res.ok) {
    throw new ApiError(res.status, `${res.status} ${res.statusText}`);
  }
  return (await res.json()) as T;
}

export const api = {
  version: () => getJson<VersionInfo>("/version"),
  hosts: () => getJson<ListResponse<Host>>("/api/v1/hosts").then((r) => r.data),
  hostHealth: (id: string) =>
    getJson<ListResponse<HealthSnapshot>>(`/api/v1/hosts/${id}/health`).then((r) => r.data),
  tenants: () => getJson<ListResponse<Tenant>>("/api/v1/tenants").then((r) => r.data),
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
