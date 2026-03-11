"use client";

import { motion } from "motion/react";
import { useParams } from "next/navigation";
import Link from "next/link";
import { cn } from "@/lib/utils";

interface Instrument {
  symbol: string;
  name: string;
  price: string;
  change: string;
  type: "profit" | "loss";
  category: string;
  confidence: number;
  direction: "BUY" | "SELL" | "NEUTRAL";
  summary: string;
  entry: string;
  tp: string;
  sl: string;
  session: string;
  timeframe: string;
  keyLevels: string[];
  technicals: { name: string; value: string; signal: "bullish" | "bearish" | "neutral" }[];
  deepAnalysis: string[];
  riskReward: string;
  winRate: string;
  avgHoldTime: string;
}

const instrumentsMap: Record<string, Instrument> = {
  "xau-usd": {
    symbol: "XAU/USD", name: "Gold", price: "2,178.40", change: "+0.89%", type: "profit",
    category: "COMMODITIES", confidence: 87, direction: "BUY",
    session: "London / New York", timeframe: "H4",
    keyLevels: ["2,155.00", "2,170.00", "2,195.00", "2,210.00"],
    entry: "2,175.00", tp: "2,210.00", sl: "2,155.00",
    riskReward: "1:1.75", winRate: "72%", avgHoldTime: "6h 30m",
    technicals: [
      { name: "RSI (14)", value: "62.4", signal: "bullish" },
      { name: "MACD", value: "Bullish Cross", signal: "bullish" },
      { name: "EMA 50/200", value: "Golden Cross", signal: "bullish" },
      { name: "Bollinger", value: "Upper Band", signal: "neutral" },
      { name: "Stochastic", value: "78.2", signal: "neutral" },
      { name: "ATR (14)", value: "18.5", signal: "neutral" },
      { name: "Volume", value: "Above Avg", signal: "bullish" },
      { name: "ADX", value: "32.1 (Trend)", signal: "bullish" },
    ],
    summary: "Gold dang trong xu huong tang manh, pha ky 2,170 resistance. RSI(14) o vung 62, chua qua mua. DXY suy yeu ho tro Gold tiep tuc rally. Target 2,210 neu giu duoc 2,155.",
    deepAnalysis: [
      "Xu huong chinh: TANG manh tren H4 va D1. EMA 50 cat len EMA 200 tao Golden Cross — tin hieu tang dai han.",
      "DXY (Dollar Index) dang suy yeu sau du lieu Non-Farm thap hon ky vong. Correlation am voi Gold dang phat huy tac dung, day gia Gold len cao hon.",
      "Volume giao dich tang 34% so voi trung binh 20 ngay, cho thay dong tien lon dang chay vao Gold. Institutional flow data tu COT report cung confirm xu huong nay.",
      "Muc khang cu quan trong tai 2,195 — neu breakout se mo duong den 2,210-2,220. Ho tro chinh tai 2,155 (EMA 50 H4).",
      "Risk event: Fed FOMC minutes se cong bo trong 48h toi. Neu tone dovish, Gold co the surge them 1-2%. Nguoc lai, hawkish surprise co the gay pullback ve 2,155.",
      "Khuyen nghi: BUY on dip tai 2,175 voi SL duoi 2,155. Chia 2 target: TP1 tai 2,195 (partial close 50%), TP2 tai 2,210 (full close).",
    ],
  },
  "xag-usd": {
    symbol: "XAG/USD", name: "Silver", price: "24.82", change: "+1.24%", type: "profit",
    category: "COMMODITIES", confidence: 74, direction: "BUY",
    session: "London / New York", timeframe: "H4",
    keyLevels: ["24.20", "24.50", "25.00", "25.40"],
    entry: "24.70", tp: "25.40", sl: "24.20",
    riskReward: "1:1.4", winRate: "65%", avgHoldTime: "8h 15m",
    technicals: [
      { name: "RSI (14)", value: "58.7", signal: "bullish" },
      { name: "MACD", value: "Bullish", signal: "bullish" },
      { name: "EMA 50/200", value: "Bullish Align", signal: "bullish" },
      { name: "Bollinger", value: "Mid Band", signal: "neutral" },
      { name: "Stochastic", value: "65.3", signal: "neutral" },
      { name: "ATR (14)", value: "0.42", signal: "neutral" },
      { name: "Volume", value: "Average", signal: "neutral" },
      { name: "ADX", value: "24.8 (Weak)", signal: "neutral" },
    ],
    summary: "Silver breakout khoi channel 23.80-24.50, momentum tang. Gold/Silver ratio dang giam, bao hieu silver outperform. Ho tro tai 24.20, resistance tiep theo 25.40.",
    deepAnalysis: [
      "Silver da pha vo channel 23.80-24.50 sau 2 tuan consolidation. Breakout duoc xac nhan boi volume tang 28%.",
      "Gold/Silver ratio giam tu 88.5 xuong 87.7 — Silver dang outperform Gold, thuong xay ra trong giai doan risk-on.",
      "Industrial demand cho Silver dang tang do nhu cau solar panel va electronic components. Fundamentals ho tro gia tang.",
      "Technical: RSI 58.7 con room tang, MACD bullish cross moi xay ra. Target tai 25.40 la confluence zone cua Fibonacci 61.8% va previous swing high.",
      "Risk: ADX chi 24.8, cho thay trend con yeu. Can them volume de xac nhan suc manh cua breakout.",
      "Khuyen nghi: BUY voi entry 24.70, SL tai 24.20 (duoi breakout level). Can nhac giam size vi conviction trung binh.",
    ],
  },
  "wti-usd": {
    symbol: "WTI/USD", name: "Crude Oil", price: "78.45", change: "-0.67%", type: "loss",
    category: "COMMODITIES", confidence: 68, direction: "SELL",
    session: "New York", timeframe: "H1",
    keyLevels: ["76.80", "78.00", "79.60", "80.50"],
    entry: "78.80", tp: "76.80", sl: "79.60",
    riskReward: "1:2.5", winRate: "58%", avgHoldTime: "4h 45m",
    technicals: [
      { name: "RSI (14)", value: "42.1", signal: "bearish" },
      { name: "MACD", value: "Bearish", signal: "bearish" },
      { name: "EMA 50/200", value: "Death Cross", signal: "bearish" },
      { name: "Bollinger", value: "Lower Band", signal: "bearish" },
      { name: "Stochastic", value: "28.4", signal: "bearish" },
      { name: "ATR (14)", value: "1.82", signal: "neutral" },
      { name: "Volume", value: "Below Avg", signal: "bearish" },
      { name: "ADX", value: "28.5 (Trend)", signal: "neutral" },
    ],
    summary: "Oil giam do lo ngai demand yeu tu Trung Quoc. OPEC+ van giu cat giam san luong nhung market khong phan ung. Support quan trong tai 76.80, pha vo se den 74.50.",
    deepAnalysis: [
      "WTI dang trong downtrend ro rang tren H1 va H4. Death Cross (EMA 50 duoi EMA 200) xac nhan trend giam.",
      "Du lieu PMI Trung Quoc thap hon ky vong (49.1 vs 50.2) gay lo ngai ve demand. Trung Quoc la nuoc nhap khau dau lon nhat the gioi.",
      "OPEC+ giu nguyen muc cat giam 2.2 trieu thung/ngay nhung market da price-in. Can them catalyst de day gia len.",
      "US inventory data: tuan truoc tang 3.2 trieu thung, cao hon du kien (1.5M). Bearish signal cho short-term.",
      "Support chinh tai 76.80 — muc nay da hold 3 lan trong 2 tuan qua. Pha vo se accelerate selling den 74.50.",
      "Khuyen nghi: SELL tai 78.80 voi SL tai 79.60. Risk/reward 1:2.5 rat hap dan. Target chinh: 76.80.",
    ],
  },
  "eur-usd": {
    symbol: "EUR/USD", name: "Euro", price: "1.0847", change: "+0.24%", type: "profit",
    category: "FOREX", confidence: 81, direction: "BUY",
    session: "London / New York", timeframe: "H1",
    keyLevels: ["1.0780", "1.0830", "1.0860", "1.0900"],
    entry: "1.0830", tp: "1.0900", sl: "1.0780",
    riskReward: "1:1.4", winRate: "68%", avgHoldTime: "3h 20m",
    technicals: [
      { name: "RSI (14)", value: "55.8", signal: "bullish" },
      { name: "MACD", value: "Bullish Cross", signal: "bullish" },
      { name: "EMA 50/200", value: "Bullish Align", signal: "bullish" },
      { name: "Bollinger", value: "Mid-Upper", signal: "bullish" },
      { name: "Stochastic", value: "62.1", signal: "neutral" },
      { name: "ATR (14)", value: "0.0058", signal: "neutral" },
      { name: "Volume", value: "Above Avg", signal: "bullish" },
      { name: "ADX", value: "26.4 (Trend)", signal: "neutral" },
    ],
    summary: "EUR/USD rebound tu support 1.0780. ECB giu lai suat on dinh trong khi Fed co tin hieu dovish. Target 1.0900 neu breakout 1.0860.",
    deepAnalysis: [
      "EUR/USD phuc hoi manh tu support zone 1.0780 — vung nay trung voi EMA 200 tren H1 va Fibonacci 38.2%.",
      "ECB giu lai suat 4.5% va signal se khong cat truoc Q3. Trong khi do, Fed dot pivot dovish voi 3 lan cat lai suat du kien trong 2024.",
      "Interest rate differential dang thu hep co loi cho EUR. Bond yield spread My-EU giam 15bps trong tuan qua.",
      "Technical: MACD bullish cross moi hinh thanh, RSI 55.8 con nhieu room tang. Key resistance 1.0860 — breakout se open target 1.0900.",
      "Risk: NFP data cuoi tuan nay. Neu jobs data manh bat ngo, USD co the rally va EUR/USD se giam.",
      "Khuyen nghi: BUY tai 1.0830 voi SL 1.0780. Position size trung binh do NFP risk event sap den.",
    ],
  },
  "btc-usd": {
    symbol: "BTC/USD", name: "Bitcoin", price: "67,842", change: "+2.14%", type: "profit",
    category: "CRYPTO", confidence: 78, direction: "BUY",
    session: "24/7", timeframe: "D1",
    keyLevels: ["65,000", "67,000", "70,000", "72,000"],
    entry: "67,500", tp: "72,000", sl: "65,000",
    riskReward: "1:1.8", winRate: "64%", avgHoldTime: "2d 4h",
    technicals: [
      { name: "RSI (14)", value: "64.2", signal: "bullish" },
      { name: "MACD", value: "Bullish", signal: "bullish" },
      { name: "EMA 50/200", value: "Golden Cross", signal: "bullish" },
      { name: "Bollinger", value: "Upper Band", signal: "neutral" },
      { name: "Stochastic", value: "72.8", signal: "neutral" },
      { name: "ATR (14)", value: "2,150", signal: "neutral" },
      { name: "Volume", value: "High", signal: "bullish" },
      { name: "ADX", value: "35.2 (Strong)", signal: "bullish" },
    ],
    summary: "BTC tang manh do ETF inflows ky luc. Halving sap den trong Q2. Support manh tai 65,000. Target 72,000 neu volume duy tri.",
    deepAnalysis: [
      "Bitcoin dang trong strong uptrend, ADX 35.2 xac nhan trend manh. Golden Cross tren D1 la bullish signal dai han.",
      "Spot Bitcoin ETF inflows dat $890M trong tuan qua — muc cao nhat ke tu ngay launch. Institutional adoption dang accelerate.",
      "Bitcoin Halving du kien vao thang 4/2024. Lich su cho thay BTC thuong tang 50-100% trong 6-12 thang sau halving.",
      "On-chain data: Whale wallets (>1000 BTC) tang them 12 trong 30 ngay qua. Exchange balance giam — dau hieu accumulation.",
      "Key resistance tai 70,000 — muc tam ly lon. Breakout se trigger FOMO wave va push den 72,000-75,000.",
      "Khuyen nghi: BUY tai 67,500 voi SL tai 65,000. Day la swing trade, expected hold time 2-5 ngay. Can nhac DCA neu gia pullback.",
    ],
  },
};

