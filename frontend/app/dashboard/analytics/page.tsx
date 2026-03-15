"use client";

import { useState, useEffect, useCallback, useMemo, memo } from "react";
import { motion, AnimatePresence } from "motion/react";
import { EmptyState } from "@/components/dashboard/empty-state";
import { PageTitle } from "@/components/dashboard/page-title";
import { StatusPill } from "@/components/dashboard/status-pill";
import { fetchDashboard } from "@/lib/intelligence-api";
import { formatDateTime, formatRelativeTime, formatCountdown } from "@/lib/formatting";
import { usePollingResource } from "@/hooks/use-polling-resource";
import { useScheduleStream } from "@/hooks/useScheduleStream";
import { useTick } from "@/hooks/use-tick";
import { cn } from "@/lib/utils";
import type { ScheduleView, ScheduleResultDetail, ToolCallDetail } from "@/lib/intelligence-types";

// ─── Helpers ───

function parseResult(raw?: string | null): ScheduleResultDetail | null {
  if (!raw) return null;
  try { return JSON.parse(raw); } catch { return null; }
}

function statusColor(status: string): string {
  switch (status) {
    case "completed": return "text-profit";
    case "running": return "text-cyan";
    case "failed": return "text-loss";
    default: return "text-text-muted";
  }
}

function toolStatusColor(status: string): string {
  if (status === "ok" || status === "success") return "text-profit";
  if (status === "error" || status === "failed") return "text-loss";
  return "text-warning";
}

// ─── LiveTime — uses shared tick ───

function LiveTime({ value, mode = "relative" }: { value?: string | null; mode?: "relative" | "countdown" }) {
  useTick();
  return <>{mode === "countdown" ? formatCountdown(value) : formatRelativeTime(value)}</>;
}

// ─── Chevron Icon ───

function ChevronIcon({ open, className }: { open: boolean; className?: string }) {
  return (
    <svg
      className={cn("h-3 w-3 transition-transform duration-200", open && "rotate-90", className)}
      viewBox="0 0 12 12"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
    >
      <path d="M4 2L8 6L4 10" />
    </svg>
  );
}

// ─── Section Component ───

type SectionKey = "prompt" | "result" | "tools" | "system_prompt";

