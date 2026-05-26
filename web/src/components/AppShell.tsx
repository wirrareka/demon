import type { ReactNode } from "react";
import { Activity, Server, ListChecks, BookOpen, ScrollText, Building2 } from "lucide-react";
import { cn } from "../lib/utils";

export type Page = "fleet" | "jobs" | "runbooks" | "audit";

interface NavItem {
  page: Page | null;
  label: string;
  icon: ReactNode;
}

const NAV: NavItem[] = [
  { page: "fleet", label: "Fleet", icon: <Server className="size-4" /> },
  { page: "jobs", label: "Jobs", icon: <ListChecks className="size-4" /> },
  { page: "runbooks", label: "Runbooks", icon: <BookOpen className="size-4" /> },
  { page: "audit", label: "Audit", icon: <ScrollText className="size-4" /> },
  { page: null, label: "Tenants", icon: <Building2 className="size-4" /> },
];

const TITLES: Record<Page, string> = {
  fleet: "Fleet overview",
  jobs: "Guarded jobs",
  runbooks: "Runbooks",
  audit: "Audit trail",
};

export function AppShell({
  region,
  version,
  live,
  active,
  onNavigate,
  children,
}: {
  region: string;
  version: string;
  live: boolean;
  active: Page;
  onNavigate: (page: Page) => void;
  children: ReactNode;
}) {
  return (
    <div className="flex h-screen text-fg">
      <aside className="flex w-52 shrink-0 flex-col border-r border-border bg-surface">
        <div className="flex items-center gap-2 px-4 py-3.5 border-b border-border">
          <Activity className="size-4 text-accent" />
          <span className="text-sm font-semibold tracking-tight">proximiio.demon</span>
        </div>
        <nav className="flex-1 p-2">
          {NAV.map((item) => {
            const isActive = item.page === active;
            const disabled = item.page === null;
            return (
              <button
                key={item.label}
                type="button"
                disabled={disabled}
                onClick={() => item.page && onNavigate(item.page)}
                className={cn(
                  "flex w-full items-center gap-2.5 rounded-md px-3 py-2 text-left text-sm",
                  isActive && "bg-surface-2 font-medium text-fg",
                  disabled && "cursor-not-allowed text-muted-2",
                  !isActive && !disabled && "text-fg-2 hover:bg-surface-2",
                )}
                title={disabled ? "Coming in a later phase" : undefined}
              >
                {item.icon}
                {item.label}
              </button>
            );
          })}
        </nav>
        <div className="border-t border-border px-4 py-2 text-xs text-muted">
          v{version || "—"}
        </div>
      </aside>

      <div className="flex min-w-0 flex-1 flex-col">
        <header className="flex items-center justify-between border-b border-border bg-surface px-5 py-3">
          <h1 className="text-sm font-medium text-fg">{TITLES[active]}</h1>
          <div className="flex items-center gap-3 text-xs">
            <span className="rounded-md border border-border bg-surface-2 px-2 py-0.5 font-medium uppercase text-fg-2">
              {region || "—"}
            </span>
            <span className="flex items-center gap-1.5 text-muted">
              <span
                className={cn(
                  "size-2 rounded-full",
                  live ? "bg-success" : "bg-muted-2",
                )}
              />
              {live ? "live" : "offline"}
            </span>
          </div>
        </header>
        <main className="min-h-0 flex-1 overflow-auto bg-bg p-5">{children}</main>
      </div>
    </div>
  );
}
