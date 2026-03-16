/** Parse a JSON-encoded array string, returning empty array on failure */
export function parseArr(v: string | null | undefined): string[] {
  if (!v) return [];
  try { return JSON.parse(v); } catch { return []; }
}

/** Parse a JSON array of URLs, keeping only http(s) entries */
export function parseUrls(v: string | null | undefined): string[] {
  return parseArr(v).filter(u => u.startsWith('http'));
}

/** Mix a hex colour toward white by `amount` (0.0 = full colour, 1.0 = white) */
export function mixWithWhite(hex: string, amount: number): string {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  const mix = (c: number) => Math.round(c + (255 - c) * amount).toString(16).padStart(2, '0');
  return `#${mix(r)}${mix(g)}${mix(b)}`;
}

/** Convert hex colour to "R, G, B" string */
export function hexToRgb(hex: string) {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  return `${r}, ${g}, ${b}`;
}

/** Calculate relative luminance of a hex colour (simplified) */
export function luminance(hex: string) {
  const r = parseInt(hex.slice(1, 3), 16) / 255;
  const g = parseInt(hex.slice(3, 5), 16) / 255;
  const b = parseInt(hex.slice(5, 7), 16) / 255;
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** Return black or white text colour for readability on a given background */
export function textOn(bg: string) {
  return luminance(bg) > 0.35 ? '#000000' : '#FFFFFF';
}
