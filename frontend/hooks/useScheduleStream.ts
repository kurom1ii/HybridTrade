"use client";

import { useState, useEffect, useRef, useCallback } from "react";

interface UseScheduleStreamOptions {
  enabled?: boolean;
  onJobUpdate?: (data: Record<string, unknown>) => void;
}

export function useScheduleStream(options: UseScheduleStreamOptions = {}) {
  const { enabled = true, onJobUpdate } = options;
  const [connected, setConnected] = useState(false);
  const onJobUpdateRef = useRef(onJobUpdate);
  onJobUpdateRef.current = onJobUpdate;

  useEffect(() => {
    if (!enabled) return;

    const es = new EventSource("/api/schedules/stream");

    es.onopen = () => setConnected(true);
    es.onerror = () => setConnected(false);

    es.addEventListener("job_status", (e) => {
      try {
        const data = JSON.parse(e.data);
        onJobUpdateRef.current?.(data);
      } catch { /* ignore parse errors */ }
    });

    return () => {
      es.close();
      setConnected(false);
    };
  }, [enabled]);

  return { connected };
}
