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

function formatCalendarTime(unix: number): string {
  const d = new Date(unix * 1000);
  return d.toLocaleTimeString("vi-VN", { hour12: false, hour: "2-digit", minute: "2-digit" });
}

// Get relative date label
function getRelativeDateLabel(dateKey: string): { label: string; isToday: boolean; isPast: boolean } {
  const today = new Date();
  today.setHours(0, 0, 0, 0);

  const [y, m, d] = dateKey.split("-").map(Number);
  const target = new Date(y, m - 1, d);
  target.setHours(0, 0, 0, 0);

  const diffDays = Math.round((target.getTime() - today.getTime()) / 86_400_000);

  if (diffDays === 0) return { label: "Hôm nay", isToday: true, isPast: false };
  if (diffDays === -1) return { label: "Hôm qua", isToday: false, isPast: true };
  if (diffDays === -2) return { label: "2 ngày trước", isToday: false, isPast: true };
  if (diffDays === 1) return { label: "Ngày mai", isToday: false, isPast: false };
  if (diffDays === 2) return { label: "2 ngày nữa", isToday: false, isPast: false };
  if (diffDays > 0) return { label: `${diffDays} ngày nữa`, isToday: false, isPast: false };
  return { label: `${Math.abs(diffDays)} ngày trước`, isToday: false, isPast: true };
}

function formatShortDate(dateKey: string): string {
  const [y, m, d] = dateKey.split("-").map(Number);
  const target = new Date(y, m - 1, d);
  const weekday = target.toLocaleDateString("vi-VN", { weekday: "short" });
  return `${weekday}, ${d}/${m}`;
}

// Group calendar events by date key (YYYY-MM-DD)
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

// Sort events by star (highest first), then by time
function sortByImportance(events: CalendarEvent[]): CalendarEvent[] {
  return [...events].sort((a, b) => {
    if (b.star !== a.star) return b.star - a.star;
    return a.releasedDate - b.releasedDate;
  });
}

const starLabel = (star: number) => {
  if (star >= 3) return { text: "HIGH", color: "text-loss", bg: "bg-loss/10" };
  if (star === 2) return { text: "MED", color: "text-warning", bg: "bg-warning/10" };
  return { text: "LOW", color: "text-cyan", bg: "bg-cyan/10" };
};

