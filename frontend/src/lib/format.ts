/** Format token count with K/M suffixes */
export function formatTokens(tokens: number | undefined): string {
  if (tokens === undefined || tokens === 0) return '0';
  if (tokens >= 1_000_000) return `${(tokens / 1_000_000).toFixed(2)}M`;
  if (tokens >= 1_000) return `${(tokens / 1_000).toFixed(1)}K`;
  return tokens.toLocaleString();
}

/** Format cost in cents to dollar string */
export function formatCost(cents: number | null | undefined): string {
  if (cents === null || cents === undefined || cents === 0) return '$0.00';
  const abs = Math.abs(cents / 100);
  return cents < 0 ? `-$${abs.toFixed(2)}` : `$${abs.toFixed(2)}`;
}

/** Format a number with K/M suffixes (generic) */
export function formatNumber(num: number): string {
  if (num >= 1_000_000) return `${(num / 1_000_000).toFixed(1)}M`;
  if (num >= 1_000) return `${(num / 1_000).toFixed(1)}K`;
  return num.toLocaleString();
}
