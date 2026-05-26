import type { ReactNode } from "react";
import { Activity, Server, ScrollText, BookOpen, Building2 } from "lucide-react";
import { cn } from "../lib/utils";

interface NavItem {
  label: string;
  icon: ReactNode;
  active?: boolean;
  disabled?: boolean;
}

const NAV: NavItem[] = [
  { label: "Fleet", icon: <Server className="size-4" />, active: true },
  { label: "Audit", icon: <ScrollText className="size-4" />, disabled: true },
  { label: "Runbooks", icon: <BookOpen className="size-4" />, disabled: true },
  { label: "Tenants", icon: <Building2 className="size-4" />, disabled: true },
];

export function AppShell({
  region,
  version,
  live,
  children,
}: {
  region: string;
  version: string;
  live: boolean;
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
          {NAV.map((item) => (
            <div
              key={item.label}
              className={cn(
                "flex items-center gap-2.5 rounded-md px-3 py-2 text-sm",
                item.active && "bg-surface-2 font-medium text-fg",
                item.disabled && "cursor-not-allowed text-muted-2",
                !item.active && !item.disabled && "text-fg-2 hover:bg-surface-2",
              )}
              title={item.disabled ? "Coming in a later phase" : undefined}
            >
              {item.icon}
              {item.label}
            </div>
          ))}
        </nav>
        <div className="border-t border-border px-4 py-2 text-xs text-muted">
          v{version || "—"}
        </div>
      </aside>

      <div className="flex min-w-0 flex-1 flex-col">
        <header className="flex items-center justify-between border-b border-border bg-surface px-5 py-3">
          <h1 className="text-sm font-medium text-fg">Fleet overview</h1>
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
