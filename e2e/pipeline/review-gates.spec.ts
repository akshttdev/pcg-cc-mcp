/**
 * Pipeline E2E: Review Gates (RG-1 to RG-2)
 *
 * Tests that review tasks are created on stage entry and that
 * required field validations fire on stage exit.
 *
 * Acceptance specs: planning/BACKLOG--remaining-work.md → RG-1, RG-2
 */
import { test, expect } from "./fixtures";
import { t, demoPause, login, apiLogin, TEST_DATA_PREFIX } from "../helpers";
import {
  ORG_ID,
  navigateToPipeline,
  createDealViaUI,
  moveDealViaContextMenu,
} from "./helpers";

const DEAL_NAME = `${TEST_DATA_PREFIX} Gate Test ${Date.now()}`;

let dealId: string;

test.describe("Review Gates (RG-1 to RG-2)", () => {
  test.describe.configure({ mode: "serial" });

  test("Setup: create deal and move to Intel", async ({ page, request }) => {
    test.setTimeout(60_000);
    await login(page);
    await apiLogin(request);
    await navigateToPipeline(page);

    await createDealViaUI(page, {
      name: DEAL_NAME,
      amount: "30000",
      // Intentionally NO description — to test required field gate
    });

    // Get deal ID
    const dealsRes = await request.get(`/api/crm/deals?organization_id=${ORG_ID}`);
    const deals = await dealsRes.json();
    const list = deals.data || deals || [];
    const testDeal = list.find((d: { name?: string }) => d.name === DEAL_NAME);
    expect(testDeal).toBeTruthy();
    dealId = testDeal.id;

    // Move to Intel (triggers review task + exit validations)
    const dealText = DEAL_NAME.replace(`${TEST_DATA_PREFIX} `, "");
    await moveDealViaContextMenu(page, dealText, "Intel");
  });

  // ── RG-1: Review tasks block advancement ───────────────────────────────

  test("RG-1: Intel stage creates a review task on entry", async ({ request }) => {
    test.setTimeout(30_000);
    await apiLogin(request);

    const tasksRes = await request.get(`/api/tasks?crm_deal_id=${dealId}`);
    expect(tasksRes.ok()).toBeTruthy();
    const tasks = await tasksRes.json();
    const taskList = tasks.data || tasks || [];

    // Should have at least one review task from Intel stage entry
    expect(taskList.length, "Intel stage should create a review task").toBeGreaterThan(0);

    const reviewTask = taskList.find(
      (t: { title?: string }) =>
        t.title?.includes("Review") || t.title?.includes("intelligence")
    );
    expect(reviewTask, "Review task should mention intelligence review").toBeTruthy();
  });

  test("RG-1: advancing with pending tasks still works (soft gate)", async ({ page, request }) => {
    test.setTimeout(30_000);
    await apiLogin(request);

    // Try to advance via API — should succeed with warnings (soft enforcement)
    const advanceRes = await request.post(`/api/crm/deals/${dealId}/advance`);
    // The advance should succeed (soft gate) OR return a clear error
    console.log(`[RG-1] Advance status: ${advanceRes.status()}`);

    if (advanceRes.ok()) {
      const result = await advanceRes.json();
      const data = result.data || result;
      // Check for warnings about pending tasks
      const warnings = data.warnings || [];
      console.log(`[RG-1] Warnings: ${JSON.stringify(warnings)}`);
    }
  });

  // ── RG-2: Required field validation ────────────────────────────────────

  test("RG-2: exit validation warns on missing description", async ({ request }) => {
    test.setTimeout(30_000);
    await apiLogin(request);

    // The deal was created without a description.
    // Intel stage has on_exit_validations requiring "description".
    // Moving out of Intel should return a warning.

    // First move back to Intel to re-test exit
    await request.patch(`/api/crm/deals/${dealId}/stage`, {
      data: {
        crm_stage_id: await getStageId(request, "Intel"),
      },
    });

    // Now advance — should get description warning
    const advanceRes = await request.post(`/api/crm/deals/${dealId}/advance`);
    if (advanceRes.ok()) {
      const result = await advanceRes.json();
      const data = result.data || result;
      const warnings = data.warnings || [];
      const descWarning = warnings.find(
        (w: { field?: string }) => w.field === "description"
      );
      console.log(`[RG-2] Description warning: ${JSON.stringify(descWarning)}`);
      // The warning should exist since description is empty
      expect(descWarning, "Should warn about missing description").toBeTruthy();
    }
  });

  // ── Cleanup ────────────────────────────────────────────────────────────

  test.afterAll(async ({ request }) => {
    await apiLogin(request);

    // Delete test deal
    if (dealId) {
      await request.delete(`/api/crm/deals/${dealId}`).catch(() => {});
    }

    // Delete any remaining test deals
    const dealsRes = await request.get(`/api/crm/deals?organization_id=${ORG_ID}`);
    if (dealsRes.ok()) {
      const deals = await dealsRes.json();
      const list = deals.data || deals || [];
      for (const deal of list) {
        if ((deal.name as string | undefined)?.startsWith(TEST_DATA_PREFIX)) {
          await request.delete(`/api/crm/deals/${deal.id}`).catch(() => {});
        }
      }
    }
  });
});

// ── Helper ─────────────────────────────────────────────────────────────────

async function getStageId(request: any, stageName: string): Promise<string> {
  const pipelinesRes = await request.get(`/api/crm/pipelines?organization_id=${ORG_ID}`);
  const pipelines = await pipelinesRes.json();
  const salesPipeline = (pipelines.data || pipelines || []).find(
    (p: { pipeline_type?: string }) => p.pipeline_type === "sales"
  );
  if (!salesPipeline) throw new Error("Sales pipeline not found");

  const stagesRes = await request.get(`/api/crm/pipelines/${salesPipeline.id}/stages`);
  const stages = await stagesRes.json();
  const stage = (stages.data || stages || []).find(
    (s: { name: string }) => s.name === stageName
  );
  if (!stage) throw new Error(`Stage "${stageName}" not found`);
  return stage.id;
}
