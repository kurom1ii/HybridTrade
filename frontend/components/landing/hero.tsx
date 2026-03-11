"use client";

import { useState, useEffect, useCallback } from "react";
import { motion, AnimatePresence } from "motion/react";
import Link from "next/link";

/* ── Sidebar nav items ── */
const sidebarItems = ["Dashboard", "Markets", "Positions", "Orders", "Agents", "Signals"] as const;
type TabKey = (typeof sidebarItems)[number];

/* ── Chart bar sets per tab ── */
const chartSets: Record<string, number[]> = {
  "EUR/USD": [38, 52, 45, 68, 55, 72, 60, 78, 65, 82, 70, 88, 75, 92, 80, 95, 72, 85, 90, 98, 82, 75, 88, 95, 78, 92, 86, 80, 94, 100],
  "BTC/USD": [55, 60, 48, 72, 80, 75, 85, 90, 78, 65, 70, 82, 95, 88, 76, 92, 98, 85, 80, 75, 68, 74, 82, 90, 96, 88, 92, 85, 78, 95],
  "XAU/USD": [42, 45, 50, 55, 52, 58, 62, 65, 60, 68, 72, 70, 75, 78, 82, 80, 85, 88, 90, 92, 88, 85, 90, 95, 92, 88, 94, 96, 98, 100],
  "ETH/USD": [30, 45, 40, 55, 50, 65, 58, 72, 68, 80, 75, 60, 55, 70, 85, 78, 90, 82, 75, 88, 92, 85, 78, 95, 88, 82, 90, 94, 88, 96],
};

/* ── Market data ── */
const allMarkets = [
  { pair: "EUR/USD", price: "1.0847", change: "+0.24%", up: true, vol: "$8.2B", high: "1.0892", low: "1.0801" },
  { pair: "BTC/USD", price: "67,842", change: "+2.14%", up: true, vol: "$42.1B", high: "68,420", low: "66,180" },
  { pair: "USD/JPY", price: "149.82", change: "-0.31%", up: false, vol: "$5.1B", high: "150.24", low: "149.50" },
  { pair: "ETH/USD", price: "3,456", change: "-1.07%", up: false, vol: "$18.7B", high: "3,520", low: "3,410" },
  { pair: "XAU/USD", price: "2,178", change: "+0.89%", up: true, vol: "$6.4B", high: "2,195", low: "2,160" },
  { pair: "GBP/JPY", price: "189.45", change: "+0.15%", up: true, vol: "$2.8B", high: "189.92", low: "188.80" },
  { pair: "SOL/USD", price: "142.80", change: "+4.32%", up: true, vol: "$3.2B", high: "145.60", low: "136.90" },
  { pair: "AUD/USD", price: "0.6542", change: "-0.18%", up: false, vol: "$1.9B", high: "0.6568", low: "0.6520" },
];

const positions = [
  { pair: "EUR/USD", side: "LONG" as const, size: "1.50", entry: "1.0842", pnl: "+$342.50", pnlPct: "+0.52%", status: "ACTIVE", duration: "2h 14m" },
  { pair: "BTC/USD", side: "LONG" as const, size: "0.25", entry: "67,240", pnl: "+$1,240.00", pnlPct: "+1.84%", status: "ACTIVE", duration: "5h 42m" },
  { pair: "GBP/JPY", side: "SHORT" as const, size: "0.80", entry: "189.45", pnl: "-$128.40", pnlPct: "-0.34%", status: "TRAILING", duration: "1h 08m" },
  { pair: "XAU/USD", side: "LONG" as const, size: "0.50", entry: "2,165.30", pnl: "+$87.20", pnlPct: "+0.41%", status: "ACTIVE", duration: "3h 21m" },
  { pair: "SOL/USD", side: "LONG" as const, size: "2.00", entry: "140.50", pnl: "+$460.00", pnlPct: "+1.64%", status: "ACTIVE", duration: "8h 05m" },
];

const orders = [
  { id: "ORD-2841", pair: "EUR/USD", type: "LIMIT BUY", price: "1.0820", size: "2.00", status: "PENDING", placed: "14:32" },
  { id: "ORD-2842", pair: "BTC/USD", type: "STOP LOSS", price: "66,500", size: "0.25", status: "ARMED", placed: "12:18" },
  { id: "ORD-2843", pair: "ETH/USD", type: "LIMIT BUY", price: "3,380", size: "1.50", status: "PENDING", placed: "11:45" },
  { id: "ORD-2844", pair: "XAU/USD", type: "TAKE PROFIT", price: "2,200", size: "0.50", status: "ARMED", placed: "09:52" },
  { id: "ORD-2845", pair: "GBP/JPY", type: "STOP LOSS", price: "190.20", size: "0.80", status: "ARMED", placed: "13:01" },
  { id: "ORD-2846", pair: "SOL/USD", type: "TAKE PROFIT", price: "155.00", size: "2.00", status: "ARMED", placed: "08:30" },
];

