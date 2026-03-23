/**
 * Pipeline E2E: Pipeline Configuration (PC-1 to PC-3)
 *
 * Tests pipeline settings UI accessibility, stage config editing,
 * and that all pipelines have correct configs.
 *
 * Acceptance specs: planning/BACKLOG--remaining-work.md → PC-1 to PC-3
 */
import { test, expect } from "@playwright/test";
import { t, demoPause, login } from "../helpers";
import { navigateToPipeline, openPipelineSettings } from "./helpers";

test.describe("Pipeline Configuration (PC-1 to PC-3)", () => {
  test.describe.configure({ mode: "serial" });

  test.beforeEach(async ({ page }) => {
    await login(page);
    await navigateToPipeline(page);
  });

  // ── PC-1: Pipeline settings accessible from board ──────────────────────

  test("PC-1: gear icon opens Pipeline Settings dialog", async ({ page }) => {
    test.setTimeout(30_000);

    await openPipelineSettings(page);

    // Verify dialog opened with expected structure
    await expect(
      page.getByRole("heading", { name: "Pipeline Settings" })
    ).toBeVisible({ timeout: t(5_000) });

    // Verify pipeline dropdown is present
    await expect(page.getByRole("combobox").first()).toBeVisible();

    // Verify three tabs inside the settings dialog
    const settingsDialog = page.locator('[role="dialog"]').last();
    await expect(settingsDialog.getByRole("tab", { name: "Stages" })).toBeVisible();
    await expect(settingsDialog.getByRole("tab", { name: "Automations" })).toBeVisible();
    await expect(settingsDialog.getByRole("tab", { name: "Pipeline" })).toBeVisible();
  });

  test("PC-1: can switch between pipelines via dropdown", async ({ page }) => {
    test.setTimeout(30_000);

    // Open dropdown
    await page.getByRole("combobox").first().click();

    // Verify multiple pipeline options exist
    const options = page.getByRole("option");
    const count = await options.count();
    expect(count, "Should have multiple pipeline options").toBeGreaterThan(1);

    // Select Sirak Studios Acquisition
    await page.getByRole("option", { name: /Sirak Studios Acquisition/i }).click();

    // Verify it shows 9 stages
    await expect(page.getByText("9 stages")).toBeVisible({ timeout: t(5_000) });
  });

  test("PC-1: Automations tab shows full pipeline overview", async ({ page }) => {
    test.setTimeout(30_000);

    await page.getByRole("tab", { name: "Automations" }).click();

    // Verify key stages are visible with their agents
    await expect(page.getByText("Intel").first()).toBeVisible();
    await expect(page.getByText("Scout").first()).toBeVisible();
    await expect(page.getByText("Business Analysis").first()).toBeVisible();
    await expect(page.getByText("Astra").first()).toBeVisible();
    await expect(page.getByText("Proposal").first()).toBeVisible();
    await expect(page.getByText("Cash").first()).toBeVisible();

    // Verify auto-trigger badges
    await expect(page.getByText("auto-trigger").first()).toBeVisible();
  });

  // ── PC-2: Stage config controls pipeline behavior ──────────────────────

  test("PC-2: edit stage dialog has three tabs", async ({ page }) => {
    test.setTimeout(30_000);

    // Switch to Stages tab
    await page.getByRole("tab", { name: "Stages" }).click();

    // Click edit on Intel stage (second stage, pencil button)
    // Find the Intel row and click its edit button
    const intelRow = page.locator('div').filter({ hasText: /^Intel\s+20%/ }).first();
    await expect(intelRow).toBeVisible({ timeout: t(5_000) });

    // Click the pencil/edit button in the Intel row
    const editButtons = page.locator('button:has(svg)');
    // We need to find the edit button for the Intel stage specifically
    // The pattern is: row has stage name, then buttons for up/down/edit/delete
    const intelSection = page.locator('div.rounded-lg').filter({ hasText: "Intel" }).filter({ hasText: "20%" });
    const editBtn = intelSection.locator('button').nth(2); // 0=up, 1=down, 2=edit, 3=delete
    await editBtn.click();

    // Verify Edit Stage dialog opens with three tabs
    await expect(
      page.getByRole("heading", { name: "Edit Stage" })
    ).toBeVisible({ timeout: t(5_000) });

    await expect(page.getByRole("tab", { name: "Stage" })).toBeVisible();
    await expect(page.getByRole("tab", { name: "Automation" })).toBeVisible();
    await expect(page.getByRole("tab", { name: "Active Rules" })).toBeVisible();
  });

  test("PC-2: Automation tab shows correct agent and checkboxes", async ({ page }) => {
    test.setTimeout(30_000);

    // Click Automation tab
    await page.getByRole("tab", { name: "Automation" }).click();

    // Verify Scout is assigned
    await expect(page.getByText("Scout (Research)")).toBeVisible({ timeout: t(5_000) });

    // Verify auto-trigger is checked
    const autoTrigger = page.getByRole("checkbox", { name: /auto-start agent/i });
    await expect(autoTrigger).toBeChecked();

    // Verify Description is checked in required fields (derived from exit validation)
    const descCheckbox = page.getByRole("checkbox", { name: /description.*operator/i });
    await expect(descCheckbox).toBeChecked();

    // Verify approval gate is checked (derived from review task)
    const approvalGate = page.getByRole("checkbox", { name: /require approval/i });
    await expect(approvalGate).toBeChecked();
  });

  test("PC-2: Active Rules tab matches Automation tab", async ({ page }) => {
    test.setTimeout(30_000);

    // Click Active Rules tab
    await page.getByRole("tab", { name: "Active Rules" }).click();

    // Verify on-enter actions
    await expect(page.getByText(/trigger scout.*research/i).first()).toBeVisible({ timeout: t(5_000) });
    await expect(page.getByText(/review task.*intelligence/i).first()).toBeVisible();

    // Verify exit validations
    await expect(page.getByText(/require.*description/i).first()).toBeVisible();
    await expect(page.getByText(/person.*intel/i).first()).toBeVisible();
    await expect(page.getByText(/company.*intel/i).first()).toBeVisible();

    // Verify stage owner
    await expect(page.getByText("Scout").first()).toBeVisible();
    await expect(page.getByText("agent").first()).toBeVisible();

    // Close the edit dialog
    await page.keyboard.press("Escape");
  });

  // ── PC-3: All pipelines have correct configs ───────────────────────────

  test("PC-3: Clients pipeline has agent assignments", async ({ page }) => {
    test.setTimeout(30_000);

    // Switch to Clients pipeline
    await page.getByRole("combobox").first().click();
    await page.getByRole("option", { name: /Clients\s+clients/i }).click();
    await expect(page.getByText("8 stages")).toBeVisible({ timeout: t(5_000) });

    // Click Automations tab
    await page.getByRole("tab", { name: "Automations" }).click();

    // Verify key agents
    await expect(page.getByText("Astra").first()).toBeVisible();
    await expect(page.getByText("Cash").first()).toBeVisible();
    await expect(page.getByText("Lux").first()).toBeVisible();
  });

  test("PC-3: Client Delivery pipeline has PM ownership", async ({ page }) => {
    test.setTimeout(30_000);

    // Switch to Client Delivery pipeline
    await page.getByRole("combobox").first().click();
    await page.getByRole("option", { name: /Client Delivery/i }).click();
    await expect(page.getByText("7 stages")).toBeVisible({ timeout: t(5_000) });

    // Click Automations tab
    await page.getByRole("tab", { name: "Automations" }).click();

    // All delivery stages should show PM ownership
    const pmCount = await page.getByText("PM").count();
    expect(pmCount, "All delivery stages should be PM-owned").toBeGreaterThanOrEqual(7);
  });

  test("PC-3: Conferences pipeline has Scout on Researching", async ({ page }) => {
    test.setTimeout(30_000);

    // Switch to Conferences pipeline
    await page.getByRole("combobox").first().click();
    await page.getByRole("option", { name: /Conferences/i }).click();
    await expect(page.getByText("6 stages")).toBeVisible({ timeout: t(5_000) });

    // Click Automations tab
    await page.getByRole("tab", { name: "Automations" }).click();

    // Researching stage should have Scout
    await expect(page.getByText("Scout").first()).toBeVisible();
    await expect(page.getByText("auto-trigger").first()).toBeVisible();

    // Close settings dialog
    await page.keyboard.press("Escape");
  });
});
