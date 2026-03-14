/**
 * Demo-specific Playwright fixtures.
 *
 * Demos always run headed with a single persistent browser window.
 * The browser context and page are shared across ALL tests within a worker —
 * no open/close between tests, so the viewer sees one continuous session.
 *
 * Demo spec files should import from this file:
 *   import { test, expect } from "./fixtures";
 */
import { test as base, expect, BrowserContext, Page } from "@playwright/test";

export { expect };

// Module-level state — persists across all tests within a worker
let _sharedContext: BrowserContext | undefined;
let _sharedPage: Page | undefined;

export const test = base.extend({
  context: async ({ browser }, use, testInfo) => {
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
    // Never close — shared across all demo tests
  },

  page: async ({ context }, use) => {
    if (!_sharedPage || _sharedPage.isClosed()) {
      _sharedPage = await context.newPage();
    }
    await use(_sharedPage);
    // Never close — shared across all demo tests
  },
});
