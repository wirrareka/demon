import { useEffect, useState } from "react";
import { api, type AuditRecord } from "../lib/api";
import { Card } from "../components/ui/card";
import { Badge } from "../components/ui/badge";
import { Table, THead, TBody, TR, TH, TD } from "../components/ui/table";
import { Loading } from "../components/ui/spinner";

function fmtTs(ms: number): string {
  return new Date(ms).toISOString().replace("T", " ").replace("Z", "");
}

export function AuditPage() {
  const [records, setRecords] = useState<AuditRecord[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [intact, setIntact] = useState<boolean | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.audit
      .list(200)
      .then(setRecords)
      .catch((e) => setError(String(e)))
      .finally(() => setLoaded(true));
    api.audit.verify().then((v) => setIntact(v.intact)).catch(() => setIntact(null));
  }, []);

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center gap-3">
        <span className="text-sm text-muted">Append-only, hash-chained.</span>
        {intact !== null && (
          <Badge
            className={
              intact
                ? "bg-success-bg text-success-fg border-success-border"
                : "bg-danger-bg text-danger-fg border-danger-border"
            }
          >
            chain {intact ? "intact" : "BROKEN"}
          </Badge>
        )}
      </div>
      {error && (
        <Card className="border-danger-border bg-danger-bg px-4 py-2 text-sm text-danger-fg">
          {error}
        </Card>
      )}
      <Card>
        <Table>
          <THead>
            <TR className="hover:bg-transparent">
              <TH>#</TH>
              <TH>Time (UTC)</TH>
              <TH>Actor</TH>
              <TH>Action</TH>
              <TH>Target</TH>
              <TH>Hash</TH>
            </TR>
          </THead>
          <TBody>
            {!loaded && (
              <TR className="hover:bg-transparent">
                <TD colSpan={6}>
                  <Loading />
                </TD>
              </TR>
            )}
            {loaded && records.length === 0 && (
              <TR className="hover:bg-transparent">
                <TD colSpan={6} className="py-6 text-center text-muted">
                  No audit records yet.
                </TD>
              </TR>
            )}
            {records.map((r) => (
              <TR key={r.seq}>
                <TD className="tnum text-muted">{r.seq}</TD>
                <TD className="tnum text-xs text-fg-2">{fmtTs(r.ts)}</TD>
                <TD className="text-fg-2">{r.actor}</TD>
                <TD className="font-mono text-xs">
                  {r.action}
                  {r.dry_run && <span className="ml-1 text-muted">(dry-run)</span>}
                </TD>
                <TD className="text-xs text-fg-2">{r.target}</TD>
                <TD className="font-mono text-[11px] text-muted">{r.hash.slice(0, 12)}…</TD>
              </TR>
            ))}
          </TBody>
        </Table>
      </Card>
    </div>
  );
}
