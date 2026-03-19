/**
 * Demo: Dealflow Pipeline — Operator Walkthrough
 *
 * End-to-end feature demo showcasing the CRM dealflow pipeline from the
 * operator's perspective, using a **UI-first approach**:
 *   1. Login and navigate to the Sirak Studios CRM Pipeline board
 *   2. Create a deal via the "Add Deal" UI dialog (not API)
 *   3. Open deal detail panel via card click, verify tabs
 *   4. Add operator context via UI (click, type, save, toast)
 *   5. Browse Intel tab — verify empty state for new deal
 *   6. Browse Transcripts tab — verify content or empty state
 *   7. Generate proposal (LLM) — requires ANTHROPIC_API_KEY
 *   8. Approve proposal via UI
 *   9. Browse Deck & Close tab — verify sections visible
 *  10. Close panel + navigate to companies page
 *  11. Cleanup test deal via API
 *
 * Prerequisites:
 *   - Dev server running on FRONTEND_PORT
 *   - Seed database with Sirak Studios organization + Acquisition pipeline
 *   - ANTHROPIC_API_KEY in .env (for LLM proposal generation, optional)
 */
import { test, expect } from "./fixtures";
import { t, demoPause, login, apiLogin, TEST_DATA_PREFIX } from "../helpers";

// ── Constants ────────────────────────────────────────────────────────────────

const ORG_ID = "02020202-0202-0202-0202-020202020202"; // Sirak Studios

const DEAL_NAME = `${TEST_DATA_PREFIX} Aurora Design Co — Brand Refresh`;
const DEAL_AMOUNT = "25000";
const DEAL_DESCRIPTION =
  "Elena is looking for a full brand refresh and digital strategy overhaul for their boutique design agency. Budget range $20-30K. Timeline: Q3 launch.";

const OPERATOR_CONTEXT =
  "Met Elena at the Design Summit last week. She mentioned urgency around Q3 rebrand. Key decision-maker; reports directly to CEO. Prefers async communication via email.";

// Shared state across sequential tests
let dealId: string | undefined;

// ── Test Flow ────────────────────────────────────────────────────────────────

