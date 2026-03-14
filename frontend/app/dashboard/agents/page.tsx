"use client";

import { useState, useMemo } from "react";
import { motion, AnimatePresence } from "motion/react";
import { PageTitle } from "@/components/dashboard/page-title";
import { StaggerGrid } from "@/components/dashboard/motion-primitives";
import { StatsCard } from "@/components/dashboard/stats-card";
import { EmptyState } from "@/components/dashboard/empty-state";
import { StatusPill } from "@/components/dashboard/status-pill";
import { fetchDashboard, createSchedule } from "@/lib/intelligence-api";
import { formatRelativeTime, titleFromRole, truncate } from "@/lib/formatting";
import { usePollingResource } from "@/hooks/use-polling-resource";
import { cn } from "@/lib/utils";

// ─── Visual Cron Picker ───
type FreqMode = "minutes" | "hourly" | "daily" | "weekly" | "custom";

const WEEKDAYS = [
  { value: 0, label: "CN" },
  { value: 1, label: "T2" },
  { value: 2, label: "T3" },
  { value: 3, label: "T4" },
  { value: 4, label: "T5" },
  { value: 5, label: "T6" },
  { value: 6, label: "T7" },
];

function buildCron(mode: FreqMode, everyMin: number, hour: number, minute: number, weekdays: number[]): string {
  switch (mode) {
    case "minutes":
      return `*/${everyMin} * * * *`;
    case "hourly":
      return `${minute} * * * *`;
    case "daily":
      return `${minute} ${hour} * * *`;
    case "weekly": {
      const days = weekdays.length > 0 ? weekdays.sort().join(",") : "*";
      return `${minute} ${hour} * * ${days}`;
    }
    case "custom":
      return `*/${everyMin} * * * *`;
  }
}

function describeCron(mode: FreqMode, everyMin: number, hour: number, minute: number, weekdays: number[]): string {
  const hh = String(hour).padStart(2, "0");
  const mm = String(minute).padStart(2, "0");
  switch (mode) {
    case "minutes":
      return `Mỗi ${everyMin} phút`;
    case "hourly":
      return `Mỗi giờ vào phút ${mm}`;
    case "daily":
      return `Mỗi ngày lúc ${hh}:${mm}`;
    case "weekly": {
      const dayNames = weekdays.length > 0
        ? weekdays.sort().map((d) => WEEKDAYS.find((w) => w.value === d)?.label).join(", ")
        : "tất cả các ngày";
      return `${dayNames} lúc ${hh}:${mm}`;
    }
    default:
      return `Mỗi ${everyMin} phút`;
  }
}

