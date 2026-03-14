"use client";

import { useState, useEffect, useRef, useCallback } from "react";
import { useRouter } from "next/navigation";
import { cn } from "@/lib/utils";

/* ═══ Trading Sessions ═══ */
interface Session {
  name: string;
  openUTC: number;
  closeUTC: number;
}

const sessions: Session[] = [
  { name: "SYDNEY",   openUTC: 21, closeUTC: 6 },
  { name: "TOKYO",    openUTC: 0,  closeUTC: 9 },
  { name: "LONDON",   openUTC: 8,  closeUTC: 17 },
  { name: "NEW YORK", openUTC: 13, closeUTC: 22 },
];

function getSessionStatus(session: Session, nowUTC: Date): { open: boolean; label: string } {
  const h = nowUTC.getUTCHours();
  const m = nowUTC.getUTCMinutes();
  const s = nowUTC.getUTCSeconds();
  const nowSecs = h * 3600 + m * 60 + s;
  const openSecs = session.openUTC * 3600;
  const closeSecs = session.closeUTC * 3600;

  const isOpen = openSecs > closeSecs
    ? (nowSecs >= openSecs || nowSecs < closeSecs)
    : (nowSecs >= openSecs && nowSecs < closeSecs);

  const fmt = (totalSecs: number) => {
    const hh = Math.floor(totalSecs / 3600);
    const mm = Math.floor((totalSecs % 3600) / 60);
    const ss = totalSecs % 60;
    return `${hh.toString().padStart(2, "0")}:${mm.toString().padStart(2, "0")}:${ss.toString().padStart(2, "0")}`;
  };

  if (isOpen) {
    let secsLeft: number;
    if (openSecs > closeSecs) {
      secsLeft = nowSecs >= openSecs ? (86400 - nowSecs) + closeSecs : closeSecs - nowSecs;
    } else {
      secsLeft = closeSecs - nowSecs;
    }
    return { open: true, label: fmt(secsLeft) };
  } else {
    let secsUntil: number;
    if (openSecs > closeSecs) {
      secsUntil = openSecs - nowSecs;
    } else {
      secsUntil = nowSecs < openSecs ? openSecs - nowSecs : (86400 - nowSecs) + openSecs;
    }
    return { open: false, label: fmt(secsUntil) };
  }
}

/* ═══ Searchable instruments ═══ */
interface SearchItem {
  symbol: string;
  name: string;
  category: string;
  direction: "BUY" | "SELL" | "NEUTRAL";
  price: string;
  change: string;
  type: "profit" | "loss";
  summary: string;
}

const searchItems: SearchItem[] = [
  { symbol: "XAU/USD", name: "Gold", category: "COMMODITIES", direction: "BUY", price: "2,178.40", change: "+0.89%", type: "profit", summary: "Xu huong tang manh, RSI(14) vung 62" },
];

