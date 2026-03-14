"use client";

import { useState, useMemo, useEffect } from "react";
import { motion, AnimatePresence } from "motion/react";
import { PageTitle } from "@/components/dashboard/page-title";
import { StaggerGrid } from "@/components/dashboard/motion-primitives";
import { StatsCard } from "@/components/dashboard/stats-card";
import { EmptyState } from "@/components/dashboard/empty-state";
import { StatusPill } from "@/components/dashboard/status-pill";
import { fetchDashboard, createSchedule, updateSchedule, deleteSchedule, fetchCapabilities } from "@/lib/intelligence-api";
import { formatRelativeTime, formatCountdown, titleFromRole, truncate } from "@/lib/formatting";
import { usePollingResource } from "@/hooks/use-polling-resource";
import { cn } from "@/lib/utils";
import type { ScheduleView, CapabilitiesView } from "@/lib/intelligence-types";

// ─── Live Relative Time (re-renders every second) ───
function LiveTime({ value, mode = "relative" }: { value?: string | null; mode?: "relative" | "countdown" }) {
  const [, tick] = useState(0);
  useEffect(() => {
    const id = setInterval(() => tick((t) => t + 1), 1000);
    return () => clearInterval(id);
  }, []);
  return <>{mode === "countdown" ? formatCountdown(value) : formatRelativeTime(value)}</>;
}

// ─── Visual Cron Picker ───
type FreqMode = "seconds" | "minutes" | "hourly" | "daily" | "weekly" | "custom";

const WEEKDAYS = [
  { value: 0, label: "CN" },
  { value: 1, label: "T2" },
  { value: 2, label: "T3" },
  { value: 3, label: "T4" },
  { value: 4, label: "T5" },
  { value: 5, label: "T6" },
  { value: 6, label: "T7" },
];

