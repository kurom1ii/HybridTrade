"use client";

import { startTransition, useCallback, useEffect, useRef, useState } from "react";

interface UsePollingOptions {
  enabled?: boolean;
  intervalMs?: number;
}

export function usePollingResource<T>(
  key: string,
  loader: () => Promise<T>,
  options?: UsePollingOptions,
) {
  const { enabled = true, intervalMs = 0 } = options ?? {};
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(enabled);
  const [error, setError] = useState<string | null>(null);
  const loaderRef = useRef(loader);
  loaderRef.current = loader;
  const inFlight = useRef(false);

  const runLoad = useCallback(async () => {
    if (inFlight.current) return;
    inFlight.current = true;
    setLoading(true);
    try {
      const nextData = await loaderRef.current();
      startTransition(() => {
        setData(nextData);
        setError(null);
      });
    } catch (loadError) {
      const message = loadError instanceof Error ? loadError.message : "Không thể tải dữ liệu";
      setError(message);
    } finally {
      setLoading(false);
      inFlight.current = false;
    }
  }, []);

  useEffect(() => {
    if (!enabled) {
      setLoading(false);
      return;
    }

    // Fetch once on mount
    void runLoad();

    // Optional polling (only if intervalMs > 0)
    if (intervalMs <= 0) return;

    const timer = window.setInterval(() => {
      // Only poll when tab is visible
      if (!document.hidden) {
        void runLoad();
      }
    }, intervalMs);
    return () => window.clearInterval(timer);
  }, [enabled, intervalMs, key, runLoad]);

  return {
    data,
    loading,
    error,
    reload: runLoad,
  };
}