function CronPicker({ value, onChange }: { value: string; onChange: (cron: string) => void }) {
  const [mode, setMode] = useState<FreqMode>("minutes");
  const [everyMin, setEveryMin] = useState(5);
  const [hour, setHour] = useState(9);
  const [minute, setMinute] = useState(0);
  const [weekdays, setWeekdays] = useState<number[]>([1, 2, 3, 4, 5]);

  const cron = useMemo(() => buildCron(mode, everyMin, hour, minute, weekdays), [mode, everyMin, hour, minute, weekdays]);
  const description = useMemo(() => describeCron(mode, everyMin, hour, minute, weekdays), [mode, everyMin, hour, minute, weekdays]);

  const updateParent = (newMode: FreqMode, newMin: number, newHour: number, newMinute: number, newWeekdays: number[]) => {
    onChange(buildCron(newMode, newMin, newHour, newMinute, newWeekdays));
  };

  const toggleWeekday = (d: number) => {
    const next = weekdays.includes(d) ? weekdays.filter((w) => w !== d) : [...weekdays, d];
    setWeekdays(next);
    updateParent(mode, everyMin, hour, minute, next);
  };

  return (
    <div className="space-y-3">
      <div>
        <div className="text-[10px] font-bold tracking-wider text-text-muted mb-2">TẦN SUẤT</div>
        <div className="flex gap-1">
          {([
            { v: "minutes" as const, l: "Mỗi X phút" },
            { v: "hourly" as const, l: "Mỗi giờ" },
            { v: "daily" as const, l: "Mỗi ngày" },
            { v: "weekly" as const, l: "Theo tuần" },
          ]).map((opt) => (
            <button
              key={opt.v}
              onClick={() => { setMode(opt.v); updateParent(opt.v, everyMin, hour, minute, weekdays); }}
              className={cn(
                "px-3 py-1.5 text-[10px] font-semibold transition-colors",
                mode === opt.v
                  ? "bg-cyan/10 text-cyan border border-cyan/30"
                  : "bg-secondary text-text-muted border border-border hover:text-foreground"
              )}
            >
              {opt.l}
            </button>
          ))}
        </div>
      </div>

      {mode === "minutes" && (
        <div>
          <div className="text-[10px] font-bold tracking-wider text-text-muted mb-2">CHẠY MỖI</div>
          <div className="flex items-center gap-2">
            {[1, 2, 3, 5, 10, 15, 30].map((m) => (
              <button
                key={m}
                onClick={() => { setEveryMin(m); updateParent(mode, m, hour, minute, weekdays); }}
                className={cn(
                  "px-3 py-1.5 text-[11px] font-bold transition-colors min-w-[40px]",
                  everyMin === m
                    ? "bg-cyan/15 text-cyan border border-cyan/30"
                    : "bg-secondary text-text-muted border border-border hover:text-foreground"
                )}
              >
                {m}p
              </button>
            ))}
          </div>
        </div>
      )}

      {(mode === "hourly" || mode === "daily" || mode === "weekly") && (
        <div className="flex gap-4">
          {(mode === "daily" || mode === "weekly") && (
            <div>
              <div className="text-[10px] font-bold tracking-wider text-text-muted mb-2">GIỜ</div>
              <select
                value={hour}
                onChange={(e) => { const h = Number(e.target.value); setHour(h); updateParent(mode, everyMin, h, minute, weekdays); }}
                className="bg-secondary border border-border px-3 py-1.5 text-[12px] font-mono focus:border-cyan/50 focus:outline-none"
              >
                {Array.from({ length: 24 }, (_, i) => (
                  <option key={i} value={i}>{String(i).padStart(2, "0")}h</option>
                ))}
              </select>
            </div>
          )}
          <div>
            <div className="text-[10px] font-bold tracking-wider text-text-muted mb-2">PHÚT</div>
            <select
              value={minute}
              onChange={(e) => { const m = Number(e.target.value); setMinute(m); updateParent(mode, everyMin, hour, m, weekdays); }}
              className="bg-secondary border border-border px-3 py-1.5 text-[12px] font-mono focus:border-cyan/50 focus:outline-none"
            >
              {Array.from({ length: 60 }, (_, i) => (
                <option key={i} value={i}>{String(i).padStart(2, "0")}</option>
              ))}
            </select>
          </div>
        </div>
      )}

      {mode === "weekly" && (
        <div>
          <div className="text-[10px] font-bold tracking-wider text-text-muted mb-2">NGÀY TRONG TUẦN</div>
          <div className="flex gap-1">
            {WEEKDAYS.map((d) => (
              <button
                key={d.value}
                onClick={() => toggleWeekday(d.value)}
                className={cn(
                  "w-10 py-1.5 text-[11px] font-bold transition-colors",
                  weekdays.includes(d.value)
                    ? "bg-cyan/15 text-cyan border border-cyan/30"
                    : "bg-secondary text-text-muted border border-border hover:text-foreground"
                )}
              >
                {d.label}
              </button>
            ))}
          </div>
        </div>
      )}

      <div className="flex items-center gap-3 bg-secondary/50 px-3 py-2 border border-border/50">
        <span className="text-[10px] font-bold text-text-muted">KẾT QUẢ:</span>
        <span className="text-[12px] font-semibold text-cyan">{description}</span>
        <span className="text-[10px] font-mono text-text-faint ml-auto">{cron}</span>
      </div>
    </div>
  );
}

