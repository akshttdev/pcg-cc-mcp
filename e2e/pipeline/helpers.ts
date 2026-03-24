/**
 * Pipeline E2E Helpers
 *
 * UI interaction helpers for CRM pipeline tests.
 * All interactions go through the UI — no API shortcuts for user actions.
 */
import type { Page, APIRequestContext } from "@playwright/test";
import { expect } from "./fixtures";
import { t, demoPause } from "../helpers";
import { dealDetail } from "./testids";

// ── Constants ────────────────────────────────────────────────────────────────

export const ORG_ID = "02020202-0202-0202-0202-020202020202"; // Sirak Studios
export const PIPELINE_URL = `/organizations/${ORG_ID}/crm/pipeline`;

// ── Navigation ───────────────────────────────────────────────────────────────

/** Navigate to the CRM Pipeline board and wait for it to render. */
export async function navigateToPipeline(page: Page) {
  await page.goto(PIPELINE_URL);
  await expect(page.getByText("Acquisition Pipeline").first()).toBeVisible({
    timeout: t(15_000),
  });
  await page.waitForTimeout(demoPause.short);
}

// ── Deal CRUD via UI ─────────────────────────────────────────────────────────

/** Create a deal via the "Add Deal" dialog on the pipeline board. */
export async function createDealViaUI(
  page: Page,
  opts: { name: string; amount?: string; description?: string }
) {
  // Click the header "Add Deal" button
  const addDealBtn = page
    .getByRole("button", { name: "Add Deal", exact: true })
    .first();
  await expect(addDealBtn).toBeVisible({ timeout: t(10_000) });
  await addDealBtn.click();

  // Wait for dialog
  await expect(
    page.getByRole("heading", { name: "Create Deal" })
  ).toBeVisible({ timeout: t(5_000) });

  // Fill fields
  await page.getByRole("textbox", { name: "Deal Name *" }).fill(opts.name);

  if (opts.amount) {
    await page.getByRole("spinbutton", { name: "Amount" }).fill(opts.amount);
  }

  if (opts.description) {
    await page
      .getByRole("textbox", { name: "Description" })
      .fill(opts.description);
  }

  await page.waitForTimeout(demoPause.short);

  // Submit
  const createBtn = page.getByRole("button", { name: "Create Deal" });
  await expect(createBtn).toBeEnabled({ timeout: t(3_000) });
  await createBtn.click();

  // Wait for dialog to close
  await expect(
    page.getByRole("heading", { name: "Create Deal" })
  ).not.toBeVisible({ timeout: t(10_000) });

  await page.waitForTimeout(demoPause.short);
}

// ── Deal Detail Panel ────────────────────────────────────────────────────────

/** Open a deal's detail panel by clicking its card on the kanban board. */
export async function openDealDetail(page: Page, dealNameFragment: string) {
  const card = page.getByRole("button", {
    name: new RegExp(dealNameFragment, "i"),
  });
  await expect(card.first()).toBeVisible({ timeout: t(10_000) });
  await card.first().click();

  // Wait for detail panel (resizable drawer, not a dialog role)
  const panel = page.getByTestId(dealDetail.panel);
  await expect(panel).toBeVisible({ timeout: t(10_000) });
  await page.waitForTimeout(demoPause.short);
  return panel;
}

/** Click a tab in the deal detail panel. */
export async function clickDetailTab(page: Page, tabName: string) {
  const tab = page.getByTestId(`tab-${tabName}`);
  if (await tab.isVisible().catch(() => false)) {
    await tab.click();
    await page.waitForTimeout(demoPause.short);
  }
}

/** Close the deal detail panel. */
export async function closeDealDetail(page: Page) {
  const panel = page.getByTestId(dealDetail.panel);
  const closeBtn = panel.getByRole("button", { name: /close/i }).first();
  if (await closeBtn.isVisible().catch(() => false)) {
    await closeBtn.click();
    await page.waitForTimeout(demoPause.short);
  }
}

// ── Stage Transitions via UI ─────────────────────────────────────────────────

/** Move a deal to a target stage via the context menu "Move to..." submenu.
 *  Requires data-testid="deal-menu-{dealId}" on the three-dot button. */
export async function moveDealViaContextMenu(
  page: Page,
  dealNameFragment: string,
  targetStage: string
) {
  // Find the deal card by text content
  const dealCard = page.getByText(dealNameFragment, { exact: false }).first();
  await expect(dealCard).toBeVisible({ timeout: t(10_000) });

  // Hover the card to reveal the hidden menu button
  await dealCard.hover();
  await page.waitForTimeout(demoPause.short);

  // Click the three-dot menu — scoped to the specific deal card
  // Strategy: find the deal-card-{id} wrapper that contains our deal text,
  // then find the deal-menu-{id} inside it
  const cardWrapper = page.locator('[data-testid^="deal-card-"]').filter({
    hasText: new RegExp(dealNameFragment, "i"),
  }).first();
  const menuBtn = cardWrapper.locator('[data-testid^="deal-menu-"]');
  await menuBtn.click({ force: true });
  await page.waitForTimeout(demoPause.short);

  // Hover "Move to..." to open the submenu (radix sub-menus open on hover)
  const moveToTrigger = page.getByText("Move to...");
  await expect(moveToTrigger).toBeVisible({ timeout: t(3_000) });
  await moveToTrigger.hover();
  await page.waitForTimeout(demoPause.short);

  // Click the target stage in the submenu
  const stageItem = page.getByRole("menuitem", { name: targetStage, exact: true });
  await expect(stageItem).toBeVisible({ timeout: t(3_000) });
  await stageItem.click();

  // Wait for the move to complete
  await page.waitForTimeout(demoPause.long);
}

