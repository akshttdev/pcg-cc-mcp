/**
 * Minimal Playwright config for running quarantined tests.
 * Usage: npx playwright test --config=e2e/quarantine/playwright.quarantine.config.ts <test-file>
 */
import { defineConfig } from '@playwright/test';
import * as dotenv from 'dotenv';

dotenv.config();

const port = process.env.FRONTEND_PORT || '3000';

export default defineConfig({
  testDir: '.',
  timeout: 60_000,
  retries: 0,
  workers: 1,
  fullyParallel: false,
  reporter: 'list',
  use: {
    baseURL: `http://localhost:${port}`,
    headless: true,
    viewport: { width: 1280, height: 720 },
  },
});
