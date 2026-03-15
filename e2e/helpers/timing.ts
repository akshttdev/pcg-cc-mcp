const headed = process.env.E2E_HEADED === "true";

/**
 * Scale a timeout for headed mode (slowMo makes everything take longer).
 * Usage: `await expect(locator).toBeVisible({ timeout: t(5_000) })`
 */
export function t(baseMs: number, headedMs?: number): number {
  if (!headed) return baseMs;
  return headedMs ?? baseMs * 2;
}

/**
 * Demo pace — controls how long pauses are during demo playback.
 *
 * Set via E2E_DEMO_PACE env var: "short" | "medium" | "long"
 * Default: "medium" (the original 1500ms base).
 *
 * Usage:
 *   await page.waitForTimeout(demoPause.short);
 *   await page.waitForTimeout(demoPause.medium);
 *   await page.waitForTimeout(demoPause.long);
 */
export type DemoPacePreset = "short" | "medium" | "long";

const PACE_MULTIPLIERS: Record<DemoPacePreset, number> = {
  short: 0.4,
  medium: 1,
  long: 2.5,
};

function resolvePace(): DemoPacePreset {
  const env = process.env.E2E_DEMO_PACE?.toLowerCase();
  if (env === "short" || env === "medium" || env === "long") return env;
  return "medium";
}

const BASE_SHORT = 600;
const BASE_MEDIUM = 1_500;
const BASE_LONG = 3_000;

const multiplier = PACE_MULTIPLIERS[resolvePace()];

export const demoPause = {
  /** Brief pause — after minor UI interactions (clicks, fills) */
  short: Math.round(BASE_SHORT * multiplier),
  /** Standard pause — after meaningful state changes (status change, toast) */
  medium: Math.round(BASE_MEDIUM * multiplier),
  /** Extended pause — after major milestones (PR created, verdict received) */
  long: Math.round(BASE_LONG * multiplier),
} as const;
