import { defineConfig, devices } from "@playwright/test";
import * as fs from "fs";
import * as path from "path";

// Load .env from project root so GITHUB_TOKEN and other vars are available to tests
const envPath = path.resolve(__dirname, ".env");
if (fs.existsSync(envPath)) {
  for (const line of fs.readFileSync(envPath, "utf-8").split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const eqIdx = trimmed.indexOf("=");
    if (eqIdx < 0) continue;
    const key = trimmed.slice(0, eqIdx).trim();
    const val = trimmed.slice(eqIdx + 1).trim();
    const unquoted = val.replace(/^['"]|['"]$/g, '');
    if (!process.env[key]) process.env[key] = unquoted; // don't override explicit env
  }
}

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
 *   E2E_SLOWMO     — milliseconds to slow down each action (default 250 in headed, 0 headless)
 *   E2E_BROWSER    — "chromium" | "firefox" | "webkit" (default "chromium")
 *   E2E_SCREENSHOTS — set to "true" to capture screenshots on failure (default off)
 *
 * Usage:
 *   npx playwright test                          # headless, chromium
 *   E2E_HEADED=true npx playwright test          # headed, slower for visual verification
 *   E2E_BROWSER=firefox npx playwright test      # different browser
 *   npx playwright test --ui                     # interactive UI mode
 *   npx playwright test -g "login"               # run tests matching pattern
 */
const headed = process.env.E2E_HEADED === "true";
const slowMo = process.env.E2E_SLOWMO
  ? parseInt(process.env.E2E_SLOWMO, 10)
  : headed ? 250 : 0;
const browser = process.env.E2E_BROWSER || "chromium";
const screenshots = process.env.E2E_SCREENSHOTS === "true";

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
    // Larger viewport for headed/demo mode so the full app is visible
    ...(headed && { viewport: { width: 1440, height: 900 } }),
    // Slow down actions so devs can visually follow along (E2E_SLOWMO, default 250ms headed)
    ...(slowMo > 0 && { launchOptions: { slowMo } }),
    trace: "on-first-retry",
    screenshot: screenshots ? "only-on-failure" : "off",
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
      testIgnore: /demos\/|quarantine\/|pipeline\//,
      use: {
        ...devices[browserDeviceMap[browser] || "Desktop Chrome"],
        storageState: authFile,
      },
    },

    // Pipeline specs — shared page, login in first test, no setup dependency
    {
      name: "pipeline",
      testMatch: /pipeline\/.+\.spec\.ts/,
      use: {
        ...devices[browserDeviceMap[browser] || "Desktop Chrome"],
      },
    },

    // Demo scripts — always headed, single browser window, no setup dependency
    {
      name: "demos",
      testMatch: /demos\/.+\.spec\.ts/,
      use: {
        ...devices[browserDeviceMap[browser] || "Desktop Chrome"],
        headless: false,
        viewport: { width: 1920, height: 1080 },
        launchOptions: { slowMo: slowMo || 250 },
      },
    },
  ],

  // Don't auto-start servers — expect them to be running already.
  // Start with: BACKEND_PORT=3001 pnpm run dev
});
