"use client";

import { useState, useEffect, useCallback, useRef } from "react";
import type { NewsItem } from "@/lib/news-types";
import { fetchLatestNews } from "@/lib/news-client";
import { useNewsWebSocket } from "./useNewsWebSocket";

interface UseNewsOptions {
  pageSize?: number;
  pollInterval?: number;
  enabled?: boolean;
}

export function useNews(options: UseNewsOptions = {}) {
  const { pageSize = 30, pollInterval = 60_000, enabled = true } = options;
  const [restItems, setRestItems] = useState<NewsItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);
  const [hasMore, setHasMore] = useState(true);
  const pageSizeRef = useRef(pageSize);
  pageSizeRef.current = pageSize;

  // Shared WebSocket — singleton, no duplicate connections
  const { connected, realtimeItems } = useNewsWebSocket();

  // Fetch from REST API — stable reference, no clearRealtime
  const refresh = useCallback(async () => {
    try {
      const data = await fetchLatestNews(pageSizeRef.current);
      setRestItems(data);
      setError(null);
      setLastUpdated(new Date());
      setHasMore(data.length >= pageSizeRef.current);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to fetch news");
    } finally {
      setLoading(false);
    }
  }, []);

  // Load more (pagination)
  const loadMore = useCallback(async () => {
    if (loadingMore || !hasMore) return;
    setLoadingMore(true);
    try {
      const allCurrent = [...realtimeItems, ...restItems];
      const oldest = allCurrent[allCurrent.length - 1];
      if (!oldest) return;

      const res = await fetch(`/api/news?pageSize=${pageSizeRef.current}&before=${oldest.releasedDateMs}&_t=${Date.now()}`, {
        cache: "no-store",
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      const newItems: NewsItem[] = data.items ?? [];

      if (newItems.length === 0) {
        setHasMore(false);
      } else {
        if (newItems.length < pageSizeRef.current) {
          setHasMore(false);
        }
        setRestItems((prev) => {
          const existingIds = new Set(prev.map((p) => p.id));
          const unique = newItems.filter((n: NewsItem) => !existingIds.has(n.id));
          return [...prev, ...unique];
        });
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load more");
    } finally {
      setLoadingMore(false);
    }
  }, [loadingMore, hasMore, realtimeItems, restItems]);

  useEffect(() => {
    if (!enabled) return;
    refresh();
    const id = setInterval(refresh, pollInterval);
    return () => clearInterval(id);
  }, [enabled, pollInterval, refresh]);

  // Merge: WS real-time items first (newest), then REST items, deduped
  const items = (() => {
    const seen = new Set<string>();
    const merged: NewsItem[] = [];
    for (const item of [...realtimeItems, ...restItems]) {
      if (!seen.has(item.id)) {
        seen.add(item.id);
        merged.push(item);
      }
    }
    return merged;
  })();

  return {
    items,
    loading,
    loadingMore,
    error,
    lastUpdated,
    hasMore,
    connected,
    refresh,
    loadMore,
  };
}
