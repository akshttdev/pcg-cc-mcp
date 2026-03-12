import { defineConfig, devices } from "@playwright/test";

/**
 * Playwright E2E configuration for ORCHA dashboard health checks.
 *
 * Prerequisites:
 *   - Backend running on BACKEND_PORT (default 3001)
 *   - Frontend running on FRONTEND_PORT (default 3000) with proxy to backend
 *
 * Environment variables:
 *   FRONTEND_PORT  — port the frontend dev server listens on (default 3000)
 *   E2E_HEADED     — set to "true" to run tests with visible browser (default headless)
 *
 * Usage:
 *   npx playwright test              # run all tests (headless)
 *   E2E_HEADED=true npx playwright test  # run with visible browser
 *   npx playwright test --ui         # interactive UI mode
 *   npx playwright test -g "login"   # run tests matching pattern
 */
const headed = process.env.E2E_HEADED === "true";

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: false, // run sequentially — tests share auth state
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: 1,
  reporter: process.env.CI ? "github" : "list",
  timeout: 30_000,

  use: {
    baseURL: `http://127.0.0.1:${process.env.FRONTEND_PORT || 3000}`,
    headless: !headed,
    trace: "on-first-retry",
    screenshot: "only-on-failure",
    video: "retain-on-failure",
  },

  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],

  // Don't auto-start servers — expect them to be running already.
  // Start with: BACKEND_PORT=3001 pnpm run dev
});
