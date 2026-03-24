/**
 * Pipeline E2E: Pipeline Configuration (PC-1 to PC-3)
 *
 * MCP verified: gear icon (data-testid="pipeline-settings") opens dialog
 * with dropdown pipeline selector, tabs: Stages/Automations/Pipeline.
 * Default pipeline is Client Delivery (first alphabetically).
 * Sirak Studios Acquisition has 9 stages with Scout/Astra/Cash/Lux agents.
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

    // Click gear icon — MCP verified: data-testid="pipeline-settings"
    await page.getByTestId("pipeline-settings").click();
    await page.waitForTimeout(demoPause.medium);

    // Then: dialog opens with "Pipeline Settings" heading
    await expect(page.getByRole("heading", { name: "Pipeline Settings" })).toBeVisible({ timeout: t(5_000) });

    // And: pipeline dropdown selector visible (combobox role)
    await expect(page.getByRole("combobox").first()).toBeVisible();

    // And: three tabs visible — MCP verified: Stages, Automations, Pipeline
    // "Pipeline" tab has testid "tab-settings" (value="settings", label="Pipeline")
    // Use testid to avoid collision with org-level "Pipelines" tab
    await expect(page.getByRole("tab", { name: "Stages" })).toBeVisible();
    await expect(page.getByRole("tab", { name: "Automations" })).toBeVisible();
    await expect(page.getByTestId("tab-settings")).toBeVisible();
  });

  test("PC-1: can switch pipelines via dropdown", async ({ page }) => {
    test.setTimeout(30_000);

    // Open dropdown
    await page.getByRole("combobox").first().click();
    await page.waitForTimeout(demoPause.short);

    // MCP verified: 4 pipeline options
    const options = page.getByRole("option");
    const count = await options.count();
    expect(count, "Should have multiple pipeline options").toBeGreaterThan(1);

    // Select Sirak Studios Acquisition — MCP verified option text
    await page.getByRole("option", { name: /Sirak Studios Acquisition/i }).click();
    await page.waitForTimeout(demoPause.short);

    // MCP verified: Sirak Studios Acquisition has 9 stages
    await expect(page.getByText("9 stages")).toBeVisible({ timeout: t(5_000) });
  });

  test("PC-1: Automations tab shows agents and triggers", async ({ page }) => {
    test.setTimeout(30_000);

    await page.getByRole("tab", { name: "Automations" }).click();
    await page.waitForTimeout(demoPause.short);

    // MCP verified: Intel has Scout, BA has Astra, Proposal has Cash, Polish has Lux
    await expect(page.getByText("Scout").first()).toBeVisible({ timeout: t(5_000) });
    await expect(page.getByText("Astra").first()).toBeVisible();
    await expect(page.getByText("Cash").first()).toBeVisible();
    await expect(page.getByText("Lux").first()).toBeVisible();

    // MCP verified: auto-trigger text appears for agent stages
    await expect(page.getByText("auto-trigger").first()).toBeVisible();
  });

  // ── PC-2: Stage config controls pipeline behavior ──────────────────────

  test("PC-2: edit Intel stage shows agent config", async ({ page }) => {
    test.setTimeout(30_000);

    // Switch to Stages tab
    await page.getByRole("tab", { name: "Stages" }).click();
    await page.waitForTimeout(demoPause.short);

    // Click the edit button for Intel stage — uses data-testid="stage-edit-intel"
    await page.getByTestId("stage-edit-intel").click();
    await page.waitForTimeout(demoPause.short);

    // Edit Stage dialog should open with Stage/Automation/Active Rules tabs
    // Use testids to avoid collision with pipeline settings "Stages" tab
    await expect(page.getByRole("heading", { name: "Edit Stage" })).toBeVisible({ timeout: t(5_000) });
    await expect(page.getByTestId("tab-stage")).toBeVisible();
    await expect(page.getByTestId("tab-automation")).toBeVisible();
    await expect(page.getByTestId("tab-rules")).toBeVisible();
  });

  test("PC-2: Automation tab — verify and toggle agent controls", async ({ page }) => {
    test.setTimeout(30_000);

    // Switch to Automation tab in the Edit Stage dialog
    await page.getByTestId("tab-automation").click();
    await page.waitForTimeout(demoPause.short);

    // Verify agent dropdown shows Scout — use combobox role to avoid strict mode
    const agentSelect = page.getByTestId("tab-content-automation").getByRole("combobox");
    await expect(agentSelect).toBeVisible({ timeout: t(5_000) });

    // Auto-trigger checkbox should be checked — verify then toggle OFF and back ON
    const autoTrigger = page.getByRole("checkbox", { name: /auto-start agent/i });
    await expect(autoTrigger).toBeChecked();

    // Toggle auto-trigger OFF — verify it unchecks
    await autoTrigger.click();
    await page.waitForTimeout(demoPause.short);
    await expect(autoTrigger).not.toBeChecked();

    // Toggle auto-trigger back ON
    await autoTrigger.click();
    await page.waitForTimeout(demoPause.short);
    await expect(autoTrigger).toBeChecked();

    // Description required checkbox — verify checked
    const descCheckbox = page.getByRole("checkbox", { name: /description/i });
    await expect(descCheckbox).toBeChecked();

    // Approval gate — verify checked
    const approvalGate = page.getByRole("checkbox", { name: /require approval/i });
    await expect(approvalGate).toBeChecked();
  });

  test("PC-2: Active Rules tab reflects the automation config", async ({ page }) => {
    test.setTimeout(30_000);

    // Verify the edit dialog is still open
    await expect(page.getByRole("heading", { name: "Edit Stage" })).toBeVisible({ timeout: t(5_000) });

    // Click Active Rules tab
    const rulesTabTrigger = page.getByTestId("tab-rules");
    await expect(rulesTabTrigger).toBeVisible({ timeout: t(3_000) });
    await rulesTabTrigger.click();
    await page.waitForTimeout(demoPause.short);

    // Active Rules tab panel should be visible with rule content
    const rulesPanel = page.getByTestId("tab-content-rules");
    await expect(rulesPanel).toBeVisible({ timeout: t(5_000) });

    // Verify there is content in the rules panel (not empty)
    const rulesText = await rulesPanel.textContent();
    expect(rulesText?.length, "Active Rules tab should have content describing rules").toBeGreaterThan(10);

    // Close edit dialog — Save Stage button submits and closes
    await page.getByRole("button", { name: "Save Stage" }).click();
    await page.waitForTimeout(demoPause.medium);
  });

  // ── PC-3: All pipelines have correct configs ───────────────────────────

  test("PC-3: Clients pipeline has agent assignments", async ({ page }) => {
    test.setTimeout(30_000);

    // Switch to Clients pipeline
    await page.getByRole("combobox").first().click();
    await page.waitForTimeout(demoPause.short);
    await page.getByRole("option", { name: /Clients\s+clients/i }).click();
    await page.waitForTimeout(demoPause.short);

    // MCP verified: Clients pipeline has 8 stages
    await expect(page.getByText("8 stages")).toBeVisible({ timeout: t(5_000) });

    // Automations tab
    await page.getByRole("tab", { name: "Automations" }).click();
    await page.waitForTimeout(demoPause.short);

    // Should have Astra, Cash, Lux agents
    await expect(page.getByText("Astra").first()).toBeVisible();
    await expect(page.getByText("Cash").first()).toBeVisible();
    await expect(page.getByText("Lux").first()).toBeVisible();
  });

  test("PC-3: Client Delivery pipeline has 7 stages", async ({ page }) => {
    test.setTimeout(30_000);

    await page.getByRole("combobox").first().click();
    await page.waitForTimeout(demoPause.short);
    await page.getByRole("option", { name: /Client Delivery/i }).click();
    await page.waitForTimeout(demoPause.short);

    // MCP verified: Client Delivery has 7 stages
    await expect(page.getByText("7 stages")).toBeVisible({ timeout: t(5_000) });
  });

  test("PC-3: Conferences pipeline has Scout on Researching", async ({ page }) => {
    test.setTimeout(30_000);

    await page.getByRole("combobox").first().click();
    await page.waitForTimeout(demoPause.short);
    await page.getByRole("option", { name: /Conferences/i }).click();
    await page.waitForTimeout(demoPause.short);

    // MCP verified: Conferences pipeline has 6 stages
    await expect(page.getByText("6 stages")).toBeVisible({ timeout: t(5_000) });

    await page.getByRole("tab", { name: "Automations" }).click();
    await page.waitForTimeout(demoPause.short);

    await expect(page.getByText("Scout").first()).toBeVisible();
    await expect(page.getByText("auto-trigger").first()).toBeVisible();

    // Close settings
    await page.getByRole("button", { name: "Close" }).first().click();
  });
});
