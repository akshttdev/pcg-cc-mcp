/**
 * Pipeline E2E Helpers
 *
 * UI interaction helpers for CRM pipeline tests.
 * All interactions go through the UI — no API shortcuts for user actions.
 */
import type { Page } from "@playwright/test";
import { expect } from "./fixtures";
import { t, demoPause } from "../helpers";

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

  // Wait for detail panel
  const dialog = page.locator('[role="dialog"]');
  await expect(dialog).toBeVisible({ timeout: t(10_000) });
  await page.waitForTimeout(demoPause.short);
  return dialog;
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
  const closeBtn = page
    .locator('[role="dialog"]')
    .getByRole("button", { name: /close/i })
    .first();
  if (await closeBtn.isVisible().catch(() => false)) {
    await closeBtn.click();
    await page.waitForTimeout(demoPause.short);
  }
}

// ── Stage Transitions via UI ─────────────────────────────────────────────────

/** Move a deal to a target stage via the context menu "Move to..." submenu. */
export async function moveDealViaContextMenu(
  page: Page,
  dealNameFragment: string,
  targetStage: string
) {
  // Find and right-click (or click the "..." menu) on the deal card
  const card = page
    .getByRole("button", { name: new RegExp(dealNameFragment, "i") })
    .first();
  await expect(card).toBeVisible({ timeout: t(10_000) });

  // Hover to reveal the "..." menu button
  await card.hover();
  await page.waitForTimeout(demoPause.short);

  // Click the three-dot menu
  const menuBtn = card.locator('button:has(svg)').last();
  await menuBtn.click();

  // Click "Move to..."
  await page.getByText("Move to...").click();

  // Click the target stage
  await page.getByRole("menuitem", { name: targetStage }).click();

  await page.waitForTimeout(demoPause.medium);
}

// ── Pipeline Settings UI ─────────────────────────────────────────────────────

/** Open pipeline settings from the gear icon on the pipeline board. */
export async function openPipelineSettings(page: Page) {
  // The gear icon is an icon-only button next to "Add Deal"
  const settingsBtn = page
    .locator('.inline-flex.items-center.justify-center.whitespace-nowrap')
    .filter({ has: page.locator('svg') })
    .last();

  // Simpler: it's the button right after "Add Deal" in the header
  const addDeal = page.getByRole("button", { name: "Add Deal", exact: true }).first();
  const gearBtn = addDeal.locator('~ button').first();

  if (await gearBtn.isVisible().catch(() => false)) {
    await gearBtn.click();
  } else {
    // Fallback: click by position relative to "Add Deal"
    const addDealBox = await addDeal.boundingBox();
    if (addDealBox) {
      await page.mouse.click(addDealBox.x + addDealBox.width + 30, addDealBox.y + addDealBox.height / 2);
    }
  }

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
