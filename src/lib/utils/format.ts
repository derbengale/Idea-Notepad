import type { FieldValue } from "@/lib/ipc/types";

export function formatDate(value: string, locale: string, withTime = false): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  return new Intl.DateTimeFormat(locale, {
    dateStyle: "medium",
    timeStyle: withTime ? "short" : undefined,
  }).format(date);
}

export function formatRelative(value: string, locale: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  const diffSeconds = Math.round((date.getTime() - Date.now()) / 1000);
  const formatter = new Intl.RelativeTimeFormat(locale, { numeric: "auto" });
  const steps: Array<[Intl.RelativeTimeFormatUnit, number]> = [
    ["year", 60 * 60 * 24 * 365],
    ["month", 60 * 60 * 24 * 30],
    ["day", 60 * 60 * 24],
    ["hour", 60 * 60],
    ["minute", 60],
  ];
  const absolute = Math.abs(diffSeconds);
  for (const [unit, seconds] of steps) {
    if (absolute >= seconds) {
      return formatter.format(Math.round(diffSeconds / seconds), unit);
    }
  }
  return formatter.format(diffSeconds, "second");
}

export function formatNumber(value: number, locale: string, digits = 0): string {
  return new Intl.NumberFormat(locale, { maximumFractionDigits: digits }).format(value);
}

export function formatBytes(bytes: number, locale: string): string {
  if (bytes < 1024) {
    return `${formatNumber(bytes, locale)} B`;
  }
  const units = ["KB", "MB", "GB"];
  let value = bytes / 1024;
  let index = 0;
  while (value >= 1024 && index < units.length - 1) {
    value /= 1024;
    index += 1;
  }
  return `${formatNumber(value, locale, 1)} ${units[index]}`;
}

export function displayValue(value: FieldValue): string {
  if (value === null || value === undefined) {
    return "";
  }
  if (typeof value === "boolean") {
    return value ? "✓" : "—";
  }
  return String(value);
}

export function wordCount(markdown: string): number {
  const text = markdown
    .replace(/```[\s\S]*?```/g, " ")
    .replace(/[#>*_`~\-[\]()]/g, " ")
    .trim();
  if (!text) {
    return 0;
  }
  return text.split(/\s+/).filter(Boolean).length;
}

export function debounce<A extends unknown[]>(
  fn: (...args: A) => void,
  delay: number,
): (...args: A) => void {
  let handle: ReturnType<typeof setTimeout> | undefined;
  return (...args: A) => {
    if (handle) {
      clearTimeout(handle);
    }
    handle = setTimeout(() => fn(...args), delay);
  };
}

export function normalizeLocale(language: string): string {
  return language === "de" ? "de" : "en";
}