// Default fallback for instruments not in the detailed map
function getDefaultInstrument(slug: string): Instrument {
  const symbol = slug.replace("-", "/").toUpperCase();
  return {
    symbol, name: symbol, price: "--", change: "--", type: "profit",
    category: "OTHER", confidence: 50, direction: "NEUTRAL",
    session: "--", timeframe: "--", keyLevels: [],
    entry: "--", tp: "--", sl: "--",
    riskReward: "--", winRate: "--", avgHoldTime: "--",
    technicals: [],
    summary: "Chua co du lieu phan tich cho cap nay.",
    deepAnalysis: ["Hien tai chua co du lieu phan tich chi tiet. Vui long quay lai sau."],
  };
}

function ConfidenceBar({ value }: { value: number }) {
  return (
    <div className="flex items-center gap-3">
      <div className="h-2.5 flex-1 bg-secondary overflow-hidden">
        <motion.div
          className="h-full"
          initial={{ width: 0 }}
          animate={{ width: `${value}%` }}
          transition={{ duration: 0.5, ease: "easeOut" }}
          style={{
            backgroundColor: value >= 80 ? "var(--profit)" : value >= 65 ? "var(--cyan)" : "var(--warning)",
          }}
        />
      </div>
      <span
        className={cn(
          "text-[18px] font-bold tabular-nums",
          value >= 80 ? "text-profit" : value >= 65 ? "text-cyan" : "text-warning"
        )}
      >
        {value}%
      </span>
    </div>
  );
}