export default function NewsPage() {
  const [activeFilter, setActiveFilter] = useState("ALL");
  const { items: liveNews, loading, loadingMore, error, hasMore, connected, refresh, loadMore } = useNews({
    pageSize: 50,
    pollInterval: 60_000,
  });
  const { events: calendarEvents, loading: calLoading } = useCalendar({ pastDays: 2, futureDays: 5 });

  // Infinite scroll
  const sentinelRef = useRef<HTMLDivElement>(null);
  const observerRef = useRef<IntersectionObserver | null>(null);

  useEffect(() => {
    if (observerRef.current) observerRef.current.disconnect();

    observerRef.current = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting && hasMore && !loadingMore) {
          loadMore();
        }
      },
      { threshold: 0.1 }
    );

    if (sentinelRef.current) {
      observerRef.current.observe(sentinelRef.current);
    }

    return () => {
      if (observerRef.current) observerRef.current.disconnect();
    };
  }, [hasMore, loadingMore, loadMore]);

  const filteredNews = activeFilter === "IMPORTANT"
    ? liveNews.filter((n) => n.important)
    : liveNews;

  const importantCount = liveNews.filter((n) => n.important).length;

  // Calendar grouped by date with sorted keys
  const calendarGrouped = useMemo(() => groupByDateKey(calendarEvents), [calendarEvents]);
  const dateKeys = useMemo(() => {
    return Array.from(calendarGrouped.keys()).sort();
  }, [calendarGrouped]);

  // Active calendar date tab — default to today
  const todayKey = useMemo(() => {
    const t = new Date();
    return `${t.getFullYear()}-${String(t.getMonth() + 1).padStart(2, "0")}-${String(t.getDate()).padStart(2, "0")}`;
  }, []);
  const [activeCalDate, setActiveCalDate] = useState<string | null>(null);

  // Set initial active date to today (or first available)
  useEffect(() => {
    if (dateKeys.length > 0 && !activeCalDate) {
      setActiveCalDate(dateKeys.includes(todayKey) ? todayKey : dateKeys[0]);
    }
  }, [dateKeys, todayKey, activeCalDate]);

  const activeEvents = useMemo(() => {
    if (!activeCalDate || !calendarGrouped.has(activeCalDate)) return [];
    return sortByImportance(calendarGrouped.get(activeCalDate)!);
  }, [activeCalDate, calendarGrouped]);

  // Group active events by importance
  const highEvents = activeEvents.filter((e) => e.star >= 3);
  const medEvents = activeEvents.filter((e) => e.star === 2);
  const lowEvents = activeEvents.filter((e) => e.star <= 1);

  return (
    <div className="flex gap-6 h-full overflow-y-auto p-6">
      <div className="flex-1 min-w-0 space-y-6">
        <motion.div
          initial={{ opacity: 0, y: 8 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.2 }}
        >
          <PageTitle title="Market News" subtitle="Tin tức thị trường" breadcrumb="NEWS / LIVE FEED" />
        </motion.div>

        {/* Filter Tabs + Status */}
        <motion.div
          initial={{ opacity: 0, y: 6 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.2, delay: 0.05 }}
          className="flex gap-1 items-center"
        >
          {filterTabs.map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveFilter(tab)}
              className={cn(
                "px-4 py-1.5 text-[12px] font-semibold tracking-wider transition-colors",
                activeFilter === tab
                  ? "bg-cyan/10 text-cyan"
                  : "text-muted-foreground hover:bg-secondary"
              )}
            >
              {tab}
              {tab === "IMPORTANT" && importantCount > 0 && (
                <span className="ml-1.5 text-[10px] text-loss font-bold">{importantCount}</span>
              )}
            </button>
          ))}
          <div className="flex-1" />

          {/* WebSocket status */}
          <span className="flex items-center gap-1.5 mr-3">
            <span className={cn(
              "h-2 w-2 rounded-full",
              connected ? "bg-profit live-dot" : "bg-muted-foreground"
            )} />
            <span className={cn(
              "text-[10px] font-bold tracking-wide",
              connected ? "text-profit" : "text-muted-foreground"
            )}>
              {connected ? "LIVE" : "POLLING"}
            </span>
          </span>

        </motion.div>

        {/* Loading */}
        {loading && liveNews.length === 0 && (
          <div className="py-12 text-center">
            <div className="text-[14px] text-muted-foreground animate-pulse">Đang tải tin tức...</div>
          </div>
        )}

        {/* Error */}
        {error && liveNews.length === 0 && (
          <div className="py-12 text-center">
            <div className="text-[14px] text-loss">{error}</div>
            <button onClick={() => location.reload()} className="mt-2 text-[13px] text-cyan hover:underline">Thử lại</button>
          </div>
        )}

        {/* Articles */}
        <div className="space-y-3">
          {filteredNews.map((article, idx) => (
            <motion.a
              key={article.id}
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.15, delay: Math.min(idx * 0.02, 0.3) }}
              href={`https://www.fastbull.com${article.path}`}
              target="_blank"
              rel="noopener noreferrer"
              className={cn(
                "block border bg-card p-5 transition-colors cursor-pointer",
                article.important
                  ? "border-loss/40 hover:border-loss/60"
                  : "border-border hover:border-cyan/30"
              )}
            >
              <div className="flex items-start justify-between gap-4">
                <div className="flex-1">
                  <div className="flex items-center gap-2 mb-2">
                    {article.important && (
                      <span className="px-2.5 py-0.5 text-[11px] font-bold bg-loss/15 text-loss">
                        QUAN TRỌNG
                      </span>
                    )}
                    <span className="text-[11px] text-muted-foreground">
                      <span className="font-mono">{formatExactTime(article.releasedDateMs)}</span>
                      {" · "}
                      {formatNewsTime(article.releasedDateMs)}
                    </span>
                  </div>
                  <h3 className={cn(
                    "text-[16px] font-semibold leading-[1.6]",
                    article.important ? "text-loss" : ""
                  )}>
                    {article.title}
                  </h3>
                </div>
              </div>
            </motion.a>
          ))}

          {/* Infinite scroll sentinel */}
          <div ref={sentinelRef} className="py-4 text-center">
            {loadingMore && (
              <div className="text-[13px] text-muted-foreground animate-pulse">Đang tải thêm...</div>
            )}
            {!hasMore && filteredNews.length > 0 && (
              <div className="text-[12px] text-muted-foreground">Đã tải hết tin tức</div>
            )}
          </div>
        </div>
      </div>

      {/* Right Sidebar — Economic Calendar */}
      <motion.div
        initial={{ opacity: 0, x: 12 }}
        animate={{ opacity: 1, x: 0 }}
        transition={{ duration: 0.25, delay: 0.1 }}
        className="w-[340px] shrink-0 space-y-4"
      >
        <div className="border border-border bg-card">
          {/* Calendar Header */}
          <div className="border-b border-border px-4 py-3">
            <div className="flex items-center justify-between">
              <h3 className="text-sm font-semibold">Lịch kinh tế</h3>
            </div>
          </div>

          {/* Date Tab Navigation */}
          {!calLoading && dateKeys.length > 0 && (
            <div className="flex overflow-x-auto border-b border-border">
              {dateKeys.map((dk) => {
                const rel = getRelativeDateLabel(dk);
                const short = formatShortDate(dk);
                const evCount = calendarGrouped.get(dk)?.length ?? 0;
                const isActive = activeCalDate === dk;
                return (
                  <button
                    key={dk}
                    onClick={() => setActiveCalDate(dk)}
                    className={cn(
                      "flex-shrink-0 px-3 py-2.5 text-center border-b-2 transition-colors min-w-[80px]",
                      isActive
                        ? "border-cyan bg-cyan/5"
                        : "border-transparent hover:bg-secondary/50",
                      rel.isPast && !isActive && "opacity-60"
                    )}
                  >
                    <div className={cn(
                      "text-[10px] font-bold tracking-wide",
                      isActive ? "text-cyan" : rel.isToday ? "text-foreground" : "text-muted-foreground"
                    )}>
                      {rel.label}
                    </div>
                    <div className="text-[9px] text-muted-foreground mt-0.5">{short}</div>
                    <div className={cn(
                      "text-[9px] font-semibold mt-0.5",
                      isActive ? "text-cyan" : "text-text-faint"
                    )}>
                      {evCount} sự kiện
                    </div>
                  </button>
                );
              })}
            </div>
          )}

          {/* Calendar Content */}
          {calLoading ? (
            <div className="px-4 py-8 text-center">
              <div className="text-[12px] text-muted-foreground animate-pulse">Đang tải lịch kinh tế...</div>
            </div>
          ) : calendarEvents.length === 0 ? (
            <div className="px-4 py-8 text-center">
              <div className="text-[12px] text-muted-foreground">Không có sự kiện</div>
            </div>
          ) : (
            <div className="max-h-[calc(100vh-280px)] overflow-y-auto">
              <AnimatePresence mode="wait">
                <motion.div
                  key={activeCalDate}
                  initial={{ opacity: 0, y: 6 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: -6 }}
                  transition={{ duration: 0.15 }}
                >
                  {/* HIGH importance */}
                  {highEvents.length > 0 && (
                    <CalendarSection label="HIGH IMPACT" events={highEvents} starLevel={3} />
                  )}

                  {/* MEDIUM importance */}
                  {medEvents.length > 0 && (
                    <CalendarSection label="MEDIUM IMPACT" events={medEvents} starLevel={2} />
                  )}

                  {/* LOW importance */}
                  {lowEvents.length > 0 && (
                    <CalendarSection label="LOW IMPACT" events={lowEvents} starLevel={1} />
                  )}

                  {activeEvents.length === 0 && (
                    <div className="px-4 py-8 text-center">
                      <div className="text-[12px] text-muted-foreground">Không có sự kiện cho ngày này</div>
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

function CalendarSection({ label, events, starLevel }: { label: string; events: CalendarEvent[]; starLevel: number }) {
  const sl = starLabel(starLevel);
  return (
    <div>
      {/* Section header */}
      <div className="sticky top-0 bg-secondary/80 backdrop-blur-sm px-4 py-2 border-b border-border/50 flex items-center gap-2">
        <span className={cn("px-1.5 py-0.5 text-[8px] font-bold tracking-wider", sl.bg, sl.color)}>
          {"★".repeat(starLevel)}
        </span>
        <span className={cn("text-[10px] font-bold tracking-wider", sl.color)}>{label}</span>
        <span className="text-[9px] text-muted-foreground ml-auto">{events.length}</span>
      </div>
      <div className="divide-y divide-border/30">
        {events.map((event, idx) => (
          <motion.div
            key={event.id}
            initial={{ opacity: 0, x: -4 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ duration: 0.12, delay: idx * 0.02 }}
            className="px-4 py-3 hover:bg-secondary/30 transition-colors"
          >
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <span className="text-[11px] text-muted-foreground font-mono">
                  {formatCalendarTime(event.releasedDate)}
                </span>
                <span className="text-[9px] font-semibold text-text-dim px-1.5 py-0.5 bg-secondary">
                  {event.country}
                </span>
              </div>
            </div>
            <div className="text-[12px] font-medium mt-1 leading-[1.5]">{event.title}</div>
            {event.type === "data" && (event.actual || event.consensus || event.previous) && (
              <div className="flex gap-3 mt-1.5 text-[10px]">
                {event.actual && (
                  <span className="text-profit font-semibold">
                    Thực tế: {event.actual}{event.unit || ""}
                  </span>
                )}
                {event.consensus && (
                  <span className="text-muted-foreground">
                    Dự báo: {event.consensus}{event.unit || ""}
                  </span>
                )}
                {event.previous && (
                  <span className="text-muted-foreground">
                    Trước: {event.previous}{event.unit || ""}
                  </span>
                )}
              </div>
            )}
          </motion.div>
        ))}
      </div>
    </div>
  );
}
