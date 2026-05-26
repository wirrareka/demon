import { useEffect, useState } from "react";
import { ArrowLeft } from "lucide-react";
import { api, type Host, type HealthSnapshot, type LoadReport } from "../lib/api";
import { Card, CardHeader, CardTitle, CardContent } from "../components/ui/card";
import { Button } from "../components/ui/button";
import { Badge } from "../components/ui/badge";
import { HealthBadge } from "../components/HealthBadge";
import type { HealthStatus } from "../lib/api";
import { cn } from "../lib/utils";

function Bar({ label, pct }: { label: string; pct: number }) {
  const v = Math.max(0, Math.min(100, pct));
  const tone = v >= 90 ? "bg-danger" : v >= 80 ? "bg-warning" : "bg-success";
  return (
    <div className="flex items-center gap-3">
      <span className="w-12 text-xs text-muted">{label}</span>
      <div className="h-2 flex-1 overflow-hidden rounded bg-surface-2">
        <div className={cn("h-full rounded", tone)} style={{ width: `${v}%` }} />
      </div>
      <span className="tnum w-12 text-right text-xs text-fg-2">{v.toFixed(0)}%</span>
    </div>
  );
}

export function HostDetailPage({
  hostId,
  onBack,
  onPlanned,
}: {
  hostId: string;
  onBack: () => void;
  onPlanned: () => void;
}) {
  const [host, setHost] = useState<Host | null>(null);
  const [actions, setActions] = useState<string[]>([]);
  const [health, setHealth] = useState<HealthSnapshot[]>([]);
  const [load, setLoad] = useState<LoadReport | null>(null);
  const [loadErr, setLoadErr] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  useEffect(() => {
    api
      .host(hostId)
      .then((r) => {
        setHost(r.data);
        setActions(r.available_actions);
      })
      .catch((e) => setError(String(e)));
    api.hostHealth(hostId).then(setHealth).catch(() => undefined);
    api
      .hostLoad(hostId)
      .then(setLoad)
      .catch((e) => setLoadErr(e instanceof Error ? e.message : String(e)));
  }, [hostId]);

  const plan = async (action: string) => {
    setBusy(action);
    setError(null);
    try {
      await api.jobs.create(action, `host:${hostId}`);
      onPlanned();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="flex flex-col gap-4">
      <button
        type="button"
        onClick={onBack}
        className="flex w-fit items-center gap-1 text-sm text-fg-2 hover:text-fg"
      >
        <ArrowLeft className="size-4" /> Fleet
      </button>

      {error && (
        <Card className="border-danger-border bg-danger-bg px-4 py-2 text-sm text-danger-fg">
          {error}
        </Card>
      )}

      <Card>
        <CardHeader>
          <CardTitle>{host?.fqdn ?? hostId}</CardTitle>
        </CardHeader>
        <CardContent className="grid grid-cols-2 gap-2 text-sm sm:grid-cols-4">
          <Meta k="ID" v={hostId} />
          <Meta k="Fleet" v={host?.fleet ?? "—"} />
          <Meta k="OS" v={host?.os ?? "—"} />
          <Meta k="WG IP" v={host?.wg_ip ?? "—"} mono />
          <Meta k="Residency" v={host?.residency_group ?? "—"} />
          <Meta k="Tenant" v={host?.tenant_id ?? "—"} mono />
        </CardContent>
      </Card>

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Health by area</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-wrap gap-2">
            {health.length === 0 && <span className="text-sm text-muted">No health observed yet.</span>}
            {health
              .slice()
              .sort((a, b) => a.area.localeCompare(b.area))
              .map((h) => (
                <span key={h.area} className="flex items-center gap-1.5">
                  <span className="text-xs text-muted">{h.area}</span>
                  <HealthBadge status={h.status as HealthStatus} />
                </span>
              ))}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Load &amp; pressure</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-2">
            {loadErr && <span className="text-xs text-muted">Metrics unavailable: {loadErr}</span>}
            {load && (
              <>
                <Bar label="cpu" pct={load.load.cpu_pct} />
                <Bar label="mem" pct={load.load.mem_pct} />
                <Bar label="disk" pct={load.load.disk_pct} />
                <div className="mt-1 flex flex-wrap gap-1.5">
                  {load.findings.length === 0 && (
                    <span className="text-xs text-muted">No pressure findings.</span>
                  )}
                  {load.findings.map((f) => (
                    <Badge
                      key={f.signal}
                      className="bg-warning-bg text-warning-fg border-warning-border"
                      title={f.detail}
                    >
                      {f.signal}: {f.confidence}
                    </Badge>
                  ))}
                </div>
              </>
            )}
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Available actions</CardTitle>
        </CardHeader>
        <CardContent className="flex flex-wrap gap-2">
          {actions.length === 0 && (
            <span className="text-sm text-muted">
              No actions available for your role.
            </span>
          )}
          {actions.map((a) => (
            <Button key={a} disabled={busy === a} onClick={() => plan(a)}>
              {a}
            </Button>
          ))}
        </CardContent>
      </Card>
      {actions.length > 0 && (
        <p className="text-xs text-muted">
          Planning an action creates a guarded job — confirm &amp; apply it on the Jobs page.
        </p>
      )}
    </div>
  );
}

function Meta({ k, v, mono }: { k: string; v: string; mono?: boolean }) {
  return (
    <div>
      <div className="text-xs uppercase tracking-wide text-muted">{k}</div>
      <div className={cn("text-fg-2", mono && "font-mono text-xs")}>{v}</div>
    </div>
  );
}
