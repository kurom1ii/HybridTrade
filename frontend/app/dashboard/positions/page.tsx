"use client";
import { motion } from "motion/react";
import { StaggerGrid, SlideIn } from "@/components/dashboard/motion-primitives";
import { StatsCard } from "@/components/dashboard/stats-card";
import { PageTitle } from "@/components/dashboard/page-title";

const positions = [
 { pair: "EUR/USD", type: "BUY", entry: "1.0872", current: "1.0891", sl: "1.0840", tp: "1.0950", pnl: "+$234.50", pnlPct: "+0.17%", pnlType: "profit" as const, size: "1.5 Lot", margin: "$1,630" },
 { pair: "GBP/JPY", type: "SELL", entry: "189.234", current: "189.012", sl: "189.800", tp: "188.200", pnl: "+$178.20", pnlPct: "+0.12%", pnlType: "profit" as const, size: "0.8 Lot", margin: "$1,140" },
 { pair: "BTC/USD", type: "BUY", entry: "43,250", current: "43,890", sl: "42,500", tp: "45,000", pnl: "+$640.00", pnlPct: "+1.48%", pnlType: "profit" as const, size: "0.1 BTC", margin: "$4,325" },
 { pair: "XAU/USD", type: "SELL", entry: "2,034.5", current: "2,041.2", sl: "2,050.0", tp: "2,010.0", pnl: "-$67.00", pnlPct: "-0.33%", pnlType: "loss" as const, size: "0.5 Lot", margin: "$1,017" },
 { pair: "GBP/JPY", type: "BUY", entry: "188.450", current: "188.120", sl: "187.900", tp: "189.500", pnl: "-$264.00", pnlPct: "-0.18%", pnlType: "loss" as const, size: "0.8 Lot", margin: "$1,130" },
 { pair: "ETH/USD", type: "BUY", entry: "2,245", current: "2,287", sl: "2,200", tp: "2,400", pnl: "+$420.00", pnlPct: "+1.87%", pnlType: "profit" as const, size: "1.0 ETH", margin: "$2,245" },
];

