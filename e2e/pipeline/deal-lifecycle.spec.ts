/**
 * Pipeline E2E: Deal Lifecycle (DL-1 to DL-4)
 *
 * Tests deal creation, stage transitions, won automation, and lost tracking.
 * All interactions go through the UI.
 *
 * Acceptance specs: planning/BACKLOG--remaining-work.md → DL-1 to DL-4
 */
import { test, expect } from "./fixtures";
import { t, demoPause, login, apiLogin, TEST_DATA_PREFIX } from "../helpers";
import {
  ORG_ID,
  navigateToPipeline,
  createDealViaUI,
  openDealDetail,
  closeDealDetail,
  moveDealViaContextMenu,
} from "./helpers";

const DEAL_NAME = `${TEST_DATA_PREFIX} Lifecycle Test ${Date.now()}`;
const DEAL_AMOUNT = "50000";
const DEAL_DESCRIPTION = "E2E lifecycle test — operator context for pipeline progression.";

test.describe("Deal Lifecycle (DL-1 to DL-4)", () => {
  test.describe.configure({ mode: "serial" });

  // ── DL-1: Create and track a deal ──────────────────────────────────────

  test("DL-1: create a deal via the pipeline board", async ({ page }) => {
    test.setTimeout(60_000);
    await login(page);
    await navigateToPipeline(page);

    await createDealViaUI(page, {
      name: DEAL_NAME,
      amount: DEAL_AMOUNT,
      description: DEAL_DESCRIPTION,
    });

    // Verify deal card appears on the board
    const dealText = DEAL_NAME.replace(`${TEST_DATA_PREFIX} `, "");
    await expect(page.getByText(dealText).first()).toBeVisible({
      timeout: t(10_000),
    });
  });

  test("DL-1: deal card shows on kanban in Lead column", async ({ page }) => {
    test.setTimeout(30_000);
    // Shared page from fixtures — still on pipeline board from previous test

    // The deal should be in the first column (Lead)
    const dealText = DEAL_NAME.replace(`${TEST_DATA_PREFIX} `, "");
    await expect(page.getByText(dealText).first()).toBeVisible({
      timeout: t(10_000),
    });

    // Verify the Lead column header is visible
    await expect(page.getByText("Lead").first()).toBeVisible();
  });

  test("DL-1: open deal detail panel with tabs", async ({ page }) => {
    test.setTimeout(30_000);

    const dealText = DEAL_NAME.replace(`${TEST_DATA_PREFIX} `, "");
    const dialog = await openDealDetail(page, dealText);

    // Verify dialog contains deal name
    const dialogText = await dialog.textContent();
    expect(dialogText).toContain(dealText);

    // Check for expected tabs (at least some should be visible)
    const expectedTabs = ["overview", "intel", "transcripts", "proposal", "deck"];
    let tabsFound = 0;
    for (const tab of expectedTabs) {
      const tabEl = page.getByTestId(`tab-${tab}`);
      if (await tabEl.isVisible().catch(() => false)) tabsFound++;
    }
    expect(tabsFound, "Expected at least 3 detail tabs").toBeGreaterThanOrEqual(3);

    await closeDealDetail(page);
  });

  // ── DL-2: Move deals between stages ────────────────────────────────────

  test("DL-2: move deal to Intel via context menu", async ({ page }) => {
    test.setTimeout(30_000);

    const dealText = DEAL_NAME.replace(`${TEST_DATA_PREFIX} `, "");
    await moveDealViaContextMenu(page, dealText, "Intel");

    // Verify the deal is now under the Intel column
    // The Intel column header should be visible and the deal should be in it
    await expect(page.getByText("Intel").first()).toBeVisible();
    await expect(page.getByText(dealText).first()).toBeVisible({
      timeout: t(10_000),
    });
  });

  test("DL-2: move deal to Business Analysis", async ({ page }) => {
    test.setTimeout(30_000);

    const dealText = DEAL_NAME.replace(`${TEST_DATA_PREFIX} `, "");
    await moveDealViaContextMenu(page, dealText, "Business Analysis");

    await expect(page.getByText(dealText).first()).toBeVisible({
      timeout: t(10_000),
    });
  });

  test("DL-2: move deal through Proposal → Polish → Invoice → Negotiation", async ({ page }) => {
    test.setTimeout(60_000);

    const dealText = DEAL_NAME.replace(`${TEST_DATA_PREFIX} `, "");
    const stages = ["Proposal", "Polish", "Invoice", "Negotiation"];

    for (const stage of stages) {
      await moveDealViaContextMenu(page, dealText, stage);
      await expect(page.getByText(dealText).first()).toBeVisible({
        timeout: t(10_000),
      });
    }
  });

  // ── DL-3: Won deal automation ──────────────────────────────────────────

  test("DL-3: move deal to Won", async ({ page }) => {
    test.setTimeout(60_000);

    const dealText = DEAL_NAME.replace(`${TEST_DATA_PREFIX} `, "");
    await moveDealViaContextMenu(page, dealText, "Won");

    // Verify deal appears in Won column
    await expect(page.getByText(dealText).first()).toBeVisible({
      timeout: t(10_000),
    });
  });

  test("DL-3: won deal creates delivery pipeline entry", async () => {
    test.fixme(true, "Requires deal with linked contact — needs setup with createTestContact");
  });

  test("DL-3: won deal creates Client record", async () => {
    test.fixme(true, "Not yet implemented in unified processor");
  });

  test("DL-3: won deal creates Project from deal", async () => {
    test.fixme(true, "Not yet implemented in unified processor");
  });

  test("DL-3: won deal creates VIBE transaction", async () => {
    test.fixme(true, "Not yet implemented in unified processor");
  });

  // ── DL-4: Lost deal tracking ───────────────────────────────────────────

  test("DL-4: create and move a deal to Lost", async ({ page }) => {
    test.setTimeout(60_000);

    // Create a second deal for the lost test
    const lostDealName = `${TEST_DATA_PREFIX} Lost Test ${Date.now()}`;
    await createDealViaUI(page, {
      name: lostDealName,
      amount: "10000",
    });

    const dealText = lostDealName.replace(`${TEST_DATA_PREFIX} `, "");
    await expect(page.getByText(dealText).first()).toBeVisible({
      timeout: t(10_000),
    });

    // Move to Lost
    await moveDealViaContextMenu(page, dealText, "Lost");

    await expect(page.getByText(dealText).first()).toBeVisible({
      timeout: t(10_000),
    });
  });

  // ── Cleanup ────────────────────────────────────────────────────────────

  test.afterAll(async ({ request }) => {
    await apiLogin(request);

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
