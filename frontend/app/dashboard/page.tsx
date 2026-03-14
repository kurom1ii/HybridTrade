"use client";

import { motion } from "motion/react";
import { useState, useMemo, useCallback, useRef, useEffect } from "react";
import Link from "next/link";
import { StaggerGrid } from "@/components/dashboard/motion-primitives";
import { cn } from "@/lib/utils";
import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  ChartLegend,
  ChartLegendContent,
  type ChartConfig,
} from "@/components/ui/chart";
import { LineChart, Line, XAxis, YAxis, CartesianGrid, LabelList } from "recharts";
import { useNews } from "@/hooks/useNews";
import { usePollingResource } from "@/hooks/use-polling-resource";
import { usePrices } from "@/hooks/usePrices";
import { fetchInstruments } from "@/lib/intelligence-api";
import type { InstrumentView } from "@/lib/intelligence-types";

type Category = "ALL" | "COMMODITIES" | "FOREX" | "INDICES" | "CRYPTO";

interface Instrument {
  symbol: string;
  name: string;
  price: string;
  change: string;
  type: "profit" | "loss";
  category: Category;
  confidence: number;
  direction: "BUY" | "SELL" | "NEUTRAL";
  summary: string;
  entry: string;
  tp: string;
  sl: string;
  session: string;
  timeframe: string;
  keyLevels: string[];
}

function apiToInstrument(iv: InstrumentView): Instrument {
  const dir = iv.direction.toLowerCase();
  const direction: Instrument["direction"] =
    dir === "bullish" ? "BUY" : dir === "bearish" ? "SELL" : "NEUTRAL";
  const type: "profit" | "loss" = iv.change_pct >= 0 ? "profit" : "loss";
  const cat = iv.category.toUpperCase() as Category;
  const category: Category = ["COMMODITIES", "FOREX", "INDICES", "CRYPTO"].includes(cat)
    ? cat
    : "FOREX";
  const keyLevels = Array.isArray(iv.key_levels)
    ? iv.key_levels.map((kl) =>
        typeof kl === "object" && kl !== null
          ? `${kl.label ?? ""} ${kl.price ?? ""}`.trim()
          : String(kl)
      )
    : [];

  return {
    symbol: iv.symbol,
    name: iv.name || iv.symbol,
    price: iv.price.toLocaleString("en-US", { minimumFractionDigits: 2 }),
    change: `${iv.change_pct >= 0 ? "+" : ""}${iv.change_pct.toFixed(2)}%`,
    type,
    category,
    confidence: iv.confidence,
    direction,
    summary: iv.analysis || "Chưa có phân tích từ agent.",
    entry: "-",
    tp: "-",
    sl: "-",
    session: iv.timeframe || "-",
    timeframe: iv.timeframe || "-",
    keyLevels,
  };
}

const fallbackInstruments: Instrument[] = [
  {
    symbol: "XAU/USD", name: "Gold", price: "2,178.40", change: "+0.89%", type: "profit",
    category: "COMMODITIES", confidence: 87, direction: "BUY",
    session: "London / New York", timeframe: "H4",
    keyLevels: ["2,155.00", "2,170.00", "2,195.00", "2,210.00"],
    summary: "Gold dang trong xu huong tang manh, pha ky 2,170 resistance. RSI(14) o vung 62, chua qua mua. DXY suy yeu ho tro Gold tiep tuc rally. Target 2,210 neu giu duoc 2,155.",
    entry: "2,175.00", tp: "2,210.00", sl: "2,155.00",
  },
];

// Money Flow chart
const flowSymbols = ["XAU/USD"] as const;
const flowColors: Record<string, string> = {
  "XAU/USD": "#FFD700",
};

type Timeframe = "1M" | "5M" | "15M" | "1H" | "4H" | "1D";
const timeframes: Timeframe[] = ["1M", "5M", "15M", "1H", "4H", "1D"];

function getPointCount(tf: Timeframe): number {
  switch (tf) {
    case "1M": return 120;
    case "5M": return 96;
    case "15M": return 96;
    case "1H": return 72;
    case "4H": return 60;
    case "1D": return 60;
  }
}

