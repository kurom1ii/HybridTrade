"use client";

import { useEffect, useEffectEvent, useState } from "react";
import { investigationStreamUrl } from "@/lib/intelligence-api";
import { AppStreamEvent } from "@/lib/intelligence-types";

export type StreamStatus = "idle" | "connecting" | "connected" | "error";

const EVENT_TYPES = [
  "investigation.updated",
  "agent.message",
  "finding.created",
  "section.concluded",
  "run.completed",
  "heartbeat",
  "job.status",
];

export function useInvestigationStream(
  investigationId: string | null,
  onEvent: (event: AppStreamEvent) => void,
) {
  const [status, setStatus] = useState<StreamStatus>(investigationId ? "connecting" : "idle");
  const handleEvent = useEffectEvent((data: string) => {
    const parsed = JSON.parse(data) as AppStreamEvent;
    onEvent(parsed);
  });

  useEffect(() => {
    if (!investigationId) {
      setStatus("idle");
      return;
    }

    setStatus("connecting");
    const source = new EventSource(investigationStreamUrl(investigationId));
    const listener = (event: Event) => {
      handleEvent((event as MessageEvent<string>).data);
    };

    source.onopen = () => setStatus("connected");
    source.onerror = () => setStatus("error");
    for (const eventType of EVENT_TYPES) {
      source.addEventListener(eventType, listener);
    }

    return () => {
      for (const eventType of EVENT_TYPES) {
        source.removeEventListener(eventType, listener);
      }
      source.close();
    };
  }, [handleEvent, investigationId]);

  return status;
}

