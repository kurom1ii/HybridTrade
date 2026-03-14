import { NextResponse } from "next/server";
import type { NewsItem } from "@/lib/news-types";

const FASTBULL_API =
  "https://api.fastbull.com/fastbull-news-service/api/getNewsPageByTagIds";

const DEFAULT_PARAMS = {
  checkImportant: "0",
  pageSize: "30",
  timestamp: "",
  includeCalendar: "1",
  tagIds: "",
};

interface FastBullPageData {
  newsId: string;
  newsTitle: string;
  important: number;
  releasedDate: number;
  tags: string;
  path: string;
  smallImg?: string | null;
}

export async function GET(request: Request) {
  const { searchParams } = new URL(request.url);
  const pageSize = searchParams.get("pageSize") || "30";
  const before = searchParams.get("before") || "";

  // Support checkImportant from agent tool (true/false/1/0)
  const importantRaw = searchParams.get("checkImportant") ?? searchParams.get("important");
  const checkImportant = (importantRaw === "true" || importantRaw === "1") ? "1" : "0";

  const params = new URLSearchParams({
    ...DEFAULT_PARAMS,
    pageSize,
    timestamp: before,
    checkImportant,
  });

  try {
    const res = await fetch(`${FASTBULL_API}?${params.toString()}`, {
      headers: {
        "Accept": "application/json",
        "Accept-Language": "vi-VN,vi;q=0.9,en;q=0.8",
        "lang": "vi",
        "User-Agent":
          "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
        "Referer": "https://www.fastbull.com/vi/express-news",
        "Origin": "https://www.fastbull.com",
      },
      cache: "no-store",
    });

    if (!res.ok) {
      return NextResponse.json(
        { error: `FastBull API returned ${res.status}` },
        { status: 502 }
      );
    }

    const data = await res.json();

    // FastBull returns code:0 for success
    if (data.code !== 0) {
      return NextResponse.json(
        { error: `FastBull API error: ${data.message}` },
        { status: 502 }
      );
    }

    let body: { pageDatas: FastBullPageData[] };
    try {
      body = typeof data.bodyMessage === "string"
        ? JSON.parse(data.bodyMessage)
        : data.bodyMessage;
    } catch {
      return NextResponse.json(
        { error: "Failed to parse FastBull bodyMessage" },
        { status: 502 }
      );
    }

    const items: NewsItem[] = (body.pageDatas || [])
      .map((p) => ({
        id: p.newsId,
        title: p.newsTitle,
        releasedDateMs: p.releasedDate,
        important: p.important === 1,
        tags: [],
        path: `/express-news/${p.path}`,
        smallImg: p.smallImg || null,
      }));

    return NextResponse.json({ items }, {
      headers: {
        "Cache-Control": "no-store, no-cache, must-revalidate",
      },
    });
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : "Unknown error";
    return NextResponse.json(
      { error: `Failed to fetch news: ${message}` },
      { status: 502 }
    );
  }
}