function formatTime(tf: Timeframe, i: number): string {
  switch (tf) {
    case "1M": {
      const mins = i;
      const h = Math.floor(mins / 60);
      const m = mins % 60;
      return `${(9 + h).toString().padStart(2, "0")}:${m.toString().padStart(2, "0")}`;
    }
    case "5M": {
      const mins = i * 5;
      const h = Math.floor(mins / 60);
      const m = mins % 60;
      return `${h.toString().padStart(2, "0")}:${m.toString().padStart(2, "0")}`;
    }
    case "15M": {
      const mins = i * 15;
      const h = Math.floor(mins / 60);
      const m = mins % 60;
      return `${h.toString().padStart(2, "0")}:${m.toString().padStart(2, "0")}`;
    }
    case "1H":
      return `${(i % 24).toString().padStart(2, "0")}:00`;
    case "4H": {
      const day = Math.floor(i / 6) + 1;
      const h = (i % 6) * 4;
      return `D${day} ${h.toString().padStart(2, "0")}h`;
    }
    case "1D": {
      const d = i + 1;
      return `Mar ${d}`;
    }
  }
}

function generateFlowDataForTF(tf: Timeframe) {
  const count = getPointCount(tf);
  const volatilityScale = tf === "1M" ? 2.5 : tf === "5M" ? 2.0 : tf === "15M" ? 1.6 : tf === "1H" ? 1.2 : tf === "4H" ? 1.0 : 0.8;
  return Array.from({ length: count }, (_, i) => {
    const point: Record<string, string | number> = { time: formatTime(tf, i) };
    flowSymbols.forEach((sym, si) => {
      const base = 50 + si * 10;
      const trendRate = 0.3;
      const trend = i * trendRate * (count < 80 ? 1 : 0.5);
      const wave1 = Math.sin(i * 0.12 + si * 1.7) * 8 * volatilityScale;
      const wave2 = Math.cos(i * 0.07 + si * 2.3) * 5 * volatilityScale;
      const noise = Math.sin(i * 0.43 + si * 5.1) * 3 * volatilityScale;
      point[sym] = Math.round((base + trend + wave1 + wave2 + noise) * 100) / 100;
    });
    return point;
  });
}

const flowChartConfig: ChartConfig = Object.fromEntries(
  flowSymbols.map((sym) => [sym, { label: sym, color: flowColors[sym] }])
);

// Watchlist symbols
const watchlistSymbols = [
  { symbol: "XAUUSD", display: "XAU/USD", tag: "METAL" },
  { symbol: "XAGUSD", display: "XAG/USD", tag: "METAL" },
  { symbol: "EURUSD", display: "EUR/USD", tag: "FX" },
  { symbol: "GBPJPY", display: "GBP/JPY", tag: "FX" },
  { symbol: "USNDAQ100", display: "NASDAQ", tag: "INDEX" },
  { symbol: "US30", display: "US30", tag: "INDEX" },
  { symbol: "US500", display: "US500", tag: "INDEX" },
  { symbol: "UK100", display: "UK100", tag: "INDEX" },
  { symbol: "BTCUSDT", display: "BTC/USDT", tag: "CRYPTO" },
  { symbol: "WTI", display: "WTI Oil", tag: "ENERGY" },
  { symbol: "BRENT", display: "Brent Oil", tag: "ENERGY" },
];


const filterTabs: { value: Category; label: string }[] = [
  { value: "ALL", label: "ALL" },
  { value: "COMMODITIES", label: "COMMODITIES" },
  { value: "FOREX", label: "FOREX" },
  { value: "INDICES", label: "INDICES" },
  { value: "CRYPTO", label: "CRYPTO" },
];

