"use client";

import { motion } from "motion/react";
import { EmptyState } from "@/components/dashboard/empty-state";
import { PageTitle } from "@/components/dashboard/page-title";
import { StatusPill } from "@/components/dashboard/status-pill";
import { fetchDashboard } from "@/lib/intelligence-api";
import { formatDateTime, formatRelativeTime } from "@/lib/formatting";
import { usePollingResource } from "@/hooks/use-polling-resource";
import { cn } from "@/lib/utils";

function statusColor(status: string): string {
  switch (status) {
    case "completed": return "text-profit";
    case "running": return "text-cyan";
    case "failed": return "text-loss";
    default: return "text-text-muted";
  }
}

export default function AnalyticsPage() {
  const { data, loading, error } = usePollingResource("analytics-dashboard", fetchDashboard, {
    intervalMs: 60_000,
  });

  const schedules = data?.schedules ?? [];
  const enabledCount = schedules.filter((s) => s.enabled).length;
  const runningCount = schedules.filter((s) => s.last_status === "running").length;
  const failedCount = schedules.filter((s) => s.last_status === "failed").length;

  return (
    <div className="space-y-6 overflow-y-auto p-6 h-full">
      <motion.div initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.2 }}>
        <PageTitle
          title="Analytics"
          subtitle="Agent task schedules — lịch sử thực thi và trạng thái"
          breadcrumb="DASHBOARD / ANALYTICS"
        />
      </motion.div>

      {error && !data ? <EmptyState title="Không tải được dữ liệu" description={error} /> : null}

      {/* Stats Row */}
      <motion.div
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.2, delay: 0.05 }}
        className="grid grid-cols-3 gap-4"
      >
        <div className="border border-border bg-card p-4">
          <div className="text-[9px] font-bold tracking-wider text-text-muted">SCHEDULES</div>
          <div className="text-[24px] font-bold mt-1">{schedules.length}</div>
          <div className="text-[10px] text-text-secondary mt-1">
            {enabledCount} enabled
          </div>
        </div>
        <div className="border border-border bg-card p-4">
          <div className="text-[9px] font-bold tracking-wider text-text-muted">RUNNING</div>
          <div className={cn("text-[24px] font-bold mt-1", runningCount > 0 ? "text-cyan" : "")}>
            {runningCount}
          </div>
          <div className="text-[10px] text-text-secondary mt-1">task đang chạy</div>
        </div>
        <div className="border border-border bg-card p-4">
          <div className="text-[9px] font-bold tracking-wider text-text-muted">FAILED</div>
          <div className={cn("text-[24px] font-bold mt-1", failedCount > 0 ? "text-loss" : "")}>
            {failedCount}
          </div>
          <div className="text-[10px] text-text-secondary mt-1">task lỗi gần nhất</div>
        </div>
      </motion.div>

      {/* Schedules Table */}
      <motion.div
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.2, delay: 0.1 }}
        className="border border-border bg-card"
      >
        <div className="grid grid-cols-[1.2fr_0.8fr_0.8fr_1fr_0.8fr_0.6fr] border-b border-border bg-card-alt px-5 py-3 text-[10px] font-bold uppercase tracking-[1px] text-text-secondary">
          <div>Name</div>
          <div>Agent</div>
          <div>Status</div>
          <div>Last Run</div>
          <div>Next Run</div>
          <div>Enabled</div>
        </div>

        <div className="divide-y divide-border">
          {loading && !data ? (
            <div className="px-5 py-6 text-[12px] text-text-secondary animate-pulse">Đang tải schedules...</div>
          ) : null}
          {!loading && schedules.length === 0 ? (
            <div className="px-5 py-6">
              <EmptyState
                title="Chưa có schedule"
                description="Tạo schedule mới từ trang Agents."
              />
            </div>
          ) : null}
          {schedules.map((schedule, idx) => (
            <motion.div
              key={schedule.id}
              initial={{ opacity: 0, x: -6 }}
              animate={{ opacity: 1, x: 0 }}
              transition={{ duration: 0.15, delay: idx * 0.03 }}
            >
              <div className="grid grid-cols-[1.2fr_0.8fr_0.8fr_1fr_0.8fr_0.6fr] items-center gap-4 px-5 py-4 text-[12px] hover:bg-secondary/30 transition-colors">
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
                  <div>{formatRelativeTime(schedule.last_run_at)}</div>
                  <div className="mt-1 text-[10px] text-text-muted">{formatDateTime(schedule.last_run_at)}</div>
                </div>
                <div className="text-text-secondary">
                  <div>{formatRelativeTime(schedule.next_run_at)}</div>
                  <div className="mt-1 text-[10px] text-text-muted">{formatDateTime(schedule.next_run_at)}</div>
                </div>
                <div>
                  <StatusPill value={schedule.enabled ? "healthy" : "idle"} />
                </div>
              </div>
              {/* Message + Result Row */}
              {(schedule.message || schedule.last_result) && (
                <div className="px-5 pb-3 space-y-1">
                  {schedule.message && (
                    <div className="text-[10px] text-text-dim">
                      <span className="font-bold text-text-muted">MSG:</span> {schedule.message.length > 120 ? schedule.message.slice(0, 120) + "..." : schedule.message}
                    </div>
                  )}
                  {schedule.last_result && (
                    <div className="text-[10px] text-text-dim bg-secondary/50 px-3 py-1.5 border border-border/30 leading-relaxed">
                      <span className="font-bold text-text-muted">RESULT:</span> {schedule.last_result.length > 200 ? schedule.last_result.slice(0, 200) + "..." : schedule.last_result}
                    </div>
                  )}
                </div>
              )}
            </motion.div>
          ))}
        </div>
      </motion.div>
    </div>
  );
}
