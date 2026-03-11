"use client";

import { motion } from "motion/react";
import { useState } from "react";
import { StaggerGrid, SlideIn } from "@/components/dashboard/motion-primitives";
import { StatsCard } from "@/components/dashboard/stats-card";
import { cn } from "@/lib/utils";

type Category = "ALL" | "COMMODITIES" | "FOREX" | "INDICES" | "CRYPTO";

const filterTabs: Category[] = ["ALL", "COMMODITIES", "FOREX", "INDICES", "CRYPTO"];

const instruments = [
  { pair: "XAU/USD", name: "Gold", price: "2,178.40", change: "+0.89%", changeType: "profit" as const, spread: "2.5", volume: "890M", category: "COMMODITIES" },
  { pair: "XAG/USD", name: "Silver", price: "24.82", change: "+1.24%", changeType: "profit" as const, spread: "1.8", volume: "320M", category: "COMMODITIES" },
  { pair: "WTI/USD", name: "Crude Oil", price: "78.45", change: "-0.67%", changeType: "loss" as const, spread: "3.0", volume: "1.2B", category: "COMMODITIES" },
  { pair: "BRENT", name: "Brent Oil", price: "82.10", change: "-0.42%", changeType: "loss" as const, spread: "3.5", volume: "980M", category: "COMMODITIES" },
  { pair: "NGAS", name: "Natural Gas", price: "2.847", change: "+2.18%", changeType: "profit" as const, spread: "4.0", volume: "450M", category: "COMMODITIES" },
  { pair: "US30", name: "Dow Jones", price: "39,142.50", change: "+0.34%", changeType: "profit" as const, spread: "2.0", volume: "5.2B", category: "INDICES" },
  { pair: "NAS100", name: "Nasdaq 100", price: "17,892.30", change: "+0.52%", changeType: "profit" as const, spread: "1.5", volume: "4.8B", category: "INDICES" },
  { pair: "SPX500", name: "S&P 500", price: "5,234.18", change: "+0.28%", changeType: "profit" as const, spread: "0.8", volume: "6.1B", category: "INDICES" },
  { pair: "EUR/USD", name: "Euro", price: "1.0847", change: "+0.24%", changeType: "profit" as const, spread: "0.8", volume: "2.4B", category: "FOREX" },
  { pair: "GBP/USD", name: "Pound", price: "1.2634", change: "+0.18%", changeType: "profit" as const, spread: "1.1", volume: "1.8B", category: "FOREX" },
  { pair: "USD/JPY", name: "Yen", price: "149.82", change: "-0.31%", changeType: "loss" as const, spread: "0.9", volume: "2.1B", category: "FOREX" },
  { pair: "USD/CHF", name: "Swiss Franc", price: "0.8812", change: "+0.05%", changeType: "profit" as const, spread: "1.2", volume: "650M", category: "FOREX" },
  { pair: "AUD/USD", name: "Aussie", price: "0.6523", change: "-0.18%", changeType: "loss" as const, spread: "0.9", volume: "580M", category: "FOREX" },
  { pair: "BTC/USD", name: "Bitcoin", price: "67,842.50", change: "+2.14%", changeType: "profit" as const, spread: "12.5", volume: "28.5B", category: "CRYPTO" },
  { pair: "ETH/USD", name: "Ethereum", price: "3,456.20", change: "-1.07%", changeType: "loss" as const, spread: "1.8", volume: "14.2B", category: "CRYPTO" },
];

const topMovers = [
  { pair: "NGAS", change: "+2.18%", type: "profit" as const },
  { pair: "BTC/USD", change: "+2.14%", type: "profit" as const },
  { pair: "XAG/USD", change: "+1.24%", type: "profit" as const },
  { pair: "XAU/USD", change: "+0.89%", type: "profit" as const },
  { pair: "ETH/USD", change: "-1.07%", type: "loss" as const },
  { pair: "WTI/USD", change: "-0.67%", type: "loss" as const },
];

