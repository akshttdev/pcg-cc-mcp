/**
 * Pipeline E2E Helpers
 *
 * UI interaction helpers for CRM pipeline tests.
 * All interactions go through the UI — no API shortcuts for user actions.
 */
import type { Page, APIRequestContext } from "@playwright/test";
import { expect } from "./fixtures";
import { t, demoPause } from "../helpers";
import { dealCard, dealDetail, review } from "./testids";

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
 * Wait for a deal card to appear in a target stage column on the kanban board.
 * Pure UI — watches for the card text inside the column testid. No API polling.
 * SSE events keep the board fresh so cards appear within ~1s of backend processing.
 */
export async function waitForCardInColumn(
  page: Page,
  dealNameFragment: string,
  stageName: string,
  timeoutMs = 15_000,
) {
  const column = page.getByTestId(`stage-column-${stageName.toLowerCase().replace(/\s+/g, '-')}`);
  await expect(column.getByText(dealNameFragment, { exact: false })).toBeVisible({ timeout: timeoutMs });
}

/**
 * Wait for a deal to reach a target stage via agent auto-advance.
 *
 * Demo-quality flow:
 * 1. Watch the kanban board (drawer closed) for card movement
 * 2. Click "Run Now" on agent toast when it appears
 * 3. When review task blocks advance: open drawer → Review tab → Mark Complete → close
 * 4. Watch card move to next column
 *
 * Uses SSE for real-time board updates — no API polling.
 */
export async function waitForDealStage(
  page: Page,
  request: APIRequestContext,
  dealId: string,
  stageName: string,
  timeoutMs = 15_000,
) {
  // Close any open drawer so we can watch the kanban board
  try {
    const closeBtn = page.getByTestId(dealDetail.close);
    if (await closeBtn.isVisible({ timeout: 300 })) await closeBtn.click();
  } catch { /* not open */ }

  const start = Date.now();
  let lastStage = '';
  let drawerAttempts = 0;

  while (Date.now() - start < timeoutMs) {
    // Click Run Now on toast if visible (bypasses cancel window)
    try {
      const runNow = page.getByRole("button", { name: "Run Now" });
      if (await runNow.isVisible({ timeout: 500 })) {
        await runNow.click();
        await page.waitForTimeout(300);
      }
    } catch { /* no toast */ }

    // Check current stage via API (read-only, not a shortcut)
    const dealRes = await request.get(`/api/crm/deals/${dealId}`);
    const deal = await dealRes.json().then((b: any) => b.data || b);
    const currentStage = deal.stage?.toLowerCase() || '';

    if (currentStage === stageName.toLowerCase()) return true;

    // Detect if we're stuck at the same stage (needs review task completion)
    const stageChanged = currentStage !== lastStage;
    lastStage = currentStage;

    if (stageChanged) {
      // Stage just changed — give agents time to start, skip drawer interaction
      drawerAttempts = 0;
      await page.waitForTimeout(1_500);
      continue;
    }

    // Only open drawer for review completion every other iteration to reduce overhead
    drawerAttempts++;
    if (drawerAttempts % 2 === 0) {
      try {
        const card = page.getByTestId(dealCard.card(dealId));
        if (await card.isVisible({ timeout: 500 })) {
          await card.click();
          await page.waitForTimeout(300);
          const reviewTab = page.getByTestId(dealDetail.tab("review"));
          if (await reviewTab.isVisible({ timeout: 300 })) {
            await reviewTab.click();
            await page.waitForTimeout(300);
            const markComplete = page.getByTestId(review.markComplete);
            if (await markComplete.isVisible({ timeout: 500 })) {
              await markComplete.click();
              await page.waitForTimeout(500);
            }
          }
          // Close drawer to watch board again
          const closeBtn = page.getByTestId(dealDetail.close);
          if (await closeBtn.isVisible({ timeout: 300 })) await closeBtn.click();
        }
      } catch { /* deal card not visible or drawer interaction failed */ }
    }

    await page.waitForTimeout(1_500);
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
