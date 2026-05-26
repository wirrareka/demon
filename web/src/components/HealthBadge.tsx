import type { HealthStatus } from "../lib/api";
import { HEALTH } from "../lib/health";
import { Badge } from "./ui/badge";
import { cn } from "../lib/utils";

export function HealthBadge({ status }: { status: HealthStatus }) {
  const h = HEALTH[status];
  return (
    <Badge className={h.cls}>
      <span className={cn("size-1.5 rounded-full", h.dot)} />
      {h.label}
    </Badge>
  );
}
