import type { NewsItem } from "@/lib/news-types";

export async function fetchLatestNews(pageSize = 30): Promise<NewsItem[]> {
  const res = await fetch(`/api/news?pageSize=${pageSize}&_t=${Date.now()}`, {
    cache: "no-store",
  });
  if (!res.ok) {
    throw new Error(`News API error: ${res.status}`);
  }
  const data = await res.json();
  return data.items ?? [];
}
