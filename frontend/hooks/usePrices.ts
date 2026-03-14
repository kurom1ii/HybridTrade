"use client";

import { useState, useEffect } from "react";
import type { PriceSnapshot } from "@/app/api/prices/route";

// ─── FastBull Tick WebSocket ───
// Real-time price feed via wss://wsspush.fastbull.com/tick
// Binary frames are UTF-8 JSON: { category: 1, data: "stockId,prev,ask,bid,open,close,high,low,,ts,change,changeRate,utc,precision,vol,turnOver,status" }

const WS_URL = "wss://wsspush.fastbull.com/tick?langId=0";
const RECONNECT_DELAY = 3000;

/** Map display symbol → FastBull stockId for WS subscription */
const SYMBOL_TO_STOCK_ID: Record<string, string> = {
  XAUUSD: "8500_XAUUSD",
  XAGUSD: "8500_XAGUSD",
  EURUSD: "8100_EURUSD",
  GBPUSD: "8100_GBPUSD",
  USDJPY: "8100_USDJPY",
  GBPJPY: "8200_GBPJPY",
  USDCAD: "8100_USDCAD",
  AUDUSD: "8100_AUDUSD",
  NZDUSD: "8100_NZDUSD",
  USDCHF: "8100_USDCHF",
  BTCUSDT: "6100_BTC-USDT",
  ETHUSDT: "6100_ETH-USDT",
  SOLUSDT: "6100_SOL-USDT",
  USNDAQ100: "8700_USNDAQ100",
  US30: "8700_US30",
  US500: "8700_USSPX500",
  UK100: "8700_UK100",
  JP225: "8700_Japan225",
  WTI: "8600_WTI",
  BRENT: "8600_BRENT",
  NATGAS: "8600_NAT.GAS",
  XPTUSD: "8500_XPTUSD",
  XPDUSD: "8500_XPDUSD",
  COPPER: "8800_COPPER",
};

/** Reverse map: stockId → display symbol */
const STOCK_ID_TO_SYMBOL: Record<string, string> = {};
for (const [sym, sid] of Object.entries(SYMBOL_TO_STOCK_ID)) {
  STOCK_ID_TO_SYMBOL[sid] = sym;
}

function categoryFromStockId(stockId: string): string {
  if (stockId.startsWith("81") || stockId.startsWith("82")) return "FOREX";
  if (stockId.startsWith("85") || stockId.startsWith("86") || stockId.startsWith("88")) return "COMMODITIES";
  if (stockId.startsWith("61") || stockId.startsWith("63")) return "CRYPTO";
  if (stockId.startsWith("87") || stockId.startsWith("91")) return "INDICES";
  return "OTHER";
}

function parseTick(csv: string): PriceSnapshot | null {
  const r = csv.split(",");
  if (r.length < 14) return null;
  const stockId = r[0];
  const symbol = STOCK_ID_TO_SYMBOL[stockId] || stockId.split("_")[1] || stockId;
  return {
    symbol,
    name: symbol,
    price: parseFloat(r[5]) || 0,  // close = current price
    bid: parseFloat(r[3]) || 0,
    ask: parseFloat(r[2]) || 0,
    change: parseFloat(r[10]) || 0,
    changePct: parseFloat(r[11]) || 0,
    high: parseFloat(r[6]) || 0,
    low: parseFloat(r[7]) || 0,
    open: parseFloat(r[4]) || 0,
    prev: parseFloat(r[1]) || 0,
    precision: parseInt(r[13]) || 2,
    category: categoryFromStockId(stockId),
    stockId,
  };
}

// ─── Singleton WebSocket (shared by all hook consumers) ───
type TickListener = (connected: boolean, prices: Map<string, PriceSnapshot>) => void;

let _ws: WebSocket | null = null;
let _connected = false;
let _prices = new Map<string, PriceSnapshot>();
let _subscribedIds = new Set<string>();
let _reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let _refCount = 0;
const _listeners = new Set<TickListener>();
const _decoder = typeof TextDecoder !== "undefined" ? new TextDecoder("utf-8") : null;

function _notify() {
  _listeners.forEach((fn) => fn(_connected, _prices));
}

