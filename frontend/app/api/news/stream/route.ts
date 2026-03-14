import type { NewsItem } from "@/lib/news-types";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

const FASTBULL_NEWS_API =
  "https://api.fastbull.com/fastbull-news-service/api/getNewsPageByTagIds";
const FASTBULL_NEWS_WS =
  "wss://wsspush.fastbull.com/news?langId=10&appType=1&dataType=3";
const HEARTBEAT_INTERVAL = 20_000;
const WS_RECONNECT_DELAY = 3_000;
const WS_HEARTBEAT_INTERVAL = 25_000;

// ─── FastBull REST fetch (initial batch) ───

async function fetchInitialNews(pageSize = 40): Promise<NewsItem[]> {
  const params = new URLSearchParams({
    checkImportant: "0",
    pageSize: String(pageSize),
    timestamp: "",
    includeCalendar: "1",
    tagIds: "",
  });
  const res = await fetch(`${FASTBULL_NEWS_API}?${params}`, {
    headers: {
      Accept: "application/json",
      "Accept-Language": "vi-VN,vi;q=0.9,en;q=0.8",
      lang: "vi",
      Referer: "https://www.fastbull.com/vi/express-news",
      Origin: "https://www.fastbull.com",
    },
    cache: "no-store",
  });
  if (!res.ok) throw new Error(`FastBull ${res.status}`);
  const data = await res.json();
  if (data.code !== 0) throw new Error(data.message);
  const body =
    typeof data.bodyMessage === "string"
      ? JSON.parse(data.bodyMessage)
      : data.bodyMessage;
  return (body.pageDatas || []).map((p: any) => ({
    id: p.newsId,
    title: p.newsTitle,
    releasedDateMs: p.releasedDate,
    important: p.important === 1,
    tags: [],
    path: `/express-news/${p.path}`,
    smallImg: p.smallImg || null,
  }));
}

// ─── Parse FastBull WS news message ───

function parseWsNews(raw: string): NewsItem | null {
  if (raw === "10") return null;
  try {
    const msg = JSON.parse(raw);
    if (msg.messageType !== "news") return null;
    const info = JSON.parse(msg.messageInfo);
    if (!info.newsTitle || !info.newsId) return null;
    return {
      id: info.newsId,
      title: info.newsTitle,
      releasedDateMs: info.releasedDate,
      important: info.important === 1,
      tags: [],
      path: `/express-news/${info.path}`,
      smallImg: info.smallImg || null,
    };
  } catch {
    return null;
  }
}

// ─── SSE endpoint ───

export async function GET() {
  const encoder = new TextEncoder();
  let ws: WebSocket | null = null;
  let heartbeatId: ReturnType<typeof setInterval> | null = null;
  let wsHeartbeatId: ReturnType<typeof setInterval> | null = null;
  let reconnectId: ReturnType<typeof setTimeout> | null = null;
  let closed = false;

  const stream = new ReadableStream({
    async start(controller) {
      // Helper: send SSE event
      const send = (event: string, data: unknown) => {
        if (closed) return;
        try {
          controller.enqueue(
            encoder.encode(`event: ${event}\ndata: ${JSON.stringify(data)}\n\n`)
          );
        } catch {
          closed = true;
        }
      };

      // 1. Send initial batch from REST
      try {
        const items = await fetchInitialNews();
        send("init", items);
      } catch (err) {
        send("error", {
          message: err instanceof Error ? err.message : "Failed to fetch news",
        });
      }

      // 2. Connect to FastBull WS for real-time push
      const connectWs = () => {
        if (closed) return;
        try {
          const socket = new WebSocket(FASTBULL_NEWS_WS);
          ws = socket;

          socket.addEventListener("open", () => {
            socket.send(JSON.stringify({ t: "SIGNAL|LANG_10" }));
            send("status", { connected: true });

            // WS keepalive
            wsHeartbeatId = setInterval(() => {
              if (socket.readyState === WebSocket.OPEN) socket.send("10");
            }, WS_HEARTBEAT_INTERVAL);
          });

          socket.addEventListener("message", (event) => {
            const data =
              typeof event.data === "string" ? event.data : null;
            if (!data) return;
            const item = parseWsNews(data);
            if (item) send("news", item);
          });

          socket.addEventListener("close", () => {
            if (wsHeartbeatId) {
              clearInterval(wsHeartbeatId);
              wsHeartbeatId = null;
            }
            send("status", { connected: false });
            // Reconnect
            if (!closed) {
              reconnectId = setTimeout(connectWs, WS_RECONNECT_DELAY);
            }
          });

          socket.addEventListener("error", () => socket.close());
        } catch {
          if (!closed) {
            reconnectId = setTimeout(connectWs, WS_RECONNECT_DELAY);
          }
        }
      };

      connectWs();

      // 3. SSE keepalive (prevent proxy/browser timeout)
      heartbeatId = setInterval(() => {
        if (closed) return;
        try {
          controller.enqueue(encoder.encode(`: heartbeat\n\n`));
        } catch {
          closed = true;
        }
      }, HEARTBEAT_INTERVAL);
    },

    cancel() {
      closed = true;
      if (ws) {
        ws.close();
        ws = null;
      }
      if (heartbeatId) clearInterval(heartbeatId);
      if (wsHeartbeatId) clearInterval(wsHeartbeatId);
      if (reconnectId) clearTimeout(reconnectId);
    },
  });

  return new Response(stream, {
    headers: {
      "Content-Type": "text/event-stream",
      "Cache-Control": "no-cache, no-store",
      Connection: "keep-alive",
      "X-Accel-Buffering": "no",
    },
  });
}
