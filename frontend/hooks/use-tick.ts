"use client";

import { useEffect, useState } from "react";

// ── Single global interval shared by all subscribers ──
// Instead of N setInterval calls (one per LiveTime), we maintain
// one interval that notifies all active listeners.

const listeners = new Set<() => void>();
let intervalId: ReturnType<typeof setInterval> | null = null;

function subscribe(fn: () => void) {
  listeners.add(fn);
  if (!intervalId) {
    intervalId = setInterval(() => {
      for (const listener of listeners) listener();
    }, 1000);
  }
}

function unsubscribe(fn: () => void) {
  listeners.delete(fn);
  if (listeners.size === 0 && intervalId) {
    clearInterval(intervalId);
    intervalId = null;
  }
}

/**
 * Returns a tick counter that increments every second.
 * All components using this hook share a single setInterval.
 */
export function useTick(): number {
  const [tick, setTick] = useState(0);
  useEffect(() => {
    const bump = () => setTick((t) => t + 1);
    subscribe(bump);
    return () => unsubscribe(bump);
  }, []);
  return tick;
}
