/**
 * Custom Playwright fixtures for ORCHA E2E tests.
 *
 * In headed mode: reuses a single browser context and page across all tests
 * to avoid window churn (no open/close per test). One stable browser window.
 *
 * In headless mode: default Playwright behavior (fresh context per test)
 * for full test isolation.
 *
 * All spec files should import { test, expect } from "./fixtures" instead
 * of "@playwright/test".
 */
import { test as base, expect, BrowserContext, Page } from "@playwright/test";

export { expect };

const headed = process.env.E2E_HEADED === "true";

// Module-level state for headed mode — persists across tests within a worker
let _sharedContext: BrowserContext | undefined;
let _sharedPage: Page | undefined;

export const test = base.extend({
  context: async ({ browser }, use, testInfo) => {
    if (!headed) {
      // Headless: default behavior — fresh context per test for isolation
      const ctx = await browser.newContext({
        storageState: testInfo.project.use.storageState as string | undefined,
        viewport: testInfo.project.use.viewport,
        baseURL: testInfo.project.use.baseURL,
      });
      await use(ctx);
      await ctx.close();
      return;
    }

    // Headed: reuse a single context for the entire run
    if (!_sharedContext) {
      _sharedContext = await browser.newContext({
        storageState: testInfo.project.use.storageState as string | undefined,
        viewport: testInfo.project.use.viewport,
        baseURL: testInfo.project.use.baseURL,
        recordVideo: testInfo.project.use.video === "on"
          ? { dir: "test-results/" }
          : undefined,
      });
    }
    await use(_sharedContext);
    // Don't close — reused across tests
  },

  page: async ({ context }, use) => {
    if (!headed) {
      // Headless: fresh page per test
      const page = await context.newPage();
      await use(page);
      await page.close();
      return;
    }

    // Headed: reuse a single page (tab)
    if (!_sharedPage || _sharedPage.isClosed()) {
      _sharedPage = await context.newPage();
    }
    await use(_sharedPage);
    // Don't close — reused across tests
  },
});