const agentsDetailed = [
  { name: "Momentum Scanner", status: "RUNNING" as const, trades: 142, winRate: "68%", pnl: "+$4,280", uptime: "99.8%", pairs: "EUR, GBP, BTC" },
  { name: "Risk Manager", status: "RUNNING" as const, trades: 0, winRate: "—", pnl: "—", uptime: "100%", pairs: "ALL" },
  { name: "News Analyzer", status: "PAUSED" as const, trades: 38, winRate: "72%", pnl: "+$1,840", uptime: "94.2%", pairs: "BTC, ETH, SOL" },
  { name: "Trend Follower", status: "RUNNING" as const, trades: 96, winRate: "61%", pnl: "+$2,960", uptime: "99.5%", pairs: "XAU, EUR, JPY" },
  { name: "Mean Reverter", status: "IDLE" as const, trades: 54, winRate: "58%", pnl: "+$820", uptime: "97.1%", pairs: "USD/JPY, AUD" },
  { name: "Scalp Engine", status: "RUNNING" as const, trades: 312, winRate: "55%", pnl: "+$1,560", uptime: "99.9%", pairs: "EUR, BTC" },
];

const signals = [
  { time: "14:32:18", pair: "EUR/USD", signal: "BUY", strength: 92, source: "Momentum", conf: "HIGH" },
  { time: "14:31:45", pair: "BTC/USD", signal: "HOLD", strength: 67, source: "Trend", conf: "MED" },
  { time: "14:30:02", pair: "XAU/USD", signal: "BUY", strength: 84, source: "Mean Rev", conf: "HIGH" },
  { time: "14:28:51", pair: "GBP/JPY", signal: "SELL", strength: 78, source: "Momentum", conf: "MED" },
  { time: "14:27:14", pair: "SOL/USD", signal: "BUY", strength: 88, source: "News AI", conf: "HIGH" },
  { time: "14:25:33", pair: "ETH/USD", signal: "SELL", strength: 71, source: "Trend", conf: "MED" },
  { time: "14:24:08", pair: "USD/JPY", signal: "HOLD", strength: 45, source: "Scalp", conf: "LOW" },
];

const watchlist = allMarkets.slice(0, 5);

/* ── KPI sets per tab ── */
const kpiSets: Record<TabKey, { label: string; value: string; sub: string; color: string }[]> = {
  Dashboard: [
    { label: "PORTFOLIO", value: "$124,532", sub: "↑ 3.2%", color: "text-foreground" },
    { label: "P&L TODAY", value: "+$2,342", sub: "+1.92%", color: "text-profit" },
    { label: "POSITIONS", value: "12", sub: "8L · 4S", color: "text-foreground" },
    { label: "AGENTS", value: "4/6", sub: "67%", color: "text-cyan" },
  ],
  Markets: [
    { label: "PAIRS", value: "52", sub: "8 active", color: "text-foreground" },
    { label: "VOLUME 24H", value: "$88.4B", sub: "↑ 12.4%", color: "text-cyan" },
    { label: "VOLATILITY", value: "0.82", sub: "Medium", color: "text-warning" },
    { label: "TRENDING", value: "BTC", sub: "+2.14%", color: "text-profit" },
  ],
  Positions: [
    { label: "OPEN", value: "5", sub: "4L · 1S", color: "text-foreground" },
    { label: "UNREALIZED", value: "+$2,001", sub: "+1.62%", color: "text-profit" },
    { label: "MARGIN USED", value: "$18,420", sub: "14.8%", color: "text-foreground" },
    { label: "RISK SCORE", value: "LOW", sub: "2.1x", color: "text-profit" },
  ],
  Orders: [
    { label: "PENDING", value: "6", sub: "2 limit · 4 stop", color: "text-foreground" },
    { label: "FILLED TODAY", value: "18", sub: "94% fill rate", color: "text-profit" },
    { label: "CANCELLED", value: "2", sub: "timeout", color: "text-loss" },
    { label: "AVG FILL", value: "0.03ms", sub: "< target", color: "text-cyan" },
  ],
  Agents: [
    { label: "ACTIVE", value: "4/6", sub: "67%", color: "text-cyan" },
    { label: "TOTAL TRADES", value: "642", sub: "↑ 24 today", color: "text-foreground" },
    { label: "AVG WIN RATE", value: "63%", sub: "↑ 2.1%", color: "text-profit" },
    { label: "TOTAL P&L", value: "+$11,460", sub: "all time", color: "text-profit" },
  ],
  Signals: [
    { label: "GENERATED", value: "847", sub: "today", color: "text-foreground" },
    { label: "ACCURACY", value: "78%", sub: "↑ 3.2%", color: "text-profit" },
    { label: "HIGH CONF", value: "124", sub: "14.6%", color: "text-cyan" },
    { label: "ACTED ON", value: "92", sub: "74% rate", color: "text-foreground" },
  ],
};

