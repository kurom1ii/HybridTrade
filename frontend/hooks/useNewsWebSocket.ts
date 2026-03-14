"use client";

import { useState, useEffect, useCallback, useRef } from "react";
import type { NewsItem } from "@/lib/news-types";

const WS_URL = "wss://wsspush.fastbull.com/news?langId=10&appType=1&dataType=3";
const RECONNECT_DELAY = 3000;
const HEARTBEAT_INTERVAL = 25000;

interface WsNewsMessage {
  messageInfo: string;
  messageType: string;
  syncApp: number[];
}

interface WsNewsInfo {
  newsId: string;
  newsTitle: string;
  important: number;
  releasedDate: number;
  langId: number;
  tags: string;
  path: string;
  smallImg?: string | null;
}

function parseWsMessage(data: string): NewsItem | null {
  if (data === "10") return null;
  try {
    const msg: WsNewsMessage = JSON.parse(data);
    if (msg.messageType !== "news") return null;
    const info: WsNewsInfo = JSON.parse(msg.messageInfo);
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

// ─── Singleton WebSocket ───
// One connection shared by all hook consumers
type WsListener = (connected: boolean, items: NewsItem[]) => void;

let _ws: WebSocket | null = null;
let _connected = false;
let _items: NewsItem[] = [];
let _reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let _heartbeatTimer: ReturnType<typeof setInterval> | null = null;
let _refCount = 0;
const _listeners = new Set<WsListener>();

function _notify() {
  _listeners.forEach((fn) => fn(_connected, _items));
}

function _connect() {
  if (_ws?.readyState === WebSocket.OPEN || _ws?.readyState === WebSocket.CONNECTING) return;
  try {
    const ws = new WebSocket(WS_URL);
    _ws = ws;

    ws.onopen = () => {
      _connected = true;
      ws.send(JSON.stringify({ t: "SIGNAL|LANG_10" }));
      _notify();
      _heartbeatTimer = setInterval(() => {
        if (ws.readyState === WebSocket.OPEN) ws.send("10");
      }, HEARTBEAT_INTERVAL);
    };

    ws.onmessage = (event) => {
      const data = typeof event.data === "string" ? event.data : null;
      if (!data) return;
      const item = parseWsMessage(data);
      if (item) {
        const exists = _items.some((p) => p.id === item.id);
        if (exists) {
          _items = _items.map((p) => (p.id === item.id ? item : p));
        } else {
          _items = [item, ..._items];
        }
        _notify();
      }
    };

    ws.onclose = () => {
      _connected = false;
      if (_heartbeatTimer) { clearInterval(_heartbeatTimer); _heartbeatTimer = null; }
      _notify();
      if (_refCount > 0) {
        _reconnectTimer = setTimeout(_connect, RECONNECT_DELAY);
      }
    };

    ws.onerror = () => ws.close();
  } catch {
    if (_refCount > 0) {
      _reconnectTimer = setTimeout(_connect, RECONNECT_DELAY);
    }
  }
}

function _disconnect() {
  if (_ws) { _ws.close(); _ws = null; }
  if (_reconnectTimer) { clearTimeout(_reconnectTimer); _reconnectTimer = null; }
  if (_heartbeatTimer) { clearInterval(_heartbeatTimer); _heartbeatTimer = null; }
  _connected = false;
}

export function useNewsWebSocket() {
  const [connected, setConnected] = useState(_connected);
  const [realtimeItems, setRealtimeItems] = useState<NewsItem[]>(_items);

  useEffect(() => {
    // Subscribe
    const listener: WsListener = (conn, items) => {
      setConnected(conn);
      setRealtimeItems(items);
    };
    _listeners.add(listener);

    // Ref-count connect
    _refCount++;
    if (_refCount === 1) {
      _connect();
    } else {
      // Already connected, sync state
      setConnected(_connected);
      setRealtimeItems(_items);
    }

    return () => {
      _listeners.delete(listener);
      _refCount--;
      if (_refCount <= 0) {
        _refCount = 0;
        _disconnect();
      }
    };
  }, []);

  return { connected, realtimeItems };
}
