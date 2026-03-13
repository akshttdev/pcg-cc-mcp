import { defineConfig, devices } from "@playwright/test";

/**
 * Playwright E2E configuration for ORCHA dashboard health checks.
 *
 * Auth: Logs in once via auth.setup.ts, reuses storage state for all tests.
 *
 * Prerequisites:
 *   - Backend running on BACKEND_PORT (default 3001)
 *   - Frontend running on FRONTEND_PORT (default 3000) with proxy to backend
 *
 * Environment variables:
 *   FRONTEND_PORT  — port the frontend dev server listens on (default 3000)
 *   E2E_HEADED     — set to "true" to run with visible browser (default headless)
 *   E2E_BROWSER    — "chromium" | "firefox" | "webkit" (default "chromium")
 *
 * Usage:
 *   npx playwright test                          # headless, chromium
 *   E2E_HEADED=true npx playwright test          # headed, slower for visual verification
 *   E2E_BROWSER=firefox npx playwright test      # different browser
 *   npx playwright test --ui                     # interactive UI mode
 *   npx playwright test -g "login"               # run tests matching pattern
 */
const headed = process.env.E2E_HEADED === "true";
const browser = process.env.E2E_BROWSER || "chromium";

const browserDeviceMap: Record<string, string> = {
  chromium: "Desktop Chrome",
  firefox: "Desktop Firefox",
  webkit: "Desktop Safari",
};

const authFile = "e2e/.auth/user.json";

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: false, // run sequentially — tests share auth state
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: 1,
  reporter: process.env.CI ? "github" : "list",
  timeout: headed ? 60_000 : 30_000,

  use: {
    baseURL: `http://127.0.0.1:${process.env.FRONTEND_PORT || 3000}`,
    headless: !headed,
    // Slow down actions in headed mode so devs can visually follow along
    ...(headed && { launchOptions: { slowMo: 250 } }),
    trace: "on-first-retry",
    screenshot: "only-on-failure",
    video: headed ? "on" : "retain-on-failure",
  },

  projects: [
    // Auth setup — runs once, saves storage state for all other projects
    {
      name: "setup",
      testMatch: /auth\.setup\.ts/,
    },

    // Main test suite — depends on auth setup
    {
      name: browser,
      dependencies: ["setup"],
      use: {
        ...devices[browserDeviceMap[browser] || "Desktop Chrome"],
        storageState: authFile,
      },
    },
  ],

  // Don't auto-start servers — expect them to be running already.
  // Start with: BACKEND_PORT=3001 pnpm run dev
});