/* ── Ticker pairs for window chrome ── */
const tickerMap: Record<TabKey, string> = {
  Dashboard: "EUR/USD · 1H",
  Markets: "52 PAIRS · LIVE",
  Positions: "5 OPEN · $2,001",
  Orders: "6 PENDING · 0.03ms",
  Agents: "4 ACTIVE · 63% WR",
  Signals: "847 TODAY · 78%",
};

/* ═══════════════════════════════════════════════
   TAB CONTENT PANELS
   ═══════════════════════════════════════════════ */

function DashboardTab() {
  return (
    <>
      {/* Chart area */}
      <div className="flex-1 min-h-0 flex flex-col">
        <div className="px-3 py-2 flex items-center justify-between border-b border-border/20">
          <div>
            <span className="text-[11px] font-bold">EUR/USD</span>
            <span className="text-[11px] font-bold text-cyan ml-2">1.0847</span>
            <span className="text-[9px] text-profit ml-2">+0.24%</span>
          </div>
          <div className="flex items-center gap-1">
            {["1M", "5M", "15M", "1H", "4H", "1D"].map((tf, i) => (
              <span key={tf} className={`px-1.5 py-0.5 text-[8px] font-bold ${i === 3 ? "bg-cyan/15 text-cyan" : "text-text-secondary/40"}`}>
                {tf}
              </span>
            ))}
          </div>
        </div>
        <div className="flex-1 flex items-end px-3 pb-2 gap-[3px]">
          {chartSets["EUR/USD"].map((h, i) => (
            <motion.div
              key={i}
              initial={{ height: 0 }}
              animate={{ height: `${h}%` }}
              transition={{ duration: 0.3, delay: i * 0.015 }}
              className="flex-1 min-w-0"
              style={{
                backgroundColor: i >= 25 ? "var(--cyan)" : h > chartSets["EUR/USD"][Math.max(0, i - 1)] ? "var(--profit)" : "var(--loss)",
                opacity: 0.15 + (i / 30) * 0.55,
              }}
            />
          ))}
        </div>
      </div>

      {/* Positions mini table */}
      <div className="border-t border-border/30">
        <div className="px-3 py-1.5 flex items-center justify-between">
          <span className="text-[9px] font-bold tracking-[1px] text-text-secondary/40">ACTIVE POSITIONS</span>
          <span className="text-[9px] font-bold text-cyan/50">4 OPEN</span>
        </div>
        <table className="w-full text-[9px]">
          <thead>
            <tr className="border-t border-border/20 text-[8px] text-text-secondary/30 font-bold tracking-[0.5px]">
              <th className="px-3 py-1 text-left">PAIR</th>
              <th className="px-3 py-1 text-left">SIDE</th>
              <th className="px-3 py-1 text-right">SIZE</th>
              <th className="px-3 py-1 text-right">P&L</th>
              <th className="px-3 py-1 text-right">STATUS</th>
            </tr>
          </thead>
          <tbody>
            {positions.slice(0, 4).map((pos, i) => (
              <motion.tr key={pos.pair} initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={{ duration: 0.2, delay: i * 0.05 }} className="border-t border-border/10">
                <td className="px-3 py-1 font-semibold">{pos.pair}</td>
                <td className={`px-3 py-1 font-bold ${pos.side === "LONG" ? "text-cyan" : "text-loss"}`}>{pos.side}</td>
                <td className="px-3 py-1 text-right text-text-secondary/50">{pos.size}</td>
                <td className={`px-3 py-1 text-right font-semibold ${pos.pnl.startsWith("+") ? "text-profit" : "text-loss"}`}>{pos.pnl}</td>
                <td className="px-3 py-1 text-right text-cyan/60">{pos.status}</td>
              </motion.tr>
            ))}
          </tbody>
        </table>
      </div>
    </>
  );
}