function ConfidenceBar({ value, size = "sm", direction = "NEUTRAL" }: { value: number; size?: "sm" | "lg"; direction?: "BUY" | "SELL" | "NEUTRAL" }) {
  const h = size === "lg" ? "h-2" : "h-1.5";
  const barColor = direction === "BUY" ? "var(--profit)" : direction === "SELL" ? "var(--loss)" : "var(--warning)";
  const textClass = direction === "BUY" ? "text-profit" : direction === "SELL" ? "text-loss" : "text-warning";
  return (
    <div className="flex items-center gap-2">
      <div className={`${h} flex-1 bg-secondary overflow-hidden`}>
        <motion.div
          className="h-full"
          initial={{ width: 0 }}
          animate={{ width: `${value}%` }}
          transition={{ duration: 0.3, ease: "easeOut" }}
          style={{ backgroundColor: barColor }}
        />
      </div>
      <span
        className={cn(
          "font-bold tabular-nums",
          size === "lg" ? "text-[14px]" : "text-[11px]",
          textClass
        )}
      >
        {value}%
      </span>
    </div>
  );
}

function MoneyFlowChart() {
  const [tf, setTf] = useState<Timeframe>("1H");
  const [brushRange, setBrushRange] = useState<{ startIndex: number; endIndex: number } | null>(null);
  const chartWrapperRef = useRef<HTMLDivElement>(null);

  const data = useMemo(() => generateFlowDataForTF(tf), [tf]);

  // Attach non-passive wheel listener to prevent page scroll on chart
  useEffect(() => {
    const chartEl = chartWrapperRef.current;
    const prevent = (e: WheelEvent) => e.preventDefault();
    const opts: AddEventListenerOptions = { passive: false };
    chartEl?.addEventListener("wheel", prevent, opts);
    return () => {
      chartEl?.removeEventListener("wheel", prevent);
    };
  }, []);

  // Reset brush when timeframe changes
  const handleTfChange = useCallback((newTf: Timeframe) => {
    setTf(newTf);
    setBrushRange(null);
  }, []);

  // Y domain — auto-fit to visible data
  const yDomain = useMemo(() => {
    const visible = brushRange
      ? data.slice(brushRange.startIndex, brushRange.endIndex + 1)
      : data;
    let min = Infinity, max = -Infinity;
    visible.forEach((pt) => {
      flowSymbols.forEach((sym) => {
        const v = pt[sym] as number;
        if (v < min) min = v;
        if (v > max) max = v;
      });
    });
    const padding = (max - min) * 0.08;
    return [Math.floor(min - padding - 1), Math.ceil(max + padding + 1)];
  }, [data, brushRange]);

  // Mouse wheel zoom on chart — horizontal zoom via brush
  const handleWheel = useCallback((e: React.WheelEvent) => {
    e.preventDefault();
    const total = data.length;
    const current = brushRange ?? { startIndex: 0, endIndex: total - 1 };
    const span = current.endIndex - current.startIndex;
    const center = Math.floor((current.startIndex + current.endIndex) / 2);

    const zoomFactor = e.deltaY > 0 ? 1.15 : 0.85;
    const newSpan = Math.max(8, Math.min(total - 1, Math.round(span * zoomFactor)));
    const half = Math.floor(newSpan / 2);
    const newStart = Math.max(0, center - half);
    const newEnd = Math.min(total - 1, newStart + newSpan);

    setBrushRange({ startIndex: newStart, endIndex: newEnd });
  }, [data.length, brushRange]);

  // Visible range info
  const visibleStart = brushRange?.startIndex ?? 0;
  const visibleEndIdx = brushRange?.endIndex ?? data.length - 1;
  const visibleSpan = visibleEndIdx - visibleStart + 1;
  const visibleData = brushRange ? data.slice(visibleStart, visibleEndIdx + 1) : data;
  const visibleEnd = visibleData.length - 1;
  const tickInterval = visibleSpan <= 20 ? 1 : visibleSpan <= 40 ? 2 : visibleSpan <= 60 ? 4 : Math.floor(visibleSpan / 12);

  return (
    <motion.div
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.25, delay: 0.15 }}
      className="border border-border bg-card"
    >
      {/* Header */}
      <div className="p-5 pb-0">
        <div className="flex items-center justify-between flex-wrap gap-4">
          <div>
            <div className="flex items-center gap-2 mb-1">
              <span className="h-1.5 w-1.5 rounded-full bg-cyan live-dot" />
              <span className="text-[9px] font-bold tracking-[2px] text-cyan/70">LIVE</span>
            </div>
            <h3 className="text-[18px] font-bold tracking-[0.3px]">Money Flow</h3>
            <p className="text-[11px] text-text-secondary mt-1">Capital flow across major instruments</p>
          </div>

          {/* Timeframe selector */}
          <div className="flex items-center gap-1">
            {timeframes.map((t) => (
              <button
                key={t}
                onClick={() => handleTfChange(t)}
                className={cn(
                  "px-3 py-1.5 text-[11px] font-bold tracking-[0.5px] transition-colors",
                  tf === t
                    ? "bg-cyan/15 text-cyan border border-cyan/30"
                    : "text-text-muted border border-transparent hover:text-foreground hover:border-border"
                )}
              >
                {t}
              </button>
            ))}
          </div>
        </div>

        {/* Legend */}
        <div className="flex gap-4 mt-4 flex-wrap">
          {flowSymbols.map((sym) => (
            <div key={sym} className="flex items-center gap-1.5">
              <div className="h-[3px] w-4" style={{ backgroundColor: flowColors[sym] }} />
              <span className="text-[10px] font-semibold text-text-secondary">{sym}</span>
            </div>
          ))}
        </div>
      </div>

      {/* Chart */}
      <div
        ref={chartWrapperRef}
        className="px-2 pt-3 pb-2"
        onWheel={handleWheel}
      >
        <ChartContainer config={flowChartConfig} className="h-[460px] w-full aspect-auto">
          <LineChart data={visibleData} margin={{ top: 12, right: 90, left: 8, bottom: 24 }}>
            <CartesianGrid
              strokeDasharray="3 3"
              stroke="var(--border)"
              vertical={false}
            />
            <XAxis
              dataKey="time"
              tickLine={false}
              axisLine={{ stroke: "var(--border)" }}
              tick={{ fill: "var(--text-muted)", fontSize: 10, fontWeight: 600 }}
              interval={tickInterval}
              dy={10}
            />
            <YAxis
              tickLine={false}
              axisLine={false}
              tick={false}
              width={8}
              domain={yDomain}
            />
            <ChartTooltip
              content={
                <ChartTooltipContent
                  className="border-border bg-card"
                  labelClassName="text-foreground font-bold"
                />
              }
            />
            {flowSymbols.map((sym) => (
              <Line
                key={sym}
                type="monotone"
                dataKey={sym}
                stroke={flowColors[sym]}
                strokeWidth={2}
                dot={false}
                activeDot={{ r: 4, strokeWidth: 0, fill: flowColors[sym] }}
                isAnimationActive={false}
              >
                <LabelList
                  dataKey={sym}
                  position="right"
                  content={({ x, y, index, value }: any) => {
                    if (index !== visibleEnd) return null;
                    return (
                      <g>
                        <text
                          x={(x ?? 0) + 8}
                          y={y}
                          fill={flowColors[sym]}
                          fontSize={10}
                          fontWeight={700}
                          dominantBaseline="middle"
                        >
                          {sym}
                        </text>
                        <text
                          x={(x ?? 0) + 8}
                          y={(y ?? 0) + 12}
                          fill={flowColors[sym]}
                          fontSize={8}
                          fontWeight={500}
                          opacity={0.6}
                          dominantBaseline="middle"
                        >
                          {typeof value === "number" ? value.toFixed(1) : value}
                        </text>
                      </g>
                    );
                  }}
                />
              </Line>
            ))}
          </LineChart>
        </ChartContainer>
      </div>

      {/* Footer */}
      <div className="px-5 pb-3 flex items-center justify-between border-t border-border pt-2">
        <span className="text-[10px] text-text-faint">Scroll to zoom in/out</span>
        <span className="text-[10px] text-text-muted tabular-nums">{visibleSpan} / {data.length} candles</span>
      </div>
    </motion.div>
  );
}

