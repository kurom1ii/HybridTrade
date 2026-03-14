"use client";

import { motion, useInView } from "motion/react";
import { useRef } from "react";

const markets = [
  { pair: "XAU/USD", name: "Gold", price: "2,178.40", change: "+0.89%", type: "profit" },
];

const features = [
  { title: "Real-Time Signals", desc: "AI-generated buy/sell signals with confidence scoring across all major pairs.", metric: "847", label: "SIGNALS / DAY" },
  { title: "Risk Management", desc: "Dynamic position sizing and portfolio-level drawdown protection.", metric: "2.1%", label: "MAX RISK" },
  { title: "Multi-Agent AI", desc: "4 specialized agents working together — scanner, strategist, risk manager, executor.", metric: "4", label: "AGENTS" },
];

export function LiveMarkets() {
  const ref = useRef(null);
  const isInView = useInView(ref, { once: true, margin: "-80px" });

  return (
    <section id="markets" className="relative py-28 px-6 overflow-hidden" ref={ref}>
      <div className="pointer-events-none absolute right-0 top-1/2 -translate-y-1/2 h-[500px] w-[500px] rounded-full bg-cyan opacity-[0.02] blur-[150px]" />

      <div className="relative z-10 mx-auto max-w-[1100px]">
        {/* Header */}
        <motion.div
          initial={{ opacity: 0, y: 16 }}
          animate={isInView ? { opacity: 1, y: 0 } : {}}
          transition={{ duration: 0.4 }}
          className="text-center mb-14"
        >
          <div className="inline-flex items-center gap-2 mb-3">
            <span className="h-1.5 w-1.5 rounded-full bg-profit live-dot" />
            <span className="text-[9px] font-bold tracking-[2px] text-profit">LIVE DATA</span>
          </div>
          <h2 className="text-[40px] font-bold tracking-[-1.5px]">Live Markets</h2>
          <p className="mx-auto mt-3 max-w-[460px] text-[13px] leading-[1.8] text-text-secondary">
            Real-time prices across commodities, forex, indices, and crypto with institutional-grade feeds.
          </p>
        </motion.div>

        <div className="grid grid-cols-1 lg:grid-cols-[1fr_380px] gap-5">
          {/* Market Cards Grid */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={isInView ? { opacity: 1, y: 0 } : {}}
            transition={{ duration: 0.4, delay: 0.1 }}
            className="grid grid-cols-2 gap-3"
          >
            {markets.map((market, i) => (
              <motion.div
                key={market.pair}
                initial={{ opacity: 0, y: 10 }}
                animate={isInView ? { opacity: 1, y: 0 } : {}}
                transition={{ duration: 0.2, delay: 0.15 + i * 0.04 }}
                className="border border-border bg-card p-4 transition-colors hover:border-cyan/15"
              >
                <div className="flex items-center justify-between mb-3">
                  <div>
                    <span className="text-[14px] font-bold">{market.pair}</span>
                    <div className="text-[10px] text-text-muted mt-0.5">{market.name}</div>
                  </div>
                  <span
                    className="h-1.5 w-1.5 rounded-full"
                    style={{ backgroundColor: market.type === "profit" ? "var(--profit)" : "var(--loss)" }}
                  />
                </div>
                <div className="flex items-end justify-between">
                  <span className="text-[18px] font-bold tabular-nums">{market.price}</span>
                  <span
                    className="text-[12px] font-semibold tabular-nums"
                    style={{ color: market.type === "profit" ? "var(--profit)" : "var(--loss)" }}
                  >
                    {market.type === "profit" ? "+" : ""}{market.change}
                  </span>
                </div>
                {/* Mini sparkline */}
                <div className="flex items-end gap-[1px] h-4 mt-3">
                  {Array.from({ length: 20 }, (_, j) => {
                    const h = Math.round((2 + Math.abs(Math.sin(i * 2.3 + j * 0.5)) * 14) * 100) / 100;
                    const op = Math.round((0.1 + (j / 20) * 0.4) * 100) / 100;
                    return (
                      <div
                        key={j}
                        className="flex-1"
                        style={{
                          height: `${h}px`,
                          backgroundColor: market.type === "profit" ? "var(--profit)" : "var(--loss)",
                          opacity: op,
                        }}
                      />
                    );
                  })}
                </div>
              </motion.div>
            ))}
          </motion.div>

          {/* Features Panel */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={isInView ? { opacity: 1, y: 0 } : {}}
            transition={{ duration: 0.4, delay: 0.2 }}
            className="space-y-4"
          >
            {features.map((feat, i) => (
              <div key={i} className="border border-border bg-card p-5">
                <div className="flex items-start justify-between mb-3">
                  <div>
                    <h3 className="text-[14px] font-bold">{feat.title}</h3>
                    <p className="mt-2 text-[12px] text-text-secondary leading-[1.7]">{feat.desc}</p>
                  </div>
                </div>
                <div className="flex items-center justify-between pt-3 border-t border-border">
                  <span className="text-[8px] font-bold tracking-[1px] text-text-muted">{feat.label}</span>
                  <span className="text-[20px] font-bold text-cyan">{feat.metric}</span>
                </div>
              </div>
            ))}
          </motion.div>
        </div>
      </div>
    </section>
  );
}
