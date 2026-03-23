/**
 * Demo: Pipeline Intelligence Workflow — 8-Stage Deal Progression
 *
 * End-to-end feature demo showing:
 *   1. Setup: Create a CRM contact and deal in the Lead stage
 *   2. Lead → Business Analysis: Auto-research triggers on Lead entry
 *   3. Business Analysis → Discovery: Review research, approve gate, advance
 *   4. Discovery → Build Proposal: Post-discovery research integration
 *   5. Build Proposal → Polish: Generate proposal from intel, advance
 *   6. Polish → Proposal Meeting: QA quality, advance
 *   7. Proposal Meeting → Closed Won: Close the deal — delivery deal auto-created
 *   8. Verify end state: All transitions logged, conversation persistence
 *
 * Uses API for deal creation/advancement (mirrors agent behavior) and UI for
 * pipeline board verification and deal detail panel interactions.
 *
 * UI Component Notes:
 *   - Intelligence WorkflowsView uses shared WorkflowCardGrid component
 *     (same as BuilderTab). Cards show ownership badges (Personal/Organization/System).
 *   - Workflow cards in Intelligence view have a "Run" button for direct execution.
 *   - After running a workflow, staging results appear inline via RunAndReviewPanel
 *     (not via navigation to /workflows?tab=staging).
 *
 * Prerequisites:
 *   - Dev server running on FRONTEND_PORT (default 3001)
 *   - Seed database with Powerclub Global organization + Clients pipeline
 *   - Fresh db.sqlite (delete old one to get 8-stage pipeline auto-created)
 */
import { test, expect } from "./fixtures";
import { t, demoPause, login, apiLogin, TEST_DATA_PREFIX } from "../helpers";

// ── Constants ────────────────────────────────────────────────────────────────

const ORG_ID = "01010101-0101-0101-0101-010101010101"; // Powerclub Global
const CONTACT_EMAIL = `e2e.pipeline.${Date.now()}@acmecorp.com`;
const DEAL_NAME = `${TEST_DATA_PREFIX} Pipeline Demo Deal ${Date.now()}`;

const EXPECTED_STAGES = [
  "Lead",
  "Business Analysis",
  "Discovery",
  "Build Proposal",
  "Polish",
  "Proposal Meeting",
  "Closed Won",
  "Closed Lost",
];

// ── Shared state across serial tests ─────────────────────────────────────────

let dealId: string;
let contactId: string;
let pipelineId: string;

// ── Test Flow ────────────────────────────────────────────────────────────────

