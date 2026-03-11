"use client";

import { motion } from "motion/react";

interface PageTitleProps {
  title: string;
  subtitle?: string;
  breadcrumb?: string;
}

export function PageTitle({ title, subtitle, breadcrumb }: PageTitleProps) {
  return (
    <motion.div
      initial={{ opacity: 0, y: -8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.4, ease: "easeOut" }}
      className="mb-6"
    >
      {breadcrumb && (
        <div className="text-[10px] font-medium tracking-[1.5px] text-text-secondary mb-1">{breadcrumb}</div>
      )}
      <h1 className="font-[family-name:var(--font-heading)] text-[20px] font-bold">{title}</h1>
      {subtitle && (
        <p className="mt-0.5 text-[12px] text-text-secondary">{subtitle}</p>
      )}
    </motion.div>
  );
}
