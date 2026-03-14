"use client";
import { useState, useRef, useEffect, useMemo } from "react";
import { motion, AnimatePresence } from "motion/react";
import { PageTitle } from "@/components/dashboard/page-title";
import { useNews } from "@/hooks/useNews";
import { useCalendar } from "@/hooks/useCalendar";
import type { CalendarEvent } from "@/lib/calendar-types";
import { cn } from "@/lib/utils";

const filterTabs = ["ALL", "IMPORTANT", "LATEST"];

function formatNewsTime(ms: number): string {
  const diff = Date.now() - ms;
  const mins = Math.floor(diff / 60_000);
  if (mins < 1) return "vừa xong";
  if (mins < 60) return `${mins} phút trước`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours} giờ trước`;
  return `${Math.floor(hours / 24)} ngày trước`;
}

function formatExactTime(ms: number): string {
  const d = new Date(ms);
  return d.toLocaleTimeString("vi-VN", { hour12: false, hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

function formatFullDate(ms: number): string {
  const d = new Date(ms);
  return d.toLocaleDateString("vi-VN", { day: "2-digit", month: "2-digit", year: "numeric" });
}

function formatCalendarTime(unix: number): string {
  const d = new Date(unix * 1000);
  return d.toLocaleTimeString("vi-VN", { hour12: false, hour: "2-digit", minute: "2-digit" });
}

function getRelativeDateLabel(dateKey: string): { label: string; isToday: boolean; isPast: boolean } {
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  const [y, m, d] = dateKey.split("-").map(Number);
  const target = new Date(y, m - 1, d);
  target.setHours(0, 0, 0, 0);
  const diffDays = Math.round((target.getTime() - today.getTime()) / 86_400_000);
  if (diffDays === 0) return { label: "Hôm nay", isToday: true, isPast: false };
  if (diffDays === -1) return { label: "Hôm qua", isToday: false, isPast: true };
  if (diffDays === 1) return { label: "Ngày mai", isToday: false, isPast: false };
  if (diffDays > 0) return { label: `${diffDays} ngày nữa`, isToday: false, isPast: false };
  return { label: `${Math.abs(diffDays)} ngày trước`, isToday: false, isPast: true };
}

function formatDayMonth(dateKey: string): { day: string; weekday: string; monthYear: string } {
  const [y, m, d] = dateKey.split("-").map(Number);
  const target = new Date(y, m - 1, d);
  const weekday = target.toLocaleDateString("vi-VN", { weekday: "short" }).toUpperCase();
  return { day: String(d), weekday, monthYear: `${m}/${y}` };
}

// Group + sort helpers
function groupByDateKey(events: CalendarEvent[]): Map<string, CalendarEvent[]> {
  const groups = new Map<string, CalendarEvent[]>();
  for (const ev of events) {
    const d = new Date(ev.releasedDate * 1000);
    const key = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key)!.push(ev);
  }
  return groups;
}

function sortEvents(events: CalendarEvent[]): CalendarEvent[] {
  return [...events].sort((a, b) => {
    if (b.star !== a.star) return b.star - a.star;
    return a.releasedDate - b.releasedDate;
  });
}

// Country flag color mapping
const COUNTRY_COLORS: Record<string, { bg: string; text: string; border: string }> = {
  US:  { bg: "rgba(37,99,235,0.12)",  text: "#60a5fa", border: "rgba(37,99,235,0.25)" },
  CA:  { bg: "rgba(220,38,38,0.10)",  text: "#fca5a5", border: "rgba(220,38,38,0.25)" },
  MX:  { bg: "rgba(22,163,74,0.10)",  text: "#86efac", border: "rgba(22,163,74,0.25)" },
  BR:  { bg: "rgba(22,163,74,0.10)",  text: "#fde047", border: "rgba(22,163,74,0.25)" },
  EU:  { bg: "rgba(37,99,235,0.12)",  text: "#93c5fd", border: "rgba(37,99,235,0.25)" },
  UK:  { bg: "rgba(220,38,38,0.10)",  text: "#fca5a5", border: "rgba(220,38,38,0.25)" },
  GB:  { bg: "rgba(220,38,38,0.10)",  text: "#fca5a5", border: "rgba(220,38,38,0.25)" },
  DE:  { bg: "rgba(234,179,8,0.10)",  text: "#fde047", border: "rgba(234,179,8,0.25)" },
  FR:  { bg: "rgba(37,99,235,0.12)",  text: "#93c5fd", border: "rgba(37,99,235,0.25)" },
  IT:  { bg: "rgba(22,163,74,0.10)",  text: "#86efac", border: "rgba(22,163,74,0.25)" },
  ES:  { bg: "rgba(234,179,8,0.10)",  text: "#fde047", border: "rgba(234,179,8,0.20)" },
  CH:  { bg: "rgba(220,38,38,0.10)",  text: "#fca5a5", border: "rgba(220,38,38,0.25)" },
  SE:  { bg: "rgba(37,99,235,0.12)",  text: "#fde047", border: "rgba(37,99,235,0.25)" },
  NO:  { bg: "rgba(220,38,38,0.10)",  text: "#93c5fd", border: "rgba(220,38,38,0.20)" },
  JP:  { bg: "rgba(220,38,38,0.10)",  text: "#fca5a5", border: "rgba(220,38,38,0.25)" },
  CN:  { bg: "rgba(220,38,38,0.12)",  text: "#fde047", border: "rgba(220,38,38,0.25)" },
  AU:  { bg: "rgba(37,99,235,0.12)",  text: "#fde047", border: "rgba(37,99,235,0.25)" },
  NZ:  { bg: "rgba(37,99,235,0.12)",  text: "#93c5fd", border: "rgba(37,99,235,0.25)" },
  KR:  { bg: "rgba(37,99,235,0.12)",  text: "#fca5a5", border: "rgba(37,99,235,0.25)" },
  IN:  { bg: "rgba(234,88,12,0.12)",  text: "#fdba74", border: "rgba(234,88,12,0.25)" },
  SG:  { bg: "rgba(220,38,38,0.10)",  text: "#fca5a5", border: "rgba(220,38,38,0.20)" },
  HK:  { bg: "rgba(220,38,38,0.10)",  text: "#fca5a5", border: "rgba(220,38,38,0.25)" },
  TW:  { bg: "rgba(37,99,235,0.12)",  text: "#fca5a5", border: "rgba(37,99,235,0.25)" },
};
const DEFAULT_CC = { bg: "rgba(100,100,120,0.10)", text: "#8c8ca4", border: "rgba(100,100,120,0.20)" };
function getCC(country: string) {
  return COUNTRY_COLORS[country.trim().toUpperCase()] ?? DEFAULT_CC;
}

// Impact filter options
type ImpactFilter = "ALL" | 3 | 2 | 1;
const IMPACT_FILTERS: { value: ImpactFilter; label: string; color: string; activeBg: string }[] = [
  { value: "ALL", label: "Tất cả", color: "text-foreground", activeBg: "bg-secondary" },
  { value: 3, label: "Cao", color: "text-loss", activeBg: "bg-loss/15" },
  { value: 2, label: "TB", color: "text-warning", activeBg: "bg-warning/15" },
  { value: 1, label: "Thấp", color: "text-cyan", activeBg: "bg-cyan/10" },
];

export default function NewsPage() {
  const [activeFilter, setActiveFilter] = useState("ALL");
  const { items: liveNews, loading, loadingMore, error, hasMore, connected, loadMore } = useNews({
    pageSize: 50,
  });
  const { events: calendarEvents, loading: calLoading } = useCalendar({ pastDays: 2, futureDays: 5 });

  // Infinite scroll for news
  const sentinelRef = useRef<HTMLDivElement>(null);
  const observerRef = useRef<IntersectionObserver | null>(null);

  useEffect(() => {
    if (observerRef.current) observerRef.current.disconnect();
    observerRef.current = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting && hasMore && !loadingMore) loadMore();
      },
      { threshold: 0.1 }
    );
    if (sentinelRef.current) observerRef.current.observe(sentinelRef.current);
    return () => { if (observerRef.current) observerRef.current.disconnect(); };
  }, [hasMore, loadingMore, loadMore]);

  const filteredNews = activeFilter === "IMPORTANT"
    ? liveNews.filter((n) => n.important)
    : liveNews;
  const importantCount = liveNews.filter((n) => n.important).length;

  // Calendar state
  const calendarGrouped = useMemo(() => groupByDateKey(calendarEvents), [calendarEvents]);
  const dateKeys = useMemo(() => Array.from(calendarGrouped.keys()).sort(), [calendarGrouped]);
  const todayKey = useMemo(() => {
    const t = new Date();
    return `${t.getFullYear()}-${String(t.getMonth() + 1).padStart(2, "0")}-${String(t.getDate()).padStart(2, "0")}`;
  }, []);

  const [activeCalDate, setActiveCalDate] = useState<string | null>(null);
  const [impactFilter, setImpactFilter] = useState<ImpactFilter>("ALL");

  // Default to today
  useEffect(() => {
    if (dateKeys.length > 0 && !activeCalDate) {
      setActiveCalDate(dateKeys.includes(todayKey) ? todayKey : dateKeys[0]);
    }
  }, [dateKeys, todayKey, activeCalDate]);

  // Scroll today tab into view
  const tabsScrollRef = useRef<HTMLDivElement>(null);
  const todayTabRef = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (todayTabRef.current && tabsScrollRef.current) {
      const container = tabsScrollRef.current;
      const tab = todayTabRef.current;
      const scrollLeft = tab.offsetLeft - container.offsetWidth / 2 + tab.offsetWidth / 2;
      container.scrollTo({ left: Math.max(0, scrollLeft), behavior: "instant" });
    }
  }, [dateKeys]);

  // Filtered events for active date
  const activeEvents = useMemo(() => {
    if (!activeCalDate || !calendarGrouped.has(activeCalDate)) return [];
    let events = sortEvents(calendarGrouped.get(activeCalDate)!);
    if (impactFilter !== "ALL") {
      events = events.filter((e) =>
        impactFilter === 1 ? e.star <= 1 : e.star === impactFilter
      );
    }
    return events;
  }, [activeCalDate, calendarGrouped, impactFilter]);

  // Stats for active date (unfiltered)
  const activeAllEvents = useMemo(() => {
    if (!activeCalDate || !calendarGrouped.has(activeCalDate)) return [];
    return calendarGrouped.get(activeCalDate)!;
  }, [activeCalDate, calendarGrouped]);

  const highCount = activeAllEvents.filter((e) => e.star >= 3).length;
  const medCount = activeAllEvents.filter((e) => e.star === 2).length;
  const lowCount = activeAllEvents.filter((e) => e.star <= 1).length;
  const impactCounts: Record<string, number> = { ALL: activeAllEvents.length, "3": highCount, "2": medCount, "1": lowCount };

  return (
    <div className="flex gap-6 h-full overflow-y-auto p-6">
      {/* Left: News Feed */}
      <div className="flex-1 min-w-0 space-y-6">
        <motion.div initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.2 }}>
          <PageTitle title="Market News" subtitle="Tin tức thị trường" breadcrumb="NEWS / LIVE FEED" />
        </motion.div>

        {/* Filter Tabs + Status */}
        <motion.div
          initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.2, delay: 0.05 }}
          className="flex gap-1 items-center"
        >
          {filterTabs.map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveFilter(tab)}
              className={cn(
                "px-4 py-1.5 text-[12px] font-semibold tracking-wider transition-colors",
                activeFilter === tab ? "bg-cyan/10 text-cyan" : "text-muted-foreground hover:bg-secondary"
              )}
            >
              {tab}
              {tab === "IMPORTANT" && importantCount > 0 && (
                <span className="ml-1.5 text-[10px] text-loss font-bold">{importantCount}</span>
              )}
            </button>
          ))}
          <div className="flex-1" />
          <span className="flex items-center gap-1.5 mr-3">
            <span className={cn("h-2 w-2 rounded-full", connected ? "bg-profit live-dot" : "bg-muted-foreground")} />
            <span className={cn("text-[10px] font-bold tracking-wide", connected ? "text-profit" : "text-muted-foreground")}>
              {connected ? "LIVE" : "POLLING"}
            </span>
          </span>
        </motion.div>

        {loading && liveNews.length === 0 && (
          <div className="py-12 text-center">
            <div className="text-[14px] text-muted-foreground animate-pulse">Đang tải tin tức...</div>
          </div>
        )}
        {error && liveNews.length === 0 && (
          <div className="py-12 text-center">
            <div className="text-[14px] text-loss">{error}</div>
            <button onClick={() => location.reload()} className="mt-2 text-[13px] text-cyan hover:underline">Thử lại</button>
          </div>
        )}

        <div className="space-y-3">
          {filteredNews.map((article, idx) => (
            <motion.a
              key={article.id}
              initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.15, delay: Math.min(idx * 0.02, 0.3) }}
              href={`https://www.fastbull.com${article.path}`}
              target="_blank" rel="noopener noreferrer"
              className={cn(
                "block border bg-card p-5 transition-colors cursor-pointer",
                article.important ? "border-loss/40 hover:border-loss/60" : "border-border hover:border-cyan/30"
              )}
            >
              <div className="flex items-start justify-between gap-4">
                <div className="flex-1">
                  <div className="flex items-center gap-2 mb-2">
                    {article.important && (
                      <span className="px-2.5 py-0.5 text-[11px] font-bold bg-loss/15 text-loss">QUAN TRỌNG</span>
                    )}
                    <span className="text-[11px] text-muted-foreground">
                      <span className="font-mono">{formatExactTime(article.releasedDateMs)}</span>
                      {" · "}{formatFullDate(article.releasedDateMs)}
                      {" · "}{formatNewsTime(article.releasedDateMs)}
                    </span>
                  </div>
                  <h3 className={cn("text-[16px] font-semibold leading-[1.6]", article.important ? "text-loss" : "")}>
                    {article.title}
                  </h3>
                </div>
              </div>
            </motion.a>
          ))}
          <div ref={sentinelRef} className="py-4 text-center">
            {loadingMore && <div className="text-[13px] text-muted-foreground animate-pulse">Đang tải thêm...</div>}
            {!hasMore && filteredNews.length > 0 && <div className="text-[12px] text-muted-foreground">Đã tải hết tin tức</div>}
          </div>
        </div>
      </div>

      {/* Right Sidebar — Economic Calendar */}
      <motion.div
        initial={{ opacity: 0, x: 12 }} animate={{ opacity: 1, x: 0 }}
        transition={{ duration: 0.25, delay: 0.1 }}
        className="w-[380px] shrink-0"
      >
        <div className="border border-border bg-card flex flex-col h-[calc(100vh-72px)]">
          {/* Header */}
          <div className="px-4 pt-3.5 pb-2.5 shrink-0">
            <div className="flex items-center justify-between mb-3">
              <h3 className="text-[13px] font-bold tracking-wide uppercase">Lịch kinh tế</h3>
              {activeCalDate && (
                <span className="text-[10px] text-muted-foreground font-mono">
                  {activeAllEvents.length} sự kiện
                </span>
              )}
            </div>

            {/* Impact filter pills */}
            <div className="flex gap-1">
              {IMPACT_FILTERS.map((f) => {
                const isActive = impactFilter === f.value;
                const count = impactCounts[String(f.value)] ?? 0;
                return (
                  <button
                    key={String(f.value)}
                    onClick={() => setImpactFilter(f.value)}
                    className={cn(
                      "px-2.5 py-1 text-[10px] font-bold tracking-wide transition-all",
                      isActive ? cn(f.activeBg, f.color) : "text-muted-foreground hover:text-foreground hover:bg-secondary/50"
                    )}
                  >
                    {f.label}
                    {count > 0 && (
                      <span className={cn("ml-1 text-[9px]", isActive ? "opacity-80" : "opacity-50")}>{count}</span>
                    )}
                  </button>
                );
              })}
            </div>
          </div>

          {/* Date tabs — horizontal scroll */}
          {!calLoading && dateKeys.length > 0 && (
            <div ref={tabsScrollRef} className="flex overflow-x-auto shrink-0 border-y border-border scrollbar-none">
              {dateKeys.map((dk) => {
                const rel = getRelativeDateLabel(dk);
                const dm = formatDayMonth(dk);
                const evCount = calendarGrouped.get(dk)?.length ?? 0;
                const isActive = activeCalDate === dk;
                const isToday = dk === todayKey;

                return (
                  <button
                    key={dk}
                    ref={isToday ? todayTabRef : undefined}
                    onClick={() => setActiveCalDate(dk)}
                    className={cn(
                      "flex-shrink-0 w-[64px] py-2.5 flex flex-col items-center gap-0.5 transition-all border-b-2 relative",
                      isActive
                        ? "border-cyan bg-cyan/[0.06]"
                        : "border-transparent hover:bg-secondary/40",
                      rel.isPast && !isActive && "opacity-40"
                    )}
                  >
                    <span className={cn(
                      "text-[8px] font-bold tracking-widest",
                      isActive ? "text-cyan" : "text-muted-foreground"
                    )}>
                      {dm.weekday}
                    </span>
                    <span className={cn(
                      "text-[18px] font-bold leading-none",
                      isActive ? "text-cyan" : isToday ? "text-foreground" : "text-foreground/70"
                    )}>
                      {dm.day}
                    </span>
                    <span className="text-[8px] text-muted-foreground">{dm.monthYear}</span>
                    {/* Event count dot */}
                    <span className={cn(
                      "text-[8px] font-bold mt-0.5",
                      isActive ? "text-cyan" : "text-text-faint"
                    )}>
                      {evCount}
                    </span>
                    {/* Today indicator */}
                    {isToday && !isActive && (
                      <span className="absolute bottom-[3px] h-[2px] w-3 bg-cyan/60 rounded-full" />
                    )}
                  </button>
                );
              })}
            </div>
          )}

          {/* Event list */}
          {calLoading ? (
            <div className="flex-1 flex items-center justify-center">
              <div className="text-[12px] text-muted-foreground animate-pulse">Đang tải lịch kinh tế...</div>
            </div>
          ) : calendarEvents.length === 0 ? (
            <div className="flex-1 flex items-center justify-center">
              <div className="text-[12px] text-muted-foreground">Không có sự kiện</div>
            </div>
          ) : (
            <div className="flex-1 overflow-y-auto">
              <AnimatePresence mode="wait">
                <motion.div
                  key={`${activeCalDate}-${impactFilter}`}
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  exit={{ opacity: 0 }}
                  transition={{ duration: 0.12 }}
                >
                  {activeEvents.length === 0 ? (
                    <div className="py-12 text-center">
                      <div className="text-[11px] text-muted-foreground">
                        Không có sự kiện {impactFilter !== "ALL" ? "ở mức này" : "cho ngày này"}
                      </div>
                    </div>
                  ) : (
                    <div className="divide-y divide-border/20">
                      {activeEvents.map((event, idx) => (
                        <CalendarRow key={event.id} event={event} delay={idx * 0.02} />
                      ))}
                    </div>
                  )}
                </motion.div>
              </AnimatePresence>
            </div>
          )}
        </div>
      </motion.div>
    </div>
  );
}