export default function MarketsPage() {
  const [activeTab, setActiveTab] = useState<Category>("ALL");

  const filtered = activeTab === "ALL"
    ? instruments
    : instruments.filter((inst) => inst.category === activeTab);

  return (
    <div className="space-y-5 overflow-y-auto p-6 h-full">
      <motion.div
        initial={{ opacity: 0, y: 6 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.15 }}
      >
        <h1 className="text-[20px] font-bold tracking-[0.5px]">Markets</h1>
        <p className="mt-1 text-[12px] text-text-secondary">Live prices across commodities, forex, indices, and crypto.</p>
      </motion.div>

      {/* Filter Tabs */}
      <div className="flex gap-1">
        {filterTabs.map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={cn(
              "px-4 py-1.5 text-[11px] font-semibold tracking-[0.5px] transition-colors",
              activeTab === tab
                ? "bg-cyan-dim text-cyan"
                : "text-text-secondary hover:text-foreground"
            )}
          >
            {tab}
          </button>
        ))}
      </div>

      {/* Stats Row */}
      <StaggerGrid>
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
          <StatsCard title="24H Volume" value="$68.4B" badge="LIVE" changeType="neutral" />
          <StatsCard title="Active Pairs" value={String(filtered.length)} changeType="neutral" />
          <StatsCard title="Avg Spread" value="2.4 pips" changeType="neutral" />
          <StatsCard title="Top Mover" value="NGAS +2.18%" changeType="profit" />
        </div>
      </StaggerGrid>

      <div className="grid gap-5 lg:grid-cols-[1fr_280px]">
        {/* Instruments Table */}
        <motion.div
          initial={{ opacity: 0, y: 6 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.15, delay: 0.05 }}
          className="border border-border bg-card"
        >
          <div className="flex items-center justify-between px-5 py-3 border-b border-border">
            <h3 className="text-[13px] font-semibold">ALL INSTRUMENTS</h3>
            <span className="text-[10px] text-text-muted">{filtered.length} pairs</span>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full text-[12px]">
              <thead>
                <tr className="bg-card-alt text-[10px] font-bold uppercase tracking-[0.5px] text-text-secondary">
                  <th className="px-5 py-2.5 text-left">Instrument</th>
                  <th className="px-5 py-2.5 text-right">Price</th>
                  <th className="px-5 py-2.5 text-right">Change</th>
                  <th className="px-5 py-2.5 text-right">Chart</th>
                  <th className="px-5 py-2.5 text-right">Volume</th>
                  <th className="px-5 py-2.5 text-right">Spread</th>
                  <th className="px-5 py-2.5 text-right">Action</th>
                </tr>
              </thead>
              <tbody>
                {filtered.map((inst, i) => (
                  <motion.tr
                    key={inst.pair}
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    transition={{ duration: 0.1, delay: i * 0.02 }}
                    className={cn(
                      "transition-colors hover:bg-row-alt",
                      i % 2 === 0 ? "bg-card-alt" : "bg-card"
                    )}
                  >
                    <td className="px-5 py-3">
                      <div className="flex items-center gap-2">
                        <span
                          className="h-1.5 w-1.5 rounded-full"
                          style={{ backgroundColor: inst.changeType === "profit" ? "var(--cyan)" : "var(--loss)" }}
                        />
                        <div>
                          <span className="font-semibold">{inst.pair}</span>
                          <span className="ml-2 text-[10px] text-text-muted">{inst.name}</span>
                        </div>
                      </div>
                    </td>
                    <td className="px-5 py-3 text-right font-medium tabular-nums">{inst.price}</td>
                    <td
                      className="px-5 py-3 text-right font-semibold tabular-nums"
                      style={{ color: inst.changeType === "profit" ? "var(--profit)" : "var(--loss)" }}
                    >
                      {inst.change}
                    </td>
                    <td className="px-5 py-3 text-right">
                      <div className="inline-flex items-end gap-[2px] h-4">
                        {Array.from({ length: 7 }, (_, j) => {
                          const h = 3 + Math.abs(Math.sin(i * 2.1 + j * 0.9)) * 13;
                          return (
                            <div
                              key={j}
                              className="w-[2px]"
                              style={{
                                height: `${h}px`,
                                backgroundColor: inst.changeType === "profit" ? "var(--profit)" : "var(--loss)",
                                opacity: 0.4 + j * 0.08,
                              }}
                            />
                          );
                        })}
                      </div>
                    </td>
                    <td className="px-5 py-3 text-right text-text-muted tabular-nums">{inst.volume}</td>
                    <td className="px-5 py-3 text-right text-text-muted tabular-nums">{inst.spread}</td>
                    <td className="px-5 py-3 text-right">
                      <button className="bg-cyan-dim px-3 py-1 text-[10px] font-bold tracking-[0.5px] text-cyan transition-colors hover:bg-cyan/20">
                        TRADE
                      </button>
                    </td>
                  </motion.tr>
                ))}
              </tbody>
            </table>
          </div>
        </motion.div>

        {/* Right Panel - Top Movers */}
        <SlideIn direction="right" delay={0.05}>
          <div className="border border-border bg-panel space-y-4 p-4">
            <h3 className="text-[11px] font-bold uppercase tracking-[1px] text-text-secondary">Top Movers</h3>
            <div className="space-y-1">
              {topMovers.map((item) => (
                <div key={item.pair} className="flex items-center justify-between px-3 py-2 hover:bg-card transition-colors">
                  <span className="text-[12px] font-semibold">{item.pair}</span>
                  <span
                    className="text-[12px] font-semibold tabular-nums"
                    style={{ color: item.type === "profit" ? "var(--profit)" : "var(--loss)" }}
                  >
                    {item.type === "profit" ? "+" : ""}{item.change}
                  </span>
                </div>
              ))}
            </div>

            <div className="h-px bg-gradient-to-r from-transparent via-border to-transparent" />

            <div>
              <h3 className="text-[11px] font-bold uppercase tracking-[1px] text-text-secondary mb-3">Commodities</h3>
              <div className="space-y-2">
                {instruments.filter((i) => i.category === "COMMODITIES").map((item) => (
                  <div key={item.pair} className="flex items-center justify-between text-[11px] px-3">
                    <span className="text-text-secondary">{item.pair}</span>
                    <span className="font-medium tabular-nums">{item.price}</span>
                  </div>
                ))}
              </div>
            </div>

            <div className="h-px bg-gradient-to-r from-transparent via-border to-transparent" />

            <div>
              <h3 className="text-[11px] font-bold uppercase tracking-[1px] text-text-secondary mb-3">Indices</h3>
              <div className="space-y-2">
                {instruments.filter((i) => i.category === "INDICES").map((item) => (
                  <div key={item.pair} className="flex items-center justify-between text-[11px] px-3">
                    <span className="text-text-secondary">{item.pair}</span>
                    <span className="font-medium tabular-nums">{item.price}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </SlideIn>
      </div>
    </div>
  );
}
