"use client";

import Link from "next/link";
import { useTheme } from "next-themes";
import { useEffect, useState } from "react";

export function LandingHeader() {
  const { theme, setTheme } = useTheme();
  const [scrolled, setScrolled] = useState(false);
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
    const handleScroll = () => setScrolled(window.scrollY > 20);
    window.addEventListener("scroll", handleScroll);
    return () => window.removeEventListener("scroll", handleScroll);
  }, []);

  return (
    <header
      className={`fixed top-[2px] left-0 right-0 z-50 transition-all duration-300 ${
        scrolled
          ? "bg-background/80 backdrop-blur-xl border-b border-border"
          : "bg-transparent"
      }`}
    >
      <div className="mx-auto flex h-16 max-w-[1100px] items-center justify-between px-6">
        {/* Logo */}
        <Link href="/" className="flex items-center gap-2.5">
          <div className="relative flex h-8 w-8 items-center justify-center rounded-lg bg-gradient-to-br from-cyan to-[#00b4d8] shadow-[0_0_10px_rgba(0,180,216,0.3)]">
            <svg width="18" height="18" viewBox="0 0 20 20" fill="none">
              <path d="M5 15L8 5" stroke="black" strokeWidth="2.5" strokeLinecap="round" />
              <path d="M12 15L15 5" stroke="black" strokeWidth="2.5" strokeLinecap="round" />
              <path d="M6.5 10H13.5" stroke="black" strokeWidth="2.5" strokeLinecap="round" />
            </svg>
          </div>
          <div className="flex items-baseline gap-0.5">
            <span className="text-[16px] font-bold tracking-[1px] text-cyan">HYBRID</span>
            <span className="text-[16px] font-light tracking-[1px] text-text-secondary">TRADE</span>
          </div>
        </Link>

        {/* Nav Links */}
        <nav className="hidden md:flex items-center gap-8">
          {["MARKETS", "FEATURES", "PRICING", "NEWS"].map((item) => (
            <a
              key={item}
              href={`#${item.toLowerCase()}`}
              className="text-[12px] font-medium tracking-[0.5px] text-text-secondary transition-colors hover:text-foreground"
            >
              {item}
            </a>
          ))}
        </nav>

        {/* Right Actions */}
        <div className="flex items-center gap-3">
          {mounted && (
            <button
              onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
              className="p-2 text-text-secondary transition-colors hover:text-foreground"
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                {theme === "dark" ? (
                  <>
                    <circle cx="12" cy="12" r="5" />
                    <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" />
                  </>
                ) : (
                  <path d="M21 12.79A9 9 0 1111.21 3 7 7 0 0021 12.79z" />
                )}
              </svg>
            </button>
          )}
          <Link
            href="/dashboard"
            className="bg-cyan px-5 py-2 text-[11px] font-bold tracking-[0.5px] text-black transition-colors hover:bg-cyan/90"
          >
            OPEN DASHBOARD →
          </Link>
        </div>
      </div>
    </header>
  );
}
