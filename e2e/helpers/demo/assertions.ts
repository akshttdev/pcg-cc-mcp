import { Page, expect } from "@playwright/test";
import { t } from "../timing";

/**
 * Wait for a toast notification matching a pattern.
 * Toasts are rendered by sonner in an `<ol>` with `data-sonner-toaster`.
 */
export async function waitForToast(
  page: Page,
  pattern: string | RegExp,
  opts?: { timeout?: number }
): Promise<void> {
  const timeout = opts?.timeout ?? t(10_000);
  const toaster = page.locator("[data-sonner-toaster]");
  const regex = typeof pattern === "string" ? new RegExp(pattern, "i") : pattern;

  await expect(
    toaster.locator("li").filter({ hasText: regex })
  ).toBeVisible({ timeout });
}

/**
 * Wait for a watcher collaborator to show a specific state in the task detail drawer.
 */
export async function waitForWatcherState(
  page: Page,
  state: string,
  opts?: { timeout?: number }
): Promise<void> {
  const timeout = opts?.timeout ?? t(10_000);
  await expect(page.getByText(state)).toBeVisible({ timeout });
}