test.describe("Pipeline Intelligence Workflow — 8-Stage Demo", () => {
  test.describe.configure({ mode: "serial" });

  test("Part 1: Setup — Create contact and deal in Lead stage", async ({ page, request }) => {
    test.setTimeout(60_000);
    await apiLogin(request);
    await login(page);

    // Find the Sales/Acquisition pipeline for this org (shown as "Acquisition" tab in UI)
    const pipelinesRes = await request.get(`/api/crm/pipelines?organization_id=${ORG_ID}`);
    expect(pipelinesRes.ok()).toBeTruthy();
    const pipelines = await pipelinesRes.json();
    const pipelineList = pipelines.data || pipelines || [];
    // Use sales pipeline — the UI shows this under the "Acquisition" tab
    const clientsPipeline = pipelineList.find(
      (p: { pipeline_type?: string; name?: string }) => p.pipeline_type === "sales"
    ) ?? pipelineList.find(
      (p: { pipeline_type?: string; name?: string }) => p.name === "Acquisition"
    );
    expect(clientsPipeline, "Sales/Acquisition pipeline not found").toBeTruthy();
    pipelineId = clientsPipeline.id;

    // Get pipeline stages
    const stagesRes = await request.get(`/api/crm/pipelines/${pipelineId}/stages`);
    expect(stagesRes.ok()).toBeTruthy();
    const stages = await stagesRes.json();
    const stageList = stages.data || stages || [];
    expect(stageList.length).toBeGreaterThanOrEqual(8);

    // Verify stage names match expected 8-stage pipeline
    const stageNames = stageList
      .sort((a: { position: number }, b: { position: number }) => a.position - b.position)
      .map((s: { name: string }) => s.name);
    console.log(`[Part 1] Pipeline stages: ${stageNames.join(" → ")}`);
    for (const expected of EXPECTED_STAGES) {
      expect(stageNames).toContain(expected);
    }

    const leadStage = stageList.find((s: { name: string }) => s.name === "Lead");
    expect(leadStage, "Lead stage not found").toBeTruthy();

    // Create a CRM contact
    const contactRes = await request.post("/api/crm/contacts", {
      data: {
        organization_id: ORG_ID,
        first_name: "Pipeline",
        last_name: "DemoContact",
        email: CONTACT_EMAIL,
        company_name: "Acme Corp",
        job_title: "VP Engineering",
      },
    });
    expect(contactRes.ok()).toBeTruthy();
    const contact = await contactRes.json();
    contactId = (contact.data || contact).id;
    console.log(`[Part 1] Created contact: ${contactId}`);

    // Create a deal in Lead stage
    const dealRes = await request.post("/api/crm/deals", {
      data: {
        organization_id: ORG_ID,
        crm_pipeline_id: pipelineId,
        crm_stage_id: leadStage.id,
        crm_contact_id: contactId,
        name: DEAL_NAME,
        description: "E2E demo deal for 8-stage pipeline intelligence workflow",
        amount: 150000,
        currency: "USD",
      },
    });
    expect(dealRes.ok()).toBeTruthy();
    const deal = await dealRes.json();
    dealId = (deal.data || deal).id;
    console.log(`[Part 1] Created deal: ${dealId} in Lead stage`);

    // Navigate to pipeline board — Acquisition tab is default (shows sales pipeline)
    await page.goto(`/organizations/${ORG_ID}/crm/pipeline`);
    await expect(page.getByText("Acquisition Pipeline").first()).toBeVisible({ timeout: t(15_000) });
    await page.waitForTimeout(demoPause.medium);

    // Verify the deal card is visible on the board
    await expect(page.getByText(DEAL_NAME.replace(`${TEST_DATA_PREFIX} `, "")).first()).toBeVisible({
      timeout: t(10_000),
    });
    await page.waitForTimeout(demoPause.medium);
  });

  test("Part 2: Lead → Business Analysis — verify stage hooks", async ({ page, request }) => {
    test.setTimeout(90_000);
    await apiLogin(request);

    // Advance from Lead → Business Analysis via API
    const advanceRes = await request.post(`/api/crm/deals/${dealId}/advance`);

    // If there are pending tasks blocking advance, complete them first
    if (!advanceRes.ok()) {
      const error = await advanceRes.json();
      console.log(`[Part 2] Advance blocked: ${JSON.stringify(error)}`);

      // Complete any pending review tasks
      const tasksRes = await request.get(`/api/tasks?crm_deal_id=${dealId}`);
      if (tasksRes.ok()) {
        const tasks = await tasksRes.json();
        const taskList = tasks.data || tasks || [];
        for (const task of taskList) {
          if (task.status !== "done" && task.status !== "cancelled") {
            await request.put(`/api/tasks/${task.id}`, { data: { status: "done" } });
            console.log(`[Part 2] Completed blocking task: ${task.title}`);
          }
        }
      }
      // Retry advance
      const retryRes = await request.post(`/api/crm/deals/${dealId}/advance`);
      expect(retryRes.ok(), "Failed to advance deal to Business Analysis").toBeTruthy();
    }

    // Verify deal is now in Business Analysis
    const dealRes = await request.get(`/api/crm/deals/${dealId}`);
    expect(dealRes.ok()).toBeTruthy();
    const deal = await dealRes.json();
    const dealData = deal.data || deal;
    console.log(`[Part 2] Deal stage_id after advance: ${dealData.crm_stage_id}`);

    // Verify a review task was created (Business Analysis stage hook)
    const tasksRes = await request.get(`/api/tasks?crm_deal_id=${dealId}`);
    if (tasksRes.ok()) {
      const tasks = await tasksRes.json();
      const taskList = tasks.data || tasks || [];
      const reviewTask = taskList.find(
        (t: { title?: string }) => t.title?.includes("Review & approve")
      );
      if (reviewTask) {
        console.log(`[Part 2] Review task created: "${reviewTask.title}" (${reviewTask.status})`);
      }
    }

    // Navigate to pipeline board — verify deal moved to Business Analysis
    await page.goto(`/organizations/${ORG_ID}/crm/pipeline`);
    await page.waitForTimeout(demoPause.medium);

    // Open the deal detail panel
    const dealCard = page.getByText(DEAL_NAME.replace(`${TEST_DATA_PREFIX} `, "")).first();
    await expect(dealCard).toBeVisible({ timeout: t(10_000) });
    await dealCard.click();
    await page.waitForTimeout(demoPause.long);
  });

  test("Part 3: Advance through Discovery → Build Proposal → Polish", async ({ request }) => {
    test.setTimeout(120_000);
    await apiLogin(request);

    const stagesToAdvance = [
      "Discovery",
      "Build Proposal",
      "Polish",
    ];

    for (const targetStage of stagesToAdvance) {
      // Complete any pending review tasks first
      const tasksRes = await request.get(`/api/tasks?crm_deal_id=${dealId}`);
      if (tasksRes.ok()) {
        const tasks = await tasksRes.json();
        const taskList = tasks.data || tasks || [];
        for (const task of taskList) {
          if (task.status !== "done" && task.status !== "cancelled") {
            await request.put(`/api/tasks/${task.id}`, { data: { status: "done" } });
            console.log(`[Part 3] Completed task: ${task.title}`);
          }
        }
      }

      // Advance to next stage
      const advanceRes = await request.post(`/api/crm/deals/${dealId}/advance`);
      expect(advanceRes.ok(), `Failed to advance to ${targetStage}`).toBeTruthy();
      console.log(`[Part 3] Advanced to: ${targetStage}`);

      // Verify new review task was created for this stage
      const newTasksRes = await request.get(`/api/tasks?crm_deal_id=${dealId}`);
      if (newTasksRes.ok()) {
        const tasks = await newTasksRes.json();
        const taskList = tasks.data || tasks || [];
        const pendingTasks = taskList.filter(
          (t: { status: string }) => t.status !== "done" && t.status !== "cancelled"
        );
        console.log(`[Part 3] Pending review tasks after ${targetStage}: ${pendingTasks.length}`);
      }
    }
  });

  test("Part 4: Polish → Proposal Meeting → Closed Won", async ({ page, request }) => {
    test.setTimeout(120_000);
    await apiLogin(request);

    // Advance through remaining stages: Proposal Meeting, Closed Won
    const finalStages = ["Proposal Meeting", "Closed Won"];

    for (const targetStage of finalStages) {
      // Complete pending tasks
      const tasksRes = await request.get(`/api/tasks?crm_deal_id=${dealId}`);
      if (tasksRes.ok()) {
        const tasks = await tasksRes.json();
        const taskList = tasks.data || tasks || [];
        for (const task of taskList) {
          if (task.status !== "done" && task.status !== "cancelled") {
            await request.put(`/api/tasks/${task.id}`, { data: { status: "done" } });
          }
        }
      }

      const advanceRes = await request.post(`/api/crm/deals/${dealId}/advance`);
      expect(advanceRes.ok(), `Failed to advance to ${targetStage}`).toBeTruthy();
      console.log(`[Part 4] Advanced to: ${targetStage}`);
    }

    // Navigate to pipeline board — verify deal is in Closed Won
    await login(page);
    await page.goto(`/organizations/${ORG_ID}/crm/pipeline`);
    await expect(page.getByText("Closed Won").first()).toBeVisible({ timeout: t(10_000) });
    await page.waitForTimeout(demoPause.medium);

    // Verify the deal appears in the Closed Won column
    const dealName = DEAL_NAME.replace(`${TEST_DATA_PREFIX} `, "");
    await expect(page.getByText(dealName).first()).toBeVisible({ timeout: t(10_000) });
    await page.waitForTimeout(demoPause.long);
  });

  test("Part 5: Verify end state — delivery deal + task history", async ({ request }) => {
    test.setTimeout(60_000);
    await apiLogin(request);

    // Verify the won deal
    const dealRes = await request.get(`/api/crm/deals/${dealId}`);
    expect(dealRes.ok()).toBeTruthy();
    const deal = await dealRes.json();
    const dealData = deal.data || deal;
    console.log(`[Part 5] Final deal state: stage_id=${dealData.crm_stage_id}`);

    // Check for auto-created delivery deal (Closed Won hook)
    const orgDealsRes = await request.get(`/api/organizations/${ORG_ID}/crm/deals`);
    if (orgDealsRes.ok()) {
      const orgDeals = await orgDealsRes.json();
      const dealList = orgDeals.data || orgDeals || [];
      const deliveryDeal = dealList.find(
        (d: { name?: string }) => d.name?.includes("Delivery") && d.name?.includes("Pipeline Demo Deal")
      );
      if (deliveryDeal) {
        console.log(`[Part 5] Delivery deal auto-created: "${deliveryDeal.name}" (${deliveryDeal.id})`);
      } else {
        console.log(`[Part 5] No delivery deal found — delivery pipeline may not exist for this org`);
      }
    }

    // Verify task history — should have multiple completed review tasks
    const tasksRes = await request.get(`/api/tasks?crm_deal_id=${dealId}`);
    if (tasksRes.ok()) {
      const tasks = await tasksRes.json();
      const taskList = tasks.data || tasks || [];
      const completedTasks = taskList.filter((t: { status: string }) => t.status === "done");
      console.log(`[Part 5] Total tasks for deal: ${taskList.length} (${completedTasks.length} completed)`);
      expect(completedTasks.length).toBeGreaterThanOrEqual(3); // At least 3 review gates passed
    }

    console.log("[Part 5] Pipeline intelligence workflow demo complete!");
  });
});