// ── Agent Auto-Advance Helpers ───────────────────────────────────────────────

/**
 * Wait for a deal to reach a target stage via agent auto-advance.
 *
 * Uses UI interactions:
 * - Clicks "Run Now" on agent toast to bypass cancel window
 * - Opens deal detail → Review tab → "Mark Review Complete" to unblock advance
 * - Polls API to check stage (read-only, not a shortcut)
 *
 * Returns true if the deal reached the stage, false on timeout.
 */
/**
 * Wait for a deal to reach a target stage via agent auto-advance.
 *
 * UI interactions per poll cycle:
 * 1. Click "Run Now" on agent toast (bypasses 30s cancel window)
 * 2. Open deal detail via testid `deal-card-{dealId}`
 * 3. Click Review tab via testid `deal-detail-tabs-review`
 * 4. Click "Mark Review Complete" via testid `review-mark-complete`
 * 5. Close panel via testid `deal-detail-close`
 * 6. Read-only API check for current stage
 */
export async function waitForDealStage(
  page: Page,
  request: APIRequestContext,
  dealId: string,
  stageName: string,
  timeoutMs = 30_000,
) {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    // 1. Click Run Now on toast if visible
    try {
      const runNow = page.getByRole("button", { name: "Run Now" });
      if (await runNow.isVisible({ timeout: 1_000 })) {
        await runNow.click();
        await page.waitForTimeout(500);
      }
    } catch { /* toast may not be visible */ }

    // 2. Open deal detail panel via card testid
    const panel = page.getByTestId(dealDetail.panel);
    const panelOpen = await panel.isVisible({ timeout: 300 }).catch(() => false);
    if (!panelOpen) {
      try {
        const card = page.getByTestId(`deal-card-${dealId}`);
        if (await card.isVisible({ timeout: 500 })) {
          await card.click();
          await page.waitForTimeout(500);
        }
      } catch { /* card may not be visible on current board view */ }
    }

    // 3-4. Review tab → Mark Review Complete
    try {
      const reviewTab = page.getByTestId(dealDetail.tab("review"));
      if (await reviewTab.isVisible({ timeout: 300 })) {
        await reviewTab.click();
        await page.waitForTimeout(300);
        const markComplete = page.getByTestId("review-mark-complete");
        if (await markComplete.isVisible({ timeout: 500 })) {
          await markComplete.click();
          await page.waitForTimeout(1_000);
        }
      }
    } catch { /* review tab or button not available */ }

    // 5. Close panel
    try {
      const closeBtn = page.getByTestId(dealDetail.close);
      if (await closeBtn.isVisible({ timeout: 300 })) {
        await closeBtn.click();
        await page.waitForTimeout(300);
      }
    } catch { /* panel may already be closed */ }

    // 6. Read-only API check for current stage
    const dealRes = await request.get(`/api/crm/deals/${dealId}`);
    const deal = (await dealRes.json()).data || (await dealRes.json());
    if (deal.stage?.toLowerCase() === stageName.toLowerCase()) return true;
    await page.waitForTimeout(2_000);
  }
  return false;
}

/**
 * Complete all pending tasks for a deal via API.
 * NOTE: Use completeDealTasksViaUI for demo-quality tests.
 */
export async function completeDealTasks(request: APIRequestContext, dealId: string) {
  const res = await request.get(`/api/tasks?crm_deal_id=${dealId}`);
  if (res.ok()) {
    for (const task of ((await res.json()).data || [])) {
      if (task.status !== "done" && task.status !== "cancelled") {
        await request.put(`/api/tasks/${task.id}`, { data: { status: "done" } });
      }
    }
  }
}

/**
 * Advance a deal from a human-owned stage to the next stage via context menu.
 * Use this when the auto-advance chain stops at a human stage (e.g., Discovery).
 */
export async function advanceDealViaUI(
  page: Page,
  dealNameFragment: string,
  targetStage: string,
) {
  await moveDealViaContextMenu(page, dealNameFragment, targetStage);
  await page.waitForTimeout(demoPause.long);
}

// ── Pipeline Settings UI ─────────────────────────────────────────────────────

/** Open pipeline settings from the gear icon on the pipeline board.
 *  Requires data-testid="pipeline-settings" on the gear button. */
export async function openPipelineSettings(page: Page) {
  const gearBtn = page.getByTestId("pipeline-settings");
  await expect(gearBtn).toBeVisible({ timeout: t(10_000) });
  await gearBtn.click();

  await expect(
    page.getByRole("heading", { name: "Pipeline Settings" })
  ).toBeVisible({ timeout: t(5_000) });
  await page.waitForTimeout(demoPause.short);
}

// ── Console Error Check ──────────────────────────────────────────────────────

/** Collect console errors from the page. Returns error messages. */
export async function getConsoleErrors(page: Page): Promise<string[]> {
  const errors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") {
      errors.push(msg.text());
    }
  });
  return errors;
}
