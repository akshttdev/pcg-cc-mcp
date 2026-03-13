import { test, expect } from "./fixtures";
import { setViewAsRole, clearViewAsRole, getViewAsRole, settingsTab, waitForSettingsReady, ensureAuthenticated, t } from "./helpers";

/**
 * ORCHA Dashboard — RBAC & View-As E2E Tests
 *
 * Auth is handled once by auth.setup.ts — all tests receive storageState.
 *
 * Tests the role-based access control system:
 *   1. Sidebar user card & view-as popover
 *   2. View-as role selection & banner
 *   3. Sidebar section visibility per role
 *   4. localStorage persistence
 *   5. Settings scope tabs & permissions
 *   6. Mobile navbar avatar
 *   7. Edge cases
 */

// ─── Sidebar User Card ─────────────────────────────────────────────────────

test.describe("Sidebar User Card", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("domcontentloaded");
  });

  test("user card shows name and role in sidebar", async ({ page }) => {
    await expect(page.getByText("Administrator")).toBeVisible({ timeout: 5_000 });
    await expect(page.getByText("Platform Admin")).toBeVisible();
  });

  test("clicking user card opens view-as popover", async ({ page }) => {
    await page.locator("button", { hasText: "Administrator" }).click();

    await expect(page.getByText("View as...")).toBeVisible({ timeout: 3_000 });
    await expect(page.getByText("Sign out")).toBeVisible();
  });

  test("popover shows grouped role options", async ({ page }) => {
    await page.locator("button", { hasText: "Administrator" }).click();
    await expect(page.getByText("View as...")).toBeVisible({ timeout: 3_000 });

    // Check group headers exist
    await expect(page.getByText("Platform", { exact: true }).first()).toBeVisible();
    await expect(page.getByText("Organization", { exact: true }).first()).toBeVisible();
    await expect(page.getByText("Client", { exact: true }).first()).toBeVisible();

    // Check specific roles
    await expect(page.getByText("Platform Member")).toBeVisible();
    await expect(page.getByText("Org Admin")).toBeVisible();
    await expect(page.getByText("Org Editor").first()).toBeVisible();
    await expect(page.getByText("Org Viewer")).toBeVisible();
    await expect(page.getByText("Client Admin")).toBeVisible();
  });
});

// ─── View-As Banner ─────────────────────────────────────────────────────────

test.describe("View-As Banner", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("domcontentloaded");
  });

  test.afterEach(async ({ page }) => {
    await clearViewAsRole(page);
  });

  test("selecting a role shows the override banner", async ({ page }) => {
    await page.locator("button", { hasText: "Administrator" }).click();
    await expect(page.getByText("View as...")).toBeVisible({ timeout: 3_000 });
    await page.locator("button", { hasText: "Org Editor" }).first().click();

    const banner = page.getByTestId("view-as-banner");
    await expect(banner).toBeVisible({ timeout: 10_000 });
    await expect(banner.getByText("Viewing as")).toBeVisible();
    await expect(banner.getByText("Org Editor")).toBeVisible();
  });

  test("banner has a working reset button", async ({ page }) => {
    await setViewAsRole(page, "org_editor");

    const banner = page.getByTestId("view-as-banner");
    await expect(banner).toBeVisible({ timeout: 10_000 });

    await banner.getByText("Reset").click();

    await expect(banner).not.toBeVisible({ timeout: 5_000 });
    const role = await getViewAsRole(page);
    expect(role).toBeNull();
  });
});

// ─── Sidebar Visibility ─────────────────────────────────────────────────────

