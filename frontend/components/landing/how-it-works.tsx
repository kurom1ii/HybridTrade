"use client";

import { motion, useInView } from "motion/react";
import { useRef } from "react";

const steps = [
  {
    number: "01",
    title: "Create Account",
    description: "Sign up with email or connect your existing trading account. Verification takes under 60 seconds.",
  },
  {
    number: "02",
    title: "Configure Agents",
    description: "Choose from pre-built strategies or customize your own. Set risk levels, pairs, and timeframes.",
  },
  {
    number: "03",
    title: "Start Trading",
    description: "Deploy agents to live markets. Monitor P&L in real-time with auto risk management.",
  },
];

export function HowItWorks() {
  const ref = useRef(null);
  const isInView = useInView(ref, { once: true, margin: "-100px" });

  return (
    <section id="features" className="relative py-32 px-6 overflow-hidden" ref={ref}>
      {/* Subtle ambient glow */}
      <div className="pointer-events-none absolute left-1/2 top-0 -translate-x-1/2 h-[500px] w-[800px] rounded-full bg-cyan opacity-[0.03] blur-[150px]" />

      <div className="relative z-10 mx-auto max-w-[1100px]">
        {/* Section header */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={isInView ? { opacity: 1, y: 0 } : {}}
          transition={{ duration: 0.6 }}
          className="text-center"
        >
          <span className="text-[9px] font-bold tracking-[2px] text-cyan/80">HOW IT WORKS</span>
          <h2 className="mt-3 text-[40px] font-bold tracking-[-1.5px]">
            Three Simple Steps
          </h2>
          <p className="mx-auto mt-3 max-w-[440px] text-[13px] leading-[1.8] text-text-secondary/70">
            From signup to live execution in under 2 minutes
          </p>
        </motion.div>

        {/* Steps */}
        <div className="mt-16 grid grid-cols-1 gap-5 md:grid-cols-3">
          {steps.map((step, i) => (
            <motion.div
              key={step.number}
              initial={{ opacity: 0, y: 30 }}
              animate={isInView ? { opacity: 1, y: 0 } : {}}
              transition={{ duration: 0.6, delay: i * 0.15 }}
              className="group relative border border-border/50 bg-card-alt/50 p-7 backdrop-blur-sm transition-all duration-300 hover:border-cyan/20 hover:bg-card"
            >
              {/* Step number */}
              <div className="text-[32px] font-bold text-cyan/20 transition-colors duration-300 group-hover:text-cyan/40">
                {step.number}
              </div>
              <h3 className="mt-3 text-[15px] font-bold">{step.title}</h3>
              <p className="mt-2 text-[12px] leading-[1.8] text-text-secondary/70">{step.description}</p>

              {/* Bottom accent line on hover */}
              <div className="absolute bottom-0 left-6 right-6 h-px bg-gradient-to-r from-transparent via-cyan/0 to-transparent transition-all duration-300 group-hover:via-cyan/30" />
            </motion.div>
          ))}
        </div>

        {/* Preview stats strip */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={isInView ? { opacity: 1, y: 0 } : {}}
          transition={{ duration: 0.6, delay: 0.5 }}
          className="mt-14 border border-border/50 bg-card-alt/30 backdrop-blur-sm overflow-hidden"
        >
          <div className="grid grid-cols-4 divide-x divide-border/30">
            {[
              ["+$12,450", "Portfolio P/L", "text-cyan"],
              ["68.5%", "Win Rate", "text-profit"],
              ["1.84", "Sharpe Ratio", "text-foreground"],
              ["+22.6%", "YTD Return", "text-cyan"],
            ].map(([val, label, color], i) => (
              <div key={i} className="px-6 py-5 text-center">
                <div className={`text-[20px] font-bold ${color}`}>{val}</div>
                <div className="mt-1 text-[9px] font-medium tracking-[1px] text-text-secondary/60">{label}</div>
              </div>
            ))}
          </div>
        </motion.div>
      </div>
    </section>
  );
}
