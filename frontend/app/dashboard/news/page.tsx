"use client";
import { motion } from "motion/react";
import { StaggerGrid, SlideIn } from "@/components/dashboard/motion-primitives";
import { StatsCard } from "@/components/dashboard/stats-card";
import { PageTitle } from "@/components/dashboard/page-title";

const filterTabs = ["BREAKING", "ANALYSIS", "MARKET", "EARNINGS", "POLITICS"];

const articles = [
 {
 title: "Federal Reserve Minutes Signal Potential Rate Pause in Q2 2024",
 summary: "The latest FOMC minutes reveal growing consensus among policymakers for a pause in rate hikes, citing improving inflation data and labor market cooling.",
 source: "Reuters",
 time: "12m ago",
 category: "Breaking",
 impact: "High",
 },
 {
 title: "EUR/USD Technical Analysis: Key Resistance at 1.0900",
 summary: "The pair approaches a critical resistance zone with RSI showing bullish divergence on the 4H timeframe.",
 source: "FXStreet",
 time: "28m ago",
 category: "Analysis",
 impact: "Medium",
 },
 {
 title: "Bitcoin ETF Records $890M in Daily Inflows",
 summary: "Institutional adoption continues to accelerate as spot Bitcoin ETFs see another record-breaking day of net inflows.",
 source: "Bloomberg",
 time: "1h ago",
 category: "Market",
 impact: "High",
 },
 {
 title: "UK GDP Growth Beats Expectations at 0.3% Q/Q",
 summary: "British economy shows resilience with stronger-than-expected quarterly growth, supporting GBP strength.",
 source: "BBC",
 time: "2h ago",
 category: "Market",
 impact: "Medium",
 },
 {
 title: "Gold Prices Consolidate Near $2,040 Ahead of CPI Data",
 summary: "Precious metals trade in a narrow range as markets await tomorrow's US Consumer Price Index report.",
 source: "Kitco",
 time: "3h ago",
 category: "Market",
 impact: "Medium",
 },
 {
 title: "OPEC+ Considering Additional Production Cuts",
 summary: "Saudi Arabia leads discussions for deeper output reductions amid concerns over global demand outlook.",
 source: "Reuters",
 time: "4h ago",
 category: "Market",
 impact: "High",
 },
];

const calendarEvents = [
 { time: "08:30", event: "US CPI m/m", impact: "High", forecast: "0.2%", previous: "0.3%" },
 { time: "10:00", event: "US Consumer Sentiment", impact: "Medium", forecast: "69.4", previous: "69.7" },
 { time: "13:00", event: "US 30-y Bond Auction", impact: "Medium", forecast: "—", previous: "4.34%" },
 { time: "14:30", event: "ECB Press Conference", impact: "High", forecast: "—", previous: "—" },
];

const trendingTopics = [
 "Fed Rate Decision", "Bitcoin ETF", "EUR/USD", "Oil Prices",
 "UK GDP", "Gold", "Japan CPI", "OPEC+",
 "Tech Earnings", "China PMI", "US Jobs", "ECB Policy",
];

