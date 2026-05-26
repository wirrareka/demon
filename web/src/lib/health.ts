import type { HealthStatus } from "./api";

/** Canonical status → label + badge classes, mapped to the kalista status hues. */
export const HEALTH: Record<HealthStatus, { label: string; cls: string; dot: string }> = {
  up: { label: "Up", cls: "bg-success-bg text-success-fg border-success-border", dot: "bg-success" },
  degraded: {
    label: "Degraded",
    cls: "bg-warning-bg text-warning-fg border-warning-border",
    dot: "bg-warning",
  },
  down: { label: "Down", cls: "bg-danger-bg text-danger-fg border-danger-border", dot: "bg-danger" },
  unknown: { label: "Unknown", cls: "bg-surface-2 text-muted border-border", dot: "bg-muted" },
};

const SEVERITY: Record<HealthStatus, number> = { up: 0, unknown: 1, degraded: 2, down: 3 };

/** Worst (most severe) status across a set — mirrors demon-core's rollup. */
export function worst(statuses: HealthStatus[]): HealthStatus {
  return statuses.reduce<HealthStatus>(
    (acc, s) => (SEVERITY[s] > SEVERITY[acc] ? s : acc),
    "up",
  );
}
