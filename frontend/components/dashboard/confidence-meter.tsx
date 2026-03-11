import { cn } from "@/lib/utils";

export function ConfidenceMeter({
  confidence,
  className,
}: {
  confidence: number | null;
  className?: string;
}) {
  const pct = Math.round((confidence ?? 0) * 100);

  return (
    <div className={cn("space-y-1", className)}>
      <div className="flex items-center justify-between text-[10px] font-medium uppercase tracking-[0.8px] text-text-muted">
        <span>Confidence</span>
        <span>{confidence == null ? "--" : `${pct}%`}</span>
      </div>
      <div className="h-1.5 w-full overflow-hidden bg-tint">
        <div
          className="h-full bg-cyan transition-all"
          style={{ width: `${confidence == null ? 0 : pct}%` }}
        />
      </div>
    </div>
  );
}