export default function NewsPage() {
 return (
 <div className="flex gap-6 h-full overflow-y-auto p-6">
 <div className="flex-1 min-w-0 space-y-6">
 <PageTitle title="Market News" subtitle="Real-time news and market analysis" breadcrumb="NEWS / MARKET FEED" />

 {/* Filter Tabs */}
 <div className="flex gap-1">
 {filterTabs.map((tab) => (
 <button
 key={tab}
 className={` px-4 py-1.5 text-[12px] font-semibold tracking-wider transition-colors ${
 tab === "BREAKING"
 ? "bg-cyan/10 text-cyan"
 : "text-muted-foreground hover:bg-secondary"
 }`}
 >
 {tab}
 </button>
 ))}
 </div>

 {/* Stats Row */}
 <StaggerGrid className="grid grid-cols-3 gap-4">
 <StatsCard title="Breaking News" value="3" change="High impact" changeType="neutral" />
 <StatsCard title="Articles Today" value="47" change="+12 vs yesterday" changeType="neutral" />
 <StatsCard title="Market Sentiment" value="Bullish" change="62% positive" changeType="profit" />
 </StaggerGrid>

 {/* Articles */}
 <motion.div className="space-y-3" initial={{ opacity: 0, y: 12 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.5, delay: 0.2 }}>
 {articles.map((article, i) => (
 <div key={i} className=" border border-border bg-card p-4 transition-colors hover:border-cyan/30 cursor-pointer">
 <div className="flex items-start justify-between gap-4">
 <div className="flex-1">
 <div className="flex items-center gap-2 mb-1">
 <span className={`rounded-full px-2 py-0.5 text-[10px] font-medium ${
 article.impact === "High"
 ? "bg-loss/10 text-loss"
 : "bg-cyan/10 text-cyan"
 }`}>
 {article.impact} Impact
 </span>
 <span className="text-[10px] text-muted-foreground">{article.category}</span>
 </div>
 <h3 className="text-[14px] font-semibold leading-snug">{article.title}</h3>
 <p className="mt-1 text-[12px] text-muted-foreground leading-relaxed">{article.summary}</p>
 <div className="mt-2 flex items-center gap-2 text-[11px] text-muted-foreground">
 <span className="font-medium">{article.source}</span>
 <span>·</span>
 <span>{article.time}</span>
 </div>
 </div>
 </div>
 </div>
 ))}
 </motion.div>
 </div>

 {/* Right Sidebar */}
 <SlideIn direction="right" delay={0.3}>
 <div className="w-[300px] shrink-0 space-y-4">
 {/* Economic Calendar */}
 <div className=" border border-border bg-card">
 <div className="border-b border-border px-4 py-3">
 <h3 className="text-sm font-semibold">Economic Calendar</h3>
 </div>
 <div className="divide-y divide-border/50">
 {calendarEvents.map((event, i) => (
 <div key={i} className="px-4 py-2.5">
 <div className="flex items-center justify-between">
 <span className="text-[11px] text-muted-foreground">{event.time}</span>
 <span className={`rounded-full px-1.5 py-0.5 text-[9px] font-medium ${
 event.impact === "High"
 ? "bg-loss/10 text-loss"
 : "bg-cyan/10 text-cyan"
 }`}>
 {event.impact}
 </span>
 </div>
 <div className="text-[12px] font-medium mt-0.5">{event.event}</div>
 <div className="flex gap-3 mt-1 text-[10px] text-muted-foreground">
 <span>F: {event.forecast}</span>
 <span>P: {event.previous}</span>
 </div>
 </div>
 ))}
 </div>
 </div>

 {/* Trending Topics */}
 <div className=" border border-border bg-card p-4">
 <h3 className="text-sm font-semibold mb-3">Trending Topics</h3>
 <div className="flex flex-wrap gap-2">
 {trendingTopics.map((topic, i) => (
 <span
 key={i}
 className="rounded-full border border-border px-3 py-1 text-[11px] text-muted-foreground transition-colors hover:border-cyan/50 hover:text-cyan cursor-pointer"
 >
 {topic}
 </span>
 ))}
 </div>
 </div>

 {/* Bookmarked */}
 <div className=" border border-border bg-card p-4">
 <h3 className="text-sm font-semibold mb-3">Bookmarked</h3>
 <div className="space-y-2">
 {["Fed Rate Decision Analysis", "BTC Weekly Outlook", "EUR/USD Trading Plan"].map((item, i) => (
 <div key={i} className="flex items-center gap-2 text-[12px]">
 <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor" className="text-cyan shrink-0">
 <path d="M19 21l-7-5-7 5V5a2 2 0 012-2h10a2 2 0 012 2z" />
 </svg>
 <span className="text-muted-foreground hover:text-foreground cursor-pointer">{item}</span>
 </div>
 ))}
 </div>
 </div>
 </div>
 </SlideIn>
 </div>
 );
}
