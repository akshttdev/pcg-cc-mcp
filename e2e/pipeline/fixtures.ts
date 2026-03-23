/**
 * Pipeline E2E Test Fixtures
 *
 * Shared browser context and page for serial pipeline tests.
 * Same pattern as e2e/demos/fixtures.ts — one persistent session
 * across all tests in a worker.
 */
import { test as base, type BrowserContext, type Page } from "@playwright/test";

let _sharedContext: BrowserContext | undefined;
let _sharedPage: Page | undefined;

export const test = base.extend({
  context: async ({ browser }, use, testInfo) => {
    if (!_sharedContext) {
      _sharedContext = await browser.newContext({
        storageState: testInfo.project.use.storageState,
        viewport: testInfo.project.use.viewport ?? { width: 1440, height: 900 },
        baseURL: testInfo.project.use.baseURL,
      });
    }
    await use(_sharedContext);
  },

  page: async ({ context }, use) => {
    if (!_sharedPage || _sharedPage.isClosed()) {
      _sharedPage = await context.newPage();
    }
    await use(_sharedPage);
  },
});

export { expect } from "@playwright/test";
