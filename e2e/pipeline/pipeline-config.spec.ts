/**
 * Pipeline E2E: Pipeline Configuration (PC-1 to PC-3)
 *
 * MCP verified: gear icon (data-testid="pipeline-settings") opens dialog
 * with dropdown pipeline selector, tabs: Stages/Automations/Pipeline.
 * Automations tab shows agents, triggers, actions per stage.
 *
 * Acceptance specs: planning/BACKLOG--remaining-work.md → PC-1 to PC-3
 */
import { test, expect } from "./fixtures";
import { t, demoPause, login } from "../helpers";
import { PIPELINE_URL } from "./helpers";

test.describe("Pipeline Configuration (PC-1 to PC-3)", () => {
  test.describe.configure({ mode: "serial" });

  // ── PC-1: Pipeline settings accessible from board ──────────────────────

  test("PC-1: gear icon opens Pipeline Settings dialog", async ({ page }) => {
    test.setTimeout(60_000);
    await login(page);
    await page.goto(PIPELINE_URL);
    await expect(page.getByText("Acquisition Pipeline")).toBeVisible({ timeout: t(15_000) });

    // Click gear icon
    await page.getByTestId("pipeline-settings").click();
    await page.waitForTimeout(demoPause.medium);

    // Then: dialog opens with "Pipeline Settings" heading
    await expect(page.getByRole("heading", { name: "Pipeline Settings" })).toBeVisible({ timeout: t(5_000) });

    // And: pipeline dropdown selector visible
    await expect(page.getByRole("combobox").first()).toBeVisible();

    // And: three tabs visible
    await expect(page.getByRole("tab", { name: "Stages" })).toBeVisible();
    await expect(page.getByRole("tab", { name: "Automations" })).toBeVisible();
    // "Pipeline" tab — use exact match to avoid conflict with page-level "Pipelines" tab
    await expect(page.getByTestId("tab-settings")).toBeVisible();
  });

  test("PC-1: can switch pipelines via dropdown", async ({ page }) => {
    test.setTimeout(30_000);

    // Open dropdown
    await page.getByRole("combobox").first().click();
    await page.waitForTimeout(demoPause.short);

    // Should have multiple options
    const options = page.getByRole("option");
    const count = await options.count();
    expect(count, "Should have multiple pipeline options").toBeGreaterThan(1);

    // Select Sirak Studios Acquisition
    await page.getByRole("option", { name: /Sirak Studios Acquisition/i }).click();
    await expect(page.getByText("9 stages")).toBeVisible({ timeout: t(5_000) });
  });

  test("PC-1: Automations tab shows agents and triggers", async ({ page }) => {
    test.setTimeout(30_000);

    await page.getByRole("tab", { name: "Automations" }).click();
    await page.waitForTimeout(demoPause.short);

    // MCP verified: Intel shows "Scout", BA shows "Astra", Proposal shows "Cash"
    await expect(page.getByText("Scout").first()).toBeVisible({ timeout: t(5_000) });
    await expect(page.getByText("Astra").first()).toBeVisible();
    await expect(page.getByText("Cash").first()).toBeVisible();
    await expect(page.getByText("auto-trigger").first()).toBeVisible();
  });

  // ── PC-2: Stage config controls pipeline behavior ──────────────────────

  test("PC-2: edit Intel stage shows agent config", async ({ page }) => {
    test.setTimeout(30_000);

    // Switch to Stages tab
    await page.getByRole("tab", { name: "Stages" }).click();
    await page.waitForTimeout(demoPause.short);

    // Find Intel row and click its edit button (pencil icon)
    // MCP showed stage rows with name + percentage + edit/delete buttons
    // The edit button is the 3rd button in each row (up, down, edit, delete)
    const intelRow = page.locator('div.rounded-lg').filter({ hasText: /Intel.*20%/ });
    await expect(intelRow).toBeVisible({ timeout: t(5_000) });

    // Click edit (3rd button — 0-indexed: 0=up, 1=down, 2=edit, 3=delete)
    await intelRow.locator('button').nth(2).click();
    await page.waitForTimeout(demoPause.short);

    // Edit Stage dialog should open with Stage/Automation/Active Rules tabs
    await expect(page.getByRole("heading", { name: "Edit Stage" })).toBeVisible({ timeout: t(5_000) });
    await expect(page.getByRole("tab", { name: "Stage" })).toBeVisible();
    await expect(page.getByRole("tab", { name: "Automation" })).toBeVisible();
    await expect(page.getByRole("tab", { name: "Active Rules" })).toBeVisible();
  });

  test("PC-2: Automation tab shows Scout assigned with auto-trigger", async ({ page }) => {
    test.setTimeout(30_000);

    await page.getByRole("tab", { name: "Automation" }).click();
    await page.waitForTimeout(demoPause.short);

    // MCP verified: Intel stage has Scout (Research) assigned, auto-trigger checked
    await expect(page.getByText("Scout (Research)")).toBeVisible({ timeout: t(5_000) });

    // Auto-trigger checkbox should be checked
    const autoTrigger = page.getByRole("checkbox", { name: /auto-start agent/i });
    await expect(autoTrigger).toBeChecked();

    // Description required field should be checked (derived from exit validation)
    const descCheckbox = page.getByRole("checkbox", { name: /description/i });
    await expect(descCheckbox).toBeChecked();

    // Approval gate should be checked (derived from review task)
    const approvalGate = page.getByRole("checkbox", { name: /require approval/i });
    await expect(approvalGate).toBeChecked();
  });

  test("PC-2: Active Rules tab shows on-enter actions and exit validations", async ({ page }) => {
    test.setTimeout(30_000);

    await page.getByRole("tab", { name: "Active Rules" }).click();
    await page.waitForTimeout(demoPause.short);

    // MCP verified: shows trigger scout, review task, require description, intel validations
    await expect(page.getByText(/trigger scout/i).first()).toBeVisible({ timeout: t(5_000) });
    await expect(page.getByText(/review task/i).first()).toBeVisible();
    await expect(page.getByText(/description/i).first()).toBeVisible();

    // Close edit dialog
    await page.getByRole("button", { name: "Close" }).first().click();
    await page.waitForTimeout(demoPause.short);
  });

  // ── PC-3: All pipelines have correct configs ───────────────────────────

  test("PC-3: Clients pipeline has agent assignments", async ({ page }) => {
    test.setTimeout(30_000);

    // Switch to Clients pipeline
    await page.getByRole("combobox").first().click();
    await page.getByRole("option", { name: /Clients\s+clients/i }).click();
    await expect(page.getByText("8 stages")).toBeVisible({ timeout: t(5_000) });

    // Automations tab
    await page.getByRole("tab", { name: "Automations" }).click();
    await page.waitForTimeout(demoPause.short);

    // Should have Astra, Cash, Lux
    await expect(page.getByText("Astra").first()).toBeVisible();
    await expect(page.getByText("Cash").first()).toBeVisible();
    await expect(page.getByText("Lux").first()).toBeVisible();
  });

  test("PC-3: Client Delivery pipeline has PM ownership", async ({ page }) => {
    test.setTimeout(30_000);

    await page.getByRole("combobox").first().click();
    await page.getByRole("option", { name: /Client Delivery/i }).click();
    await expect(page.getByText("7 stages")).toBeVisible({ timeout: t(5_000) });

    await page.getByRole("tab", { name: "Automations" }).click();
    await page.waitForTimeout(demoPause.short);

    // All delivery stages should show PM ownership
    const pmCount = await page.getByText("PM").count();
    expect(pmCount, "All delivery stages should be PM-owned").toBeGreaterThanOrEqual(7);
  });

  test("PC-3: Conferences pipeline has Scout on Researching", async ({ page }) => {
    test.setTimeout(30_000);

    await page.getByRole("combobox").first().click();
    await page.getByRole("option", { name: /Conferences/i }).click();
    await expect(page.getByText("6 stages")).toBeVisible({ timeout: t(5_000) });

    await page.getByRole("tab", { name: "Automations" }).click();
    await page.waitForTimeout(demoPause.short);

    await expect(page.getByText("Scout").first()).toBeVisible();
    await expect(page.getByText("auto-trigger").first()).toBeVisible();

    // Close settings
    await page.getByRole("button", { name: "Close" }).first().click();
  });
});
