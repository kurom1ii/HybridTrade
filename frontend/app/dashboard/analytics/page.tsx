"use client";

import { EmptyState } from "@/components/dashboard/empty-state";
import { PageTitle } from "@/components/dashboard/page-title";
import { StatusPill } from "@/components/dashboard/status-pill";
import { fetchSchedules } from "@/lib/intelligence-api";
import { formatDateTime, formatRelativeTime } from "@/lib/formatting";
import { usePollingResource } from "@/hooks/use-polling-resource";

export default function SchedulesPage() {
  const { data, loading, error } = usePollingResource("schedules", fetchSchedules, {
    intervalMs: 15_000,
  });

  return (
    <div className="space-y-6 overflow-y-auto p-6 h-full">
      <PageTitle
        title="Schedules"
        subtitle="Cron jobs duoc luu trong SQLite va thuc thi boi scheduler in-process."
        breadcrumb="DASHBOARD / SCHEDULES"
      />

      {error && !data ? <EmptyState title="Khong tai duoc schedules" description={error} /> : null}

      <div className="border border-border bg-card">
        <div className="grid grid-cols-[1.4fr_1fr_1fr_1fr] border-b border-border bg-card-alt px-5 py-3 text-[10px] font-bold uppercase tracking-[1px] text-text-secondary">
          <div>Name</div>
          <div>Job</div>
          <div>Last Run</div>
          <div>Next Run</div>
        </div>

        <div className="divide-y divide-border">
          {loading && !data ? <div className="px-5 py-6 text-[12px] text-text-secondary">Dang tai schedules...</div> : null}
          {!loading && data && data.length === 0 ? (
            <div className="px-5 py-6">
              <EmptyState
                title="Chua co schedule"
                description="Cac cron job mac dinh se duoc bootstrap tu file cau hinh Rust khi backend khoi dong."
              />
            </div>
          ) : null}
          {data?.map((schedule) => (
            <div key={schedule.id} className="grid grid-cols-[1.4fr_1fr_1fr_1fr] items-center gap-4 px-5 py-4 text-[12px]">
              <div>
                <div className="font-semibold">{schedule.name}</div>
                <div className="mt-1 text-[10px] text-text-muted">{schedule.cron_expr}</div>
              </div>
              <div>
                <div>{schedule.job_type}</div>
                <div className="mt-1"><StatusPill value={schedule.enabled ? "healthy" : "idle"} /></div>
              </div>
              <div className="text-text-secondary">
                <div>{formatRelativeTime(schedule.last_run_at)}</div>
                <div className="mt-1 text-[10px] text-text-muted">{formatDateTime(schedule.last_run_at)}</div>
              </div>
              <div className="text-text-secondary">
                <div>{formatRelativeTime(schedule.next_run_at)}</div>
                <div className="mt-1 text-[10px] text-text-muted">{formatDateTime(schedule.next_run_at)}</div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