export default function PositionsPage() {
 return (
 <div className="flex gap-6 h-full overflow-y-auto p-6">
 <div className="flex-1 min-w-0 space-y-6">
 <PageTitle title="Positions" subtitle="Manage your open and closed positions" breadcrumb="POSITIONS / OPEN TRADES" />

 {/* Stats Row */}
 <StaggerGrid className="grid grid-cols-4 gap-4">
 <StatsCard title="Total P/L" value="+$1,141.70" change="+0.92% portfolio" changeType="profit" />
 <StatsCard title="Open Positions" value="6" change="4 profitable" changeType="neutral" />
 <StatsCard title="Today's P/L" value="+$985.70" change="+0.79%" changeType="profit" />
 <StatsCard title="Margin Used" value="$11,487" change="9.2% of equity" changeType="neutral" />
 </StaggerGrid>

 {/* Positions Table */}
 <motion.div initial={{ opacity: 0, y: 12 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.5, delay: 0.2 }}>
 <div className=" border border-border bg-card">
 <div className="flex items-center justify-between border-b border-border px-4 py-3">
 <h3 className="text-sm font-semibold">Open Positions</h3>
 <div className="flex gap-1">
 <button className="px-3 py-1 text-[11px] font-medium bg-cyan/10 text-cyan">ALL</button>
 <button className="px-3 py-1 text-[11px] font-medium text-muted-foreground hover:bg-secondary">PROFIT</button>
 <button className="px-3 py-1 text-[11px] font-medium text-muted-foreground hover:bg-secondary">LOSS</button>
 </div>
 </div>
 <div className="overflow-x-auto">
 <table className="w-full text-[13px]">
 <thead>
 <tr className="border-b border-border text-[11px] uppercase tracking-wider text-muted-foreground">
 <th className="px-4 py-3 text-left font-medium">Pair</th>
 <th className="px-4 py-3 text-left font-medium">Type</th>
 <th className="px-4 py-3 text-right font-medium">Size</th>
 <th className="px-4 py-3 text-right font-medium">Entry</th>
 <th className="px-4 py-3 text-right font-medium">Current</th>
 <th className="px-4 py-3 text-right font-medium">S/L</th>
 <th className="px-4 py-3 text-right font-medium">T/P</th>
 <th className="px-4 py-3 text-right font-medium">P/L</th>
 <th className="px-4 py-3 text-right font-medium">Margin</th>
 </tr>
 </thead>
 <tbody>
 {positions.map((pos, i) => (
 <motion.tr key={i} initial={{ opacity: 0, x: -8 }} animate={{ opacity: 1, x: 0 }} transition={{ duration: 0.3, delay: 0.1 + i * 0.04 }} className="border-b border-border/50 transition-colors hover:bg-card-alt cursor-pointer">
 <td className="px-4 py-3 font-semibold">{pos.pair}</td>
 <td className="px-4 py-3">
 <span className={`px-2 py-0.5 text-[11px] font-semibold ${
 pos.type === "BUY" ? "bg-profit/10 text-profit" : "bg-loss/10 text-loss"
 }`}>
 {pos.type}
 </span>
 </td>
 <td className="px-4 py-3 text-right text-muted-foreground">{pos.size}</td>
 <td className="px-4 py-3 text-right text-muted-foreground">{pos.entry}</td>
 <td className="px-4 py-3 text-right">{pos.current}</td>
 <td className="px-4 py-3 text-right text-muted-foreground">{pos.sl}</td>
 <td className="px-4 py-3 text-right text-muted-foreground">{pos.tp}</td>
 <td className="px-4 py-3 text-right">
 <div className={`font-semibold ${pos.pnlType === "profit" ? "text-profit" : "text-loss"}`}>
 {pos.pnl}
 </div>
 <div className={`text-[11px] ${pos.pnlType === "profit" ? "text-profit" : "text-loss"}`}>
 {pos.pnlPct}
 </div>
 </td>
 <td className="px-4 py-3 text-right text-muted-foreground">{pos.margin}</td>
 </motion.tr>
 ))}
 </tbody>
 </table>
 </div>
 </div>
 </motion.div>
 </div>

 {/* Position Details Panel */}
 <SlideIn direction="right" delay={0.3}>
 <div className="w-[320px] shrink-0">
 <div className=" border border-border bg-card">
 <div className="border-b border-border px-4 py-3">
 <h3 className="text-sm font-semibold">Position Details</h3>
 </div>
 <div className="p-4 space-y-4">
 <div className="flex items-center justify-between">
 <span className="text-lg font-bold">EUR/USD</span>
 <span className="bg-profit/10 px-2 py-0.5 text-[11px] font-semibold text-profit">BUY</span>
 </div>

 <div className="space-y-3">
 {[
 ["Entry Price", "1.0872"],
 ["Current Price", "1.0891"],
 ["Stop Loss", "1.0840"],
 ["Take Profit", "1.0950"],
 ["Lot Size", "1.5"],
 ["Margin", "$1,630"],
 ["Swap", "-$2.40"],
 ["Commission", "-$4.50"],
 ].map(([label, value]) => (
 <div key={label} className="flex items-center justify-between text-[13px]">
 <span className="text-muted-foreground">{label}</span>
 <span className="font-medium">{value}</span>
 </div>
 ))}
 </div>

 <div className=" bg-profit/5 border border-profit/20 p-3 text-center">
 <div className="text-[11px] text-muted-foreground">Unrealized P/L</div>
 <div className="text-xl font-bold text-profit">+$234.50</div>
 <div className="text-[12px] text-profit">+0.17%</div>
 </div>

 <div className="flex gap-2">
 <button className="flex-1 bg-secondary py-2 text-[12px] font-semibold transition-colors hover:bg-secondary/80">
 Modify
 </button>
 <button className="flex-1 bg-loss/10 py-2 text-[12px] font-semibold text-loss transition-colors hover:bg-loss/20">
 Close
 </button>
 </div>
 </div>
 </div>
 </div>
 </SlideIn>
 </div>
 );
}