// ── Calendar Event Row ──────────────────────────────────────────────────────
function CalendarRow({ event, delay }: { event: CalendarEvent; delay: number }) {
  const cc = getCC(event.country);
  const impactColor = event.star >= 3 ? "bg-loss" : event.star === 2 ? "bg-warning" : "bg-cyan/40";
  const hasData = event.type === "data" && (event.actual || event.consensus || event.previous);

  return (
    <motion.div
      initial={{ opacity: 0, y: 4 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.15, delay }}
      className="flex hover:bg-secondary/20 transition-colors"
    >
      {/* Left accent — importance color */}
      <div className={cn("w-[3px] shrink-0", impactColor)} />

      {/* Time column */}
      <div className="w-[52px] shrink-0 flex items-start justify-center pt-3">
        <span className="text-[11px] font-mono text-muted-foreground tabular-nums">
          {formatCalendarTime(event.releasedDate)}
        </span>
      </div>

      {/* Content */}
      <div className="flex-1 py-2.5 pr-3 min-w-0">
        {/* Country + Impact */}
        <div className="flex items-center gap-2 mb-1">
          <span
            className="text-[9px] font-bold tracking-wide px-1.5 py-[2px] rounded-sm leading-none"
            style={{ backgroundColor: cc.bg, color: cc.text, border: `1px solid ${cc.border}` }}
          >
            {event.country}
          </span>
          <span className="flex items-center gap-[3px] ml-auto shrink-0">
            {[1, 2, 3].map((lvl) => (
              <span
                key={lvl}
                className={cn(
                  "h-[5px] w-[5px] rounded-full",
                  lvl <= event.star
                    ? event.star >= 3 ? "bg-loss" : event.star === 2 ? "bg-warning" : "bg-cyan/60"
                    : "bg-border/40"
                )}
              />
            ))}
          </span>
        </div>

        {/* Title */}
        <p className="text-[12px] font-medium leading-[1.55] text-foreground/85">{event.title}</p>

        {/* Data row */}
        {hasData && (
          <div className="flex items-center gap-1 mt-2 flex-wrap">
            {event.actual != null && (
              <span className="inline-flex items-center gap-1 text-[10px] px-1.5 py-0.5 bg-profit/10 rounded-sm">
                <span className="text-profit/60 font-medium">TT</span>
                <span className="text-profit font-bold">{event.actual}{event.unit || ""}</span>
              </span>
            )}
            {event.consensus != null && (
              <span className="inline-flex items-center gap-1 text-[10px] px-1.5 py-0.5 bg-secondary rounded-sm">
                <span className="text-muted-foreground font-medium">DB</span>
                <span className="text-foreground/70 font-semibold">{event.consensus}{event.unit || ""}</span>
              </span>
            )}
            {event.previous != null && (
              <span className="inline-flex items-center gap-1 text-[10px] px-1.5 py-0.5 bg-secondary rounded-sm">
                <span className="text-muted-foreground font-medium">Tr</span>
                <span className="text-foreground/50 font-semibold">{event.previous}{event.unit || ""}</span>
              </span>
            )}
          </div>
        )}
      </div>
    </motion.div>
  );
}
