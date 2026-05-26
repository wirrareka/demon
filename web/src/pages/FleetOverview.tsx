import type { Host, HealthSnapshot, HealthStatus } from "../lib/api";
import { HEALTH, worst } from "../lib/health";
import { Card, CardContent } from "../components/ui/card";
import { Table, THead, TBody, TR, TH, TD } from "../components/ui/table";
import { HealthBadge } from "../components/HealthBadge";
import { cn } from "../lib/utils";

type HealthByHost = Record<string, HealthSnapshot[]>;

const ALL_STATUSES: HealthStatus[] = ["up", "degraded", "down", "unknown"];

function StatTile({ label, value, accent }: { label: string; value: number; accent?: string }) {
  return (
    <Card>
      <CardContent className="py-3">
        <div className="text-xs uppercase tracking-wide text-muted">{label}</div>
        <div className={cn("mt-1 text-2xl font-semibold tnum", accent)}>{value}</div>
      </CardContent>
    </Card>
  );
}

export function FleetOverview({
  hosts,
  healthByHost,
  error,
}: {
  hosts: Host[];
  healthByHost: HealthByHost;
  error: string | null;
}) {
  if (error) {
    return (
      <Card>
        <CardContent>
          <p className="text-sm text-danger-fg">Could not load fleet: {error}</p>
          <p className="mt-1 text-xs text-muted">
            The API is fail-closed — log in via <code>/auth/login</code>, or run the daemon with{" "}
            <code>DEMON_DEV_NO_AUTH=1</code> for a local trial.
          </p>
        </CardContent>
      </Card>
    );
  }

  const rollup = (id: string): HealthStatus =>
    worst((healthByHost[id] ?? []).map((s) => s.status));

  const counts = ALL_STATUSES.reduce<Record<HealthStatus, number>>(
    (acc, st) => {
      acc[st] = hosts.filter((h) => rollup(h.id) === st).length;
      return acc;
    },
    { up: 0, degraded: 0, down: 0, unknown: 0 },
  );

  return (
    <div className="flex flex-col gap-5">
      <div className="grid grid-cols-2 gap-3 sm:grid-cols-4 lg:grid-cols-5">
        <StatTile label="Hosts" value={hosts.length} />
        <StatTile label="Up" value={counts.up} accent="text-success" />
        <StatTile label="Degraded" value={counts.degraded} accent="text-warning" />
        <StatTile label="Down" value={counts.down} accent="text-danger" />
        <StatTile label="Unknown" value={counts.unknown} accent="text-muted" />
      </div>

      <Card>
        <Table>
          <THead>
            <TR className="hover:bg-transparent">
              <TH>Host</TH>
              <TH>Fleet</TH>
              <TH>OS</TH>
              <TH>WG IP</TH>
              <TH>Health</TH>
              <TH>Areas</TH>
            </TR>
          </THead>
          <TBody>
            {hosts.length === 0 && (
              <TR className="hover:bg-transparent">
                <TD colSpan={6} className="py-6 text-center text-muted">
                  No hosts enrolled.
                </TD>
              </TR>
            )}
            {hosts.map((h) => {
              const areas = (healthByHost[h.id] ?? [])
                .slice()
                .sort((a, b) => a.area.localeCompare(b.area));
              return (
                <TR key={h.id}>
                  <TD>
                    <div className="font-medium">{h.fqdn}</div>
                    <div className="text-xs text-muted">{h.id}</div>
                  </TD>
                  <TD className="text-fg-2">{h.fleet}</TD>
                  <TD className="text-fg-2">{h.os}</TD>
                  <TD className="font-mono text-xs text-fg-2">{h.wg_ip ?? "—"}</TD>
                  <TD>
                    <HealthBadge status={rollup(h.id)} />
                  </TD>
                  <TD>
                    <div className="flex flex-wrap gap-1.5">
                      {areas.length === 0 && <span className="text-xs text-muted">—</span>}
                      {areas.map((a) => (
                        <span
                          key={a.area}
                          title={`${a.area}: ${HEALTH[a.status].label}`}
                          className={cn(
                            "inline-flex items-center gap-1 rounded border px-1.5 py-0.5 text-[11px]",
                            HEALTH[a.status].cls,
                          )}
                        >
                          <span className={cn("size-1.5 rounded-full", HEALTH[a.status].dot)} />
                          {a.area}
                        </span>
                      ))}
                    </div>
                  </TD>
                </TR>
              );
            })}
          </TBody>
        </Table>
      </Card>
    </div>
  );
}
