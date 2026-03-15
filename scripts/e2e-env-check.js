#!/usr/bin/env node
/**
 * E2E Environment Pre-flight Check
 *
 * Validates that all required environment variables and services are
 * available before running Playwright tests. Exits non-zero on failure
 * so the test runner never starts in a broken state.
 *
 * Usage:
 *   node scripts/e2e-env-check.js           # check core requirements
 *   node scripts/e2e-env-check.js --demos   # also check demo requirements (GITHUB_TOKEN)
 *
 * Required env vars (loaded from .env automatically):
 *   BACKEND_PORT   — port the backend server listens on
 *   DATABASE_URL   — SQLite database path
 *
 * Required for --demos:
 *   GITHUB_TOKEN   — GitHub PAT for demo sandbox repo operations
 */

const http = require("http");
const fs = require("fs");
const path = require("path");

const isDemos = process.argv.includes("--demos");

// ── Load .env if present ────────────────────────────────────────────────────

const envPath = path.resolve(__dirname, "..", ".env");
if (fs.existsSync(envPath)) {
  const lines = fs.readFileSync(envPath, "utf-8").split("\n");
  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const eqIdx = trimmed.indexOf("=");
    if (eqIdx === -1) continue;
    const key = trimmed.slice(0, eqIdx).trim();
    const val = trimmed.slice(eqIdx + 1).trim();
    // Don't override existing env vars
    if (!process.env[key]) {
      process.env[key] = val;
    }
  }
}

// ── Checks ──────────────────────────────────────────────────────────────────

const errors = [];
const warnings = [];

const frontendPort = process.env.FRONTEND_PORT || "3000";
const backendPort = process.env.BACKEND_PORT;

if (!backendPort) {
  errors.push("BACKEND_PORT is not set. Add it to .env or export it.");
}

if (!process.env.DATABASE_URL) {
  warnings.push("DATABASE_URL is not set — backend may fail to start.");
}

if (isDemos && !process.env.GITHUB_TOKEN) {
  errors.push(
    "GITHUB_TOKEN is required for demo tests.\n" +
    "  Create a PAT at https://github.com/settings/tokens with repo scope.\n" +
    "  Export it: export GITHUB_TOKEN=ghp_..."
  );
}

// ── Check that servers are reachable ────────────────────────────────────────

function checkServer(label, port) {
  return new Promise((resolve) => {
    const req = http.get(`http://127.0.0.1:${port}`, (res) => {
      resolve(true);
    });
    req.on("error", () => resolve(false));
    req.setTimeout(2000, () => {
      req.destroy();
      resolve(false);
    });
  });
}

async function main() {
  // Print header
  console.log("\n  E2E Pre-flight Check\n");

  // Check frontend
  const frontendUp = await checkServer("Frontend", frontendPort);
  if (frontendUp) {
    console.log(`  ✓ Frontend reachable on :${frontendPort}`);
  } else {
    errors.push(
      `Frontend not reachable on :${frontendPort}.\n` +
      `  Start it with: pnpm run dev`
    );
  }

  // Check backend
  if (backendPort) {
    const backendUp = await checkServer("Backend", backendPort);
    if (backendUp) {
      console.log(`  ✓ Backend reachable on :${backendPort}`);
    } else {
      errors.push(
        `Backend not reachable on :${backendPort}.\n` +
        `  Start it with: pnpm run dev`
      );
    }
  }

  // Check Playwright installed
  try {
    require.resolve("@playwright/test");
    console.log("  ✓ Playwright installed");
  } catch {
    errors.push(
      "Playwright not installed.\n" +
      "  Run: pnpm run test:e2e:install"
    );
  }

  // Check browser binaries
  const browsersPath = path.resolve(__dirname, "..", "node_modules", "playwright-core", ".local-browsers");
  if (fs.existsSync(browsersPath)) {
    console.log("  ✓ Browser binaries present");
  } else {
    warnings.push(
      "Browser binaries may not be installed.\n" +
      "  Run: pnpm run test:e2e:install"
    );
  }

  // Demo-specific checks
  if (isDemos) {
    if (process.env.GITHUB_TOKEN) {
      console.log("  ✓ GITHUB_TOKEN set");
    }
  }

  // Print results
  if (warnings.length > 0) {
    console.log("");
    for (const w of warnings) {
      console.log(`  ⚠ ${w}`);
    }
  }

  if (errors.length > 0) {
    console.log("");
    for (const e of errors) {
      console.log(`  ✗ ${e}`);
    }
    console.log("\n  Pre-flight failed. Fix the issues above and retry.\n");
    process.exit(1);
  }

  console.log("\n  All checks passed — starting tests...\n");
}

main();
