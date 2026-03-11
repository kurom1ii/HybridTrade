import { StatusPill } from "@/components/dashboard/status-pill";
import { truncate } from "@/lib/formatting";
import { SubagentTask } from "@/lib/forex-intelligence";

export function SubagentTaskCard({ task }: { task: SubagentTask }) {
  return (
    <div className="border border-border bg-card-alt p-4">
      <div className="flex items-start justify-between gap-4">
        <div>
          <div className="text-[10px] font-bold uppercase tracking-[1px] text-text-secondary">
            {task.role}
          </div>
          <h4 className="mt-1 text-[12px] font-semibold">{task.task}</h4>
        </div>
        <StatusPill value={task.status} className="shrink-0" />
      </div>
      <p className="mt-3 text-[11px] leading-relaxed text-text-secondary">{truncate(task.note, 220)}</p>
    </div>
  );
}