test.describe("Sidebar Visibility by Role", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("domcontentloaded");
  });

  test.afterEach(async ({ page }) => {
    await clearViewAsRole(page);
  });

  test("admin sees all sidebar sections", async ({ page }) => {
    await expect(page.getByText("Admin Platforms")).toBeVisible({ timeout: 5_000 });
    await expect(page.getByText("Management")).toBeVisible();
    await expect(page.getByText("Global Views")).toBeVisible();
    await expect(page.getByText("My Workspace")).toBeVisible();
  });

  test("org_editor cannot see admin-only sidebar sections", async ({ page }) => {
    await setViewAsRole(page, "org_editor");

    // Admin sections are hidden via animated wrapper (opacity-0 + max-h-0 + overflow-hidden).
    // Playwright doesn't treat opacity:0 ancestors as "hidden", so we verify
    // the wrapper's computed styles directly.
    for (const label of ["Admin Platforms", "Management", "Global Views"]) {
      const isHidden = await page.getByText(label).evaluate((el) => {
        let n: Element | null = el;
        while (n && n !== document.body) {
          const cs = getComputedStyle(n);
          if (cs.opacity === "0" && cs.maxHeight === "0px") return true;
          n = n.parentElement;
        }
        return false;
      });
      expect(isHidden, `"${label}" should be visually hidden`).toBe(true);
    }

    // User section should still be visible
    await expect(page.getByText("My Workspace")).toBeVisible();
  });

  test("org_editor sidebar shows correct role label", async ({ page }) => {
    await setViewAsRole(page, "org_editor");

    // The role label in the user card at the bottom of the sidebar
    await expect(page.getByText("Org Editor").first()).toBeVisible({ timeout: 3_000 });
  });
});

// ─── localStorage Persistence ───────────────────────────────────────────────

test.describe("View-As Persistence", () => {
  test.afterEach(async ({ page }) => {
    await clearViewAsRole(page);
  });

  test("view-as role persists across page reload", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("domcontentloaded");
    await setViewAsRole(page, "org_editor");

    await expect(page.getByTestId("view-as-banner")).toBeVisible({ timeout: 10_000 });

    await page.reload({ waitUntil: "domcontentloaded" });
    await page.waitForLoadState("networkidle");

    await expect(page.getByTestId("view-as-banner")).toBeVisible({ timeout: 10_000 });
    const role = await getViewAsRole(page);
    expect(role).toBe("org_editor");
  });
});

// ─── Settings Scope Tabs ────────────────────────────────────────────────────

test.describe("Settings Scope Tabs", () => {

  test.afterEach(async ({ page }) => {
    await clearViewAsRole(page);
  });

  test("admin sees all 4 scope tabs", async ({ page }) => {
    await page.goto("/settings/general");
    await waitForSettingsReady(page);

    await expect(settingsTab(page, "User")).toBeVisible();
    await expect(settingsTab(page, "Admin")).toBeVisible();
    await expect(settingsTab(page, "Org")).toBeVisible();
    await expect(settingsTab(page, "Client")).toBeVisible();
  });

  test("switching tabs changes visible settings items", async ({ page }) => {
    await page.goto("/settings/general");
    await waitForSettingsReady(page);

    // User tab should show General, Wallet, Profile
    await expect(page.locator("aside").getByText("General")).toBeVisible();
    await expect(page.locator("aside").getByText("Wallet")).toBeVisible();
    await expect(page.locator("aside").getByText("Profile")).toBeVisible();

    // Click Org tab
    await settingsTab(page, "Org").click();

    // Org tab should show Agents, Models, MCP Servers
    await expect(page.locator("aside").getByText("Agents")).toBeVisible({ timeout: 5_000 });
    await expect(page.locator("aside").getByText("Models")).toBeVisible();
    await expect(page.locator("aside").getByText("MCP Servers")).toBeVisible();
  });

  test("tab selection persists in URL as ?scope param", async ({ page }) => {
    await page.goto("/settings/general");
    await waitForSettingsReady(page);

    await settingsTab(page, "Admin").click();
    await expect(page).toHaveURL(/scope=system/, { timeout: 5_000 });

    await settingsTab(page, "Org").click();
    await expect(page).toHaveURL(/scope=org/, { timeout: 5_000 });

    await settingsTab(page, "User").click();
    // Wait for URL to update — User tab clears the scope param
    await expect(page).not.toHaveURL(/scope=/, { timeout: 5_000 });
  });

  test("direct URL ?scope=org loads correct tab", async ({ page }) => {
    await page.goto("/settings/general?scope=org");
    await waitForSettingsReady(page);

    await expect(page.locator("aside").getByText("Agents")).toBeVisible();
    await expect(page.locator("aside").getByText("MCP Servers")).toBeVisible();
  });

  test("org_editor cannot see Admin tab", async ({ page }) => {
    await setViewAsRole(page, "org_editor");

    await page.goto("/settings/general");
    await waitForSettingsReady(page);

    // Admin tab should not exist in aside
    await expect(settingsTab(page, "Admin")).toHaveCount(0);

    // User and Org tabs should be visible
    await expect(settingsTab(page, "User")).toBeVisible();
    await expect(settingsTab(page, "Org")).toBeVisible();
  });

  test("permission guard: ?scope=system falls back for non-admin", async ({ page }) => {
    await setViewAsRole(page, "org_editor");

    await page.goto("/settings/general?scope=system");
    await waitForSettingsReady(page);

    // Should fall back to User tab content
    await expect(page.locator("aside").getByText("Wallet")).toBeVisible({ timeout: 3_000 });
    await expect(page.locator("aside").getByText("Profile")).toBeVisible();

    // Admin tab should not exist
    await expect(settingsTab(page, "Admin")).toHaveCount(0);
  });

  test("planned items show Coming soon badge", async ({ page }) => {
    await page.goto("/settings/general");
    await waitForSettingsReady(page);

    await expect(page.getByText("Coming soon").first()).toBeVisible();
  });
});

