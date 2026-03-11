import { ConfidenceMeter } from "@/components/dashboard/confidence-meter";
import { StatusPill } from "@/components/dashboard/status-pill";
import { ForexPairInsight } from "@/lib/forex-intelligence";
import { cn } from "@/lib/utils";

export function ForexPairTile({
  insight,
  selected,
  onSelect,
}: {
  insight: ForexPairInsight;
  selected: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onSelect}
      className={cn(
        "min-w-0 border border-border bg-card px-3 py-3 text-left transition-all hover:border-cyan/30 hover:bg-card-alt",
        selected && "border-cyan/50 bg-cyan/8",
      )}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="text-[12px] font-semibold tracking-[0.4px]">{insight.pair.symbol}</div>
          <div className="mt-1 text-[10px] uppercase tracking-[0.8px] text-text-muted">
            {insight.pair.session}
          </div>
        </div>
        <StatusPill value={insight.coverageStatus} className="shrink-0" />
      </div>

      <div className="mt-3 flex items-center justify-between gap-3 text-[10px] uppercase tracking-[0.8px] text-text-muted">
        <span>{insight.bias}</span>
        <span>{insight.timeframe}</span>
      </div>

      <ConfidenceMeter confidence={insight.confidence} className="mt-3" />
    </button>
  );
}

