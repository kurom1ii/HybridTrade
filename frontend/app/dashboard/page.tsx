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

const instruments: Instrument[] = [
  {
    symbol: "XAU/USD", name: "Gold", price: "2,178.40", change: "+0.89%", type: "profit",
    category: "COMMODITIES", confidence: 87, direction: "BUY",
    session: "London / New York", timeframe: "H4",
    keyLevels: ["2,155.00", "2,170.00", "2,195.00", "2,210.00"],
    summary: "Gold dang trong xu huong tang manh, pha ky 2,170 resistance. RSI(14) o vung 62, chua qua mua. DXY suy yeu ho tro Gold tiep tuc rally. Target 2,210 neu giu duoc 2,155.",
    entry: "2,175.00", tp: "2,210.00", sl: "2,155.00",
  },
  {
    symbol: "XAG/USD", name: "Silver", price: "24.82", change: "+1.24%", type: "profit",
    category: "COMMODITIES", confidence: 74, direction: "BUY",
    session: "London / New York", timeframe: "H4",
    keyLevels: ["24.20", "24.50", "25.00", "25.40"],
    summary: "Silver breakout khoi channel 23.80-24.50, momentum tang. Gold/Silver ratio dang giam, bao hieu silver outperform. Ho tro tai 24.20, resistance tiep theo 25.40.",
    entry: "24.70", tp: "25.40", sl: "24.20",
  },
  {
    symbol: "WTI/USD", name: "Crude Oil", price: "78.45", change: "-0.67%", type: "loss",
    category: "COMMODITIES", confidence: 68, direction: "SELL",
    session: "New York", timeframe: "H1",
    keyLevels: ["76.80", "78.00", "79.60", "80.50"],
    summary: "Oil giam do lo ngai demand yeu tu Trung Quoc. OPEC+ van giu cat giam san luong nhung market khong phan ung. Support quan trong tai 76.80, pha vo se den 74.50.",
    entry: "78.80", tp: "76.80", sl: "79.60",
  },
  {
    symbol: "BRENT", name: "Brent Oil", price: "82.10", change: "-0.42%", type: "loss",
    category: "COMMODITIES", confidence: 64, direction: "SELL",
    session: "London", timeframe: "H4",
    keyLevels: ["80.50", "82.00", "83.20", "84.80"],
    summary: "Brent test duong trend giam tu thang 9. Spread voi WTI thu hep, cho thay demand chau Au yeu. Nen thoi candle 4H cho thay selling pressure tang.",
    entry: "82.40", tp: "80.50", sl: "83.20",
  },
  {
    symbol: "NGAS", name: "Natural Gas", price: "2.847", change: "+2.18%", type: "profit",
    category: "COMMODITIES", confidence: 71, direction: "BUY",
    session: "New York", timeframe: "D1",
    keyLevels: ["2.70", "2.85", "3.00", "3.15"],
    summary: "Natural Gas tang manh do du bao thoi tiet lanh bat thuong. Inventory thap hon trung binh 5 nam. Resistance tai 3.00 la muc tam ly quan trong.",
    entry: "2.82", tp: "3.05", sl: "2.70",
  },
  {
    symbol: "US30", name: "Dow Jones", price: "39,142", change: "+0.34%", type: "profit",
    category: "INDICES", confidence: 72, direction: "SELL",
    session: "New York", timeframe: "H4",
    keyLevels: ["38,700", "39,000", "39,200", "39,500"],
    summary: "US30 dat all-time high nhung RSI divergence am. Ky vong Fed giu lai suat cao hon lau hon. Nen thoi candle ngay cho thay buyer kiet suc. Short neu mat 39,000.",
    entry: "39,200", tp: "38,700", sl: "39,500",
  },
  {
    symbol: "NAS100", name: "Nasdaq", price: "17,892", change: "+0.52%", type: "profit",
    category: "INDICES", confidence: 66, direction: "NEUTRAL",
    session: "New York", timeframe: "H4",
    keyLevels: ["17,600", "17,800", "18,000", "18,200"],
    summary: "Nasdaq sideway trong range 17,600-18,000. Tech earnings season dang den, volatility se tang. Cho breakout ro truoc khi vao lenh. Canh gioi NVDA va AAPL report.",
    entry: "--", tp: "--", sl: "--",
  },
  {
    symbol: "EUR/USD", name: "Euro", price: "1.0847", change: "+0.24%", type: "profit",
    category: "FOREX", confidence: 81, direction: "BUY",
    session: "London / New York", timeframe: "H1",
    keyLevels: ["1.0780", "1.0830", "1.0860", "1.0900"],
    summary: "EUR/USD rebound tu support 1.0780. ECB giu lai suat on dinh trong khi Fed co tin hieu dovish. Target 1.0900 neu breakout 1.0860. RSI dang phuc hoi tu vung oversold.",
    entry: "1.0830", tp: "1.0900", sl: "1.0780",
  },
  {
    symbol: "GBP/USD", name: "Pound", price: "1.2634", change: "+0.18%", type: "profit",
    category: "FOREX", confidence: 69, direction: "BUY",
    session: "London", timeframe: "H4",
    keyLevels: ["1.2560", "1.2620", "1.2650", "1.2750"],
    summary: "Cable test resistance 1.2650. BoE hawkish hon ky vong, inflation van cao. Neu pha duoc 1.2650 se den 1.2750. Risk: DXY rebound se day GBP xuong.",
    entry: "1.2620", tp: "1.2750", sl: "1.2560",
  },
  {
    symbol: "USD/JPY", name: "Yen", price: "149.82", change: "-0.31%", type: "loss",
    category: "FOREX", confidence: 76, direction: "SELL",
    session: "Tokyo / London", timeframe: "H1",
    keyLevels: ["148.50", "149.00", "150.00", "150.50"],
    summary: "USD/JPY gan muc 150 — nguong can thiep cua BOJ. Yield gap My-Nhat thu hep. Risk/reward tot cho short tai 150. BOJ co the thay doi chinh sach bat ngo.",
    entry: "149.90", tp: "148.50", sl: "150.50",
  },
  {
    symbol: "BTC/USD", name: "Bitcoin", price: "67,842", change: "+2.14%", type: "profit",
    category: "CRYPTO", confidence: 78, direction: "BUY",
    session: "24/7", timeframe: "D1",
    keyLevels: ["65,000", "67,000", "70,000", "72,000"],
    summary: "BTC tang manh do ETF inflows ky luc. Halving sap den trong Q2. Support manh tai 65,000. Target 72,000 neu volume duy tri. On-chain data cho thay accumulation tu whale.",
    entry: "67,500", tp: "72,000", sl: "65,000",
  },
  {
    symbol: "ETH/USD", name: "Ethereum", price: "3,456", change: "-1.07%", type: "loss",
    category: "CRYPTO", confidence: 62, direction: "NEUTRAL",
    session: "24/7", timeframe: "D1",
    keyLevels: ["3,300", "3,400", "3,500", "3,600"],
    summary: "ETH underperform BTC, ETH/BTC ratio giam. ETF Ethereum chua duoc duyet, sentiment trung tinh. Support 3,300, resistance 3,600. Cho tin ETF truoc khi vao lenh lon.",
    entry: "--", tp: "--", sl: "--",
  },
];