function Section({
  label,
  badge,
  open,
  onToggle,
  children,
}: {
  label: string;
  badge?: string;
  open: boolean;
  onToggle: () => void;
  children: React.ReactNode;
}) {
  return (
    <div>
      <button
        onClick={onToggle}
        className="flex items-center gap-2 w-full text-left py-1.5 hover:bg-secondary/20 transition-colors px-1 -mx-1 rounded"
      >
        <ChevronIcon open={open} className="text-text-muted shrink-0" />
        <span className="text-[10px] uppercase tracking-[1px] font-bold text-text-muted">
          {label}
        </span>
        {badge && (
          <span className="text-[9px] font-mono text-text-secondary bg-secondary/40 px-1.5 py-0.5 rounded">
            {badge}
          </span>
        )}
      </button>
      <AnimatePresence>
        {open && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.15 }}
            className="overflow-hidden"
          >
            <div className="bg-secondary/20 border-l-2 border-cyan/30 pl-4 py-2 mt-1 mb-2">
              {children}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

// ─── Tool Call Row (memoized) ───

const ToolCallRow = memo(function ToolCallRow({ tc }: { tc: ToolCallDetail }) {
  const [showInput, setShowInput] = useState(false);
  const [showOutput, setShowOutput] = useState(true);
  const hasInput = tc.input && Object.keys(tc.input).length > 0;
  return (
    <div className="py-2">
      <div className="flex items-center gap-2 text-[11px]">
        <span className="font-semibold text-foreground">{tc.name}</span>
        <span className="text-text-muted font-mono text-[9px]">[{tc.source}]</span>
        <span className={cn("font-bold text-[10px]", toolStatusColor(tc.status))}>{tc.status}</span>
      </div>

      {hasInput && (
        <div className="mt-1">
          <button
            onClick={() => setShowInput(!showInput)}
            className="text-[9px] text-text-secondary hover:text-cyan transition-colors flex items-center gap-1"
          >
            <ChevronIcon open={showInput} className="h-2 w-2" />
            Input
          </button>
          <AnimatePresence>
            {showInput && (
              <motion.pre
                initial={{ height: 0, opacity: 0 }}
                animate={{ height: "auto", opacity: 1 }}
                exit={{ height: 0, opacity: 0 }}
                transition={{ duration: 0.1 }}
                className="overflow-hidden text-[10px] font-mono text-text-dim bg-panel/50 px-3 py-1.5 mt-1 rounded max-h-40 overflow-y-auto whitespace-pre-wrap break-all"
              >
                {JSON.stringify(tc.input, null, 2)}
              </motion.pre>
            )}
          </AnimatePresence>
        </div>
      )}

      {tc.output_preview && (
        <div className="mt-1">
          <button
            onClick={() => setShowOutput(!showOutput)}
            className="text-[9px] text-text-secondary hover:text-cyan transition-colors flex items-center gap-1"
          >
            <ChevronIcon open={showOutput} className="h-2 w-2" />
            Output
            <span className="text-text-muted font-mono">({tc.output_preview.length} chars)</span>
          </button>
          <AnimatePresence>
            {showOutput && (
              <motion.pre
                initial={{ height: 0, opacity: 0 }}
                animate={{ height: "auto", opacity: 1 }}
                exit={{ height: 0, opacity: 0 }}
                transition={{ duration: 0.1 }}
                className="overflow-hidden text-[10px] font-mono text-foreground/70 bg-panel/50 px-3 py-1.5 mt-1 rounded max-h-60 overflow-y-auto whitespace-pre-wrap break-all leading-relaxed"
              >
                {tc.output_preview}
              </motion.pre>
            )}
          </AnimatePresence>
        </div>
      )}
    </div>
  );
});

// ─── Expandable Schedule Row (memoized) ───

const ScheduleRow = memo(function ScheduleRow({
  schedule,
  detail,
  expanded,
  onToggle,
  idx,
}: {
  schedule: ScheduleView;
  detail: ScheduleResultDetail | null;
  expanded: boolean;
  onToggle: () => void;
  idx: number;
}) {
  const [openSections, setOpenSections] = useState<Set<SectionKey>>(new Set(["result", "tools", "system_prompt"]));

  const toggleSection = useCallback((key: SectionKey) =>
    setOpenSections((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    }), []);

  return (
    <motion.div
      initial={{ opacity: 0, x: -6 }}
      animate={{ opacity: 1, x: 0 }}
      transition={{ duration: 0.15, delay: idx * 0.02 }}
    >
      {/* Header Row */}
      <button
        onClick={onToggle}
        className="w-full grid grid-cols-[1.2fr_0.8fr_0.8fr_1fr_0.8fr_0.6fr_24px] items-center gap-4 px-5 py-4 text-[12px] hover:bg-secondary/30 transition-colors text-left"
      >
        <div>
          <div className="font-semibold">{schedule.name}</div>
          <div className="mt-1 text-[10px] text-text-muted font-mono">{schedule.cron_expr}</div>
        </div>
        <div>
          <div className="text-[11px] font-semibold">{schedule.agent_role}</div>
        </div>
        <div>
          <div className={cn("text-[11px] font-bold uppercase", statusColor(schedule.last_status))}>
            {schedule.last_status}
          </div>
        </div>
        <div className="text-text-secondary">
          <div><LiveTime value={schedule.last_run_at} mode="relative" /></div>
          <div className="mt-1 text-[10px] text-text-muted">{formatDateTime(schedule.last_run_at)}</div>
        </div>
        <div className="text-text-secondary">
          <div className="text-cyan font-mono font-bold"><LiveTime value={schedule.next_run_at} mode="countdown" /></div>
          <div className="mt-1 text-[10px] text-text-muted">{formatDateTime(schedule.next_run_at)}</div>
        </div>
        <div>
          <StatusPill value={schedule.enabled ? "healthy" : "idle"} />
        </div>
        <div className="flex items-center justify-center">
          <ChevronIcon open={expanded} className="text-text-muted" />
        </div>
      </button>

      {/* Expandable Detail */}
      <AnimatePresence>
        {expanded && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.2 }}
            className="overflow-hidden"
          >
            <div className="px-5 pb-4 space-y-1 border-t border-border/30">
              {/* PROMPT */}
              <Section
                label="Prompt"
                badge={schedule.message ? `${schedule.message.length} chars` : undefined}
                open={openSections.has("prompt")}
                onToggle={() => toggleSection("prompt")}
              >
                <pre className="text-[11px] font-mono whitespace-pre-wrap text-text-dim leading-relaxed max-h-40 overflow-y-auto">
                  {schedule.message || "(empty)"}
                </pre>
              </Section>

              {/* RESULT */}
              <Section
                label="Result"
                badge={detail ? `${detail.provider} / ${detail.model}` : undefined}
                open={openSections.has("result")}
                onToggle={() => toggleSection("result")}
              >
                {detail ? (
                  <pre className="text-[11px] font-mono whitespace-pre-wrap text-foreground/80 leading-relaxed max-h-80 overflow-y-auto">
                    {detail.content}
                  </pre>
                ) : schedule.last_result ? (
                  <pre className="text-[11px] font-mono whitespace-pre-wrap text-foreground/80 leading-relaxed max-h-80 overflow-y-auto">
                    {schedule.last_result}
                  </pre>
                ) : (
                  <span className="text-[11px] text-text-muted italic">No result yet</span>
                )}
              </Section>

              {/* TOOLS */}
              {detail && detail.tool_calls.length > 0 && (
                <Section
                  label="Tools"
                  badge={`${detail.tool_calls.length} call${detail.tool_calls.length > 1 ? "s" : ""}`}
                  open={openSections.has("tools")}
                  onToggle={() => toggleSection("tools")}
                >
                  <div className="space-y-1 divide-y divide-border/20">
                    {detail.tool_calls.map((tc, i) => (
                      <ToolCallRow key={`${tc.name}-${i}`} tc={tc} />
                    ))}
                  </div>
                </Section>
              )}

              {/* SYSTEM PROMPT */}
              {detail?.system_prompt && (
                <Section
                  label="System Prompt"
                  badge={`${detail.system_prompt.length} chars`}
                  open={openSections.has("system_prompt")}
                  onToggle={() => toggleSection("system_prompt")}
                >
                  <pre className="text-[11px] font-mono whitespace-pre-wrap text-text-dim leading-relaxed overflow-y-auto">
                    {detail.system_prompt}
                  </pre>
                </Section>
              )}

              {/* Available Tools List */}
              {detail?.available_tools && detail.available_tools.length > 0 && (
                <div className="pt-1">
                  <div className="text-[9px] uppercase tracking-[1px] text-text-muted font-bold">
                    Available Tools ({detail.available_tools.length})
                  </div>
                  <div className="flex flex-wrap gap-1 mt-1">
                    {detail.available_tools.map((t) => (
                      <span
                        key={t}
                        className="text-[9px] font-mono bg-secondary/40 text-text-secondary px-1.5 py-0.5 rounded"
                      >
                        {t}
                      </span>
                    ))}
                  </div>
                </div>
              )}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </motion.div>
  );
});

