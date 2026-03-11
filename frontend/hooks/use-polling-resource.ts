"use client";

import { startTransition, useEffect, useEffectEvent, useState } from "react";

interface UsePollingOptions {
  enabled?: boolean;
  intervalMs?: number;
}

export function usePollingResource<T>(
  key: string,
  loader: () => Promise<T>,
  options?: UsePollingOptions,
) {
  const { enabled = true, intervalMs = 15_000 } = options ?? {};
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(enabled);
  const [error, setError] = useState<string | null>(null);

  const runLoad = useEffectEvent(async () => {
    if (!enabled) return;
    setLoading(true);
    try {
      const nextData = await loader();
      startTransition(() => {
        setData(nextData);
        setError(null);
      });
    } catch (loadError) {
      const message = loadError instanceof Error ? loadError.message : "Khong the tai du lieu";
      setError(message);
    } finally {
      setLoading(false);
    }
  });

  useEffect(() => {
    if (!enabled) {
      setLoading(false);
      return;
    }

    void runLoad();
    if (intervalMs <= 0) return;

    const timer = window.setInterval(() => {
      void runLoad();
    }, intervalMs);
    return () => window.clearInterval(timer);
  }, [enabled, intervalMs, key, runLoad]);

  return {
    data,
    loading,
    error,
    reload: () => {
      void runLoad();
    },
  };
}

