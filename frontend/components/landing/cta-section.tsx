"use client";

import { motion, useInView } from "motion/react";
import { useRef } from "react";
import Link from "next/link";

export function CTASection() {
  const ref = useRef(null);
  const isInView = useInView(ref, { once: true, margin: "-80px" });

  return (
    <section className="relative py-28 px-6 overflow-hidden" ref={ref}>
      <div className="pointer-events-none absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 h-[400px] w-[400px] rounded-full bg-cyan opacity-[0.04] blur-[150px]" />

      <div className="relative z-10 mx-auto max-w-[700px]">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={isInView ? { opacity: 1, y: 0 } : {}}
          transition={{ duration: 0.4 }}
          className="text-center"
        >
          <span className="text-[9px] font-bold tracking-[2px] text-cyan/70">GET STARTED</span>

          <h2 className="mt-4 text-[44px] font-bold tracking-[-2px] leading-[1.05]">
            Ready to Trade{" "}
            <span className="text-cyan">Smarter?</span>
          </h2>

          <p className="mt-5 text-[13px] text-text-secondary leading-[1.9] max-w-[480px] mx-auto">
            Join thousands of traders using AI-powered multi-agent strategies.
            Automate your portfolio with institutional-grade algorithms.
          </p>

          {/* CTA Button */}
          <motion.div
            initial={{ opacity: 0, y: 10 }}
            animate={isInView ? { opacity: 1, y: 0 } : {}}
            transition={{ duration: 0.3, delay: 0.2 }}
            className="relative mt-10 inline-block"
          >
            <div className="absolute inset-0 bg-cyan/20 blur-2xl" />
            <Link
              href="/dashboard"
              className="relative glow-cyan bg-cyan px-14 py-4 text-[13px] font-bold tracking-[1px] text-black transition-all hover:bg-cyan/90 inline-block"
            >
              START TRADING →
            </Link>
          </motion.div>

          <p className="mt-6 text-[10px] tracking-[0.5px] text-text-muted">
            14-Day Free Trial &nbsp;·&nbsp; No Credit Card &nbsp;·&nbsp; Cancel Anytime
          </p>
        </motion.div>
      </div>
    </section>
  );
}
