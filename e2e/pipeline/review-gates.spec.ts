/**
 * Pipeline E2E: Review Gates (RG-1 to RG-2)
 *
 * MCP verified: Intel stage creates a review task ("Needs review" badge),
 * and the advance API returns warnings for missing description/intel.
 *
 * Acceptance specs: planning/BACKLOG--remaining-work.md → RG-1, RG-2
 */
import { test, expect } from "./fixtures";
import { t, demoPause, login, apiLogin, TEST_DATA_PREFIX } from "../helpers";
import { ORG_ID, PIPELINE_URL, moveDealViaContextMenu } from "./helpers";

let dealId: string;
let dealName: string;
let dealText: string;

test.describe("Review Gates (RG-1 to RG-2)", () => {
  test.describe.configure({ mode: "serial" });

  test("setup: create deal without description, move to Intel", async ({ page, request }) => {
    test.setTimeout(60_000);
    await apiLogin(request);
    await login(page);

    // Get pipeline + lead stage
    const pipelinesRes = await request.get(`/api/crm/pipelines?organization_id=${ORG_ID}`);
    const pipelines = (await pipelinesRes.json()).data || [];
    const salesPipeline = pipelines.find((p: { pipeline_type: string }) => p.pipeline_type === "sales");
    const stagesRes = await request.get(`/api/crm/pipelines/${salesPipeline.id}/stages`);
    const stages = (await stagesRes.json()).data || [];
    const leadStage = stages.find((s: { name: string }) => s.name === "Lead");

    // Create deal WITHOUT description (to test required field gate)
    dealName = `${TEST_DATA_PREFIX} Gate ${Date.now()}`;
    dealText = dealName.replace(`${TEST_DATA_PREFIX} `, "");
    const dealRes = await request.post("/api/crm/deals", {
      data: {
        organization_id: ORG_ID,
        crm_pipeline_id: salesPipeline.id,
        crm_stage_id: leadStage.id,
        name: dealName,
        amount: 30000,
        // NO description — intentional for RG-2 test
      },
    });
    const deal = (await dealRes.json()).data || (await dealRes.json());
    dealId = deal.id;

    // Navigate and move to Intel
    await page.goto(PIPELINE_URL);
    await expect(page.getByText("Acquisition Pipeline")).toBeVisible({ timeout: t(15_000) });
    await expect(page.getByText(dealText).first()).toBeVisible({ timeout: t(10_000) });
    await moveDealViaContextMenu(page, dealText, "Intel");
  });

  // ── RG-1: Review tasks gate deal advancement ───────────────────────────

  test("RG-1: Intel stage creates review task — 'Needs review' badge visible", async ({ page }) => {
    test.setTimeout(30_000);
    // MCP snapshot showed "Needs review" badge on the card after Intel move
    await expect(page.getByText("Needs review").first()).toBeVisible({ timeout: t(10_000) });
  });

  test("RG-1: review task linked to deal via API", async ({ request }) => {
    test.setTimeout(30_000);
    await apiLogin(request);

    const tasksRes = await request.get(`/api/tasks?crm_deal_id=${dealId}`);
    expect(tasksRes.ok()).toBeTruthy();
    const tasks = (await tasksRes.json()).data || [];
    expect(tasks.length, "Should have at least one review task").toBeGreaterThan(0);

    // Task should exist and not be done
    const task = tasks[0];
    expect(task.status).not.toBe("done");
    // Verify it's a review task (title should mention review/intel)
    expect((task.title || "").toLowerCase()).toMatch(/review|approve|intel/);
  });

  test("RG-1: advance API returns 400 with pending tasks", async ({ request }) => {
    test.setTimeout(30_000);
    await apiLogin(request);

    // Try to advance — should fail because of pending review tasks and missing description
    const advanceRes = await request.post(`/api/crm/deals/${dealId}/advance`);
    expect(advanceRes.status()).toBe(400);

    const body = await advanceRes.json();
    const message = body.message || "";
    // Should mention something about requirements not met
    expect(message.length, "Error message should explain why advance failed").toBeGreaterThan(10);
  });

  // ── RG-2: Required field validation on stage exit ──────────────────────

  test("RG-2: advance from Intel warns about missing description", async ({ request }) => {
    test.setTimeout(30_000);
    await apiLogin(request);

    // Complete review tasks first so we can see the description validation
    const tasksRes = await request.get(`/api/tasks?crm_deal_id=${dealId}`);
    if (tasksRes.ok()) {
      for (const task of ((await tasksRes.json()).data || [])) {
        if (task.status !== "done" && task.status !== "cancelled") {
          await request.put(`/api/tasks/${task.id}`, { data: { status: "done" } });
        }
      }
    }

    // Now advance — should fail on description requirement
    const advanceRes = await request.post(`/api/crm/deals/${dealId}/advance`);
    const body = await advanceRes.json();
    const message = (body.message || "").toLowerCase();

    // Should mention description/operator context/intelligence
    expect(
      message.includes("description") || message.includes("operator context") ||
      message.includes("intelligence") || message.includes("required"),
      `Advance error should mention a requirement: "${body.message}"`
    ).toBeTruthy();
  });

  // ── Cleanup ────────────────────────────────────────────────────────────

  test.afterAll(async ({ request }) => {
    await apiLogin(request);
    if (dealId) {
      await request.delete(`/api/crm/deals/${dealId}`).catch(() => {});
    }
  });
});
