import { useEffect, useState, useCallback } from "react";
import { api, ApiError, type Job } from "../lib/api";
import { registerPasskey, stepUp } from "../lib/webauthn";
import { stateClass } from "../lib/jobstate";
import { Card } from "../components/ui/card";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import { Table, THead, TBody, TR, TH, TD } from "../components/ui/table";
import { cn } from "../lib/utils";

export function JobsPage() {
  const [jobs, setJobs] = useState<Job[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  const reload = useCallback(async () => {
    try {
      setJobs(await api.jobs.list());
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  const act = async (fn: () => Promise<unknown>, key: string) => {
    setBusy(key);
    setError(null);
    try {
      await fn();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(null);
      void reload();
    }
  };

  // Apply with automatic touch-per-op step-up: if the gate returns 403, run a fresh
  // WebAuthn assertion scoped to this job, then retry.
  const applyJob = async (id: string) => {
    setBusy(id);
    setError(null);
    try {
      await api.jobs.apply(id);
    } catch (e) {
      if (e instanceof ApiError && e.status === 403) {
        try {
          await stepUp(id);
          await api.jobs.apply(id);
        } catch (e2) {
          setError(e2 instanceof Error ? e2.message : String(e2));
        }
      } else {
        setError(e instanceof Error ? e.message : String(e));
      }
    } finally {
      setBusy(null);
      void reload();
    }
  };

  const register = async () => {
    setError(null);
    try {
      await registerPasskey();
      setError("passkey registered ✓");
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <p className="text-xs text-muted">
          Destructive / secret / CA actions require a touch-per-op WebAuthn step-up on Apply.
        </p>
        <Button onClick={register}>Register passkey</Button>
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
              <TH>Action</TH>
              <TH>Target</TH>
              <TH>State</TH>
              <TH>Dual-control</TH>
              <TH>Actions</TH>
            </TR>
          </THead>
          <TBody>
            {jobs.length === 0 && (
              <TR className="hover:bg-transparent">
                <TD colSpan={5} className="py-6 text-center text-muted">
                  No jobs yet — start one from Runbooks, or POST /api/v1/jobs.
                </TD>
              </TR>
            )}
            {jobs.map((j) => {
              const canConfirm = j.state === "planned" || j.state === "awaiting_approval";
              const canApply = j.state === "confirmed";
              return (
                <TR key={j.id}>
                  <TD className="font-mono text-xs">{j.action}</TD>
                  <TD className="text-fg-2">{j.target}</TD>
                  <TD>
                    <Badge className={stateClass(j.state)}>{j.state}</Badge>
                  </TD>
                  <TD className="text-xs text-muted">
                    {j.dual_control ? (j.dual_satisfied ? "satisfied" : "required") : "—"}
                  </TD>
                  <TD>
                    <div className="flex gap-1.5">
                      <Button
                        disabled={!canConfirm || busy === j.id}
                        onClick={() => act(() => api.jobs.confirm(j.id, j.confirm_phrase), j.id)}
                        title={`Types: "${j.confirm_phrase}"`}
                      >
                        Confirm
                      </Button>
                      <Button
                        disabled={!canApply || busy === j.id}
                        className={cn(canApply && "border-danger-border text-danger-fg")}
                        onClick={() => applyJob(j.id)}
                      >
                        Apply
                      </Button>
                    </div>
                    {j.report && <div className="mt-1 text-[11px] text-muted">{j.report}</div>}
                  </TD>
                </TR>
              );
            })}
          </TBody>
        </Table>
      </Card>
      <p className="text-xs text-muted">
        Confirm types the exact phrase for you. Dual-control actions (cert / drain /
        scale-out) need a distinct approver — a single operator cannot self-approve.
      </p>
    </div>
  );
}
