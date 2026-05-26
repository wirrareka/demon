import { Loader2 } from "lucide-react";
import { cn } from "../../lib/utils";

/** Inline "loading…" row used while a page's first fetch is in flight. */
export function Loading({ label = "Loading…", className }: { label?: string; className?: string }) {
  return (
    <div className={cn("flex items-center gap-2 py-6 text-sm text-muted", className)}>
      <Loader2 className="size-4 animate-spin" />
      {label}
    </div>
  );
}
