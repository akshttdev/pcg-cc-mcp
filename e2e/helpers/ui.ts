import { Page, expect } from "@playwright/test";
import { t } from "./timing";
import { login } from "./auth";

/**
 * Open the Feedback dialog. The feedback button lives inside the "More" popover
 * in the sidebar, so we need to open the popover first.
 */
export async function openFeedbackDialog(page: Page) {
  const feedbackBtn = page.locator('[data-testid="feedback-button"]');

  // If the button is already visible (sidebar expanded with direct links), click directly
  if (await feedbackBtn.isVisible().catch(() => false)) {
    await feedbackBtn.click();
  } else {
    // Open the "More" popover in the sidebar first
    const moreBtn = page.getByRole("button", { name: "More" });
    await moreBtn.click();
    await expect(feedbackBtn).toBeVisible({ timeout: t(3_000) });
    await feedbackBtn.click();
  }

  await expect(
    page.getByRole("heading", { name: "Submit Feedback" })
  ).toBeVisible({ timeout: t(5_000) });
}

// ─── View-As Helpers ────────────────────────────────────────────────────────

/** Ensure page is on the app (not about:blank) so localStorage is accessible */
async function ensureOnApp(page: Page) {
  if (page.url() === "about:blank") {
    await page.goto("/");
    await page.waitForLoadState("domcontentloaded");
  }
}

/** Wait for the app to finish rendering after navigation/reload */
async function waitForAppReady(page: Page) {
  await page.waitForLoadState("networkidle");
}

/** Set a view-as role override via localStorage and reload */
export async function setViewAsRole(page: Page, role: string) {
  await ensureOnApp(page);
  await page.evaluate((r) => localStorage.setItem("pcg:view-as-role", r), role);
  await page.reload({ waitUntil: "domcontentloaded" });
  await waitForAppReady(page);
}

/** Clear view-as override via localStorage and reload */
export async function clearViewAsRole(page: Page) {
  await ensureOnApp(page);
  await page.evaluate(() => localStorage.removeItem("pcg:view-as-role"));
  await page.reload({ waitUntil: "domcontentloaded" });
  await waitForAppReady(page);
}

/** Locate a settings scope tab button by exact label (scoped to settings aside) */
export function settingsTab(page: Page, label: string) {
  return page.locator("aside button", { hasText: new RegExp(`^${label}$`) });
}

/** Get the current view-as role from localStorage */
export async function getViewAsRole(page: Page): Promise<string | null> {
  return page.evaluate(() => localStorage.getItem("pcg:view-as-role"));
}

/** Wait for the settings page to be ready (scope tabs mounted in aside) */
export async function waitForSettingsReady(page: Page) {
  await expect(page.locator("aside button").first()).toBeVisible({ timeout: 10_000 });
}
