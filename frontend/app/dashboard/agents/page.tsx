"use client";

import { motion } from "motion/react";
import { PageTitle } from "@/components/dashboard/page-title";
import { SlideIn, StaggerGrid } from "@/components/dashboard/motion-primitives";
import { StatsCard } from "@/components/dashboard/stats-card";
import { EmptyState } from "@/components/dashboard/empty-state";
import { StatusPill } from "@/components/dashboard/status-pill";
import { fetchDashboard } from "@/lib/intelligence-api";
import { formatRelativeTime, titleFromRole, truncate } from "@/lib/formatting";
import { usePollingResource } from "@/hooks/use-polling-resource";

export default function AgentsPage() {
  const { data, loading, error } = usePollingResource("agent-console", fetchDashboard, {
    intervalMs: 10_000,
  });

  return (
    <div className="flex h-full gap-6 overflow-y-auto p-6">
      <div className="min-w-0 flex-1 space-y-6">
        <PageTitle
          title="Agent Console"
          subtitle="Trang thai cac debug agent, heartbeat gan nhat, va danh sach investigation duoc luu trong backend."
          breadcrumb="DASHBOARD / AGENTS"
        />

        <StaggerGrid>
          <div className="grid grid-cols-3 gap-4">
            <StatsCard
              title="Healthy Agents"
              value={String(data?.agent_statuses.filter((item) => item.status === "healthy").length ?? 0)}
              change="Heartbeat va tool calls on dinh"
              changeType="profit"
            />
            <StatsCard
              title="Available Agents"
              value={String(data?.agent_statuses.length ?? 0)}
              change="So agent role backend expose cho debug chat"
              changeType="neutral"
            />
            <StatsCard
              title="Warnings"
              value={String(data?.agent_statuses.filter((item) => item.status === "warning" || item.status === "stale").length ?? 0)}
              change={error ? error : "Agent roles co warning/stale se can can thiệp"}
              changeType={(data?.agent_statuses.some((item) => item.status === "warning" || item.status === "stale") || Boolean(error)) ? "loss" : "neutral"}
            />
          </div>
        </StaggerGrid>

        <motion.div
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.4, delay: 0.15 }}
          className="grid grid-cols-2 gap-4"
        >
          {data?.agent_statuses.length ? (
            data.agent_statuses.map((agent) => (
              <div key={agent.role} className="border border-border bg-card-alt p-5">
                <div className="flex items-start justify-between gap-4">
                  <div>
                    <div className="text-[10px] font-bold uppercase tracking-[1px] text-text-secondary">
                      {titleFromRole(agent.role)}
                    </div>
                    <h3 className="mt-1 text-[16px] font-semibold">{agent.label}</h3>
                  </div>
                  <StatusPill value={agent.status} />
                </div>

                <div className="mt-4 flex items-center gap-3 text-[10px] uppercase tracking-[0.8px] text-text-muted">
                  <span>Heartbeat {formatRelativeTime(agent.last_seen_at)}</span>
                </div>

                <p className="mt-3 text-[11px] leading-relaxed text-text-secondary">
                  {agent.last_message ? truncate(agent.last_message, 180) : "Chua co message moi tu agent nay."}
                </p>
              </div>
            ))
          ) : (
            <div className="col-span-2">
              <EmptyState
                title="Chua co agent status"
                description={loading ? "Dang tai trang thai agent..." : "Khoi dong backend de xem heartbeat va capability cua agent."}
              />
            </div>
          )}
        </motion.div>
      </div>

      <SlideIn direction="right" delay={0.2}>
        <div className="-m-6 w-[340px] shrink-0 space-y-4 border-l border-border bg-panel p-5">
          <div>
            <h3 className="mb-3 text-[11px] font-bold uppercase tracking-[1px] text-text-secondary">Investigation Queue</h3>
            <div className="space-y-2">
              {data?.recent_investigations.length ? (
                data.recent_investigations.map((item) => (
                  <div key={item.id} className="border border-border bg-card px-3 py-2">
                    <div className="flex items-center justify-between gap-3">
                      <div className="min-w-0 text-[11px] font-semibold">{truncate(item.topic, 48)}</div>
                      <StatusPill value={item.status} className="shrink-0" />
                    </div>
                    <div className="mt-2 text-[10px] text-text-muted">{formatRelativeTime(item.updated_at)}</div>
                  </div>
                ))
              ) : (
                <EmptyState
                  title="Queue rong"
                  description="Khi co investigation moi, snapshot metadata se xuat hien tai day."
                />
              )}
            </div>
          </div>
        </div>
      </SlideIn>
    </div>
  );
}
