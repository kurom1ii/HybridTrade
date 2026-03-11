"use client";

import { cn } from "@/lib/utils";
import { type ReactNode } from "react";
import { motion } from "motion/react";

interface StatsCardProps {
  title: string;
  value: string;
  change?: string;
  changeType?: "profit" | "loss" | "neutral";
  icon?: ReactNode;
  badge?: string;
  className?: string;
  children?: ReactNode;
}

export function StatsCard({ title, value, change, changeType = "neutral", badge, className, children }: StatsCardProps) {
  return (
    <motion.div
      variants={{
        hidden: { opacity: 0, y: 8 },
        visible: { opacity: 1, y: 0, transition: { duration: 0.2, ease: "easeOut" } },
      }}
      className={cn("group relative overflow-hidden border border-border bg-card shadow-sm transition-all duration-200 hover:border-cyan/30 hover:shadow-md hover:-translate-y-[1px]", className)}
    >
      <div className="h-[2px] w-full bg-cyan" />
      <div className="relative z-10 p-4">
        <div className="flex items-center justify-between">
          <span className="text-[11px] font-semibold uppercase tracking-wider text-text-secondary">{title}</span>
          {badge && (
            <span className="bg-cyan-dim px-2 py-0.5 text-[9px] font-bold tracking-wide text-cyan">{badge}</span>
          )}
        </div>
        <div className="mt-2 text-[26px] font-bold tracking-tight">{value}</div>
        {change && (
          <div className={cn("mt-1 text-[11px] font-medium",
            changeType === "profit" && "text-profit",
            changeType === "loss" && "text-loss",
            changeType === "neutral" && "text-text-muted"
          )}>
            {change}
          </div>
        )}
        {children}
      </div>
    </motion.div>
  );
}

export function DistributionBar({
  leftLabel, rightLabel, leftValue, rightValue,
  leftColor = "var(--cyan)", rightColor = "var(--loss)",
}: {
  leftLabel: string; rightLabel: string; leftValue: number; rightValue: number;
  leftColor?: string; rightColor?: string;
}) {
  const total = leftValue + rightValue;
  const leftPct = total > 0 ? (leftValue / total) * 100 : 50;
  return (
    <div className="mt-3">
      <motion.div
        className="h-2 w-full overflow-hidden bg-tint"
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ delay: 0.1 }}
      >
        <motion.div
          className="h-full"
          initial={{ width: 0 }}
          animate={{ width: `${leftPct}%` }}
          transition={{ duration: 0.4, delay: 0.15, ease: "easeOut" }}
          style={{ background: leftColor }}
        />
      </motion.div>
      <div className="mt-1.5 flex items-center justify-between text-[9px] font-bold tracking-wide">
        <span style={{ color: leftColor }}>{leftLabel}</span>
        <span style={{ color: rightColor }}>{rightLabel}</span>
      </div>
    </div>
  );
}
