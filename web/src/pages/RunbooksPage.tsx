import { useEffect, useState } from "react";
import { api, type RunbookCatalogEntry, type RunbookRun } from "../lib/api";
import { stateClass } from "../lib/jobstate";
import { Card, CardHeader, CardTitle, CardContent } from "../components/ui/card";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";

export function RunbooksPage({ onStarted }: { onStarted: () => void }) {
  const [catalog, setCatalog] = useState<RunbookCatalogEntry[]>([]);
  const [target, setTarget] = useState("host:core-1/service:opensearch");
  const [run, setRun] = useState<RunbookRun | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  useEffect(() => {
    api
      .runbooks.catalog()
      .then(setCatalog)
      .catch((e) => setError(e instanceof Error ? e.message : String(e)));
  }, []);

  const start = async (id: string) => {
    setBusy(id);
    setError(null);
    try {
      setRun(await api.runbooks.start(id, target));
      onStarted();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="flex flex-col gap-4">
      {error && (
        <Card className="border-danger-border bg-danger-bg px-4 py-2 text-sm text-danger-fg">
          {error}
        </Card>
      )}

      <Card>
        <CardHeader className="flex items-center justify-between gap-3">
          <CardTitle>Target</CardTitle>
          <input
            value={target}
            onChange={(e) => setTarget(e.target.value)}
            className="w-96 rounded-md border border-border bg-bg px-2 py-1 font-mono text-xs text-fg outline-none focus:border-accent"
            placeholder="host:<id>/service:<kind>"
          />
        </CardHeader>
        <CardContent className="grid grid-cols-1 gap-3 sm:grid-cols-2">
          {catalog.map((rb) => (
            <div
              key={rb.id}
              className="flex flex-col gap-2 rounded-md border border-border bg-surface-2 p-3"
            >
              <div className="flex items-center justify-between">
                <div>
                  <div className="text-sm font-medium">{rb.title}</div>
                  <div className="font-mono text-[11px] text-muted">{rb.id}</div>
                </div>
                <Button disabled={busy === rb.id} onClick={() => start(rb.id)}>
                  Start
                </Button>
              </div>
              <ol className="ml-4 list-decimal text-xs text-fg-2">
                {rb.steps.map((s, i) => (
                  <li key={i}>
                    <span className="font-mono">{s.action}</span> — {s.description}
                  </li>
                ))}
              </ol>
            </div>
          ))}
        </CardContent>
      </Card>

      {run && (
        <Card>
          <CardHeader>
            <CardTitle>
              Run {run.runbook} → {run.target} · {run.current}/{run.total}{" "}
              <Badge className={stateClass(run.status)}>{run.status}</Badge>
            </CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-2">
            {run.steps.map((st, i) => (
              <div key={st.job_id} className="flex items-center gap-2 text-sm">
                <span className="text-muted">{i + 1}.</span>
                <span className="font-mono text-xs">{st.action}</span>
                <Badge className={stateClass(st.state)}>{st.state}</Badge>
                {st.dual_control && (
                  <Badge className="bg-warning-bg text-warning-fg border-warning-border">
                    dual-control
                  </Badge>
                )}
                <span className="text-xs text-muted">{st.description}</span>
              </div>
            ))}
            <p className="text-xs text-muted">
              Each step is a guarded job — confirm &amp; apply them in order on the Jobs
              page.
            </p>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