function MarketsTab() {
  return (
    <div className="flex-1 min-h-0 overflow-hidden">
      {/* Markets header */}
      <div className="px-3 py-2 flex items-center justify-between border-b border-border/20">
        <span className="text-[9px] font-bold tracking-[1px] text-text-secondary/40">ALL MARKETS</span>
        <div className="flex items-center gap-2">
          {["ALL", "FOREX", "CRYPTO", "METALS"].map((f, i) => (
            <span key={f} className={`px-1.5 py-0.5 text-[8px] font-bold ${i === 0 ? "bg-cyan/15 text-cyan" : "text-text-secondary/40"}`}>{f}</span>
          ))}
        </div>
      </div>
      {/* Markets table */}
      <table className="w-full text-[9px]">
        <thead>
          <tr className="text-[8px] text-text-secondary/30 font-bold tracking-[0.5px] border-b border-border/20">
            <th className="px-3 py-1.5 text-left">PAIR</th>
            <th className="px-3 py-1.5 text-right">PRICE</th>
            <th className="px-3 py-1.5 text-right">24H</th>
            <th className="px-3 py-1.5 text-right">VOLUME</th>
            <th className="px-3 py-1.5 text-right">HIGH</th>
            <th className="px-3 py-1.5 text-right">LOW</th>
          </tr>
        </thead>
        <tbody>
          {allMarkets.map((m, i) => (
            <motion.tr key={m.pair} initial={{ opacity: 0, x: -10 }} animate={{ opacity: 1, x: 0 }} transition={{ duration: 0.2, delay: i * 0.04 }} className="border-b border-border/10 hover:bg-card-alt/30">
              <td className="px-3 py-1.5 font-semibold">
                <span className={`inline-block h-1.5 w-1.5 rounded-full mr-1.5 ${m.up ? "bg-profit" : "bg-loss"}`} />
                {m.pair}
              </td>
              <td className="px-3 py-1.5 text-right font-medium">{m.price}</td>
              <td className={`px-3 py-1.5 text-right font-bold ${m.up ? "text-profit" : "text-loss"}`}>{m.change}</td>
              <td className="px-3 py-1.5 text-right text-text-secondary/50">{m.vol}</td>
              <td className="px-3 py-1.5 text-right text-text-secondary/40">{m.high}</td>
              <td className="px-3 py-1.5 text-right text-text-secondary/40">{m.low}</td>
            </motion.tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function PositionsTab() {
  return (
    <div className="flex-1 min-h-0 overflow-hidden flex flex-col">
      <div className="px-3 py-2 flex items-center justify-between border-b border-border/20">
        <span className="text-[9px] font-bold tracking-[1px] text-text-secondary/40">OPEN POSITIONS</span>
        <span className="text-[9px] font-bold text-cyan/50">5 ACTIVE</span>
      </div>
      <table className="w-full text-[9px]">
        <thead>
          <tr className="text-[8px] text-text-secondary/30 font-bold tracking-[0.5px] border-b border-border/20">
            <th className="px-3 py-1.5 text-left">PAIR</th>
            <th className="px-3 py-1.5 text-left">SIDE</th>
            <th className="px-3 py-1.5 text-right">SIZE</th>
            <th className="px-3 py-1.5 text-right">ENTRY</th>
            <th className="px-3 py-1.5 text-right">P&L</th>
            <th className="px-3 py-1.5 text-right">%</th>
            <th className="px-3 py-1.5 text-right">DURATION</th>
          </tr>
        </thead>
        <tbody>
          {positions.map((pos, i) => (
            <motion.tr key={pos.pair} initial={{ opacity: 0, x: -10 }} animate={{ opacity: 1, x: 0 }} transition={{ duration: 0.2, delay: i * 0.05 }} className="border-b border-border/10">
              <td className="px-3 py-1.5 font-semibold">{pos.pair}</td>
              <td className={`px-3 py-1.5 font-bold ${pos.side === "LONG" ? "text-cyan" : "text-loss"}`}>{pos.side}</td>
              <td className="px-3 py-1.5 text-right text-text-secondary/50">{pos.size}</td>
              <td className="px-3 py-1.5 text-right text-text-secondary/60">{pos.entry}</td>
              <td className={`px-3 py-1.5 text-right font-semibold ${pos.pnl.startsWith("+") ? "text-profit" : "text-loss"}`}>{pos.pnl}</td>
              <td className={`px-3 py-1.5 text-right text-[8px] ${pos.pnlPct.startsWith("+") ? "text-profit/70" : "text-loss/70"}`}>{pos.pnlPct}</td>
              <td className="px-3 py-1.5 text-right text-text-secondary/40">{pos.duration}</td>
            </motion.tr>
          ))}
        </tbody>
      </table>
      {/* Totals row */}
      <div className="mt-auto border-t border-border/30 px-3 py-2 flex items-center justify-between">
        <span className="text-[9px] font-bold text-text-secondary/40">TOTAL UNREALIZED</span>
        <span className="text-[11px] font-bold text-profit">+$2,001.30</span>
      </div>
    </div>
  );
}

function OrdersTab() {
  return (
    <div className="flex-1 min-h-0 overflow-hidden flex flex-col">
      <div className="px-3 py-2 flex items-center justify-between border-b border-border/20">
        <span className="text-[9px] font-bold tracking-[1px] text-text-secondary/40">PENDING ORDERS</span>
        <div className="flex items-center gap-2">
          {["ALL", "LIMIT", "STOP", "TP/SL"].map((f, i) => (
            <span key={f} className={`px-1.5 py-0.5 text-[8px] font-bold ${i === 0 ? "bg-cyan/15 text-cyan" : "text-text-secondary/40"}`}>{f}</span>
          ))}
        </div>
      </div>
      <table className="w-full text-[9px]">
        <thead>
          <tr className="text-[8px] text-text-secondary/30 font-bold tracking-[0.5px] border-b border-border/20">
            <th className="px-3 py-1.5 text-left">ID</th>
            <th className="px-3 py-1.5 text-left">PAIR</th>
            <th className="px-3 py-1.5 text-left">TYPE</th>
            <th className="px-3 py-1.5 text-right">PRICE</th>
            <th className="px-3 py-1.5 text-right">SIZE</th>
            <th className="px-3 py-1.5 text-right">STATUS</th>
          </tr>
        </thead>
        <tbody>
          {orders.map((o, i) => (
            <motion.tr key={o.id} initial={{ opacity: 0, x: -10 }} animate={{ opacity: 1, x: 0 }} transition={{ duration: 0.2, delay: i * 0.04 }} className="border-b border-border/10">
              <td className="px-3 py-1.5 text-text-secondary/40 font-mono">{o.id}</td>
              <td className="px-3 py-1.5 font-semibold">{o.pair}</td>
              <td className={`px-3 py-1.5 font-bold text-[8px] ${o.type.includes("BUY") || o.type.includes("PROFIT") ? "text-profit" : "text-loss"}`}>{o.type}</td>
              <td className="px-3 py-1.5 text-right font-medium">{o.price}</td>
              <td className="px-3 py-1.5 text-right text-text-secondary/50">{o.size}</td>
              <td className={`px-3 py-1.5 text-right font-bold text-[8px] ${o.status === "ARMED" ? "text-warning" : "text-cyan/60"}`}>{o.status}</td>
            </motion.tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function AgentsTab() {
  return (
    <div className="flex-1 min-h-0 overflow-hidden flex flex-col">
      <div className="px-3 py-2 flex items-center justify-between border-b border-border/20">
        <span className="text-[9px] font-bold tracking-[1px] text-text-secondary/40">AI TRADING AGENTS</span>
        <span className="text-[9px] font-bold text-profit/60">4 RUNNING</span>
      </div>
      <table className="w-full text-[9px]">
        <thead>
          <tr className="text-[8px] text-text-secondary/30 font-bold tracking-[0.5px] border-b border-border/20">
            <th className="px-3 py-1.5 text-left">AGENT</th>
            <th className="px-3 py-1.5 text-left">STATUS</th>
            <th className="px-3 py-1.5 text-right">TRADES</th>
            <th className="px-3 py-1.5 text-right">WIN %</th>
            <th className="px-3 py-1.5 text-right">P&L</th>
            <th className="px-3 py-1.5 text-right">PAIRS</th>
          </tr>
        </thead>
        <tbody>
          {agentsDetailed.map((a, i) => (
            <motion.tr key={a.name} initial={{ opacity: 0, x: -10 }} animate={{ opacity: 1, x: 0 }} transition={{ duration: 0.2, delay: i * 0.05 }} className="border-b border-border/10">
              <td className="px-3 py-1.5 font-semibold">{a.name}</td>
              <td className="px-3 py-1.5">
                <span className={`inline-flex items-center gap-1 text-[8px] font-bold ${a.status === "RUNNING" ? "text-profit" : a.status === "PAUSED" ? "text-warning" : "text-text-secondary/40"}`}>
                  <span className={`h-1.5 w-1.5 rounded-full ${a.status === "RUNNING" ? "bg-profit live-dot" : a.status === "PAUSED" ? "bg-warning" : "bg-text-secondary/30"}`} />
                  {a.status}
                </span>
              </td>
              <td className="px-3 py-1.5 text-right text-text-secondary/60">{a.trades}</td>
              <td className={`px-3 py-1.5 text-right font-medium ${a.winRate !== "—" ? "text-foreground" : "text-text-secondary/30"}`}>{a.winRate}</td>
              <td className={`px-3 py-1.5 text-right font-semibold ${a.pnl.startsWith("+") ? "text-profit" : a.pnl === "—" ? "text-text-secondary/30" : "text-loss"}`}>{a.pnl}</td>
              <td className="px-3 py-1.5 text-right text-text-secondary/40 text-[8px]">{a.pairs}</td>
            </motion.tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function SignalsTab() {
  return (
    <div className="flex-1 min-h-0 overflow-hidden flex flex-col">
      <div className="px-3 py-2 flex items-center justify-between border-b border-border/20">
        <span className="text-[9px] font-bold tracking-[1px] text-text-secondary/40">SIGNAL FEED — LIVE</span>
        <div className="flex items-center gap-1.5">
          <span className="h-1.5 w-1.5 rounded-full bg-profit live-dot" />
          <span className="text-[8px] font-bold text-profit/70">STREAMING</span>
        </div>
      </div>
      <table className="w-full text-[9px]">
        <thead>
          <tr className="text-[8px] text-text-secondary/30 font-bold tracking-[0.5px] border-b border-border/20">
            <th className="px-3 py-1.5 text-left">TIME</th>
            <th className="px-3 py-1.5 text-left">PAIR</th>
            <th className="px-3 py-1.5 text-left">SIGNAL</th>
            <th className="px-3 py-1.5 text-right">STR</th>
            <th className="px-3 py-1.5 text-right">SOURCE</th>
            <th className="px-3 py-1.5 text-right">CONF</th>
          </tr>
        </thead>
        <tbody>
          {signals.map((s, i) => (
            <motion.tr key={s.time} initial={{ opacity: 0, x: -10 }} animate={{ opacity: 1, x: 0 }} transition={{ duration: 0.2, delay: i * 0.04 }} className="border-b border-border/10">
              <td className="px-3 py-1.5 text-text-secondary/40 font-mono">{s.time}</td>
              <td className="px-3 py-1.5 font-semibold">{s.pair}</td>
              <td className={`px-3 py-1.5 font-bold text-[8px] ${s.signal === "BUY" ? "text-profit" : s.signal === "SELL" ? "text-loss" : "text-text-secondary/50"}`}>{s.signal}</td>
              <td className="px-3 py-1.5 text-right">
                <span className={`text-[8px] font-bold ${s.strength >= 80 ? "text-profit" : s.strength >= 60 ? "text-warning" : "text-text-secondary/40"}`}>{s.strength}%</span>
              </td>
              <td className="px-3 py-1.5 text-right text-text-secondary/50">{s.source}</td>
              <td className={`px-3 py-1.5 text-right text-[8px] font-bold ${s.conf === "HIGH" ? "text-cyan" : s.conf === "MED" ? "text-warning/70" : "text-text-secondary/40"}`}>{s.conf}</td>
            </motion.tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/* ═══════════════════════════════════════════════
   MAIN HERO
   ═══════════════════════════════════════════════ */

export function Hero() {
  const [activeTab, setActiveTab] = useState<TabKey>("Dashboard");
  const [isPaused, setIsPaused] = useState(false);

  /* Auto-cycle tabs every 4s */
  const nextTab = useCallback(() => {
    setActiveTab((prev) => {
      const idx = sidebarItems.indexOf(prev);
      return sidebarItems[(idx + 1) % sidebarItems.length];
    });
  }, []);

  useEffect(() => {
    if (isPaused) return;
    const id = setInterval(nextTab, 4000);
    return () => clearInterval(id);
  }, [isPaused, nextTab]);

  const tabContent: Record<TabKey, React.ReactNode> = {
    Dashboard: <DashboardTab />,
    Markets: <MarketsTab />,
    Positions: <PositionsTab />,
    Orders: <OrdersTab />,
    Agents: <AgentsTab />,
    Signals: <SignalsTab />,
  };

  return (
    <section className="relative overflow-hidden pb-12">
      {/* Side mesh gradients */}
      <div className="pointer-events-none absolute inset-0">
        <div className="absolute left-0 top-0 h-full w-[30%]">
          <div className="absolute inset-0 bg-gradient-to-r from-cyan/[0.04] via-[#9b59b6]/[0.03] to-transparent" />
          <div className="absolute left-[10%] top-[20%] h-[400px] w-[400px] rounded-full bg-[#9b59b6] opacity-[0.06] blur-[150px]" />
          <div className="absolute left-[5%] bottom-[30%] h-[300px] w-[300px] rounded-full bg-cyan opacity-[0.05] blur-[120px]" />
        </div>
        <div className="absolute right-0 top-0 h-full w-[30%]">
          <div className="absolute inset-0 bg-gradient-to-l from-cyan/[0.04] via-[#9b59b6]/[0.03] to-transparent" />
          <div className="absolute right-[10%] top-[30%] h-[400px] w-[400px] rounded-full bg-cyan opacity-[0.06] blur-[150px]" />
          <div className="absolute right-[5%] bottom-[20%] h-[300px] w-[300px] rounded-full bg-[#9b59b6] opacity-[0.05] blur-[120px]" />
        </div>
        <div className="absolute left-[40%] top-[5%] h-[500px] w-[500px] rounded-full bg-cyan opacity-[0.04] blur-[200px]" />
      </div>

      {/* Grid lines on sides */}
      <div className="pointer-events-none absolute inset-0 overflow-hidden">
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 0.03 }}
          transition={{ duration: 2, delay: 0.5 }}
          className="absolute left-0 top-0 h-full w-[25%]"
          style={{
            backgroundImage: "linear-gradient(to right, var(--cyan) 1px, transparent 1px), linear-gradient(to bottom, var(--cyan) 1px, transparent 1px)",
            backgroundSize: "60px 60px",
          }}
        />
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 0.03 }}
          transition={{ duration: 2, delay: 0.5 }}
          className="absolute right-0 top-0 h-full w-[25%]"
          style={{
            backgroundImage: "linear-gradient(to right, var(--cyan) 1px, transparent 1px), linear-gradient(to bottom, var(--cyan) 1px, transparent 1px)",
            backgroundSize: "60px 60px",
          }}
        />
      </div>

      {/* Image — moved up 45% */}
      <motion.div
        initial={{ opacity: 0, scale: 1.02 }}
        animate={{ opacity: 1, scale: 1 }}
        transition={{ duration: 1.2, ease: "easeOut" }}
        className="relative mx-auto max-w-[1100px]"
      >
        <div className="overflow-hidden max-h-[600px]">
          <img
            src="/images/landing.png"
            alt="HybridTrade dashboard preview"
            className="w-full object-cover"
            style={{ objectPosition: "center 5%" }}
          />
        </div>
        <div className="pointer-events-none absolute inset-x-0 bottom-0 h-[20%] bg-gradient-to-t from-background to-transparent" />
      </motion.div>

      {/* Hero text — lowered */}
      <div className="relative z-10 mx-auto max-w-[1100px] px-6">
        <motion.div
          initial={{ opacity: 0, y: 30 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.8, delay: 0.4, ease: "easeOut" }}
          className="relative -mt-8 text-center"
        >
          <h1 className="text-[52px] font-bold leading-[1.05] tracking-[-2px] md:text-[72px]">
            <motion.span initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.6, delay: 0.5 }} className="inline-block">
              TRADE SMARTER.
            </motion.span>
            <br />
            <motion.span initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.6, delay: 0.7 }} className="inline-block text-cyan">
              TRADE HYBRID.
            </motion.span>
          </h1>

          <motion.p
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 0.8, delay: 0.9 }}
            className="mx-auto mt-5 max-w-[520px] text-[13px] leading-[1.9] tracking-[0.3px] text-text-secondary/70"
          >
            AI-powered agents across forex and crypto markets.
            Real-time signals, automated execution, zero latency.
          </motion.p>

          <motion.div initial={{ opacity: 0, y: 15 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.6, delay: 1.1 }} className="mt-8 flex items-center justify-center gap-4">
            <Link href="/dashboard" className="glow-cyan bg-cyan px-8 py-3 text-[12px] font-bold tracking-[1px] text-black transition-all hover:bg-cyan/90">
              START TRADING →
            </Link>
            <Link href="/dashboard" className="border border-border/60 bg-background/40 px-8 py-3 text-[12px] font-bold tracking-[1px] transition-colors hover:bg-card">
              OPEN DASHBOARD →
            </Link>
          </motion.div>

        </motion.div>
      </div>

      {/* ═══ Live Dashboard Preview Panel ═══ */}
      <motion.div
        initial={{ opacity: 0, y: 50 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 1, delay: 1.8 }}
        className="relative z-10 mx-auto mt-16 max-w-[1000px] px-6"
        onMouseEnter={() => setIsPaused(true)}
        onMouseLeave={() => setIsPaused(false)}
      >
        <div className="relative border border-border/60 bg-card/90 overflow-hidden">
          {/* ── Window chrome ── */}
          <div className="flex items-center justify-between border-b border-border/40 px-4 py-2">
            <div className="flex items-center gap-3">
              <div className="flex items-center gap-1.5">
                <div className="h-2.5 w-2.5 rounded-full bg-loss/60" />
                <div className="h-2.5 w-2.5 rounded-full bg-warning/60" />
                <div className="h-2.5 w-2.5 rounded-full bg-profit/60" />
              </div>
              <span className="text-[10px] font-bold tracking-[1px] text-text-secondary/40">
                HYBRIDTRADE — {activeTab.toUpperCase()}
              </span>
            </div>
            <div className="flex items-center gap-3">
              <div className="flex items-center gap-1.5">
                <span className="h-1.5 w-1.5 rounded-full bg-profit live-dot" />
                <span className="text-[9px] font-bold tracking-[1px] text-profit/70">LIVE</span>
              </div>
              <motion.span key={activeTab} initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="text-[9px] text-text-secondary/30">
                {tickerMap[activeTab]}
              </motion.span>
            </div>
          </div>

          {/* ── Dashboard body ── */}
          <div className="flex" style={{ height: 380 }}>
            {/* Sidebar with animated active state */}
            <div className="w-[140px] shrink-0 border-r border-border/30 bg-panel/50 py-3 px-2.5">
              <div className="flex items-center gap-1.5 px-2 mb-4">
                <div className="h-2 w-2 rounded-full bg-cyan" />
                <span className="text-[10px] font-bold tracking-[0.5px]">HT</span>
              </div>
              {sidebarItems.map((item) => (
                <button
                  key={item}
                  onClick={() => { setActiveTab(item); setIsPaused(true); }}
                  className={`relative w-full text-left px-2 py-1.5 mb-0.5 text-[10px] transition-all duration-300 ${
                    item === activeTab
                      ? "text-cyan font-semibold"
                      : "text-text-secondary/50 hover:text-text-secondary/70"
                  }`}
                >
                  {item === activeTab && (
                    <motion.div
                      layoutId="sidebar-active"
                      className="absolute inset-0 bg-cyan-dim"
                      transition={{ type: "spring", stiffness: 350, damping: 30 }}
                    />
                  )}
                  <span className="relative z-10">{item}</span>
                  {item === activeTab && (
                    <motion.div
                      layoutId="sidebar-bar"
                      className="absolute left-0 top-0 bottom-0 w-[3px] bg-cyan"
                      transition={{ type: "spring", stiffness: 350, damping: 30 }}
                    />
                  )}
                </button>
              ))}

              {/* Mini progress bar showing auto-cycle timer */}
              {!isPaused && (
                <div className="mt-4 px-2">
                  <div className="h-[2px] bg-border/20 overflow-hidden">
                    <motion.div
                      key={activeTab}
                      initial={{ width: "0%" }}
                      animate={{ width: "100%" }}
                      transition={{ duration: 4, ease: "linear" }}
                      className="h-full bg-cyan/40"
                    />
                  </div>
                  <span className="text-[7px] text-text-secondary/30 mt-1 block">AUTO-DEMO</span>
                </div>
              )}
            </div>

            {/* Main content area with tab switching */}
            <div className="flex-1 min-w-0 flex flex-col">
              {/* KPI row — animated per tab */}
              <div className="grid grid-cols-4 gap-px bg-border/20 border-b border-border/30">
                <AnimatePresence mode="popLayout">
                  {kpiSets[activeTab].map((kpi) => (
                    <motion.div
                      key={`${activeTab}-${kpi.label}`}
                      initial={{ opacity: 0, y: 8 }}
                      animate={{ opacity: 1, y: 0 }}
                      exit={{ opacity: 0, y: -8 }}
                      transition={{ duration: 0.25 }}
                      className="bg-card/80 px-3 py-2.5"
                    >
                      <div className="text-[8px] font-bold tracking-[1px] text-text-secondary/40">{kpi.label}</div>
                      <div className={`text-[16px] font-bold mt-0.5 ${kpi.color}`}>{kpi.value}</div>
                      <div className="text-[9px] text-text-secondary/40 mt-0.5">{kpi.sub}</div>
                    </motion.div>
                  ))}
                </AnimatePresence>
              </div>

              {/* Tab content */}
              <AnimatePresence mode="wait">
                <motion.div
                  key={activeTab}
                  initial={{ opacity: 0, y: 12 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: -12 }}
                  transition={{ duration: 0.3 }}
                  className="flex-1 min-h-0 flex flex-col"
                >
                  {tabContent[activeTab]}
                </motion.div>
              </AnimatePresence>
            </div>

            {/* Right panel — watchlist + agents always visible */}
            <div className="w-[180px] shrink-0 border-l border-border/30 flex flex-col">
              <div className="flex-1">
                <div className="px-3 py-2 border-b border-border/20">
                  <span className="text-[8px] font-bold tracking-[1px] text-text-secondary/40">WATCHLIST</span>
                </div>
                {watchlist.map((item) => (
                  <div key={item.pair} className="flex items-center justify-between px-3 py-1.5 border-b border-border/10">
                    <div className="flex items-center gap-1.5">
                      <span className={`h-1 w-1 rounded-full ${item.up ? "bg-cyan" : "bg-loss"}`} />
                      <span className="text-[9px] font-semibold">{item.pair}</span>
                    </div>
                    <div className="text-right">
                      <div className="text-[9px] font-medium">{item.price}</div>
                      <div className={`text-[8px] font-bold ${item.up ? "text-profit" : "text-loss"}`}>{item.change}</div>
                    </div>
                  </div>
                ))}
              </div>

              <div>
                <div className="px-3 py-2 border-t border-border/30 border-b border-border/20">
                  <span className="text-[8px] font-bold tracking-[1px] text-text-secondary/40">AGENTS</span>
                </div>
                {agentsDetailed.slice(0, 4).map((agent) => (
                  <div key={agent.name} className="flex items-center justify-between px-3 py-1.5 border-b border-border/10">
                    <span className="text-[9px] text-text-secondary/60">{agent.name}</span>
                    <span className={`text-[8px] font-bold ${agent.status === "RUNNING" ? "text-profit" : agent.status === "PAUSED" ? "text-warning" : "text-text-secondary/40"}`}>
                      {agent.status}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          </div>

          {/* Bottom status bar */}
          <div className="flex items-center justify-between border-t border-border/40 px-4 py-1.5 text-[8px] bg-panel/30">
            <div className="flex items-center gap-3">
              <span className="text-cyan/50">4 AGENTS ACTIVE</span>
              <span className="text-text-secondary/20">|</span>
              <span className="text-profit/50">+$2,342.50 TODAY</span>
              <span className="text-text-secondary/20">|</span>
              <span className="text-text-secondary/30">52 PAIRS</span>
            </div>
            <div className="flex items-center gap-3">
              <span className="text-text-secondary/30">0.08ms</span>
              <span className="text-text-secondary/20">|</span>
              <span className="text-text-secondary/30">99.99% UPTIME</span>
            </div>
          </div>
        </div>

        {/* Glow under dashboard */}
        <div className="pointer-events-none absolute -bottom-10 left-1/2 -translate-x-1/2 h-[100px] w-[80%] bg-cyan/[0.06] blur-[60px]" />
      </motion.div>
    </section>
  );
}