export default function DeepAnalysisPage() {
  const params = useParams();
  const slug = params.symbol as string;
  const inst = instrumentsMap[slug] || getDefaultInstrument(slug);

  const dirColor = inst.direction === "BUY" ? "var(--profit)" : inst.direction === "SELL" ? "var(--loss)" : "var(--border)";

  return (
    <div className="space-y-6 overflow-y-auto p-6 h-full">
      {/* Back + Header */}
      <motion.div initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.15 }}>
        <Link href="/dashboard" className="inline-flex items-center gap-2 text-[12px] text-text-secondary hover:text-foreground transition-colors mb-4 outline-none">
          <span>&larr;</span> Back to Dashboard
        </Link>
        <div className="flex items-start justify-between">
          <div>
            <div className="flex items-center gap-3">
              <h1 className="text-[32px] font-bold tracking-[0.5px]">{inst.symbol}</h1>
              <span
                className={cn(
                  "px-3 py-1.5 text-[11px] font-bold tracking-wider",
                  inst.direction === "BUY" && "bg-profit/12 text-profit border border-profit/20",
                  inst.direction === "SELL" && "bg-loss/12 text-loss border border-loss/20",
                  inst.direction === "NEUTRAL" && "bg-card-alt text-text-muted border border-border",
                )}
              >
                {inst.direction}
              </span>
            </div>
            <div className="mt-1 flex items-center gap-2 text-[13px] text-text-muted">
              <span>{inst.name}</span>
              <span className="text-text-faint">|</span>
              <span>{inst.category}</span>
              <span className="text-text-faint">|</span>
              <span>{inst.timeframe}</span>
              <span className="text-text-faint">|</span>
              <span>{inst.session}</span>
            </div>
          </div>
          <div className="text-right">
            <div className="text-[36px] font-bold tabular-nums">{inst.price}</div>
            <div className={cn("text-[16px] font-semibold tabular-nums", inst.type === "profit" ? "text-profit" : "text-loss")}>
              {inst.type === "profit" ? "+" : ""}{inst.change}
            </div>
          </div>
        </div>
      </motion.div>

      {/* Top row: Confidence + Stats */}
      <motion.div initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.2, delay: 0.05 }} className="grid grid-cols-1 lg:grid-cols-[1fr_380px] gap-5">
        {/* Confidence + Chart */}
        <div className="border border-border bg-card">
          <div className="h-[2px] w-full" style={{ backgroundColor: dirColor }} />
          <div className="p-6 space-y-5">
            <div>
              <div className="text-[10px] font-bold uppercase tracking-[1.5px] text-text-muted mb-3">AI CONFIDENCE</div>
              <ConfidenceBar value={inst.confidence} />
            </div>
            {/* Large Chart */}
            <div>
              <div className="text-[10px] font-bold uppercase tracking-[1.5px] text-text-muted mb-3">PRICE ACTION</div>
              <div className="relative h-48 w-full">
                <svg viewBox="0 0 600 180" className="w-full h-full" preserveAspectRatio="none">
                  {[0, 36, 72, 108, 144, 180].map((y) => (
                    <line key={y} x1="0" y1={y} x2="600" y2={y} stroke="var(--border)" strokeWidth="0.5" strokeDasharray="4 4" />
                  ))}
                  <path
                    d={(() => {
                      const pts = Array.from({ length: 80 }, (_, j) => {
                        const x = (j / 79) * 600;
                        const base = 90 + Math.sin(j * 0.12) * 30;
                        const noise = Math.sin(j * 0.35) * 15 + Math.cos(j * 0.22) * 10;
                        const trend = inst.type === "profit" ? -j * 0.4 : j * 0.4;
                        const y = Math.max(5, Math.min(175, base + noise + trend));
                        return { x, y };
                      });
                      const line = pts.map((p, i) => `${i === 0 ? "M" : "L"}${p.x.toFixed(1)},${p.y.toFixed(1)}`).join(" ");
                      return `${line} L600,180 L0,180 Z`;
                    })()}
                    fill={inst.type === "profit" ? "var(--profit)" : "var(--loss)"}
                    fillOpacity="0.06"
                  />
                  <polyline
                    points={Array.from({ length: 80 }, (_, j) => {
                      const x = (j / 79) * 600;
                      const base = 90 + Math.sin(j * 0.12) * 30;
                      const noise = Math.sin(j * 0.35) * 15 + Math.cos(j * 0.22) * 10;
                      const trend = inst.type === "profit" ? -j * 0.4 : j * 0.4;
                      const y = Math.max(5, Math.min(175, base + noise + trend));
                      return `${x.toFixed(1)},${y.toFixed(1)}`;
                    }).join(" ")}
                    fill="none"
                    stroke={inst.type === "profit" ? "var(--profit)" : "var(--loss)"}
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  />
                </svg>
              </div>
            </div>
          </div>
        </div>

        {/* Right: Entry/TP/SL + Key Stats */}
        <div className="space-y-5">
          {/* Entry / TP / SL */}
          {inst.direction !== "NEUTRAL" && (
            <div className="border border-border bg-card">
              <div className="h-[2px] w-full" style={{ backgroundColor: dirColor }} />
              <div className="p-5 space-y-4">
                <div className="text-[10px] font-bold uppercase tracking-[1.5px] text-text-muted">TRADE SETUP</div>
                {[
                  { label: "ENTRY", value: inst.entry, cls: "text-foreground", bg: "bg-card-alt border-border" },
                  { label: "TAKE PROFIT", value: inst.tp, cls: "text-profit", bg: "bg-profit/5 border-profit/15" },
                  { label: "STOP LOSS", value: inst.sl, cls: "text-loss", bg: "bg-loss/5 border-loss/15" },
                ].map((item) => (
                  <div key={item.label} className={`flex items-center justify-between border p-4 ${item.bg}`}>
                    <span className={`text-[10px] font-bold uppercase tracking-wider ${item.cls}`}>{item.label}</span>
                    <span className={`text-[18px] font-bold tabular-nums ${item.cls}`}>{item.value}</span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Stats */}
          <div className="grid grid-cols-3 gap-3">
            {[
              { label: "R:R", value: inst.riskReward },
              { label: "WIN RATE", value: inst.winRate },
              { label: "AVG HOLD", value: inst.avgHoldTime },
            ].map((s) => (
              <div key={s.label} className="border border-border bg-card p-4 text-center">
                <div className="text-[9px] font-bold uppercase tracking-wider text-text-muted">{s.label}</div>
                <div className="mt-2 text-[16px] font-bold text-cyan">{s.value}</div>
              </div>
            ))}
          </div>

          {/* Key Levels */}
          <div className="border border-border bg-card p-5">
            <div className="text-[10px] font-bold uppercase tracking-[1.5px] text-text-muted mb-3">KEY LEVELS</div>
            <div className="flex flex-wrap gap-2">
              {inst.keyLevels.map((level) => (
                <span key={level} className="border border-border bg-card-alt px-3 py-1.5 text-[14px] font-bold tabular-nums">
                  {level}
                </span>
              ))}
            </div>
          </div>
        </div>
      </motion.div>

      {/* Technical Indicators */}
      {inst.technicals.length > 0 && (
        <motion.div initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.2, delay: 0.1 }} className="border border-border bg-card">
          <div className="p-5">
            <div className="text-[10px] font-bold uppercase tracking-[1.5px] text-text-muted mb-4">TECHNICAL INDICATORS</div>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
              {inst.technicals.map((tech) => (
                <div key={tech.name} className="border border-border bg-card-alt p-4">
                  <div className="text-[10px] font-bold tracking-wider text-text-muted">{tech.name}</div>
                  <div className="mt-2 text-[15px] font-bold">{tech.value}</div>
                  <div className={cn(
                    "mt-1 text-[9px] font-bold uppercase tracking-wider",
                    tech.signal === "bullish" ? "text-profit" : tech.signal === "bearish" ? "text-loss" : "text-text-muted"
                  )}>
                    {tech.signal.toUpperCase()}
                  </div>
                </div>
              ))}
            </div>
          </div>
        </motion.div>
      )}

      {/* Deep AI Analysis */}
      <motion.div initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.2, delay: 0.15 }} className="border border-border bg-card">
        <div className="h-[2px] w-full bg-cyan" />
        <div className="p-6">
          <div className="text-[10px] font-bold uppercase tracking-[1.5px] text-cyan mb-5">DEEP AI ANALYSIS</div>
          <div className="space-y-4">
            {inst.deepAnalysis.map((paragraph, i) => (
              <motion.div
                key={i}
                initial={{ opacity: 0, x: -8 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ duration: 0.2, delay: 0.2 + i * 0.05 }}
                className="flex gap-4"
              >
                <div className="shrink-0 mt-1">
                  <div className="h-6 w-6 flex items-center justify-center border border-cyan/20 bg-cyan/5 text-[10px] font-bold text-cyan">
                    {i + 1}
                  </div>
                </div>
                <p className="text-[14px] font-bold leading-[1.9] text-foreground/80">
                  {paragraph}
                </p>
              </motion.div>
            ))}
          </div>
        </div>
      </motion.div>
    </div>
  );
}
