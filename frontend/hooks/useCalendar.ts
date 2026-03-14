"use client";

import { useState, useEffect, useCallback, useRef } from "react";
import type { CalendarEvent } from "@/lib/calendar-types";

interface UseCalendarOptions {
  date?: string; // YYYY-MM-DD (single day mode)
  pastDays?: number; // how many past days to include (default 2)
  futureDays?: number; // how many future days to include (default 5)
  days?: number; // legacy: fetch N past days including today
  importance?: string;
  pollInterval?: number;
}

function formatDate(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

export function useCalendar(options: UseCalendarOptions = {}) {
  const { date, pastDays = 2, futureDays = 5, days, importance, pollInterval = 300_000 } = options;
  const [events, setEvents] = useState<CalendarEvent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const pastDaysRef = useRef(pastDays);
  const futureDaysRef = useRef(futureDays);
  const daysRef = useRef(days);
  pastDaysRef.current = pastDays;
  futureDaysRef.current = futureDays;
  daysRef.current = days;

  const refresh = useCallback(async () => {
    try {
      if (date) {
        // Single day mode
        const params = new URLSearchParams({ date });
        if (importance) params.set("importance", importance);
        const res = await fetch(`/api/calendar?${params.toString()}`);
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        const data = await res.json();
        setEvents(data.events ?? []);
      } else {
        // Multi-day mode: past + today + future
        const today = new Date();
        const dates: string[] = [];

        if (daysRef.current && daysRef.current > 1) {
          // Legacy mode: N past days
          for (let i = daysRef.current - 1; i >= 0; i--) {
            const d = new Date(today);
            d.setDate(d.getDate() - i);
            dates.push(formatDate(d));
          }
        } else {
          // New mode: pastDays + today + futureDays
          for (let i = pastDaysRef.current; i >= 1; i--) {
            const d = new Date(today);
            d.setDate(d.getDate() - i);
            dates.push(formatDate(d));
          }
          dates.push(formatDate(today));
          for (let i = 1; i <= futureDaysRef.current; i++) {
            const d = new Date(today);
            d.setDate(d.getDate() + i);
            dates.push(formatDate(d));
          }
        }

        const results = await Promise.all(
          dates.map(async (dt) => {
            const params = new URLSearchParams({ date: dt });
            if (importance) params.set("importance", importance);
            const res = await fetch(`/api/calendar?${params.toString()}`);
            if (!res.ok) return [];
            const data = await res.json();
            return (data.events ?? []) as CalendarEvent[];
          })
        );

        const all = results.flat();
        all.sort((a, b) => a.releasedDate - b.releasedDate);
        setEvents(all);
      }
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to fetch calendar");
    } finally {
      setLoading(false);
    }
  }, [date, importance]);

  useEffect(() => {
    refresh();
    const interval = setInterval(refresh, pollInterval);
    return () => clearInterval(interval);
  }, [refresh, pollInterval]);

  return { events, loading, error, refresh };
}