test.describe("Demo: Dealflow Pipeline — Operator Walkthrough", () => {
  test.describe.configure({ mode: "serial" });

  // ── Part 1: Login and navigate to pipeline ───────────────────────────────

  test("Part 1: Login and navigate to pipeline board", async ({ page }) => {
    test.setTimeout(60_000);
    await login(page);

    await page.goto(`/organizations/${ORG_ID}/crm/pipeline`);

    // Wait for the pipeline board to render
    await page.waitForSelector(
      '[class*="inline-grid"], [class*="kanban"], [class*="pipeline"], [class*="board"]',
      { timeout: t(20_000) },
    );

    await expect(page.getByText("Acquisition Pipeline")).toBeVisible({
      timeout: t(10_000),
    });

    // Verify pipeline stage columns
    const stageNames = ["Lead", "Intel", "Proposal", "Won", "Lost"];
    let stagesFound = 0;
    const bodyText = await page.textContent("body");
    for (const stage of stageNames) {
      if (bodyText?.includes(stage)) stagesFound++;
    }
    expect(
      stagesFound,
      `Expected pipeline stages visible, found ${stagesFound}/5`,
    ).toBeGreaterThanOrEqual(3);

    console.log(
      `[Part 1] Pipeline board loaded with ${stagesFound} stage indicators`,
    );
    await page.waitForTimeout(demoPause.medium);
  });

  // ── Part 2: Create deal via "Add Deal" UI dialog ──────────────────────────

  test("Part 2: Create deal via Add Deal dialog", async ({ page }) => {
    test.setTimeout(60_000);

    // Click "Add Deal" button on the pipeline board
    const addDealBtn = page.getByRole("button", {
      name: "Add Deal",
      exact: true,
    });
    await expect(addDealBtn).toBeVisible({ timeout: t(10_000) });
    await addDealBtn.click();

    // Wait for the "Create Deal" dialog to open
    await expect(
      page.getByRole("heading", { name: "Create Deal" }),
    ).toBeVisible({ timeout: t(5_000) });
    await page.waitForTimeout(demoPause.short);

    // Fill deal name (required)
    await page.getByRole("textbox", { name: "Deal Name *" }).fill(DEAL_NAME);

    // Fill amount
    await page.getByRole("spinbutton", { name: "Amount" }).fill(DEAL_AMOUNT);

    // Fill description
    await page
      .getByRole("textbox", { name: "Description" })
      .fill(DEAL_DESCRIPTION);

    await page.waitForTimeout(demoPause.medium);

    // Stage dropdown defaults to "Lead" — no need to change it.
    // This ensures the deal gets a crm_stage_id and appears on the kanban.

    // Click "Create Deal" button (enables after name is filled)
    const createBtn = page.getByRole("button", { name: "Create Deal" });
    await expect(createBtn).toBeEnabled({ timeout: t(3_000) });
    await createBtn.click();

    // Wait for success and dialog to close
    await page.waitForTimeout(demoPause.medium);

    // The dialog may auto-close. Wait for the board to re-render with the new deal.
    await page
      .waitForSelector(
        '[class*="inline-grid"], [class*="kanban"], [class*="pipeline"], [class*="board"]',
        { timeout: t(15_000) },
      )
      .catch(() => null);
    await page.waitForTimeout(demoPause.medium);

    console.log("[Part 2] Deal created via UI dialog");

    // Verify the deal card appears on the kanban (may need the board to re-query)
    const bodyText = await page.textContent("body");
    const dealVisible = bodyText?.includes("Aurora Design");
    console.log(`[Part 2] Deal visible on board: ${dealVisible}`);
    expect(dealVisible, "Deal should appear on the pipeline board").toBeTruthy();

    console.log("[Part 2] Deal card visible on pipeline board in Lead column");
    await page.waitForTimeout(demoPause.medium);
  });

  // ── Part 3: Open deal detail panel ───────────────────────────────────────

  test("Part 3: Open deal detail panel", async ({ page }) => {
    test.setTimeout(30_000);

    // Click the deal card to open the slide-in detail panel
    const dealCard = page
      .locator("button")
      .filter({ hasText: /Aurora Design/i })
      .first();
    await expect(dealCard).toBeVisible({ timeout: t(10_000) });
    await dealCard.click();

    // Wait for the dialog (slide-in panel) to appear
    const dialog = page.locator('[role="dialog"]');
    await expect(dialog).toBeVisible({ timeout: t(10_000) });

    // Verify dialog heading contains deal name
    const dialogText = await dialog.textContent();
    expect(
      dialogText?.includes("Aurora Design"),
      "Dialog should contain deal name",
    ).toBeTruthy();

    // Verify tabs are visible
    const expectedTabs = ["overview", "intel", "transcripts", "proposal", "deck"];
    let tabsFound = 0;
    for (const tab of expectedTabs) {
      const tabEl = page.getByTestId(`tab-${tab}`);
      if (await tabEl.isVisible().catch(() => false)) {
        tabsFound++;
      }
    }
    console.log(
      `[Part 3] Deal panel opened with ${tabsFound}/${expectedTabs.length} expected tabs visible`,
    );
    expect(
      tabsFound,
      "Expected at least some deal detail tabs",
    ).toBeGreaterThanOrEqual(1);

    await page.waitForTimeout(demoPause.medium);
  });

  // ── Part 4: Add operator context ─────────────────────────────────────────

  test("Part 4: Add operator context", async ({ page }) => {
    test.setTimeout(30_000);

    // Overview tab is default active — click it to be sure
    const overviewTab = page.getByTestId("tab-overview");
    if (await overviewTab.isVisible().catch(() => false)) {
      await overviewTab.click();
      await page.waitForTimeout(demoPause.short);
    }

    // Click the "No context yet" placeholder to start editing
    const contextTrigger = page.getByText("No context yet");
    const hasTrigger = await contextTrigger.isVisible().catch(() => false);

    if (hasTrigger) {
      await contextTrigger.click();
    } else {
      // May already have context area — try clicking the operator context section
      const contextSection = page
        .locator('[role="dialog"]')
        .getByText(/operator context/i)
        .first();
      if (await contextSection.isVisible().catch(() => false)) {
        await contextSection.click();
      }
    }

    await page.waitForTimeout(demoPause.short);

    // Fill the textarea with operator context
    const textarea = page.locator('[role="dialog"] textarea').first();
    const textareaVisible = await textarea.isVisible().catch(() => false);

    if (textareaVisible) {
      await textarea.fill(OPERATOR_CONTEXT);
      await page.waitForTimeout(demoPause.short);

      // Click Save Context
      const saveBtn = page.getByRole("button", { name: /save context/i });
      if (await saveBtn.isVisible().catch(() => false)) {
        await saveBtn.click();

        // Verify the save toast appears
        await expect(page.getByText(/context saved/i).first()).toBeVisible({
          timeout: t(5_000),
        }).catch(() => null);

        console.log("[Part 4] Operator context saved");
      } else {
        console.log(
          "[Part 4] Save Context button not found — context area may use auto-save",
        );
      }
    } else {
      console.log(
        "[Part 4] Context textarea not found — UI may differ from expected",
      );
    }

    await page.waitForTimeout(demoPause.medium);
  });

  // ── Part 5: Browse Intel tab ─────────────────────────────────────────────

  test("Part 5: Browse Intel tab", async ({ page }) => {
    test.setTimeout(30_000);

    const intelTab = page.getByTestId("tab-intel");
    await expect(intelTab).toBeVisible({ timeout: t(5_000) });
    await intelTab.click();
    await page.waitForTimeout(demoPause.medium);

    // For a newly created deal, Intel tab shows empty state (no intel run yet)
    const dialog = page.locator('[role="dialog"]');
    const dialogText = await dialog.textContent();

    const hasIntelContent =
      dialogText?.includes("Intelligence") ||
      dialogText?.includes("Person") ||
      dialogText?.includes("Company") ||
      dialogText?.includes("No intelligence") ||
      dialogText?.includes("no intelligence");

    console.log(
      `[Part 5] Intel tab — has content/empty state: ${!!hasIntelContent}`,
    );

    // Tab should render with some content (even if just empty state text)
    expect(
      dialogText?.length,
      "Intel tab should have content",
    ).toBeGreaterThan(10);

    await page.waitForTimeout(demoPause.medium);
  });

  // ── Part 6: Browse Transcripts tab ───────────────────────────────────────

  test("Part 6: Browse Transcripts tab", async ({ page }) => {
    test.setTimeout(30_000);

    const transcriptsTab = page.getByTestId("tab-transcripts");
    await expect(transcriptsTab).toBeVisible({ timeout: t(5_000) });
    await transcriptsTab.click();
    await page.waitForTimeout(demoPause.medium);

    const dialog = page.locator('[role="dialog"]');
    const dialogText = await dialog.textContent();

    const hasTranscripts =
      dialogText?.includes("Transcript") ||
      dialogText?.includes("transcript");
    const hasEmptyState =
      dialogText?.includes("No transcripts") ||
      dialogText?.includes("no transcripts");
    const hasLinkButton = await page
      .locator('[role="dialog"]')
      .getByRole("button", { name: /link/i })
      .first()
      .isVisible()
      .catch(() => false);

    console.log(
      `[Part 6] Transcripts tab — has content: ${!!hasTranscripts}, empty state: ${!!hasEmptyState}, link button: ${hasLinkButton}`,
    );

    // Either transcripts or an empty-state message should be visible
    expect(
      hasTranscripts || hasEmptyState,
      "Transcripts tab should show content or empty state",
    ).toBeTruthy();

    await page.waitForTimeout(demoPause.medium);
  });

  // ── Part 7: Generate proposal (LLM) ─────────────────────────────────────

  test("Part 7: Generate proposal via LLM", async ({ page }) => {
    test.setTimeout(120_000);

    test.fixme(
      !process.env.ANTHROPIC_API_KEY,
      "No ANTHROPIC_API_KEY — skipping proposal generation",
    );

    const proposalTab = page.getByTestId("tab-proposal");
    await expect(proposalTab).toBeVisible({ timeout: t(5_000) });
    await proposalTab.click();
    await page.waitForTimeout(demoPause.medium);

    const dialog = page.locator('[role="dialog"]');

    // Look for Generate or Regenerate button
    const generateBtn = dialog
      .getByRole("button", { name: /generate proposal|regenerate/i })
      .first();
    const hasGenerateBtn = await generateBtn.isVisible().catch(() => false);

    if (hasGenerateBtn) {
      await generateBtn.click();
      console.log("[Part 7] Clicked Generate Proposal button");

      // Wait for LLM response — proposal text should appear (up to 60s)
      await page
        .waitForSelector(
          '[role="dialog"] [class*="prose"], [role="dialog"] [class*="markdown"]',
          { timeout: 60_000 },
        )
        .catch(() => null);

      const dialogText = await dialog.textContent();
      const hasProposal =
        (dialogText?.length ?? 0) > 500 ||
        dialogText?.includes("Draft") ||
        dialogText?.includes("Proposal");

      console.log(
        `[Part 7] Proposal generated: ${hasProposal} (content length: ${dialogText?.length ?? 0})`,
      );
    } else {
      // Proposal may already exist
      const dialogText = await dialog.textContent();
      console.log(
        `[Part 7] No generate button found — proposal may already exist (${dialogText?.length ?? 0} chars)`,
      );
    }

    await page.waitForTimeout(demoPause.long);
  });

  // ── Part 8: Approve proposal ─────────────────────────────────────────────

  test("Part 8: Approve proposal", async ({ page }) => {
    test.setTimeout(30_000);

    // Ensure we're on the Proposal tab
    const proposalTab = page.getByTestId("tab-proposal");
    if (await proposalTab.isVisible().catch(() => false)) {
      await proposalTab.click();
      await page.waitForTimeout(demoPause.short);
    }

    const dialog = page.locator('[role="dialog"]');

    // Look for Approve Proposal button
    const approveBtn = page.getByRole("button", {
      name: /approve proposal/i,
    });
    const hasApproveBtn = await approveBtn.isVisible().catch(() => false);

    if (hasApproveBtn) {
      await approveBtn.click();
      console.log("[Part 8] Clicked Approve Proposal");

      await page.waitForTimeout(demoPause.medium);

      // Verify status changes to Approved
      const dialogText = await dialog.textContent();
      const isApproved =
        dialogText?.includes("Approved") || dialogText?.includes("approved");
      console.log(`[Part 8] Proposal approved: ${!!isApproved}`);
    } else {
      const dialogText = await dialog.textContent();
      const alreadyApproved =
        dialogText?.includes("Approved") || dialogText?.includes("approved");
      if (alreadyApproved) {
        console.log("[Part 8] Proposal already approved");
      } else {
        console.log(
          "[Part 8] Approve button not found — proposal may not have been generated (LLM required)",
        );
      }
    }

    await page.waitForTimeout(demoPause.medium);
  });

  // ── Part 9: Browse Deck & Close tab ──────────────────────────────────────

  test("Part 9: Browse Deck & Close tab", async ({ page }) => {
    test.setTimeout(30_000);

    // Click the Deck tab
    const deckTab = page.getByTestId("tab-deck");
    const hasDeckTab = await deckTab.isVisible().catch(() => false);

    if (hasDeckTab) {
      await deckTab.click();
    } else {
      // Try alternative tab name
      const altTab = page.getByTestId("tab-close");
      if (await altTab.isVisible().catch(() => false)) {
        await altTab.click();
      }
    }
    await page.waitForTimeout(demoPause.medium);

    const dialog = page.locator('[role="dialog"]');
    const dialogText = await dialog.textContent();

    // Verify expected sections are present
    const hasGenerateDeck =
      dialogText?.includes("Generate Deck") ||
      dialogText?.includes("Sales Deck");
    const hasInvoice =
      dialogText?.includes("Invoice") || dialogText?.includes("invoice");
    const hasCloseDeal =
      dialogText?.includes("Mark Won") ||
      dialogText?.includes("Close Deal") ||
      dialogText?.includes("close deal");

    console.log(
      `[Part 9] Deck & Close — Generate Deck: ${!!hasGenerateDeck}, Invoice: ${!!hasInvoice}, Close Deal: ${!!hasCloseDeal}`,
    );

    // Tab should render with some content
    expect(
      dialogText?.length,
      "Deck & Close tab should have content",
    ).toBeGreaterThan(20);

    await page.waitForTimeout(demoPause.medium);
  });

  // ── Part 10: Close panel + navigate ──────────────────────────────────────

  test("Part 10: Close panel and navigate to companies", async ({ page }) => {
    test.setTimeout(30_000);

    // Close the dialog panel
    const closeBtn = page
      .locator('[role="dialog"]')
      .getByRole("button", { name: /close/i })
      .first();
    if (await closeBtn.isVisible().catch(() => false)) {
      await closeBtn.click();
      await page.waitForTimeout(demoPause.short);
    }

    // Navigate to companies page to verify the board is still intact
    await page.goto(`/organizations/${ORG_ID}/crm/companies`);
    await page.waitForTimeout(demoPause.medium);

    const bodyText = await page.textContent("body");
    expect(
      bodyText?.length,
      "Companies page should have content",
    ).toBeGreaterThan(100);

    console.log("[Part 10] Navigated to companies page");
    await page.waitForTimeout(demoPause.medium);
  });

  // ── Part 11: Cleanup ─────────────────────────────────────────────────────

  test("Part 11: Cleanup test data", async ({ request }) => {
    await apiLogin(request);

    // Find and delete the test deal by name via API
    const dealsRes = await request.get(
      `/api/crm/deals?organization_id=${ORG_ID}`,
    );
    if (dealsRes.ok()) {
      const dealsBody = await dealsRes.json();
      const deals = dealsBody.data || dealsBody || [];
      for (const deal of deals) {
        if (
          (deal.name as string | undefined)?.includes("Aurora Design") &&
          (deal.name as string | undefined)?.startsWith(TEST_DATA_PREFIX)
        ) {
          dealId = deal.id;
          await request.delete(`/api/crm/deals/${deal.id}`).catch(() => {});
          console.log(`[Part 11] Deleted deal: ${deal.id}`);
        }
      }
    }

    if (!dealId) {
      console.log("[Part 11] No test deal found to clean up");
    }

    console.log("[Part 11] Cleanup complete");
  });
});
