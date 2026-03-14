"use client";

import { useState, useEffect, useCallback, useRef } from "react";
import type { NewsItem } from "@/lib/news-types";

interface UseNewsOptions {
  pageSize?: number;
  enabled?: boolean;
}

export function useNews(options: UseNewsOptions = {}) {
  const { pageSize = 30, enabled = true } = options;
  const [items, setItems] = useState<NewsItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);
  const [hasMore, setHasMore] = useState(true);
  const [connected, setConnected] = useState(false);
  const pageSizeRef = useRef(pageSize);
  pageSizeRef.current = pageSize;

  // SSE connection — replaces polling + direct WS
  useEffect(() => {
    if (!enabled) return;

    const es = new EventSource("/api/news/stream");

    es.addEventListener("init", (e) => {
      const initial: NewsItem[] = JSON.parse(e.data);
      setItems(initial);
      setLoading(false);
      setLastUpdated(new Date());
      setHasMore(initial.length >= pageSizeRef.current);
    });

    es.addEventListener("news", (e) => {
      const item: NewsItem = JSON.parse(e.data);
      setItems((prev) => {
        if (prev.some((p) => p.id === item.id)) {
          return prev.map((p) => (p.id === item.id ? item : p));
        }
        return [item, ...prev];
      });
      setLastUpdated(new Date());
    });

    es.addEventListener("status", (e) => {
      const { connected: conn } = JSON.parse(e.data);
      setConnected(conn);
    });

    es.addEventListener("error", (e) => {
      try {
        const { message } = JSON.parse((e as MessageEvent).data);
        setError(message);
      } catch {
        // Browser-level SSE error (disconnect)
        setConnected(false);
      }
      setLoading(false);
    });

    es.onopen = () => setConnected(true);

    return () => es.close();
  }, [enabled]);

  // Load more (pagination) — still uses REST endpoint for older news
  const loadMore = useCallback(async () => {
    if (loadingMore || !hasMore) return;
    setLoadingMore(true);
    try {
      const oldest = items[items.length - 1];
      if (!oldest) return;

      const res = await fetch(
        `/api/news?pageSize=${pageSizeRef.current}&before=${oldest.releasedDateMs}&_t=${Date.now()}`,
        { cache: "no-store" }
      );
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      const newItems: NewsItem[] = data.items ?? [];

      if (newItems.length === 0 || newItems.length < pageSizeRef.current) {
        setHasMore(false);
      }
      if (newItems.length > 0) {
        setItems((prev) => {
          const existingIds = new Set(prev.map((p) => p.id));
          const unique = newItems.filter((n) => !existingIds.has(n.id));
          return [...prev, ...unique];
        });
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load more");
    } finally {
      setLoadingMore(false);
    }
  }, [loadingMore, hasMore, items]);

  return {
    items,
    loading,
    loadingMore,
    error,
    lastUpdated,
    hasMore,
    connected,
    loadMore,
  };
}
