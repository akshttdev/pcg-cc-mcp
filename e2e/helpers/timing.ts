const headed = process.env.E2E_HEADED === "true";

/**
 * Scale a timeout for headed mode (slowMo makes everything take longer).
 * Usage: `await expect(locator).toBeVisible({ timeout: t(5_000) })`
 */
export function t(baseMs: number, headedMs?: number): number {
  if (!headed) return baseMs;
  return headedMs ?? baseMs * 2;
}