// Money Flow chart
const flowSymbols = ["XAU/USD", "WTI/USD", "EUR/USD", "US30", "BTC/USD", "XAG/USD"] as const;
const flowColors: Record<string, string> = {
  "XAU/USD": "#FFD700",
  "WTI/USD": "#ff4444",
  "EUR/USD": "#2196F3",
  "US30": "#f39c12",
  "BTC/USD": "#22c55e",
  "XAG/USD": "#C0C0C0",
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
      const trendRate = sym === "BTC/USD" ? 0.4 : sym === "WTI/USD" ? -0.2 : sym === "XAU/USD" ? 0.3 : sym === "EUR/USD" ? 0.15 : sym === "US30" ? -0.1 : 0.25;
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

// Watchlist data (from design)
const watchlist = [
  { symbol: "EUR/USD", price: "1.0842", change: "+0.32%", type: "profit" as const, tag: "FX" },
  { symbol: "GBP/USD", price: "1.2651", change: "-0.15%", type: "loss" as const, tag: "FX" },
  { symbol: "USD/JPY", price: "149.82", change: "+0.58%", type: "profit" as const, tag: "FX" },
  { symbol: "BTC/USD", price: "67,432.50", change: "+2.41%", type: "profit" as const, tag: "CRYPTO" },
  { symbol: "ETH/USD", price: "3,521.80", change: "+1.87%", type: "profit" as const, tag: "CRYPTO" },
  { symbol: "XAU/USD", price: "2,178.40", change: "-0.22%", type: "loss" as const, tag: "METAL" },
];

// News data (from design)
const newsItems = [
  {
    time: "14:32 UTC",
    title: "Fed signals potential rate pause amid inflation cooldown",
    source: "REUTERS",
    sentiment: "BULLISH" as const,
    impact: "HIGH IMPACT",
    category: "MACRO",
  },
  {
    time: "13:15 UTC",
    title: "Bitcoin ETF inflows surge past $2B weekly record",
    source: "BLOOMBERG",
    sentiment: "BULLISH" as const,
    impact: "HIGH IMPACT",
    category: "CRYPTO",
  },
  {
    time: "11:48 UTC",
    title: "ECB maintains hawkish stance on euro zone recovery",
    source: "FT",
    sentiment: "BEARISH" as const,
    impact: "MED IMPACT",
    category: "MACRO",
  },
  {
    time: "09:22 UTC",
    title: "Gold retreats as dollar strengthens on jobs data",
    source: "CNBC",
    sentiment: "BEARISH" as const,
    impact: "LOW IMPACT",
    category: "COMMODITY",
  },
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

function RightPanel() {
  return (
    <motion.div
      initial={{ opacity: 0, x: 12 }}
      animate={{ opacity: 1, x: 0 }}
      transition={{ duration: 0.2, delay: 0.1 }}
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
                {/* Mini sparkline bars */}
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

      {/* News Feed */}
      <div className="px-3 py-3 flex-1">
        <div className="text-[10px] font-bold tracking-[1.5px] text-cyan mb-3 px-2">// LATEST NEWS</div>
        <div className="space-y-2">
          {newsItems.map((news, i) => (
            <div key={i} className="flex bg-card-alt overflow-hidden">
              {/* Left accent bar */}
              <div className="w-[2px] shrink-0 bg-cyan" />
              <div className="p-3 space-y-2 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="text-[9px] text-text-faint">{news.time} · TODAY</span>
                </div>
                <p className="text-[11px] font-semibold leading-[1.5] text-foreground/85">{news.title}</p>
                <span className="text-[10px] font-bold tracking-[0.5px] text-cyan">{news.source}</span>
                <div className="flex gap-2 flex-wrap">
                  <span
                    className={cn(
                      "text-[8px] font-bold tracking-[0.5px] px-2 py-0.5",
                      news.sentiment === "BULLISH"
                        ? "bg-cyan/10 text-cyan"
                        : "bg-loss/10 text-loss"
                    )}
                  >
                    {news.sentiment}
                  </span>
                  <span
                    className={cn(
                      "text-[8px] font-bold tracking-[0.5px] px-2 py-0.5",
                      news.impact === "HIGH IMPACT"
                        ? "bg-warning/10 text-warning"
                        : news.impact === "MED IMPACT"
                          ? "bg-warning/10 text-warning/70"
                          : "bg-card text-text-muted"
                    )}
                  >
                    {news.impact}
                  </span>
                  <span
                    className={cn(
                      "text-[8px] font-bold tracking-[0.5px] px-2 py-0.5",
                      news.category === "CRYPTO"
                        ? "bg-[#9b59b6]/10 text-[#9b59b6]"
                        : news.category === "COMMODITY"
                          ? "bg-warning/10 text-warning"
                          : "bg-card text-text-muted"
                    )}
                  >
                    {news.category}
                  </span>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </motion.div>
  );
}

export default function DashboardPage() {
  const [activeTab, setActiveTab] = useState<Category>("ALL");

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
            className="group bg-card rounded-lg overflow-hidden cursor-pointer border"
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
                    {inst.type === "profit" ? "+" : ""}{inst.change}
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
