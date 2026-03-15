export const runtime = "nodejs";
export const dynamic = "force-dynamic";

const BACKEND_URL =
  process.env.NEXT_PUBLIC_API_BASE_URL ?? "http://127.0.0.1:8080";

export async function GET() {
  const encoder = new TextEncoder();
  let upstream: ReadableStreamDefaultReader<Uint8Array> | null = null;
  let closed = false;
  let heartbeatId: ReturnType<typeof setInterval> | null = null;

  const stream = new ReadableStream({
    async start(controller) {
      // SSE keepalive
      heartbeatId = setInterval(() => {
        if (closed) return;
        try {
          controller.enqueue(encoder.encode(`: heartbeat\n\n`));
        } catch {
          closed = true;
        }
      }, 15_000);

      try {
        const res = await fetch(`${BACKEND_URL}/api/schedules/stream`, {
          headers: { Accept: "text/event-stream" },
          cache: "no-store",
        });
        if (!res.ok || !res.body) {
          controller.enqueue(
            encoder.encode(
              `event: error\ndata: ${JSON.stringify({ message: `Backend ${res.status}` })}\n\n`
            )
          );
          controller.close();
          return;
        }

        upstream = res.body.getReader();
        const decoder = new TextDecoder();

        while (!closed) {
          const { done, value } = await upstream.read();
          if (done) break;
          const chunk = decoder.decode(value, { stream: true });
          try {
            controller.enqueue(encoder.encode(chunk));
          } catch {
            closed = true;
          }
        }
      } catch (err) {
        if (!closed) {
          try {
            controller.enqueue(
              encoder.encode(
                `event: error\ndata: ${JSON.stringify({
                  message: err instanceof Error ? err.message : "upstream error",
                })}\n\n`
              )
            );
          } catch { /* already closed */ }
        }
      } finally {
        if (heartbeatId) clearInterval(heartbeatId);
        try { controller.close(); } catch { /* already closed */ }
      }
    },

    cancel() {
      closed = true;
      if (heartbeatId) clearInterval(heartbeatId);
      if (upstream) upstream.cancel().catch(() => {});
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