function buildCron(mode: FreqMode, everySec: number, everyMin: number, hour: number, minute: number, weekdays: number[]): string {
  switch (mode) {
    case "seconds":
      // 6-field cron: sec min hour dom month dow
      return `*/${everySec} * * * * *`;
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

function describeCron(mode: FreqMode, everySec: number, everyMin: number, hour: number, minute: number, weekdays: number[]): string {
  const hh = String(hour).padStart(2, "0");
  const mm = String(minute).padStart(2, "0");
  switch (mode) {
    case "seconds":
      return `Mỗi ${everySec} giây`;
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
  const [everySec, setEverySec] = useState(30);
  const [everyMin, setEveryMin] = useState(5);
  const [hour, setHour] = useState(9);
  const [minute, setMinute] = useState(0);
  const [weekdays, setWeekdays] = useState<number[]>([1, 2, 3, 4, 5]);

  const cron = useMemo(() => buildCron(mode, everySec, everyMin, hour, minute, weekdays), [mode, everySec, everyMin, hour, minute, weekdays]);
  const description = useMemo(() => describeCron(mode, everySec, everyMin, hour, minute, weekdays), [mode, everySec, everyMin, hour, minute, weekdays]);

  const updateParent = (newMode: FreqMode, newSec: number, newMin: number, newHour: number, newMinute: number, newWeekdays: number[]) => {
    onChange(buildCron(newMode, newSec, newMin, newHour, newMinute, newWeekdays));
  };

  const toggleWeekday = (d: number) => {
    const next = weekdays.includes(d) ? weekdays.filter((w) => w !== d) : [...weekdays, d];
    setWeekdays(next);
    updateParent(mode, everySec, everyMin, hour, minute, next);
  };

  return (
    <div className="space-y-3">
      <div>
        <div className="text-[10px] font-bold tracking-wider text-text-muted mb-2">TẦN SUẤT</div>
        <div className="flex gap-1">
          {([
            { v: "seconds" as const, l: "Mỗi X giây" },
            { v: "minutes" as const, l: "Mỗi X phút" },
            { v: "hourly" as const, l: "Mỗi giờ" },
            { v: "daily" as const, l: "Mỗi ngày" },
            { v: "weekly" as const, l: "Theo tuần" },
          ]).map((opt) => (
            <button
              key={opt.v}
              onClick={() => { setMode(opt.v); updateParent(opt.v, everySec, everyMin, hour, minute, weekdays); }}
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

      {mode === "seconds" && (
        <div>
          <div className="text-[10px] font-bold tracking-wider text-text-muted mb-2">CHẠY MỖI</div>
          <div className="flex items-center gap-2">
            {[5, 10, 15, 20, 30, 45, 60].map((s) => (
              <button
                key={s}
                onClick={() => { setEverySec(s); updateParent(mode, s, everyMin, hour, minute, weekdays); }}
                className={cn(
                  "px-3 py-1.5 text-[11px] font-bold transition-colors min-w-[40px]",
                  everySec === s
                    ? "bg-cyan/15 text-cyan border border-cyan/30"
                    : "bg-secondary text-text-muted border border-border hover:text-foreground"
                )}
              >
                {s}s
              </button>
            ))}
          </div>
        </div>
      )}

      {mode === "minutes" && (
        <div>
          <div className="text-[10px] font-bold tracking-wider text-text-muted mb-2">CHẠY MỖI</div>
          <div className="flex items-center gap-2">
            {[1, 2, 3, 5, 10, 15, 30].map((m) => (
              <button
                key={m}
                onClick={() => { setEveryMin(m); updateParent(mode, everySec, m, hour, minute, weekdays); }}
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
                onChange={(e) => { const h = Number(e.target.value); setHour(h); updateParent(mode, everySec, everyMin, h, minute, weekdays); }}
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
              onChange={(e) => { const m = Number(e.target.value); setMinute(m); updateParent(mode, everySec, everyMin, hour, m, weekdays); }}
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

// ─── Chip Toggle Group (multi-select) ───
function ChipToggleGroup({
  label,
  allItems,
  selected,
  onChange,
  allowAllChip,
}: {
  label: string;
  allItems: string[];
  selected: string[] | null;
  onChange: (v: string[] | null) => void;
  allowAllChip: boolean;
}) {
  const toggle = (item: string) => {
    const current = selected ?? allItems.slice();
    const next = current.includes(item)
      ? current.filter((i) => i !== item)
      : [...current, item];
    onChange(allowAllChip && next.length === allItems.length ? null : next);
  };

  if (allItems.length === 0) return null;

  return (
    <div>
      <div className="text-[10px] font-bold tracking-wider text-text-muted mb-2">{label}</div>
      <div className="flex gap-1 flex-wrap">
        {allowAllChip && (
          <button
            type="button"
            onClick={() => onChange(null)}
            className={cn(
              "px-3 py-1.5 text-[10px] font-semibold transition-colors",
              selected === null
                ? "bg-cyan/15 text-cyan border border-cyan/30"
                : "bg-secondary text-text-muted border border-border hover:text-foreground"
            )}
          >
            ALL
          </button>
        )}
        {allItems.map((item) => (
          <button
            type="button"
            key={item}
            onClick={() => toggle(item)}
            className={cn(
              "px-3 py-1.5 text-[10px] font-semibold transition-colors",
              (selected === null ? true : selected.includes(item))
                ? "bg-cyan/15 text-cyan border border-cyan/30"
                : "bg-secondary text-text-muted border border-border hover:text-foreground"
            )}
          >
            {item}
          </button>
        ))}
      </div>
    </div>
  );
}

// ─── Edit Schedule Modal ───
function EditScheduleModal({
  schedule,
  onClose,
  onSaved,
}: {
  schedule: ScheduleView;
  onClose: () => void;
  onSaved: () => void;
}) {
  const [name, setName] = useState(schedule.name);
  const [cronExpr, setCronExpr] = useState(schedule.cron_expr);
  const [agentRole, setAgentRole] = useState(schedule.agent_role);
  const [message, setMessage] = useState(schedule.message);
  const [enabled, setEnabled] = useState(schedule.enabled);
  const [allowedTools, setAllowedTools] = useState<string[] | null>(schedule.allowed_tools ?? null);
  const [allowedMcps, setAllowedMcps] = useState<string[] | null>(schedule.allowed_mcps ?? null);
  const [scheduleSkills, setScheduleSkills] = useState<string[]>(schedule.skills ?? []);
  const [capabilities, setCapabilities] = useState<CapabilitiesView | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchCapabilities().then(setCapabilities).catch(() => {});
  }, []);

  const handleSave = async () => {
    if (!name.trim() || !message.trim()) return;
    setSaving(true);
    setError(null);
    try {
      await updateSchedule(schedule.id, {
        name: name.trim(),
        cron_expr: cronExpr.trim(),
        agent_role: agentRole.trim(),
        message: message.trim(),
        enabled,
        allowed_tools: allowedTools,
        allowed_mcps: allowedMcps,
        skills: scheduleSkills.length > 0 ? scheduleSkills : null,
      });
      onSaved();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Không thể cập nhật");
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/70 backdrop-blur-sm" onClick={onClose} />
      <motion.div
        initial={{ opacity: 0, scale: 0.95, y: 12 }}
        animate={{ opacity: 1, scale: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.95, y: 12 }}
        transition={{ duration: 0.15 }}
        className="relative z-10 w-full max-w-[640px] max-h-[85vh] overflow-y-auto border border-border bg-card shadow-2xl"
      >
        <div className="border-b border-border px-5 py-3 flex items-center justify-between">
          <h3 className="text-[13px] font-bold uppercase tracking-wider">Chỉnh sửa Schedule</h3>
          <button onClick={onClose} className="text-text-muted hover:text-foreground text-[18px] leading-none px-1">&times;</button>
        </div>

        <div className="px-5 py-4 space-y-4">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="text-[10px] font-bold tracking-wider text-text-muted block mb-1.5">TÊN SCHEDULE</label>
              <input
                type="text"
                value={name}
                onChange={(e) => setName(e.target.value)}
                className="w-full bg-secondary border border-border px-3 py-2 text-[12px] focus:border-cyan/50 focus:outline-none transition-colors"
              />
            </div>
            <div>
              <label className="text-[10px] font-bold tracking-wider text-text-muted block mb-1.5">AGENT ROLE</label>
              <div className="flex gap-2">
                <select
                  value={agentRole}
                  onChange={(e) => setAgentRole(e.target.value)}
                  className="flex-1 bg-secondary border border-border px-3 py-2 text-[12px] focus:border-cyan/50 focus:outline-none transition-colors"
                >
                  <option value="kuromi">Kuromi Finance</option>
                </select>
                <label className="flex items-center gap-2 cursor-pointer px-2">
                  <input
                    type="checkbox"
                    checked={enabled}
                    onChange={(e) => setEnabled(e.target.checked)}
                    className="accent-cyan"
                  />
                  <span className="text-[10px] font-semibold text-text-muted">ON</span>
                </label>
              </div>
            </div>
          </div>

          <div>
            <label className="text-[10px] font-bold tracking-wider text-text-muted block mb-1.5">MESSAGE</label>
            <textarea
              value={message}
              onChange={(e) => setMessage(e.target.value)}
              rows={3}
              className="w-full bg-secondary border border-border px-3 py-2 text-[12px] focus:border-cyan/50 focus:outline-none transition-colors resize-none"
            />
          </div>

          <CronPicker value={cronExpr} onChange={setCronExpr} />

          {capabilities && (
            <div className="space-y-3 border border-border/50 bg-secondary/20 px-4 py-3">
              <div className="text-[10px] font-bold tracking-wider text-text-muted">TOOLS / MCP / SKILLS</div>
              <ChipToggleGroup
                label="NATIVE TOOLS"
                allItems={capabilities.tools}
                selected={allowedTools}
                onChange={setAllowedTools}
                allowAllChip
              />
              {capabilities.mcps.length > 0 && (
                <ChipToggleGroup
                  label="MCP SERVERS"
                  allItems={capabilities.mcps}
                  selected={allowedMcps}
                  onChange={setAllowedMcps}
                  allowAllChip
                />
              )}
              {capabilities.skills.length > 0 && (
                <ChipToggleGroup
                  label="SKILLS"
                  allItems={capabilities.skills}
                  selected={scheduleSkills}
                  onChange={(v) => setScheduleSkills(v ?? [])}
                  allowAllChip={false}
                />
              )}
            </div>
          )}

          {error && <div className="text-[11px] text-loss bg-loss/10 px-3 py-2">{error}</div>}

          <div className="flex gap-2">
            <button
              onClick={onClose}
              className="px-4 py-2 text-[11px] font-bold tracking-wider bg-secondary text-text-muted hover:text-foreground transition-colors"
            >
              HỦY
            </button>
            <button
              onClick={handleSave}
              disabled={saving || !name.trim() || !message.trim()}
              className={cn(
                "flex-1 px-4 py-2 text-[11px] font-bold tracking-wider transition-colors",
                saving || !name.trim() || !message.trim()
                  ? "bg-secondary text-text-muted cursor-not-allowed"
                  : "bg-cyan/10 text-cyan hover:bg-cyan/20 border border-cyan/20"
              )}
            >
              {saving ? "ĐANG LƯU..." : "LƯU THAY ĐỔI"}
            </button>
          </div>
        </div>
      </motion.div>
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
    intervalMs: 10_000,
  });

  const [showForm, setShowForm] = useState(false);
  const [editingSchedule, setEditingSchedule] = useState<ScheduleView | null>(null);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [togglingId, setTogglingId] = useState<string | null>(null);
  const [formName, setFormName] = useState("");
  const [formCron, setFormCron] = useState("*/5 * * * *");
  const [formAgentRole, setFormAgentRole] = useState("kuromi");
  const [formMessage, setFormMessage] = useState("");
  const [formEnabled, setFormEnabled] = useState(true);
  const [formAllowedTools, setFormAllowedTools] = useState<string[] | null>(null);
  const [formAllowedMcps, setFormAllowedMcps] = useState<string[] | null>(null);
  const [formSkills, setFormSkills] = useState<string[]>([]);
  const [capabilities, setCapabilities] = useState<CapabilitiesView | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [submitSuccess, setSubmitSuccess] = useState(false);

  const schedules = data?.schedules ?? [];
  const activeSchedules = schedules.filter((s) => s.enabled);
  const runningTasks = schedules.filter((s) => s.last_status === "running");

  useEffect(() => {
    fetchCapabilities().then(setCapabilities).catch(() => {});
  }, []);

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
        allowed_tools: formAllowedTools,
        allowed_mcps: formAllowedMcps,
        skills: formSkills.length > 0 ? formSkills : null,
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

  const handleToggle = async (schedule: ScheduleView) => {
    setTogglingId(schedule.id);
    try {
      await updateSchedule(schedule.id, { enabled: !schedule.enabled });
      reload();
    } catch {
      // silent
    } finally {
      setTogglingId(null);
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await deleteSchedule(id);
      setDeletingId(null);
      reload();
    } catch {
      // silent
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
                  <span>Last run <LiveTime value={agent.last_seen_at} /></span>
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
                      {(schedule.allowed_tools || schedule.allowed_mcps || (schedule.skills && schedule.skills.length > 0)) && (
                        <div className="mt-1.5 flex gap-1.5 flex-wrap">
                          {schedule.allowed_tools && (
                            <span className="text-[9px] font-bold tracking-wide bg-cyan/8 text-cyan/80 px-1.5 py-0.5 border border-cyan/15">
                              TOOLS: {schedule.allowed_tools.length}
                            </span>
                          )}
                          {schedule.allowed_mcps && (
                            <span className="text-[9px] font-bold tracking-wide bg-cyan/8 text-cyan/80 px-1.5 py-0.5 border border-cyan/15">
                              MCP: {schedule.allowed_mcps.length}
                            </span>
                          )}
                          {schedule.skills && schedule.skills.length > 0 && (
                            <span className="text-[9px] font-bold tracking-wide bg-purple-500/8 text-purple-400/80 px-1.5 py-0.5 border border-purple-500/15">
                              SKILLS: {schedule.skills.join(", ")}
                            </span>
                          )}
                        </div>
                      )}
                    </div>
                    <div className="flex items-start gap-3">
                      {/* Action buttons */}
                      <div className="flex items-center gap-1 shrink-0">
                        <button
                          onClick={() => handleToggle(schedule)}
                          disabled={togglingId === schedule.id}
                          title={schedule.enabled ? "Tạm dừng" : "Tiếp tục"}
                          className={cn(
                            "px-2.5 py-1 text-[9px] font-bold tracking-wider transition-colors border",
                            togglingId === schedule.id
                              ? "bg-secondary text-text-muted border-border cursor-wait"
                              : schedule.enabled
                                ? "bg-warning/10 text-warning border-warning/20 hover:bg-warning/20"
                                : "bg-profit/10 text-profit border-profit/20 hover:bg-profit/20"
                          )}
                        >
                          {togglingId === schedule.id ? "..." : schedule.enabled ? "TẠM DỪNG" : "TIẾP TỤC"}
                        </button>
                        <button
                          onClick={() => setEditingSchedule(schedule)}
                          title="Chỉnh sửa"
                          className="px-2.5 py-1 text-[9px] font-bold tracking-wider bg-cyan/10 text-cyan border border-cyan/20 hover:bg-cyan/20 transition-colors"
                        >
                          SỬA
                        </button>
                        {deletingId === schedule.id ? (
                          <div className="flex items-center gap-1">
                            <button
                              onClick={() => handleDelete(schedule.id)}
                              className="px-2.5 py-1 text-[9px] font-bold tracking-wider bg-loss/10 text-loss border border-loss/20 hover:bg-loss/20 transition-colors"
                            >
                              XÁC NHẬN
                            </button>
                            <button
                              onClick={() => setDeletingId(null)}
                              className="px-2.5 py-1 text-[9px] font-bold tracking-wider bg-secondary text-text-muted border border-border hover:text-foreground transition-colors"
                            >
                              HỦY
                            </button>
                          </div>
                        ) : (
                          <button
                            onClick={() => setDeletingId(schedule.id)}
                            title="Xóa"
                            className="px-2.5 py-1 text-[9px] font-bold tracking-wider bg-secondary text-text-muted border border-border hover:bg-loss/10 hover:text-loss hover:border-loss/20 transition-colors"
                          >
                            XÓA
                          </button>
                        )}
                      </div>
                      {/* Status info */}
                      <div className="text-right shrink-0">
                        <div className={cn("text-[11px] font-bold uppercase", statusColor(schedule.last_status))}>
                          {schedule.last_status}
                        </div>
                        <div className="text-[10px] text-text-muted mt-0.5">
                          Ran <LiveTime value={schedule.last_run_at} />
                        </div>
                        <div className="text-[10px] text-text-faint mt-0.5">
                          Next: <span className="text-cyan font-mono"><LiveTime value={schedule.next_run_at} mode="countdown" /></span>
                        </div>
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

                    {/* Tool / MCP / Skill Filter */}
                    {capabilities && (
                      <div className="space-y-3 border border-border/50 bg-secondary/20 px-4 py-3">
                        <div className="text-[10px] font-bold tracking-wider text-text-muted">TOOLS / MCP / SKILLS</div>
                        <ChipToggleGroup
                          label="NATIVE TOOLS"
                          allItems={capabilities.tools}
                          selected={formAllowedTools}
                          onChange={setFormAllowedTools}
                          allowAllChip
                        />
                        {capabilities.mcps.length > 0 && (
                          <ChipToggleGroup
                            label="MCP SERVERS"
                            allItems={capabilities.mcps}
                            selected={formAllowedMcps}
                            onChange={setFormAllowedMcps}
                            allowAllChip
                          />
                        )}
                        {capabilities.skills.length > 0 && (
                          <ChipToggleGroup
                            label="SKILLS"
                            allItems={capabilities.skills}
                            selected={formSkills}
                            onChange={(v) => setFormSkills(v ?? [])}
                            allowAllChip={false}
                          />
                        )}
                      </div>
                    )}

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

      {/* Edit Modal */}
      <AnimatePresence>
        {editingSchedule && (
          <EditScheduleModal
            schedule={editingSchedule}
            onClose={() => setEditingSchedule(null)}
            onSaved={() => {
              setEditingSchedule(null);
              reload();
            }}
          />
        )}
      </AnimatePresence>
    </div>
  );
}