// ─── Main Page ───

export default function AnalyticsPage() {
  const { data, loading, error, reload } = usePollingResource("analytics-dashboard", fetchDashboard, {
    intervalMs: 10_000,
  });

  const { connected } = useScheduleStream({
    onJobUpdate: useCallback(() => {
      reload();
    }, [reload]),
  });

  const [expandedRows, setExpandedRows] = useState<Set<string>>(new Set());

  const toggleRow = useCallback((id: string) =>
    setExpandedRows((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    }), []);

  const schedules = data?.schedules ?? [];

  // Pre-parse all results once per data fetch — avoids JSON.parse inside each row render
  const parsedDetails = useMemo(() => {
    const map = new Map<string, ScheduleResultDetail | null>();
    for (const s of schedules) {
      map.set(s.id, parseResult(s.last_result));
    }
    return map;
  }, [schedules]);

  const enabledCount = schedules.filter((s) => s.enabled).length;
  const runningCount = schedules.filter((s) => s.last_status === "running").length;
  const failedCount = schedules.filter((s) => s.last_status === "failed").length;

  return (
    <div className="space-y-6 overflow-y-auto p-6 h-full">
      <motion.div initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.2 }}>
        <PageTitle
          title="Analytics"
          subtitle="Agent task schedules — execution history and status"
          breadcrumb="DASHBOARD / ANALYTICS"
        />
      </motion.div>

      {error && !data ? <EmptyState title="Failed to load data" description={error} /> : null}

      {/* Stats Row */}
      <motion.div
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.2, delay: 0.05 }}
        className="grid grid-cols-4 gap-4"
      >
        <div className="border border-border bg-card p-4">
          <div className="text-[9px] font-bold tracking-wider text-text-muted">SCHEDULES</div>
          <div className="text-[24px] font-bold mt-1">{schedules.length}</div>
          <div className="text-[10px] text-text-secondary mt-1">{enabledCount} enabled</div>
        </div>
        <div className="border border-border bg-card p-4">
          <div className="text-[9px] font-bold tracking-wider text-text-muted">RUNNING</div>
          <div className={cn("text-[24px] font-bold mt-1", runningCount > 0 ? "text-cyan" : "")}>
            {runningCount}
          </div>
          <div className="text-[10px] text-text-secondary mt-1">active tasks</div>
        </div>
        <div className="border border-border bg-card p-4">
          <div className="text-[9px] font-bold tracking-wider text-text-muted">FAILED</div>
          <div className={cn("text-[24px] font-bold mt-1", failedCount > 0 ? "text-loss" : "")}>
            {failedCount}
          </div>
          <div className="text-[10px] text-text-secondary mt-1">recent errors</div>
        </div>
        <div className="border border-border bg-card p-4">
          <div className="text-[9px] font-bold tracking-wider text-text-muted">STREAM</div>
          <div className="flex items-center gap-2 mt-2">
            <div className={cn("h-2 w-2 rounded-full", connected ? "bg-profit animate-pulse" : "bg-text-muted")} />
            <span className="text-[11px] text-text-secondary">{connected ? "Live" : "Offline"}</span>
          </div>
          <div className="text-[10px] text-text-secondary mt-1">SSE connection</div>
        </div>
      </motion.div>

      {/* Schedules Table */}
      <motion.div
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.2, delay: 0.1 }}
        className="border border-border bg-card"
      >
        <div className="grid grid-cols-[1.2fr_0.8fr_0.8fr_1fr_0.8fr_0.6fr_24px] border-b border-border bg-card-alt px-5 py-3 text-[10px] font-bold uppercase tracking-[1px] text-text-secondary">
          <div>Name</div>
          <div>Agent</div>
          <div>Status</div>
          <div>Last Run</div>
          <div>Next Run</div>
          <div>Enabled</div>
          <div></div>
        </div>

        <div className="divide-y divide-border">
          {loading && !data ? (
            <div className="px-5 py-6 text-[12px] text-text-secondary animate-pulse">Loading schedules...</div>
          ) : null}
          {!loading && schedules.length === 0 ? (
            <div className="px-5 py-6">
              <EmptyState title="No schedules" description="Create a new schedule from the Agents page." />
            </div>
          ) : null}
          {schedules.map((schedule, idx) => (
            <ScheduleRow
              key={schedule.id}
              schedule={schedule}
              detail={parsedDetails.get(schedule.id) ?? null}
              expanded={expandedRows.has(schedule.id)}
              onToggle={() => toggleRow(schedule.id)}
              idx={idx}
            />
          ))}
        </div>
      </motion.div>
    </div>
  );
}