export function TopBar() {
  const [now, setNow] = useState<Date | null>(null);
  const [searchOpen, setSearchOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [selectedIdx, setSelectedIdx] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const resultsRef = useRef<HTMLDivElement>(null);
  const router = useRouter();

  useEffect(() => {
    setNow(new Date());
    const id = setInterval(() => setNow(new Date()), 1_000);
    return () => clearInterval(id);
  }, []);

  // Ctrl+K shortcut
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "k" && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        setSearchOpen(true);
        setQuery("");
      }
      if (e.key === "Escape") {
        setSearchOpen(false);
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  // Focus input when opened
  useEffect(() => {
    if (searchOpen) {
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [searchOpen]);

  const filtered = query.trim()
    ? searchItems.filter((item) => {
        const q = query.toLowerCase();
        return (
          item.symbol.toLowerCase().includes(q) ||
          item.name.toLowerCase().includes(q) ||
          item.category.toLowerCase().includes(q) ||
          item.summary.toLowerCase().includes(q) ||
          item.direction.toLowerCase().includes(q)
        );
      })
    : searchItems;

  // Reset selection when query or results change
  useEffect(() => { setSelectedIdx(0); }, [query]);

  const handleSelect = useCallback((symbol: string) => {
    setSearchOpen(false);
    setQuery("");
    router.push(`/dashboard/analytics/${symbol.replace("/", "-").toLowerCase()}`);
  }, [router]);

  const handleSearchKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      if (filtered.length > 0) handleSelect(filtered[selectedIdx]?.symbol ?? filtered[0].symbol);
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIdx((prev) => {
        const next = (prev + 1) % filtered.length;
        resultsRef.current?.children[next]?.scrollIntoView({ block: "nearest" });
        return next;
      });
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIdx((prev) => {
        const next = (prev - 1 + filtered.length) % filtered.length;
        resultsRef.current?.children[next]?.scrollIntoView({ block: "nearest" });
        return next;
      });
    }
  }, [filtered, selectedIdx, handleSelect]);

  const utcStr = now
    ? `${now.getUTCHours().toString().padStart(2, "0")}:${now.getUTCMinutes().toString().padStart(2, "0")}:${now.getUTCSeconds().toString().padStart(2, "0")} UTC`
    : "--:--:-- UTC";

  return (
    <>
      <header className="flex h-12 items-center border-b border-border bg-background px-4 gap-3">
        {/* Search trigger */}
        <button
          onClick={() => { setSearchOpen(true); setQuery(""); }}
          className="flex items-center gap-2 h-[30px] w-[200px] shrink-0 border border-border bg-input px-3 text-[11px] text-text-muted hover:border-cyan/30 transition-colors outline-none"
        >
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="11" cy="11" r="8" /><path d="M21 21l-4.35-4.35" />
          </svg>
          <span className="flex-1 text-left">Search...</span>
          <kbd className="border border-border bg-secondary px-1 py-0.5 text-[9px]">Ctrl+K</kbd>
        </button>

        {/* Divider */}
        <div className="h-5 w-px bg-border shrink-0" />

        {/* Trading Sessions */}
        <div className="flex items-center gap-1.5 flex-1 min-w-0">
          {sessions.map((session) => {
            const status = now ? getSessionStatus(session, now) : { open: false, label: "--:--:--" };
            return (
              <div
                key={session.name}
                className={cn(
                  "flex items-center gap-1.5 px-2.5 h-[30px] shrink-0 transition-colors",
                  status.open
                    ? "border border-border bg-card"
                    : "border border-transparent"
                )}
              >
                <span
                  className={cn(
                    "h-[5px] w-[5px] rounded-full shrink-0",
                    status.open ? "bg-profit live-dot" : "bg-text-faint"
                  )}
                />
                <span className={cn(
                  "text-[10px] font-bold tracking-[0.5px]",
                  status.open ? "text-foreground" : "text-text-muted"
                )}>
                  {session.name}
                </span>
                <span className={cn(
                  "text-[9px] font-semibold tabular-nums",
                  status.open ? "text-profit" : "text-text-faint"
                )}>
                  {status.label}
                </span>
              </div>
            );
          })}
        </div>

        {/* UTC Clock */}
        <div className="shrink-0 text-[10px] font-bold text-text-muted tabular-nums tracking-[0.5px]">
          {utcStr}
        </div>
      </header>

      {/* Search overlay */}
      {searchOpen && (
        <div className="fixed inset-0 z-50 flex items-start justify-center pt-[10vh]" onClick={() => setSearchOpen(false)}>
          {/* Backdrop */}
          <div className="absolute inset-0 bg-black/60" />

          {/* Search panel */}
          <div
            className="relative w-full max-w-[560px] bg-background shadow-2xl shadow-black/80 outline-none"
            onClick={(e) => e.stopPropagation()}
            onKeyDown={(e) => {
              // Keep focus on input for all keyboard interactions
              if (e.key === "Tab") {
                e.preventDefault();
                inputRef.current?.focus();
              }
            }}
          >
            {/* Search input */}
            <div className="flex items-center gap-3 border-b border-border/30 px-4 h-[48px]">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--cyan)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="11" cy="11" r="8" /><path d="M21 21l-4.35-4.35" />
              </svg>
              <input
                ref={inputRef}
                type="text"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                onKeyDown={handleSearchKeyDown}
                placeholder="Search symbols, names, categories..."
                className="flex-1 bg-transparent text-[13px] text-foreground placeholder:text-text-muted outline-none"
              />
              <kbd
                tabIndex={-1}
                onClick={() => setSearchOpen(false)}
                className="border border-border bg-secondary px-1.5 py-0.5 text-[9px] text-text-muted cursor-pointer hover:text-foreground outline-none"
              >
                ESC
              </kbd>
            </div>

            {/* Results */}
            <div ref={resultsRef} className="max-h-[400px] overflow-y-auto">
              {filtered.length === 0 ? (
                <div className="px-4 py-8 text-center text-[12px] text-text-muted">No results found</div>
              ) : (
                filtered.map((item, idx) => (
                  <div
                    key={item.symbol}
                    onClick={() => handleSelect(item.symbol)}
                    onMouseEnter={() => setSelectedIdx(idx)}
                    className={cn(
                      "w-full flex items-center gap-4 px-4 py-3 border-b border-border/50 text-left cursor-pointer transition-all duration-150 ease-out",
                      idx === selectedIdx ? "bg-tint" : "hover:bg-secondary"
                    )}
                  >
                    {/* Symbol + Direction */}
                    <div className="w-[100px] shrink-0">
                      <div className="text-[14px] font-bold">{item.symbol}</div>
                      <div className="text-[10px] text-text-muted">{item.name}</div>
                    </div>

                    {/* Direction badge */}
                    <span
                      className={cn(
                        "px-2 py-0.5 text-[9px] font-bold tracking-wider shrink-0",
                        item.direction === "BUY" && "bg-profit/12 text-profit",
                        item.direction === "SELL" && "bg-loss/12 text-loss",
                        item.direction === "NEUTRAL" && "bg-[#FFD700]/12 text-[#FFD700]",
                      )}
                    >
                      {item.direction}
                    </span>

                    {/* Summary */}
                    <div className="flex-1 min-w-0">
                      <p className="text-[11px] text-text-secondary truncate">{item.summary}</p>
                    </div>

                    {/* Price + Change */}
                    <div className="text-right shrink-0">
                      <div className="text-[13px] font-bold tabular-nums">{item.price}</div>
                      <div className={cn(
                        "text-[11px] font-semibold tabular-nums",
                        item.type === "profit" ? "text-profit" : "text-loss"
                      )}>
                        {item.change}
                      </div>
                    </div>
                  </div>
                ))
              )}
            </div>

            {/* Footer hint */}
            <div className="flex items-center gap-4 px-4 py-2 border-t border-border/30 text-[10px] text-text-faint">
              <span>↑↓ Navigate</span>
              <span>↵ Open analysis</span>
              <span>Esc Close</span>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
