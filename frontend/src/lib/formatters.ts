/**
 * Centralized formatting utilities.
 * All date, currency, and number formatting should import from here.
 */

const DATE_OPTIONS: Intl.DateTimeFormatOptions = {
  year: 'numeric',
  month: 'short',
  day: 'numeric',
};

/** Format a date (string or Date) as "Mar 15, 2026". Returns fallback for nullish/invalid input. */
export function formatDate(value: string | Date | null | undefined, fallback = '—'): string {
  if (!value) return fallback;
  try {
    const d = value instanceof Date ? value : new Date(value);
    if (isNaN(d.getTime())) return fallback;
    return d.toLocaleDateString('en-US', DATE_OPTIONS);
  } catch {
    return fallback;
  }
}

/** Format a date (string or Date) as full locale string including time. */
export function formatDateTime(value: string | Date | null | undefined, fallback = '—'): string {
  if (!value) return fallback;
  try {
    const d = value instanceof Date ? value : new Date(value);
    if (isNaN(d.getTime())) return fallback;
    return d.toLocaleString();
  } catch {
    return fallback;
  }
}

/** Compact currency: $1.2M, $50k, $500. */
export function formatCurrency(amount: number): string {
  if (amount >= 1_000_000) return `$${(amount / 1_000_000).toFixed(1)}M`;
  if (amount >= 1_000) return `$${(amount / 1_000).toFixed(0)}k`;
  return `$${amount.toFixed(0)}`;
}

/** Full Intl currency format: $1,200,000. */
export function formatCurrencyFull(amount: number): string {
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: 'USD',
    minimumFractionDigits: 0,
    maximumFractionDigits: 0,
  }).format(amount);
}

/** Compact number: 1.2M, 50.0k, 999. Handles nullish input. */
export function formatCompactNumber(n: number | null | undefined, fallback = '—'): string {
  if (n == null) return fallback;
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return n.toString();
}

/** Parse a JSON string as string[]. Returns [] on nullish/invalid input. */
export function parseJsonArray(val: string | null | undefined): string[] {
  if (!val) return [];
  try {
    return JSON.parse(val);
  } catch {
    return [];
  }
}
