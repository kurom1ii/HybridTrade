"use client";
import { motion } from "motion/react";
import { StaggerGrid, SlideIn } from "@/components/dashboard/motion-primitives";
import { StatsCard } from "@/components/dashboard/stats-card";
import { PageTitle } from "@/components/dashboard/page-title";

const filterTabs = ["ALL", "BUY", "SELL", "STRONG"];

const signals = [
 {
 pair: "EUR/USD",
 direction: "BUY",
 strength: "Strong",
 entry: "1.0875",
 sl: "1.0840",
 tp: "1.0950",
 confidence: "87%",
 agent: "Momentum Scanner",
 time: "5m ago",
 },
 {
 pair: "GBP/JPY",
 direction: "SELL",
 strength: "Medium",
 entry: "189.200",
 sl: "189.600",
 tp: "188.400",
 confidence: "72%",
 agent: "Trend Follower",
 time: "12m ago",
 },
 {
 pair: "BTC/USD",
 direction: "BUY",
 strength: "Strong",
 entry: "43,800",
 sl: "43,200",
 tp: "45,000",
 confidence: "81%",
 agent: "News Analyzer",
 time: "18m ago",
 },
 {
 pair: "XAU/USD",
 direction: "SELL",
 strength: "Weak",
 entry: "2,042",
 sl: "2,055",
 tp: "2,020",
 confidence: "58%",
 agent: "Sentiment Bot",
 time: "25m ago",
 },
 {
 pair: "USD/JPY",
 direction: "BUY",
 strength: "Medium",
 entry: "149.70",
 sl: "149.20",
 tp: "150.60",
 confidence: "74%",
 agent: "Momentum Scanner",
 time: "32m ago",
 },
 {
 pair: "ETH/USD",
 direction: "BUY",
 strength: "Strong",
 entry: "2,280",
 sl: "2,230",
 tp: "2,400",
 confidence: "85%",
 agent: "Trend Follower",
 time: "45m ago",
 },
];

export default function SignalsPage() {
 return (
 <div className="flex gap-6 h-full overflow-y-auto p-6">
 <div className="flex-1 min-w-0 space-y-6">
 <PageTitle title="Trading Signals" subtitle="AI-generated trading signals and recommendations" breadcrumb="SIGNALS / AI SIGNALS" />

 {/* Filter Tabs */}
 <div className="flex gap-1">
 {filterTabs.map((tab) => (
 <button
 key={tab}
 className={` px-4 py-1.5 text-[12px] font-semibold tracking-wider transition-colors ${
 tab === "ALL"
 ? "bg-cyan/10 text-cyan"
 : "text-muted-foreground hover:bg-secondary"
 }`}
 >
 {tab}
 </button>
 ))}
 </div>

 {/* Stats Row */}
 <StaggerGrid className="grid grid-cols-4 gap-4">
 <StatsCard title="Active Signals" value="6" change="3 strong" changeType="profit" />
 <StatsCard title="Accuracy (30d)" value="74%" change="+5% vs last month" changeType="profit" />
 <StatsCard title="Avg Profit/Signal" value="+$124" change="Per executed signal" changeType="profit" />
 <StatsCard title="Today's Signals" value="14" change="8 executed" changeType="neutral" />
 </StaggerGrid>

 {/* Signal Cards */}
 <motion.div className="space-y-3" initial={{ opacity: 0, y: 12 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.5, delay: 0.2 }}>
 {signals.map((signal, i) => (
 <div key={i} className=" border border-border bg-card p-4 transition-colors hover:border-cyan/30">
 <div className="flex items-center justify-between">
 <div className="flex items-center gap-3">
 <span className="text-[16px] font-bold">{signal.pair}</span>
 <span className={`px-2 py-0.5 text-[11px] font-semibold ${
 signal.direction === "BUY" ? "bg-profit/10 text-profit" : "bg-loss/10 text-loss"
 }`}>
 {signal.direction}
 </span>
 <span className={`rounded-full px-2 py-0.5 text-[10px] font-medium ${
 signal.strength === "Strong"
 ? "bg-profit/10 text-profit"
 : signal.strength === "Medium"
 ? "bg-cyan/10 text-cyan"
 : "bg-muted text-muted-foreground"
 }`}>
 {signal.strength}
 </span>
 </div>
 <div className="text-right">
 <div className="text-[11px] text-muted-foreground">{signal.agent}</div>
 <div className="text-[10px] text-muted-foreground">{signal.time}</div>
 </div>
 </div>
 <div className="mt-3 flex gap-6 text-[12px]">
 <div>
 <span className="text-muted-foreground">Entry: </span>
 <span className="font-semibold">{signal.entry}</span>
 </div>
 <div>
 <span className="text-muted-foreground">S/L: </span>
 <span className="font-semibold text-loss">{signal.sl}</span>
 </div>
 <div>
 <span className="text-muted-foreground">T/P: </span>
 <span className="font-semibold text-profit">{signal.tp}</span>
 </div>
 <div>
 <span className="text-muted-foreground">Confidence: </span>
 <span className="font-semibold text-cyan">{signal.confidence}</span>
 </div>
 </div>
 </div>
 ))}
 </motion.div>
 </div>

 {/* Right Panel */}
 <SlideIn direction="right" delay={0.3}>
 <div className="w-[300px] shrink-0 space-y-4">
 {/* Performance */}
 <div className=" border border-border bg-card p-4">
 <h3 className="text-sm font-semibold mb-4">Signal Performance</h3>
 <div className="text-center mb-4">
 <div className="text-4xl font-bold text-cyan">74%</div>
 <div className="text-[12px] text-muted-foreground mt-1">Overall Accuracy</div>
 </div>
 <div className="space-y-3">
 {[
 ["Signals Generated", "342"],
 ["Signals Executed", "198"],
 ["Profitable", "147"],
 ["Unprofitable", "51"],
 ["Avg R:R", "1:2.4"],
 ["Total P/L", "+$24,560"],
 ].map(([label, value]) => (
 <div key={label} className="flex items-center justify-between text-[12px]">
 <span className="text-muted-foreground">{label}</span>
 <span className="font-semibold">{value}</span>
 </div>
 ))}
 </div>
 </div>

 {/* By Agent */}
 <div className=" border border-border bg-card p-4">
 <h3 className="text-sm font-semibold mb-3">Accuracy by Agent</h3>
 <div className="space-y-2">
 {[
 ["Momentum Scanner", "78%"],
 ["Trend Follower", "71%"],
 ["News Analyzer", "74%"],
 ["Sentiment Bot", "68%"],
 ].map(([agent, accuracy]) => (
 <div key={agent} className="flex items-center justify-between text-[12px]">
 <span className="text-muted-foreground">{agent}</span>
 <span className="font-semibold text-cyan">{accuracy}</span>
 </div>
 ))}
 </div>
 </div>
 </div>
 </SlideIn>
 </div>
 );
}
