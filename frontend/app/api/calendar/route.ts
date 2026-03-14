import { NextResponse } from "next/server";
import { createHash } from "crypto";
import type { CalendarEvent } from "@/lib/calendar-types";

const CALENDAR_API =
  "https://api.fastbull.com/fastbull-news-service/api/getMergeCalendarV1Page";
const API_PATH = "/fastbull-news-service/api/getMergeCalendarV1Page";
const ENV_UUID = "1c6d674e8701f635b4c4933b0b1a360b";
const DEVICE_ID = "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6"; // static device ID for server

function md5(input: string): string {
  return createHash("md5").update(input).digest("hex");
}

function generateNonce(length = 8): string {
  const chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
  let result = "";
  for (let i = 0; i < length; i++) {
    result += chars[Math.floor(Math.random() * chars.length)];
  }
  return result;
}

function buildHeaders(): Record<string, string> {
  const timestamp = String(Date.now());
  const nonce = generateNonce();
  const sign = md5(DEVICE_ID + API_PATH + timestamp + nonce);
  const bToken = md5(`Web_4.2.0_${ENV_UUID}_${DEVICE_ID}`);

  return {
    "timestamp": timestamp,
    "nonce": nonce,
    "sign": sign,
    "b-token": bToken,
    "lang": "vi",
    "client-type": "Web",
    "client-version": "4.2.0",
    "device-id": DEVICE_ID,
    "version": "4",
    "Accept": "application/json",
    "Origin": "https://www.fastbull.com",
    "Referer": "https://www.fastbull.com/calendar",
    "User-Agent":
      "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
  };
}

interface CalendarDataModel {
  calendarId: number;
  title: string;
  country: string;
  countryImg?: string | null;
  releasedDate: number;
  actual: string | null;
  consensus: string | null;
  previous: string | null;
  unit?: string | null;
  star: number;
  path?: string | null;
}

interface CalendarEventModel {
  calendarId: string;
  eventContent: string;
  country: string;
  releasedDate: number;
  star: number;
  path?: string | null;
}

interface MergeItem {
  type: number; // 1=data, 2=event, 3=holiday
  calendarDataModel?: CalendarDataModel | null;
  calenderEventModel?: CalendarEventModel | null;
}

export async function GET(request: Request) {
  const { searchParams } = new URL(request.url);
  const dateParam = searchParams.get("date"); // YYYY-MM-DD or empty for today
  const importanceRaw = searchParams.get("importance") || "";

  // Map human-readable importance (from agent tool) to FastBull star levels
  const importanceMap: Record<string, string> = {
    high: "3", medium: "2", low: "1",
    "3": "3", "2": "2", "1": "1",
  };
  const importance = importanceMap[importanceRaw.toLowerCase()] || importanceRaw;

  // Calculate start/end timestamps for the day (UTC+7)
  const targetDate = dateParam ? new Date(dateParam + "T00:00:00+07:00") : new Date();
  if (!dateParam) {
    // Set to start of today in UTC+7
    targetDate.setHours(targetDate.getHours() - targetDate.getTimezoneOffset() / 60);
    targetDate.setHours(0, 0, 0, 0);
  }
  const startOfDay = new Date(targetDate);
  startOfDay.setHours(0, 0, 0, 0);
  const endOfDay = new Date(targetDate);
  endOfDay.setHours(23, 59, 59, 0);

  const startTimestamp = startOfDay.getTime();
  const endTimestamp = endOfDay.getTime();

  const params = new URLSearchParams({
    attributeIds: "",
    countryIds: "",
    categoryIds: "",
    importance,
    releaseState: "",
    startTimestamp: String(startTimestamp),
    endTimestamp: String(endTimestamp),
    keyword: "",
    types: "1,2,3",
    pageSize: "50",
  });

  try {
    const res = await fetch(`${CALENDAR_API}?${params.toString()}`, {
      headers: buildHeaders(),
      next: { revalidate: 120 },
    });

    if (!res.ok) {
      return NextResponse.json(
        { error: `Calendar API returned ${res.status}` },
        { status: 502 }
      );
    }

    const data = await res.json();
    if (data.code !== 0) {
      return NextResponse.json(
        { error: `Calendar API error: ${data.message}` },
        { status: 502 }
      );
    }

    let body: { mergeList: MergeItem[] };
    try {
      body = typeof data.bodyMessage === "string"
        ? JSON.parse(data.bodyMessage)
        : data.bodyMessage;
    } catch {
      return NextResponse.json(
        { error: "Failed to parse calendar bodyMessage" },
        { status: 502 }
      );
    }

    const events: CalendarEvent[] = (body.mergeList || []).map((item) => {
      if (item.type === 1 && item.calendarDataModel) {
        const d = item.calendarDataModel;
        return {
          id: String(d.calendarId),
          title: d.title,
          country: d.country,
          countryImg: d.countryImg || null,
          releasedDate: d.releasedDate,
          star: d.star,
          type: "data" as const,
          actual: d.actual,
          consensus: d.consensus,
          previous: d.previous,
          unit: d.unit || null,
          path: d.path ? `/calendar/${d.path}` : null,
        };
      } else if (item.type === 2 && item.calenderEventModel) {
        const e = item.calenderEventModel;
        return {
          id: String(e.calendarId),
          title: e.eventContent,
          country: e.country,
          countryImg: null,
          releasedDate: e.releasedDate,
          star: e.star,
          type: "event" as const,
          actual: null,
          consensus: null,
          previous: null,
          unit: null,
          path: e.path ? `/calendar/${e.path}` : null,
        };
      }
      // type 3 = holiday or unknown
      return {
        id: `holiday-${item.type}-${Math.random()}`,
        title: "Holiday",
        country: "",
        countryImg: null,
        releasedDate: 0,
        star: 1,
        type: "holiday" as const,
        actual: null,
        consensus: null,
        previous: null,
        unit: null,
        path: null,
      };
    }).filter((e) => e.releasedDate > 0);

    // Sort by time
    events.sort((a, b) => a.releasedDate - b.releasedDate);

    return NextResponse.json({ events }, {
      headers: {
        "Cache-Control": "public, s-maxage=120, stale-while-revalidate=300",
      },
    });
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : "Unknown error";
    return NextResponse.json(
      { error: `Failed to fetch calendar: ${message}` },
      { status: 502 }
    );
  }
}
