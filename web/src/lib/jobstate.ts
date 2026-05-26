/** Job/run state → badge classes (kalista status hues). */
export function stateClass(state: string): string {
  switch (state) {
    case "verified":
    case "completed":
      return "bg-success-bg text-success-fg border-success-border";
    case "failed":
    case "rolled_back":
      return "bg-danger-bg text-danger-fg border-danger-border";
    case "applying":
    case "in_progress":
      return "bg-info-bg text-info-fg border-info-border";
    case "awaiting_approval":
      return "bg-warning-bg text-warning-fg border-warning-border";
    default:
      return "bg-surface-2 text-muted border-border";
  }
}
