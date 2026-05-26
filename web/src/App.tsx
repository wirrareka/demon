import { useEffect, useState } from "react";
import { AppShell, type Page } from "./components/AppShell";
import { FleetOverview } from "./pages/FleetOverview";
import { JobsPage } from "./pages/JobsPage";
import { RunbooksPage } from "./pages/RunbooksPage";
import { AuditPage } from "./pages/AuditPage";
import { TenantsPage } from "./pages/TenantsPage";
import { api, openStream, type Host, type HealthSnapshot } from "./lib/api";

export function App() {
  const [region, setRegion] = useState("");
  const [version, setVersion] = useState("");
  const [hosts, setHosts] = useState<Host[]>([]);
  const [healthByHost, setHealthByHost] = useState<Record<string, HealthSnapshot[]>>({});
  const [live, setLive] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [page, setPage] = useState<Page>("fleet");

  // Version is public (works even when the API is gated); fleet needs auth/dev-mode.
  useEffect(() => {
    let cancelled = false;
    api
      .version()
      .then((v) => {
        if (!cancelled) {
          setRegion(v.region);
          setVersion(v.version);
        }
      })
      .catch(() => {});

    (async () => {
      try {
        const hs = await api.hosts();
        if (cancelled) return;
        setHosts(hs);
        const entries = await Promise.all(
          hs.map(async (h) => [h.id, await api.hostHealth(h.id)] as const),
        );
        if (!cancelled) setHealthByHost(Object.fromEntries(entries));
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : String(e));
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

  // Live health stream: upsert the latest snapshot per (host, area).
  useEffect(() => {
    const ws = openStream((snap) => {
      setHealthByHost((prev) => {
        const rest = (prev[snap.target_id] ?? []).filter((s) => s.area !== snap.area);
        return { ...prev, [snap.target_id]: [...rest, snap] };
      });
    });
    ws.onopen = () => setLive(true);
    ws.onclose = () => setLive(false);
    ws.onerror = () => setLive(false);
    return () => ws.close();
  }, []);

  return (
    <AppShell region={region} version={version} live={live} active={page} onNavigate={setPage}>
      {page === "fleet" && (
        <FleetOverview hosts={hosts} healthByHost={healthByHost} error={error} />
      )}
      {page === "jobs" && <JobsPage />}
      {page === "runbooks" && <RunbooksPage onStarted={() => undefined} />}
      {page === "audit" && <AuditPage />}
      {page === "tenants" && <TenantsPage />}
    </AppShell>
  );
}