function formatNewsTime(ms: number): string {
  const diff = Date.now() - ms;
  const mins = Math.floor(diff / 60_000);
  if (mins < 1) return "vừa xong";
  if (mins < 60) return `${mins}m`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h`;
  return `${Math.floor(hours / 24)}d`;
}

function formatExactTime(ms: number): string {
  const d = new Date(ms);
  return d.toLocaleTimeString("vi-VN", { hour12: false, hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

function formatFullDate(ms: number): string {
  const d = new Date(ms);
  return d.toLocaleDateString("vi-VN", { day: "2-digit", month: "2-digit", year: "numeric" });
}

function RightPanel() {
  const { items: liveNews, loading, error, lastUpdated, connected, loadMore, loadingMore, hasMore } = useNews({ pageSize: 15 });
  const { prices: livePrices } = usePrices({
    symbols: watchlistSymbols.map((w) => w.symbol),
  });

  const watchlist = watchlistSymbols.map((w) => {
    const p = livePrices.find((x) => x.symbol === w.symbol);
    const pctStr = p
      ? `${p.changePct >= 0 ? "+" : ""}${(p.changePct * 100).toFixed(2)}%`
      : "—";
    const priceStr = p
      ? p.price.toLocaleString("en-US", {
          minimumFractionDigits: Math.min(p.precision, 2),
          maximumFractionDigits: p.precision,
        })
      : "—";
    return {
      symbol: w.display,
      price: priceStr,
      change: pctStr,
      type: (p ? (p.changePct >= 0 ? "profit" : "loss") : "profit") as "profit" | "loss",
      tag: w.tag,
    };
  });

  const sentinelRef = useRef<HTMLDivElement>(null);
  const observerRef = useRef<IntersectionObserver | null>(null);

  useEffect(() => {
    if (observerRef.current) observerRef.current.disconnect();
    observerRef.current = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting && hasMore && !loadingMore) {
          loadMore();
        }
      },
      { threshold: 0.1 }
    );
    if (sentinelRef.current) {
      observerRef.current.observe(sentinelRef.current);
    }
    return () => {
      if (observerRef.current) observerRef.current.disconnect();
    };
  }, [hasMore, loadingMore, loadMore]);

  return (
    <div
      className="w-[320px] shrink-0 border-l border-border bg-panel overflow-y-auto hidden xl:flex flex-col"
    >
      {/* Header */}
      <div className="flex items-center justify-between px-5 py-4">
        <span className="text-[11px] font-bold tracking-[1px] text-cyan">MARKET FEED</span>
        <span className="h-1.5 w-1.5 rounded-full bg-cyan live-dot" />
      </div>

      {/* Watchlist */}
      <div className="px-3 pb-3">
        <div className="text-[10px] font-bold tracking-[1.5px] text-cyan mb-3 px-2">WATCHLIST</div>
        <div className="space-y-1">
          {watchlist.map((w) => (
            <div
              key={w.symbol}
              className="flex items-center justify-between bg-card-alt px-4 py-3 transition-colors hover:bg-card"
            >
              <div className="flex items-center gap-3 min-w-0">
                <span className="text-[12px] font-bold">{w.symbol}</span>
                <span
                  className={cn(
                    "text-[8px] font-bold tracking-[0.5px]",
                    w.tag === "FX" ? "text-cyan/60" : w.tag === "CRYPTO" ? "text-[#9b59b6]/60" : "text-warning/60"
                  )}
                >
                  {w.tag}
                </span>
              </div>
              <div className="flex items-center gap-3">
                <div className="flex items-end gap-[2px] h-4">
                  {Array.from({ length: 7 }, (_, i) => {
                    const h = 4 + Math.abs(Math.sin(w.symbol.length * 2.3 + i * 0.8)) * 12;
                    const isProfit = w.type === "profit";
                    return (
                      <div
                        key={i}
                        className="w-[3px]"
                        style={{
                          height: `${h}px`,
                          backgroundColor: isProfit ? "var(--cyan)" : "var(--loss)",
                          opacity: 0.3 + (i / 7) * 0.7,
                        }}
                      />
                    );
                  })}
                </div>
                <span className="text-[11px] font-medium tabular-nums text-text-dim">{w.price}</span>
                <span
                  className={cn(
                    "text-[10px] font-bold tabular-nums px-1.5 py-0.5",
                    w.type === "profit"
                      ? "text-cyan bg-cyan/10"
                      : "text-loss bg-loss/10"
                  )}
                >
                  {w.type === "profit" ? "▲" : "▼"} {w.change}
                </span>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Divider */}
      <div className="h-px bg-gradient-to-r from-transparent via-border to-transparent mx-3" />

      {/* Live News Feed */}
      <div className="px-3 py-3 flex-1">
        <div className="flex items-center justify-between mb-3 px-2">
          <div className="flex items-center gap-2">
            <div className="text-[10px] font-bold tracking-[1.5px] text-cyan">// LATEST NEWS</div>
            {connected && (
              <span className="flex items-center gap-1">
                <span className="h-1.5 w-1.5 rounded-full bg-profit live-dot" />
                <span className="text-[8px] font-bold text-profit">LIVE</span>
              </span>
            )}
          </div>
          {lastUpdated && (
            <span className="text-[8px] text-text-faint">
              {lastUpdated.toLocaleTimeString()}
            </span>
          )}
        </div>

        {loading && liveNews.length === 0 && (
          <div className="px-2 py-8 text-center">
            <div className="text-[11px] text-text-muted animate-pulse">Loading news...</div>
          </div>
        )}

        {error && liveNews.length === 0 && (
          <div className="px-2 py-4 text-center">
            <div className="text-[11px] text-loss">{error}</div>
          </div>
        )}

        <div className="space-y-2">
          {liveNews.map((news) => (
            <a
              key={news.id}
              href={`https://www.fastbull.com${news.path}`}
              target="_blank"
              rel="noopener noreferrer"
              className="flex bg-card-alt overflow-hidden cursor-pointer group transition-colors hover:bg-card"
            >
              {/* Left accent bar — red for important, cyan for normal */}
              <div className={cn(
                "w-[2px] shrink-0",
                news.important ? "bg-loss" : "bg-cyan"
              )} />
              <div className="p-3 space-y-1.5 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="text-[10px] text-text-faint">
                    <span className="text-text-muted">{formatExactTime(news.releasedDateMs)}</span>
                    {" · "}
                    {formatFullDate(news.releasedDateMs)}
                    {" · "}
                    {formatNewsTime(news.releasedDateMs)}
                  </span>
                  {news.important && (
                    <span className="text-[9px] font-bold tracking-[0.5px] px-1.5 py-0.5 bg-loss/15 text-loss">
                      IMPORTANT
                    </span>
                  )}
                </div>
                <p className={cn(
                  "text-[13px] font-semibold leading-[1.6] group-hover:text-foreground transition-colors",
                  news.important
                    ? "text-loss"
                    : "text-foreground/85"
                )}>
                  {news.title}
                </p>
              </div>
            </a>
          ))}
          {/* Infinite scroll sentinel */}
          <div ref={sentinelRef} className="py-2 text-center">
            {loadingMore && (
              <div className="text-[10px] text-text-muted animate-pulse">Đang tải thêm...</div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

export default function DashboardPage() {
  const [activeTab, setActiveTab] = useState<Category>("ALL");

  const { data: apiInstruments } = usePollingResource(
    "instruments",
    fetchInstruments,
    { intervalMs: 0 },
  );

  const DASHBOARD_SYMBOLS = [
    "XAUUSD", "XAGUSD",
    "EURUSD", "GBPJPY",
    "USNDAQ100", "US30", "US500", "UK100",
    "BTCUSDT",
    "WTI", "BRENT",
  ];

  const SYMBOL_NAMES: Record<string, [string, Category, string]> = {
    XAUUSD: ["Gold / US Dollar", "COMMODITIES", "XAU/USD"],
    XAGUSD: ["Silver / US Dollar", "COMMODITIES", "XAG/USD"],
    EURUSD: ["Euro / US Dollar", "FOREX", "EUR/USD"],
    GBPJPY: ["Pound Sterling / Japanese Yen", "FOREX", "GBP/JPY"],
    USNDAQ100: ["US 100 Tech Index", "INDICES", "NASDAQ"],
    US30: ["Dow Jones Industrial", "INDICES", "US30"],
    US500: ["S&P 500 Index", "INDICES", "US500"],
    UK100: ["UK FTSE 100", "INDICES", "UK100"],
    BTCUSDT: ["Bitcoin / Tether", "CRYPTO", "BTC/USDT"],
    WTI: ["WTI Crude Oil", "COMMODITIES", "WTI"],
    BRENT: ["Brent Crude Oil", "COMMODITIES", "BRENT"],
  };

  const { prices: livePrices } = usePrices({
    symbols: DASHBOARD_SYMBOLS,
  });

  const instruments: Instrument[] = useMemo(() => {
    // Build a map of backend instruments keyed by normalized symbol
    const apiMap = new Map<string, InstrumentView>();
    if (apiInstruments) {
      for (const iv of apiInstruments) {
        apiMap.set(iv.symbol.replace("/", "").replace("-", "").toUpperCase(), iv);
      }
    }

    // Build a map of live prices keyed by symbol
    const priceMap = new Map(livePrices.map((p) => [p.symbol, p]));

    // Merge: for each dashboard symbol, use backend analysis if available, overlay live price
    return DASHBOARD_SYMBOLS.map((sym) => {
      const apiData = apiMap.get(sym);
      const livePrice = priceMap.get(sym);
      const [defaultName, defaultCategory, displaySymbol] = SYMBOL_NAMES[sym] || [sym, "FOREX" as Category, sym];

      // Start with backend data or defaults
      let inst: Instrument;
      if (apiData) {
        inst = apiToInstrument(apiData);
      } else {
        inst = {
          symbol: displaySymbol,
          name: defaultName,
          price: "—",
          change: "—",
          type: "profit",
          category: defaultCategory,
          confidence: 0,
          direction: "NEUTRAL",
          summary: "Đang chờ phân tích từ agent AI.",
          entry: "-", tp: "-", sl: "-",
          session: "-", timeframe: "-",
          keyLevels: [],
        };
      }

      // Overlay live price if available
      if (livePrice) {
        inst.price = livePrice.price.toLocaleString("en-US", {
          minimumFractionDigits: Math.min(livePrice.precision, 2),
          maximumFractionDigits: livePrice.precision,
        });
        inst.change = `${livePrice.changePct >= 0 ? "+" : ""}${(livePrice.changePct * 100).toFixed(2)}%`;
        inst.type = livePrice.changePct >= 0 ? "profit" : "loss";
      }

      return inst;
    });
  }, [apiInstruments, livePrices]);

  const filtered = activeTab === "ALL"
    ? instruments
    : instruments.filter((inst) => inst.category === activeTab);

  return (
    <div className="flex h-full">
      {/* Main Content */}
      <div className="flex-1 overflow-y-auto p-6 space-y-6 min-w-0">
      {/* Header */}
      <motion.div
        initial={{ opacity: 0, y: 6 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.15 }}
      >
        <h1 className="text-[22px] font-bold tracking-[0.5px]">Trading Dashboard</h1>
        <p className="mt-1 text-[12px] text-text-secondary">AI-powered analysis across commodities, forex, indices, and crypto.</p>
      </motion.div>

      {/* Filter Tabs */}
      <div className="flex gap-1">
        {filterTabs.map((tab) => (
          <button
            key={tab.value}
            onClick={() => setActiveTab(tab.value)}
            className={cn(
              "px-5 py-2 text-[11px] font-semibold tracking-[0.5px] transition-colors",
              activeTab === tab.value
                ? "bg-cyan-dim text-cyan border border-cyan/20"
                : "text-text-secondary border border-transparent hover:text-foreground hover:border-border"
            )}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Instrument Cards Grid */}
      <StaggerGrid className="grid gap-5 md:grid-cols-2 2xl:grid-cols-3">
        {filtered.map((inst) => (
          <motion.div
            key={inst.symbol}
            variants={{
              hidden: { opacity: 0, y: 10 },
              visible: { opacity: 1, y: 0, transition: { duration: 0.2, ease: "easeOut" } },
            }}
            whileHover={{ y: -4, scale: 1.015, transition: { duration: 0.2, ease: "easeOut" } }}
            className="group bg-card overflow-hidden cursor-pointer border"
            style={{
              borderColor: inst.direction === "BUY"
                ? "var(--profit)"
                : inst.direction === "SELL"
                  ? "var(--loss)"
                  : "var(--warning)",
            }}
          >
            <div className="p-6 space-y-5">
              {/* Row 1: Symbol + Price */}
              <div className="flex items-start justify-between">
                <div>
                  <div className="flex items-center gap-2.5">
                    <h3 className="text-[24px] font-bold tracking-[0.3px]">{inst.symbol}</h3>
                    <span
                      className={cn(
                        "px-2.5 py-1 text-[10px] font-bold tracking-wider",
                        inst.direction === "BUY" && "bg-profit/12 text-profit border border-profit/20",
                        inst.direction === "SELL" && "bg-loss/12 text-loss border border-loss/20",
                        inst.direction === "NEUTRAL" && "bg-card-alt text-text-muted border border-border",
                      )}
                    >
                      {inst.direction}
                    </span>
                  </div>
                  <div className="mt-1 flex items-center gap-2 text-[12px] text-text-muted">
                    <span>{inst.name}</span>
                    <span className="text-text-faint">|</span>
                    <span>{inst.category}</span>
                    <span className="text-text-faint">|</span>
                    <span>{inst.timeframe}</span>
                  </div>
                </div>
                <div className="text-right">
                  <div className="text-[26px] font-bold tabular-nums">{inst.price}</div>
                  <div
                    className={cn(
                      "text-[14px] font-semibold tabular-nums",
                      inst.type === "profit" ? "text-profit" : "text-loss"
                    )}
                  >
                    {inst.change}
                  </div>
                </div>
              </div>

              {/* Row 2: Confidence Bar */}
              <div>
                <div className="flex items-center justify-between mb-1.5">
                  <span className="text-[10px] font-bold uppercase tracking-[1px] text-text-muted">AI CONFIDENCE</span>
                  <span className="text-[10px] text-text-muted">{inst.session}</span>
                </div>
                <ConfidenceBar value={inst.confidence} size="lg" direction={inst.direction} />
              </div>

              {/* Row 3: AI Summary */}
              <div className="bg-card-alt border border-border p-4">
                <div className="text-[9px] font-bold uppercase tracking-[1.5px] text-text-muted mb-2">AI ANALYSIS</div>
                <p className="text-[13px] font-bold leading-[1.8] text-foreground">
                  {inst.summary}
                </p>
              </div>

              {/* Row 4: Key Levels */}
              <div>
                <div className="text-[9px] font-bold uppercase tracking-[1.5px] text-text-muted mb-2">KEY LEVELS</div>
                <div className="flex flex-wrap gap-2">
                  {inst.keyLevels.map((level) => (
                    <span key={level} className="border border-border bg-card-alt px-3 py-1.5 text-[13px] font-semibold tabular-nums">
                      {level}
                    </span>
                  ))}
                </div>
              </div>

              {/* Row 5: Deep Analysis Button */}
              <Link
                href={`/dashboard/analytics/${inst.symbol.replace("/", "-").toLowerCase()}`}
                className="flex items-center justify-center gap-2 border border-cyan/20 bg-cyan/5 py-3 text-[12px] font-bold tracking-[1px] text-cyan transition-all hover:bg-cyan/10 hover:border-cyan/40"
              >
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M3 3v18h18" /><path d="M7 16l4-8 4 4 6-10" />
                </svg>
                DEEP ANALYSIS
                <span className="text-[10px] text-cyan/50">&rarr;</span>
              </Link>

            </div>
          </motion.div>
        ))}
      </StaggerGrid>

      {/* Aggregated Money Flow Chart — TradingView Style */}
      <MoneyFlowChart />
      </div>

      {/* Right Panel — Watchlist + News */}
      <RightPanel />
    </div>
  );
}
