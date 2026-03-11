"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useTheme } from "next-themes";
import { useEffect, useState } from "react";
import { cn } from "@/lib/utils";

const navItems = [
  { href: "/dashboard", label: "DASHBOARD", icon: "grid" },
  { href: "/dashboard/markets", label: "MARKETS", icon: "chart" },
  { href: "/dashboard/positions", label: "POSITIONS", icon: "layers" },
  { href: "/dashboard/analytics", label: "ANALYSIS", icon: "bar-chart" },
];

const iconMap: Record<string, React.ReactNode> = {
  grid: (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
      <rect x="3" y="3" width="7" height="7" rx="1" /><rect x="14" y="3" width="7" height="7" rx="1" /><rect x="3" y="14" width="7" height="7" rx="1" /><rect x="14" y="14" width="7" height="7" rx="1" />
    </svg>
  ),
  chart: (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
      <path d="M3 3v18h18" /><path d="M7 16l4-8 4 4 6-10" />
    </svg>
  ),
  layers: (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
      <path d="M12 2L2 7l10 5 10-5-10-5z" /><path d="M2 17l10 5 10-5" /><path d="M2 12l10 5 10-5" />
    </svg>
  ),
  "bar-chart": (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
      <path d="M12 20V10M18 20V4M6 20v-4" />
    </svg>
  ),
};

export function Sidebar() {
  const pathname = usePathname();
  const { theme, setTheme } = useTheme();
  const [mounted, setMounted] = useState(false);

  useEffect(() => setMounted(true), []);

  return (
    <aside className="flex h-screen w-[240px] flex-col bg-panel border-r border-border">
      {/* Logo */}
      <div className="flex h-14 items-center gap-3 px-5">
        <div className="relative flex h-9 w-9 items-center justify-center rounded-xl bg-gradient-to-br from-cyan to-[#00b4d8] shadow-[0_0_12px_rgba(0,180,216,0.2)]">
          <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
            <path d="M5 15L8 5" stroke="black" strokeWidth="2.5" strokeLinecap="round" />
            <path d="M12 15L15 5" stroke="black" strokeWidth="2.5" strokeLinecap="round" />
            <path d="M6.5 10H13.5" stroke="black" strokeWidth="2.5" strokeLinecap="round" />
          </svg>
        </div>
        <div className="flex items-baseline gap-0.5">
          <span className="text-[15px] font-bold tracking-[0.5px] text-cyan">HYBRID</span>
          <span className="text-[15px] font-light tracking-[0.5px] text-text-secondary">TRADE</span>
        </div>
      </div>

      <div className="mx-4 h-px bg-border" />

      {/* Navigation */}
      <nav className="flex-1 px-3 py-4 space-y-0.5">
        {navItems.map((item) => {
          const isActive =
            item.href === "/dashboard"
              ? pathname === "/dashboard"
              : pathname.startsWith(item.href);
          return (
            <Link
              key={item.href}
              href={item.href}
              className={cn(
                "relative flex items-center gap-3 px-4 py-2.5 text-[13px] font-medium tracking-wide transition-all",
                isActive
                  ? "bg-cyan-dim font-semibold text-cyan"
                  : "text-text-secondary hover:bg-sidebar-hover hover:text-foreground"
              )}
            >
              {isActive && (
                <span className="absolute left-0 top-1/2 -translate-y-1/2 h-6 w-[3px] bg-cyan" />
              )}
              <span className={cn(isActive ? "text-cyan" : "text-text-secondary")}>
                {iconMap[item.icon]}
              </span>
              {item.label}
            </Link>
          );
        })}
      </nav>

      <div className="mx-4 h-px bg-border" />

      {/* Bottom */}
      <div className="p-3 space-y-0.5">
        <button className="flex w-full items-center gap-3 px-4 py-2.5 text-[13px] font-medium tracking-wide text-text-muted transition-colors hover:bg-sidebar-hover hover:text-foreground">
          SETTINGS
        </button>
      </div>

      <div className="mx-4 h-px bg-border" />

      {/* User + Theme toggle */}
      <div className="p-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="flex h-8 w-8 items-center justify-center rounded-full bg-cyan-dim text-[11px] font-bold text-cyan">
              K
            </div>
            <div>
              <div className="text-[13px] font-medium">Kuromi</div>
              <div className="text-[11px] text-text-muted">Trader</div>
            </div>
          </div>
          {mounted && (
            <button
              onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
              className="flex h-8 w-8 items-center justify-center border border-border text-text-secondary transition-colors hover:bg-sidebar-hover hover:text-foreground"
              aria-label="Toggle theme"
            >
              {theme === "dark" ? (
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                  <circle cx="12" cy="12" r="5" />
                  <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" />
                </svg>
              ) : (
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
                </svg>
              )}
            </button>
          )}
        </div>
      </div>
    </aside>
  );
}