// ─── Presets ───
const presetJobs = [
  { name: "XAU/USD Analysis", cron: "*/5 * * * *", role: "kuromi", message: "Phân tích kỹ thuật XAU/USD: xu hướng, hỗ trợ/kháng cự, tín hiệu giao dịch.", desc: "Phân tích XAU/USD mỗi 5 phút" },
  { name: "News Digest", cron: "0 */6 * * *", role: "kuromi", message: "Tổng hợp tin tức tài chính quan trọng trong 6 giờ qua.", desc: "Tổng hợp tin tức mỗi 6 giờ" },
  { name: "Market Overview", cron: "0 * * * *", role: "kuromi", message: "Tổng quan thị trường: các cặp tiền tệ chính, vàng, chỉ số.", desc: "Tổng quan thị trường mỗi giờ" },
];

function statusColor(status: string): string {
  switch (status) {
    case "completed": return "text-profit";
    case "running": return "text-cyan";
    case "failed": return "text-loss";
    default: return "text-text-muted";
  }
}

export default function AgentsPage() {
  const { data, loading, error, reload } = usePollingResource("agent-console", fetchDashboard, {
    intervalMs: 60_000,
  });

  const [showForm, setShowForm] = useState(false);
  const [formName, setFormName] = useState("");
  const [formCron, setFormCron] = useState("*/5 * * * *");
  const [formAgentRole, setFormAgentRole] = useState("kuromi");
  const [formMessage, setFormMessage] = useState("");
  const [formEnabled, setFormEnabled] = useState(true);
  const [submitting, setSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [submitSuccess, setSubmitSuccess] = useState(false);

  const schedules = data?.schedules ?? [];
  const activeSchedules = schedules.filter((s) => s.enabled);
  const runningTasks = schedules.filter((s) => s.last_status === "running");

  const handlePreset = (preset: typeof presetJobs[number]) => {
    setFormName(preset.name);
    setFormCron(preset.cron);
    setFormAgentRole(preset.role);
    setFormMessage(preset.message);
    setShowForm(true);
  };

  const handleSubmit = async () => {
    if (!formName.trim() || !formCron.trim() || !formMessage.trim()) return;
    setSubmitting(true);
    setSubmitError(null);
    setSubmitSuccess(false);
    try {
      await createSchedule({
        name: formName.trim(),
        cron_expr: formCron.trim(),
        job_type: "agent_task",
        enabled: formEnabled,
        agent_role: formAgentRole.trim(),
        message: formMessage.trim(),
      });
      setSubmitSuccess(true);
      setFormName("");
      setFormMessage("");
      setFormCron("*/5 * * * *");
      setTimeout(() => {
        setShowForm(false);
        setSubmitSuccess(false);
        reload();
      }, 1500);
    } catch (err) {
      setSubmitError(err instanceof Error ? err.message : "Không thể tạo schedule");
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="flex h-full gap-6 overflow-y-auto p-6">
      <div className="min-w-0 flex-1 space-y-6">
        <motion.div initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.2 }}>
          <PageTitle
            title="Agent Console"
            subtitle="Quản lý agents và lập lịch tự động gửi task tới agent"
            breadcrumb="DASHBOARD / AGENTS"
          />
        </motion.div>

        <StaggerGrid>
          <div className="grid grid-cols-3 gap-4">
            <StatsCard
              title="Active Agents"
              value={String(data?.agent_statuses.length ?? 0)}
              change="Agent đã đăng ký"
              changeType="profit"
            />
            <StatsCard
              title="Schedules"
              value={String(activeSchedules.length)}
              change={`${schedules.length} tổng cộng`}
              changeType="neutral"
            />
            <StatsCard
              title="Running Tasks"
              value={String(runningTasks.length)}
              change="Task đang chạy"
              changeType={runningTasks.length > 0 ? "profit" : "neutral"}
            />
          </div>
        </StaggerGrid>

        {/* Agent Cards */}
        <motion.div
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.25, delay: 0.1 }}
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
                  <span>Last run {formatRelativeTime(agent.last_seen_at)}</span>
                  {agent.open_runs > 0 && (
                    <span className="text-cyan font-bold">{agent.open_runs} RUNNING</span>
                  )}
                </div>
                <p className="mt-3 text-[11px] leading-relaxed text-text-secondary">
                  {agent.last_message ? truncate(agent.last_message, 180) : "Chưa có kết quả task mới."}
                </p>
              </div>
            ))
          ) : (
            <div className="col-span-2">
              <EmptyState
                title="Chưa có agent"
                description={loading ? "Đang tải trạng thái agent..." : "Khởi động backend để xem agent status."}
              />
            </div>
          )}
        </motion.div>

        {/* Schedule List */}
        {schedules.length > 0 && (
          <motion.div
            initial={{ opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.25, delay: 0.12 }}
            className="border border-border bg-card"
          >
            <div className="border-b border-border bg-card-alt px-5 py-3">
              <h3 className="text-[12px] font-bold uppercase tracking-wider">Agent Task Schedules</h3>
            </div>
            <div className="divide-y divide-border">
              {schedules.map((schedule, idx) => (
                <motion.div
                  key={schedule.id}
                  initial={{ opacity: 0, x: -6 }}
                  animate={{ opacity: 1, x: 0 }}
                  transition={{ duration: 0.15, delay: idx * 0.03 }}
                  className="px-5 py-4 hover:bg-secondary/30 transition-colors"
                >
                  <div className="flex items-start justify-between gap-4">
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-2">
                        <span className="font-semibold text-[13px]">{schedule.name}</span>
                        <span className="text-[10px] font-mono text-text-faint">{schedule.cron_expr}</span>
                        <StatusPill value={schedule.enabled ? "healthy" : "idle"} />
                      </div>
                      <div className="mt-1 text-[11px] text-text-secondary">
                        <span className="text-text-muted font-semibold">{schedule.agent_role}</span>
                        {" — "}
                        {truncate(schedule.message, 100)}
                      </div>
                    </div>
                    <div className="text-right shrink-0">
                      <div className={cn("text-[11px] font-bold uppercase", statusColor(schedule.last_status))}>
                        {schedule.last_status}
                      </div>
                      <div className="text-[10px] text-text-muted mt-0.5">
                        {formatRelativeTime(schedule.last_run_at)}
                      </div>
                    </div>
                  </div>
                  {schedule.last_result && (
                    <div className="mt-2 text-[10px] text-text-dim bg-secondary/50 px-3 py-2 border border-border/30 leading-relaxed">
                      {truncate(schedule.last_result, 200)}
                    </div>
                  )}
                </motion.div>
              ))}
            </div>
          </motion.div>
        )}

        {/* Create Schedule Section */}
        <motion.div
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.25, delay: 0.15 }}
        >
          <div className="border border-border bg-card">
            <div className="border-b border-border px-5 py-3 flex items-center justify-between">
              <div>
                <h3 className="text-[13px] font-bold">Tạo Agent Task Schedule</h3>
                <p className="text-[10px] text-text-muted mt-0.5">Lập lịch gửi message tới agent theo cron</p>
              </div>
              <button
                onClick={() => setShowForm(!showForm)}
                className={cn(
                  "px-4 py-1.5 text-[11px] font-bold tracking-wider transition-colors",
                  showForm
                    ? "bg-secondary text-foreground"
                    : "bg-cyan/10 text-cyan hover:bg-cyan/15"
                )}
              >
                {showForm ? "ĐÓNG" : "+ TẠO MỚI"}
              </button>
            </div>

            {/* Quick Presets */}
            <div className="px-5 py-3 border-b border-border/50">
              <div className="text-[9px] font-bold tracking-wider text-text-muted mb-2">CHỌN NHANH</div>
              <div className="flex gap-2 flex-wrap">
                {presetJobs.map((preset) => (
                  <button
                    key={preset.name}
                    onClick={() => handlePreset(preset)}
                    className="px-3 py-1.5 text-[10px] font-semibold bg-secondary hover:bg-cyan/10 hover:text-cyan transition-colors border border-border"
                  >
                    {preset.desc}
                  </button>
                ))}
              </div>
            </div>

            {/* Form */}
            <AnimatePresence>
              {showForm && (
                <motion.div
                  initial={{ height: 0, opacity: 0 }}
                  animate={{ height: "auto", opacity: 1 }}
                  exit={{ height: 0, opacity: 0 }}
                  transition={{ duration: 0.2 }}
                  className="overflow-hidden"
                >
                  <div className="px-5 py-4 space-y-4">
                    {/* Name + Agent Role */}
                    <div className="grid grid-cols-2 gap-4">
                      <div>
                        <label className="text-[10px] font-bold tracking-wider text-text-muted block mb-1.5">TÊN SCHEDULE</label>
                        <input
                          type="text"
                          value={formName}
                          onChange={(e) => setFormName(e.target.value)}
                          placeholder="VD: XAU/USD Analysis"
                          className="w-full bg-secondary border border-border px-3 py-2 text-[12px] focus:border-cyan/50 focus:outline-none transition-colors"
                        />
                      </div>
                      <div>
                        <label className="text-[10px] font-bold tracking-wider text-text-muted block mb-1.5">AGENT ROLE</label>
                        <div className="flex gap-2">
                          <select
                            value={formAgentRole}
                            onChange={(e) => setFormAgentRole(e.target.value)}
                            className="flex-1 bg-secondary border border-border px-3 py-2 text-[12px] focus:border-cyan/50 focus:outline-none transition-colors"
                          >
                            <option value="kuromi">Kuromi Finance</option>
                          </select>
                          <label className="flex items-center gap-2 cursor-pointer px-2">
                            <input
                              type="checkbox"
                              checked={formEnabled}
                              onChange={(e) => setFormEnabled(e.target.checked)}
                              className="accent-cyan"
                            />
                            <span className="text-[10px] font-semibold text-text-muted">ON</span>
                          </label>
                        </div>
                      </div>
                    </div>

                    {/* Message */}
                    <div>
                      <label className="text-[10px] font-bold tracking-wider text-text-muted block mb-1.5">MESSAGE GỬI CHO AGENT</label>
                      <textarea
                        value={formMessage}
                        onChange={(e) => setFormMessage(e.target.value)}
                        placeholder="VD: Phân tích kỹ thuật XAU/USD, đưa ra tín hiệu mua/bán..."
                        rows={3}
                        className="w-full bg-secondary border border-border px-3 py-2 text-[12px] focus:border-cyan/50 focus:outline-none transition-colors resize-none"
                      />
                    </div>

                    {/* Visual Cron Picker */}
                    <CronPicker value={formCron} onChange={setFormCron} />

                    {submitError && (
                      <div className="text-[11px] text-loss bg-loss/10 px-3 py-2">{submitError}</div>
                    )}
                    {submitSuccess && (
                      <motion.div
                        initial={{ opacity: 0 }}
                        animate={{ opacity: 1 }}
                        className="text-[11px] text-profit bg-profit/10 px-3 py-2 font-semibold"
                      >
                        Schedule đã được tạo thành công!
                      </motion.div>
                    )}

                    <button
                      onClick={handleSubmit}
                      disabled={submitting || !formName.trim() || !formMessage.trim()}
                      className={cn(
                        "px-5 py-2.5 text-[11px] font-bold tracking-wider transition-colors w-full",
                        submitting || !formName.trim() || !formMessage.trim()
                          ? "bg-secondary text-text-muted cursor-not-allowed"
                          : "bg-cyan/10 text-cyan hover:bg-cyan/20 border border-cyan/20"
                      )}
                    >
                      {submitting ? "ĐANG TẠO..." : "TẠO AGENT TASK SCHEDULE"}
                    </button>
                  </div>
                </motion.div>
              )}
            </AnimatePresence>
          </div>
        </motion.div>
      </div>
    </div>
  );
}