// ─── Mobile Navbar Avatar ───────────────────────────────────────────────────

test.describe("Mobile Navbar Avatar", () => {
  test.beforeEach(async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 812 });
    await page.goto("/");
    await page.waitForLoadState("domcontentloaded");
  });

  test.afterEach(async ({ page }) => {
    await clearViewAsRole(page);
    // Restore default desktop viewport
    await page.setViewportSize({ width: 1280, height: 720 });
  });

  test("avatar button visible in navbar on mobile", async ({ page }) => {
    const avatar = page.locator('button[aria-label="User menu"]');
    await expect(avatar).toBeVisible({ timeout: 5_000 });
  });

  test("avatar popover opens with view-as options", async ({ page }) => {
    await page.locator('button[aria-label="User menu"]').click();

    await expect(page.getByText("View as...")).toBeVisible({ timeout: 3_000 });
    await expect(page.getByText("Platform Member")).toBeVisible();
    await expect(page.getByText("Sign out")).toBeVisible();
  });
});

test.describe("Desktop hides mobile avatar", () => {
  test("avatar button not visible on desktop", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("domcontentloaded");
    const avatar = page.locator('button[aria-label="User menu"]');
    await expect(avatar).not.toBeVisible();
  });
});

// ─── Edge Cases ─────────────────────────────────────────────────────────────

test.describe("View-As Edge Cases", () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test.afterEach(async ({ page }) => {
    await clearViewAsRole(page);
  });

  test("setting view-as to own role shows no banner", async ({ page }) => {
    await setViewAsRole(page, "platform_admin");
    await expect(page.getByTestId("view-as-banner")).not.toBeVisible();
  });

  test("invalid role in localStorage shows no banner", async ({ page }) => {
    await setViewAsRole(page, "garbage_role_xyz");
    await expect(page.getByTestId("view-as-banner")).not.toBeVisible();
  });

  // Skip in headed mode — shared context accumulates sidebar state that
  // prevents "Admin Platforms" from rendering after 60+ prior tests.
  // Passes reliably in headless mode (fresh context per test).
  const headed = process.env.E2E_HEADED === "true";
  (headed ? test.skip : test)("higher role than actual is ignored", async ({ page }) => {
    await setViewAsRole(page, "platform_admin");
    await expect(page.getByText("Admin Platforms")).toBeVisible({ timeout: t(5_000) });
    await expect(page.getByText("Management")).toBeVisible();
  });
});
