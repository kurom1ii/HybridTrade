const relativeFormatter = new Intl.RelativeTimeFormat("vi-VN", { numeric: "auto" });

export function formatDateTime(value?: string | null): string {
  if (!value) return "Chua co";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;

  return new Intl.DateTimeFormat("vi-VN", {
    hour: "2-digit",
    minute: "2-digit",
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
  }).format(date);
}

export function formatRelativeTime(value?: string | null): string {
  if (!value) return "Chua co";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;

  const diffSeconds = Math.round((date.getTime() - Date.now()) / 1000);
  const absSeconds = Math.abs(diffSeconds);

  if (absSeconds < 60) return relativeFormatter.format(diffSeconds, "second");
  if (absSeconds < 3600) return relativeFormatter.format(Math.round(diffSeconds / 60), "minute");
  if (absSeconds < 86_400) return relativeFormatter.format(Math.round(diffSeconds / 3600), "hour");
  return relativeFormatter.format(Math.round(diffSeconds / 86_400), "day");
}

/** Show remaining time until a future timestamp as a short countdown string.
 *  e.g. "42s", "3m 12s", "1h 5m", or "đã qua" if in the past. */
export function formatCountdown(value?: string | null): string {
  if (!value) return "--";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "--";

  const remaining = Math.round((date.getTime() - Date.now()) / 1000);
  if (remaining <= 0) return "đang chờ chạy";

  if (remaining < 60) return `${remaining}s`;
  if (remaining < 3600) {
    const m = Math.floor(remaining / 60);
    const s = remaining % 60;
    return s > 0 ? `${m}m ${s}s` : `${m}m`;
  }
  const h = Math.floor(remaining / 3600);
  const m = Math.floor((remaining % 3600) / 60);
  return m > 0 ? `${h}h ${m}m` : `${h}h`;
}

export function truncate(value: string, maxLength = 120): string {
  if (value.length <= maxLength) return value;
  return `${value.slice(0, maxLength - 1)}…`;
}

export function titleFromRole(role: string): string {
  const normalized = role.trim().toLowerCase();
  const labels: Record<string, string> = {
    kuromi: "Kuromi Finance",
    "kuromi_finance": "Kuromi Finance",
    "kuromi-finance": "Kuromi Finance",
    coordinator: "Kuromi Finance",
    user: "User",
  };

  return (
    labels[normalized] ||
    role
      .split(/[_-]+/)
      .map((part) => `${part.slice(0, 1).toUpperCase()}${part.slice(1)}`)
      .join(" ")
  );
}

export function formatConfidence(confidence: number | null): string {
  if (confidence == null) return "--";
  return `${Math.round(confidence * 100)}%`;
}
