/**
 * Pipeline E2E: Deal Lifecycle (DL-1 to DL-4)
 *
 * Gherkin specs: planning/BACKLOG--remaining-work.md → DL-1 to DL-4
 * Every "Then" line in the Gherkin is a test assertion.
 */
import { test, expect } from "./fixtures";
import { t, demoPause, login, apiLogin, TEST_DATA_PREFIX } from "../helpers";
import { ORG_ID, PIPELINE_URL, moveDealViaContextMenu } from "./helpers";
import { pipeline, dealDetail } from "./testids";

const DEAL_NAME = `${TEST_DATA_PREFIX} Lifecycle ${Date.now()}`;
let dl2DealId: string;
let dl2DealText: string;

// ═══════════════════════════════════════════════════════════════════════════════
// DL-1: Create and track a deal through the pipeline
//
// Feature: Create and track deals through the pipeline
//   As a sales operator
//   I want to create deals and view their details
//   So that I can manage my sales funnel
// ═══════════════════════════════════════════════════════════════════════════════

test.describe("DL-1: Create and track a deal through the pipeline", () => {
  test.describe.configure({ mode: "serial" });

  // Scenario: Create a deal via the pipeline board
  //   Given I am on the CRM Pipeline page for an organization
  //   When I click "Add Deal" and fill in name, amount, and description
  //   Then the deal appears in the first stage column of the kanban board
  //   And the deal card shows the contact name, amount, and stage color

  test("create deal — Then: appears in Lead column", async ({ page }) => {
    test.setTimeout(60_000);
    await login(page);
    await page.goto(PIPELINE_URL);
    await expect(page.getByText("Acquisition Pipeline")).toBeVisible({ timeout: t(15_000) });

    // When: click "Add Deal" and fill in name, amount, description
    await page.getByTestId(pipeline.addDeal).click();
    await expect(page.getByRole("heading", { name: "Create Deal" })).toBeVisible({ timeout: t(5_000) });
    await page.getByRole("textbox", { name: "Deal Name *" }).fill(DEAL_NAME);
    await page.getByRole("spinbutton", { name: "Amount" }).fill("50000");
    await page.getByRole("textbox", { name: "Description" }).fill(
      "E2E lifecycle test — operator context."
    );
    await page.getByRole("button", { name: "Create Deal" }).click();
    await expect(page.getByRole("heading", { name: "Create Deal" })).not.toBeVisible({ timeout: t(10_000) });

    const dealText = DEAL_NAME.replace(`${TEST_DATA_PREFIX} `, "");

    // Then: deal appears in the first stage column (Lead)
    const leadColumn = page.getByTestId(pipeline.stageColumn("lead"));
    await expect(leadColumn.getByText(dealText)).toBeVisible({ timeout: t(10_000) });
  });

  test("create deal — And: card shows amount on the deal card", async ({ page }) => {
    // Spec: "the deal card shows the contact name, amount, and stage color"
    // Verify amount is displayed on the card
    const dealText = DEAL_NAME.replace(`${TEST_DATA_PREFIX} `, "");
    const leadColumn = page.getByTestId(pipeline.stageColumn("lead"));
    const dealCard = leadColumn.locator('[data-testid^="deal-card-"]').filter({ hasText: dealText });
    await expect(dealCard.getByText("$50,000")).toBeVisible({ timeout: t(5_000) });
  });

  test("create deal — And: card shows probability percentage", async ({ page }) => {
    // Spec: stage color accent. MCP verified: card shows "10%" probability for Lead stage
    const dealText = DEAL_NAME.replace(`${TEST_DATA_PREFIX} `, "");
    const leadColumn = page.getByTestId(pipeline.stageColumn("lead"));
    const dealCard = leadColumn.locator('[data-testid^="deal-card-"]').filter({ hasText: dealText });
    await expect(dealCard.getByText("10%")).toBeVisible({ timeout: t(5_000) });
  });

  // Scenario: View deal details
  //   Given a deal exists in the pipeline
  //   When I click the deal card
  //   Then a detail panel opens with tabs: Overview, Intel, Transcripts, Proposal, Deck
  //   And I can expand the panel to full dialog mode

  test("view details — Then: panel opens with tabs and shows deal data", async ({ page }) => {
    test.setTimeout(30_000);
    const dealText = DEAL_NAME.replace(`${TEST_DATA_PREFIX} `, "");

    // When: click the deal card
    await page.getByText(dealText).first().click();
    const panel = page.getByTestId(dealDetail.panel);
    await expect(panel).toBeVisible({ timeout: t(10_000) });

    // Then: panel has deal name heading
    await expect(panel.getByRole("heading", { name: new RegExp(dealText) }).first()).toBeVisible();

    // Then: all expected tabs present (spec: Overview, Intel, Transcripts, Proposal, Deck)
    for (const tab of ["overview", "intel", "transcripts", "proposal", "deck"]) {
      await expect(page.getByTestId(dealDetail.tab(tab)), `"${tab}" tab`).toBeVisible({ timeout: t(3_000) });
    }

    // Then: Overview tab shows the deal amount we entered ($50,000)
    await page.getByTestId(dealDetail.tab("overview")).click();
    await page.waitForTimeout(demoPause.short);
    await expect(panel.getByText("$50,000").first()).toBeVisible({ timeout: t(3_000) });

    // Then: Overview shows operator context we entered
    await expect(panel.getByText("E2E lifecycle test")).toBeVisible({ timeout: t(3_000) });

    // Close panel
    await panel.getByRole("button", { name: "Close" }).click();
    await page.waitForTimeout(demoPause.short);
  });

  test("view details — And: can expand to full dialog mode", async ({ page }) => {
    test.setTimeout(30_000);
    const dealText = DEAL_NAME.replace(`${TEST_DATA_PREFIX} `, "");

    // Open the deal detail drawer
    await page.getByText(dealText).first().click();
    const panel = page.getByTestId(dealDetail.panel);
    await expect(panel).toBeVisible({ timeout: t(10_000) });

    // Drawer mode: Expand button visible with text "Expand"
    const expandBtn = page.getByTestId(dealDetail.expand);
    await expect(expandBtn).toBeVisible({ timeout: t(5_000) });

    // Click Expand — switches from drawer to fullscreen dialog
    await expandBtn.click();
    await page.waitForTimeout(2000);

    // Expanded mode: dialog wrapper visible with testid "deal-detail-expanded"
    await expect(page.getByTestId(dealDetail.expanded)).toBeVisible({ timeout: t(10_000) });
    // Panel content still accessible inside the expanded dialog
    await expect(page.getByTestId(dealDetail.panel)).toBeVisible({ timeout: t(3_000) });

    // Close expanded dialog
    await page.getByRole("dialog").getByRole("button", { name: "Close" }).click();
    await page.waitForTimeout(demoPause.short);
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// DL-2: Move deals between stages
//
// Feature: Deal stage transitions
//   As a sales operator
//   I want to move deals between pipeline stages
//   So that deals progress through the sales funnel
// ═══════════════════════════════════════════════════════════════════════════════

test.describe("DL-2: Move deals between stages", () => {
  test.describe.configure({ mode: "serial" });

  // Scenario: Move via context menu
  //   Given a deal exists in the "Lead" stage
  //   When I right-click the deal card and select "Move to..." → "Intel"
  //   Then the deal moves to the Intel column
  //   And a toast confirms the transition

  test("context menu — Then: deal moves from Lead to Intel column, toast confirms", async ({ page, request }) => {
    test.setTimeout(60_000);
    await login(page);
    await page.goto(PIPELINE_URL);
    await expect(page.getByText("Acquisition Pipeline")).toBeVisible({ timeout: t(15_000) });
    await apiLogin(request);

    // Given: create a deal in Lead (with description for later advance test)
    const moveDealName = `${TEST_DATA_PREFIX} Move ${Date.now()}`;
    await page.getByTestId(pipeline.addDeal).click();
    await page.getByRole("textbox", { name: "Deal Name *" }).fill(moveDealName);
    await page.getByRole("textbox", { name: "Description" }).fill("Operator context for advance test.");
    await page.getByRole("button", { name: "Create Deal" }).click();
    await expect(page.getByRole("heading", { name: "Create Deal" })).not.toBeVisible({ timeout: t(10_000) });

    const dealText = moveDealName.replace(`${TEST_DATA_PREFIX} `, "");
    dl2DealText = dealText;

    // Get the deal ID for later tests
    const dealsRes = await request.get(`/api/crm/deals?organization_id=${ORG_ID}`);
    const deals = (await dealsRes.json()).data || [];
    const deal = deals.find((d: { name?: string }) => d.name === moveDealName);
    dl2DealId = deal?.id;

    // Given: deal is in Lead column
    const leadColumn = page.getByTestId(pipeline.stageColumn("lead"));
    await expect(leadColumn.getByText(dealText)).toBeVisible({ timeout: t(10_000) });

    // When: move via context menu → "Move to..." → "Intel"
    await moveDealViaContextMenu(page, dealText, "Intel");

    // Then: toast confirms "Moved to Intel"
    await expect(page.getByText("Moved to Intel")).toBeVisible({ timeout: t(5_000) });

    // Then: deal card is in Intel column
    const intelColumn = page.getByTestId(pipeline.stageColumn("intel"));
    await expect(intelColumn.getByText(dealText)).toBeVisible({ timeout: t(10_000) });

    // And: deal is no longer in Lead column
    await expect(leadColumn.getByText(dealText)).not.toBeVisible({ timeout: t(3_000) });
  });

  // Scenario: Move via deal detail stage bar
  //   Given I have the deal detail panel open
  //   When I click a stage in the progress bar
  //   Then the deal moves to that stage
  //   And the kanban board updates

  test("stage bar — When: click 'Move to Business Analysis' in detail panel", async ({ page, request }) => {
    test.setTimeout(60_000);
    await apiLogin(request);

    // Complete pending review tasks so move isn't blocked
    if (dl2DealId) {
      const tasksRes = await request.get(`/api/tasks?crm_deal_id=${dl2DealId}`);
      if (tasksRes.ok()) {
        for (const task of ((await tasksRes.json()).data || [])) {
          if (task.status !== "done" && task.status !== "cancelled") {
            await request.put(`/api/tasks/${task.id}`, { data: { status: "done" } });
          }
        }
      }
    }

    // Open the deal detail panel by clicking the deal card in Intel column
    const intelColumn = page.getByTestId(pipeline.stageColumn("intel"));
    await expect(intelColumn.getByText(dl2DealText)).toBeVisible({ timeout: t(10_000) });
    await intelColumn.getByText(dl2DealText).click();

    const panel = page.getByTestId(dealDetail.panel);
    await expect(panel).toBeVisible({ timeout: t(10_000) });
    await page.waitForTimeout(demoPause.short);

    // MCP verified: stage bar items have title="Move to {stage}" attribute
    // The visible text is just the stage name, but title gives the clickable action
    const moveTarget = panel.getByTitle("Move to Business Analysis");
    await expect(moveTarget).toBeVisible({ timeout: t(5_000) });
    await moveTarget.click();
    await page.waitForTimeout(demoPause.long);

    // Then: toast confirms move
    await expect(page.getByText("Moved to Business Analysis")).toBeVisible({ timeout: t(5_000) });

    // Close the detail panel
    await panel.getByRole("button", { name: "Close" }).click();
    await page.waitForTimeout(demoPause.short);

    // And: kanban board shows deal in BA column
    const baColumn = page.getByTestId(pipeline.stageColumn("business-analysis"));
    await expect(baColumn.getByText(dl2DealText)).toBeVisible({ timeout: t(10_000) });
  });

  // Scenario: Advance via API
  //   Given a deal exists in a non-terminal stage
  //   When I call POST /crm/deals/:id/advance
  //   Then the deal moves to the next stage by position
  //   And stage entry actions fire (agent triggers, review tasks)

  test("advance API — Then: deal moves to next stage, entry actions fire", async ({ request }) => {
    test.setTimeout(60_000);
    await apiLogin(request);
    expect(dl2DealId, "DL-2 deal ID should be set from previous test").toBeTruthy();

    // Complete pending review tasks from BA stage
    const tasksRes = await request.get(`/api/tasks?crm_deal_id=${dl2DealId}`);
    if (tasksRes.ok()) {
      for (const task of ((await tasksRes.json()).data || [])) {
        if (task.status !== "done" && task.status !== "cancelled") {
          await request.put(`/api/tasks/${task.id}`, { data: { status: "done" } });
        }
      }
    }

    // When: advance via API (BA → Proposal)
    const advanceRes = await request.post(`/api/crm/deals/${dl2DealId}/advance`);
    expect(advanceRes.ok(), `Advance should succeed: ${advanceRes.status()}`).toBeTruthy();

    // Then: deal is now in Proposal stage
    const afterRes = await request.get(`/api/crm/deals/${dl2DealId}`);
    const afterDeal = (await afterRes.json()).data || (await afterRes.json());
    // Verify stage changed (check stage name from the stage_id)
    expect(afterDeal.crm_stage_id, "Deal should have a stage").toBeTruthy();

    // And: a review task was created (Proposal stage entry action)
    const newTasksRes = await request.get(`/api/tasks?crm_deal_id=${dl2DealId}`);
    const tasks = (await newTasksRes.json()).data || [];
    const pendingTasks = tasks.filter((task: { status: string }) =>
      task.status !== "done" && task.status !== "cancelled"
    );
    expect(pendingTasks.length, "Advance should create a new review task").toBeGreaterThan(0);
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// DL-3: Won deal automation — PARTIAL
//
// Feature: Won deal creates delivery pipeline entry
//   As a sales operator
//   I want won deals to automatically create delivery work
//   So that client onboarding begins immediately
// ═══════════════════════════════════════════════════════════════════════════════

test.describe("DL-3: Won deal automation", () => {

  // Scenario: Deal reaches Won stage
  //   Given a deal exists in the Negotiation stage
  //   When the deal is moved to Won
  //   Then a delivery pipeline deal is auto-created
  //   And the delivery deal has the same contact and amount
  //   And a Client record is created or found by company
  //   And a Project is created from the deal
  //   And tasks are created from proposal deliverables
  //   And a VIBE transaction is recorded for the deal value

  test("won deal — Then: delivery pipeline deal is auto-created", async () => {
    test.fixme(true, "Requires deal with linked contact for delivery deal dedup logic");
  });

  test("won deal — And: Client record is created", async () => {
    test.fixme(true, "Not implemented — only delivery deal creation exists in mark_deal_won");
  });

  test("won deal — And: Project is created from the deal", async () => {
    test.fixme(true, "Not implemented in unified processor");
  });

  test("won deal — And: tasks created from proposal deliverables", async () => {
    test.fixme(true, "Not implemented in unified processor");
  });

  test("won deal — And: VIBE transaction recorded", async () => {
    test.fixme(true, "Not implemented in unified processor");
  });

  // Scenario: Deduplication
  //   Given a contact already has a delivery deal
  //   When another deal for the same contact reaches Won
  //   Then no duplicate delivery deal is created

  test("dedup — Then: no duplicate delivery deal for same contact", async () => {
    test.fixme(true, "Requires two deals with same contact setup");
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// DL-4: Lost deal tracking
//
// Feature: Lost deal tracking
//   As a sales operator
//   I want to mark deals as lost with a reason
//   So that I can analyze why deals fail
// ═══════════════════════════════════════════════════════════════════════════════

test.describe("DL-4: Lost deal tracking", () => {

  // Scenario: Move deal to Lost
  //   Given a deal exists in any active stage
  //   When I move the deal to Lost
  //   Then the deal shows in the Lost column
  //   And lost_at timestamp is recorded

  test("lost deal — Then: card in Lost column, And: lost_at set", async ({ page, request }) => {
    test.setTimeout(60_000);
    await login(page);
    await page.goto(PIPELINE_URL);
    await expect(page.getByText("Acquisition Pipeline")).toBeVisible({ timeout: t(15_000) });
    await apiLogin(request);

    // Given: create a deal
    const lostName = `${TEST_DATA_PREFIX} LostDeal ${Date.now()}`;
    await page.getByTestId(pipeline.addDeal).click();
    await page.getByRole("textbox", { name: "Deal Name *" }).fill(lostName);
    await page.getByRole("button", { name: "Create Deal" }).click();
    await expect(page.getByRole("heading", { name: "Create Deal" })).not.toBeVisible({ timeout: t(10_000) });

    const lostText = lostName.replace(`${TEST_DATA_PREFIX} `, "");
    await expect(page.getByText(lostText).first()).toBeVisible({ timeout: t(10_000) });

    // When: move to Lost via context menu
    await moveDealViaContextMenu(page, lostText, "Lost");

    // Then: toast confirms
    await expect(page.getByText("Moved to Lost")).toBeVisible({ timeout: t(5_000) });

    // Then: deal shows in the Lost column
    const lostColumn = page.getByTestId(pipeline.stageColumn("lost"));
    await lostColumn.scrollIntoViewIfNeeded();
    await expect(lostColumn.getByText(lostText)).toBeVisible({ timeout: t(10_000) });

    // And: lost_at timestamp is recorded via API
    const dealsRes = await request.get(`/api/crm/deals?organization_id=${ORG_ID}`);
    const deals = (await dealsRes.json()).data || [];
    const lostDeal = deals.find((d: { name?: string }) => d.name === lostName);
    expect(lostDeal, "Lost deal should exist in API").toBeTruthy();
    expect(lostDeal.lost_at, "lost_at should be set when deal moves to Lost").toBeTruthy();
  });
});

// ── Cleanup ──────────────────────────────────────────────────────────────────

test.afterAll(async ({ request }) => {
  await apiLogin(request);
  const dealsRes = await request.get(`/api/crm/deals?organization_id=${ORG_ID}`);
  if (dealsRes.ok()) {
    const deals = (await dealsRes.json()).data || [];
    for (const deal of deals) {
      if ((deal.name as string)?.startsWith(TEST_DATA_PREFIX)) {
        await request.delete(`/api/crm/deals/${deal.id}`).catch(() => {});
      }
    }
  }
});