function _subscribe(symbols: string[]) {
  const ids = symbols
    .map((s) => SYMBOL_TO_STOCK_ID[s])
    .filter(Boolean);

  // Track what we're subscribed to
  ids.forEach((id) => _subscribedIds.add(id));

  if (_ws?.readyState === WebSocket.OPEN && ids.length > 0) {
    _ws.send(JSON.stringify({ t: ids.join("|") }));
  }
}

function _connect() {
  if (_ws?.readyState === WebSocket.OPEN || _ws?.readyState === WebSocket.CONNECTING) return;
  try {
    const ws = new WebSocket(WS_URL);
    ws.binaryType = "arraybuffer";
    _ws = ws;

    ws.onopen = () => {
      _connected = true;
      // Re-subscribe all tracked symbols
      if (_subscribedIds.size > 0) {
        ws.send(JSON.stringify({ t: Array.from(_subscribedIds).join("|") }));
      }
      _notify();
    };

    ws.onmessage = (event) => {
      if (!(event.data instanceof ArrayBuffer)) return;
      const buf = event.data as ArrayBuffer;
      // Heartbeat: single byte "1" (0x31)
      if (buf.byteLength <= 2) return;

      if (!_decoder) return;
      try {
        const text = _decoder.decode(new Uint8Array(buf));
        const msg = JSON.parse(text);
        // category 1 or 4 = price tick
        if ((msg.category === 1 || msg.category === 4) && msg.data) {
          const tick = parseTick(msg.data);
          if (tick) {
            _prices = new Map(_prices);
            _prices.set(tick.symbol, tick);
            _notify();
          }
        }
      } catch {
        // Ignore malformed messages
      }
    };

    ws.onclose = () => {
      _connected = false;
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
  _connected = false;
}

// ─── Public Hook ───

interface UsePricesOptions {
  symbols?: string[];
}

interface UsePricesResult {
  prices: PriceSnapshot[];
  priceMap: Map<string, PriceSnapshot>;
  connected: boolean;
  loading: boolean;
}

const DEFAULT_SYMBOLS = [
  "XAUUSD", "XAGUSD", "EURUSD", "GBPUSD", "USDJPY",
  "BTCUSDT", "ETHUSDT", "US500", "WTI",
];

export function usePrices({
  symbols = DEFAULT_SYMBOLS,
}: UsePricesOptions = {}): UsePricesResult {
  const [connected, setConnected] = useState(_connected);
  const [priceMap, setPriceMap] = useState<Map<string, PriceSnapshot>>(_prices);
  const [loading, setLoading] = useState(true);
  const [initialFetched, setInitialFetched] = useState(false);

  useEffect(() => {
    // Listener for WS updates
    const listener: TickListener = (conn, prices) => {
      setConnected(conn);
      setPriceMap(prices);
      if (prices.size > 0) setLoading(false);
    };
    _listeners.add(listener);

    // Ref-count connect
    _refCount++;
    if (_refCount === 1) {
      _connect();
    } else {
      setConnected(_connected);
      setPriceMap(_prices);
      if (_prices.size > 0) setLoading(false);
    }

    // Subscribe to requested symbols
    _subscribe(symbols);

    return () => {
      _listeners.delete(listener);
      _refCount--;
      if (_refCount <= 0) {
        _refCount = 0;
        _disconnect();
      }
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [symbols.join(",")]);

  // Fetch initial snapshot via HTTP API to seed data immediately
  // (WS may take a moment to connect, and markets may be closed)
  useEffect(() => {
    if (initialFetched) return;
    setInitialFetched(true);
    const params = new URLSearchParams({ symbols: symbols.join(",") });
    fetch(`/api/prices?${params}`)
      .then((res) => res.ok ? res.json() : null)
      .then((data) => {
        if (!data?.prices) return;
        const snaps: PriceSnapshot[] = data.prices;
        // Only seed symbols that don't already have WS data
        const newMap = new Map(_prices);
        let changed = false;
        for (const p of snaps) {
          const key = p.symbol.replace(/-/g, "");
          if (!newMap.has(key)) {
            newMap.set(key, { ...p, symbol: key });
            changed = true;
          }
        }
        if (changed) {
          _prices = newMap;
          _notify();
        }
        setLoading(false);
      })
      .catch(() => setLoading(false));
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const prices = symbols
    .map((s) => priceMap.get(s))
    .filter((p): p is PriceSnapshot => !!p);

  return { prices, priceMap, connected, loading };
}
