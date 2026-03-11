import { cn } from "@/lib/utils";

const STATUS_STYLES: Record<string, string> = {
  queued: "bg-secondary text-text-secondary",
  running: "bg-cyan-dim text-cyan",
  completed: "bg-profit/15 text-profit",
  covered: "bg-profit/15 text-profit",
  uncovered: "bg-secondary text-text-muted",
  awaiting: "bg-secondary text-text-secondary",
  bullish: "bg-profit/15 text-profit",
  bearish: "bg-loss/15 text-loss",
  mixed: "bg-warning-dim text-warning",
  waiting_followup: "bg-warning-dim text-warning",
  failed: "bg-loss/15 text-loss",
  stale: "bg-loss/15 text-loss",
  delayed: "bg-warning-dim text-warning",
  healthy: "bg-profit/15 text-profit",
  warning: "bg-warning-dim text-warning",
  idle: "bg-secondary text-text-secondary",
  error: "bg-loss/15 text-loss",
  connected: "bg-profit/15 text-profit",
  connecting: "bg-warning-dim text-warning",
};

export function StatusPill({ value, className }: { value: string; className?: string }) {
  const normalized = value.toLowerCase();
  return (
    <span
      className={cn(
        "inline-flex items-center px-2 py-0.5 text-[10px] font-bold uppercase tracking-[0.6px]",
        STATUS_STYLES[normalized] ?? "bg-secondary text-text-secondary",
        className,
      )}
    >
      {normalized.replace(/_/g, " ")}
    </span>
  );
}
