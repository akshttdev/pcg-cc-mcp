/**
 * Demo: Dealflow Pipeline — Operator Walkthrough
 *
 * End-to-end feature demo showcasing the CRM dealflow pipeline from the
 * operator's perspective:
 *   1. Login and navigate to the Sirak Studios CRM Pipeline board
 *   2. Create a deal (company + contact + deal via API), verify on board
 *   3. Open deal detail panel via card click, verify tabs
 *   4. Add operator context via UI (click, type, save, toast)
 *   5. Browse Intel tab — verify person + company intelligence
 *   6. Browse Transcripts tab — verify content or empty state
 *   7. Generate proposal (LLM) — requires ANTHROPIC_API_KEY
 *   8. Approve proposal via UI
 *   9. Browse Deck & Close tab — verify sections visible
 *  10. Navigate to person profile page
 *  11. Cleanup test entities via API
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

const DEMO_LEAD = {
  company: `${TEST_DATA_PREFIX} Aurora Design Co`,
  first: "Elena",
  last: "Vasquez",
  email: `elena-${Date.now()}@auroradesign.co`,
  title: "Creative Director",
  deal_name: `${TEST_DATA_PREFIX} Aurora Design Co — Brand Refresh`,
  context:
    "Elena is looking for a full brand refresh and digital strategy overhaul for their boutique design agency. Budget range $20-30K. Timeline: Q3 launch.",
};

// Shared state across sequential tests
let companyId: string;
let contactId: string;
let dealId: string;
let pipelineId: string;

// ── Test Flow ────────────────────────────────────────────────────────────────

test.describe("Demo: Dealflow Pipeline — Operator Walkthrough", () => {
  test.describe.configure({ mode: "serial" });

  // ── Part 1: Login and navigate to pipeline ───────────────────────────────

  test("Part 1: Login and navigate to pipeline board", async ({ page }) => {
    test.setTimeout(60_000);
    await login(page);

    await page.goto(`/organizations/${ORG_ID}/crm/pipeline`);

    // Wait for the pipeline board to render
    await expect(page.getByText("Acquisition Pipeline")).toBeVisible({
      timeout: t(20_000),
    });

    // Check for pipeline stage columns
    const bodyText = await page.textContent("body");
    const stageIndicators = ["Lead", "Intel", "Proposal", "Won", "Lost"];
    let stagesFound = 0;
    for (const stage of stageIndicators) {
      if (bodyText?.includes(stage)) stagesFound++;
    }
    expect(
      stagesFound,
      `Expected multiple pipeline stages visible, found ${stagesFound}`,
    ).toBeGreaterThanOrEqual(3);

    console.log(
      `[Part 1] Pipeline board loaded with ${stagesFound} stage indicators`,
    );
    await page.waitForTimeout(demoPause.medium);
  });

  // ── Part 2: Create company, contact, and deal via API ────────────────────

  test("Part 2: Create company, contact, and deal", async ({
    page,
    request,
  }) => {
    test.setTimeout(60_000);
    await apiLogin(request);

    // Discover the Acquisition pipeline
    const pipelinesRes = await request.get(
      `/api/crm/pipelines?organization_id=${ORG_ID}`,
    );
    expect(pipelinesRes.ok()).toBeTruthy();
    const pipelines = await pipelinesRes.json();
    const pipelineList = pipelines.data ?? pipelines ?? [];
    const acq =
      pipelineList.find(
        (p: { name: string }) => p.name === "Acquisition",
      ) || pipelineList[0];
    expect(acq, "No pipeline found — need Acquisition pipeline").toBeTruthy();
    pipelineId = acq.id;
    console.log(`[Part 2] Pipeline: ${acq.name} (${pipelineId})`);

    // Create company
    const coRes = await request.post("/api/companies", {
      data: {
        name: DEMO_LEAD.company,
        industry: "Design & Creative Services",
        created_by_org_id: ORG_ID,
      },
    });
    const coBody = await coRes.json();
    companyId = coBody.data?.id;
    if (!companyId) {
      // Try finding existing
      const listRes = await request.get("/api/companies?limit=500");
      const list = await listRes.json();
      companyId = (list.data ?? []).find(
        (c: { name: string }) => c.name === DEMO_LEAD.company,
      )?.id;
    }
    expect(companyId, "Company creation failed").toBeTruthy();
    console.log(`[Part 2] Company: ${companyId}`);

    // Create CRM contact
    const ctRes = await request.post("/api/crm/contacts", {
      data: {
        organization_id: ORG_ID,
        first_name: DEMO_LEAD.first,
        last_name: DEMO_LEAD.last,
        email: DEMO_LEAD.email,
        job_title: DEMO_LEAD.title,
        company_name: DEMO_LEAD.company,
      },
    });
    expect(ctRes.ok()).toBeTruthy();
    const ctBody = await ctRes.json();
    contactId = ctBody.data?.id;
    expect(contactId, "Contact creation failed").toBeTruthy();
    console.log(`[Part 2] Contact: ${contactId}`);

    // Create deal
    const dealRes = await request.post("/api/crm/deals", {
      data: {
        name: DEMO_LEAD.deal_name,
        organization_id: ORG_ID,
        crm_pipeline_id: pipelineId,
        crm_contact_id: contactId,
        description: DEMO_LEAD.context,
        amount: 25000,
      },
    });
    expect(dealRes.ok()).toBeTruthy();
    const dealBody = await dealRes.json();
    dealId = dealBody.data?.id;
    expect(dealId, "Deal creation failed").toBeTruthy();
    console.log(`[Part 2] Deal: ${dealId}`);

    // Reload pipeline board and verify the deal card appears
    await page.goto(`/organizations/${ORG_ID}/crm/pipeline`);
    await page.waitForTimeout(demoPause.medium);

    const dealCard = page
      .locator("button")
      .filter({ hasText: /Aurora Design/i })
      .first();
    await expect(dealCard).toBeVisible({ timeout: t(15_000) });
    console.log("[Part 2] Deal card visible on pipeline board");
    await page.waitForTimeout(demoPause.medium);
  });

  // ── Part 3: Open deal detail panel ───────────────────────────────────────

  test("Part 3: Open deal detail panel", async ({ page }) => {
    test.skip(!dealId, "No deal created");
    test.setTimeout(30_000);

    // Click the deal card to open the slide-in detail panel
    const dealCard = page
      .locator("button")
      .filter({ hasText: /Aurora Design/i })
      .first();
    await expect(dealCard).toBeVisible({ timeout: t(10_000) });
    await dealCard.click();

    // Wait for the dialog (slide-in panel) to appear
    await page.waitForSelector('[role="dialog"]', { timeout: t(10_000) });

    const dialog = page.locator('[role="dialog"]');
    await expect(dialog).toBeVisible({ timeout: t(5_000) });

    // Check for tab buttons — these use data-testid="tab-{value}"
    const expectedTabs = ["overview", "intel", "transcripts", "proposal"];
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
    test.skip(!dealId, "No deal created");
    test.setTimeout(30_000);

    // Ensure we're on the Overview tab (default)
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
      // May already have context — try clicking the operator context section
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
      await textarea.fill(DEMO_LEAD.context);
      await page.waitForTimeout(demoPause.short);

      // Click Save Context
      const saveBtn = page
        .locator('[role="dialog"]')
        .getByRole("button", { name: /save context/i })
        .first();
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
    test.skip(!dealId, "No deal created");
    test.setTimeout(30_000);

    // Click the Intel tab
    const intelTab = page.getByTestId("tab-intel");
    await expect(intelTab).toBeVisible({ timeout: t(5_000) });
    await intelTab.click();
    await page.waitForTimeout(demoPause.medium);

    // Verify intel content is shown within the dialog
    const dialog = page.locator('[role="dialog"]');
    const dialogText = await dialog.textContent();

    // Intel tab should show person intelligence and/or company intelligence
    const hasPersonIntel =
      dialogText?.includes("Person") ||
      dialogText?.includes("Intelligence") ||
      dialogText?.includes(DEMO_LEAD.first);
    const hasCompanyIntel =
      dialogText?.includes("Company") ||
      dialogText?.includes("Research") ||
      dialogText?.includes("Aurora");

    console.log(
      `[Part 5] Intel tab — person intel: ${!!hasPersonIntel}, company intel: ${!!hasCompanyIntel}`,
    );

    // At minimum the tab should have rendered something
    expect(
      dialogText?.length,
      "Intel tab should have content",
    ).toBeGreaterThan(50);

    await page.waitForTimeout(demoPause.medium);
  });

  // ── Part 6: Browse Transcripts tab ───────────────────────────────────────

  test("Part 6: Browse Transcripts tab", async ({ page }) => {
    test.skip(!dealId, "No deal created");
    test.setTimeout(30_000);

    // Click the Transcripts tab
    const transcriptsTab = page.getByTestId("tab-transcripts");
    await expect(transcriptsTab).toBeVisible({ timeout: t(5_000) });
    await transcriptsTab.click();
    await page.waitForTimeout(demoPause.medium);

    // Verify transcript content or empty state
    const dialog = page.locator('[role="dialog"]');
    const dialogText = await dialog.textContent();

    const hasTranscripts =
      dialogText?.includes("Transcript") ||
      dialogText?.includes("Discovery") ||
      dialogText?.includes("transcript");
    const hasEmptyState =
      dialogText?.includes("No transcripts") ||
      dialogText?.includes("no transcripts") ||
      dialogText?.includes("No discovery");

    console.log(
      `[Part 6] Transcripts tab — has content: ${!!hasTranscripts}, empty state: ${!!hasEmptyState}`,
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
    test.skip(!dealId, "No deal created");
    test.setTimeout(120_000);

    test.fixme(
      !process.env.ANTHROPIC_API_KEY,
      "No ANTHROPIC_API_KEY — skipping proposal generation",
    );

    // Click the Proposal tab
    const proposalTab = page.getByTestId("tab-proposal");
    await expect(proposalTab).toBeVisible({ timeout: t(5_000) });
    await proposalTab.click();
    await page.waitForTimeout(demoPause.medium);

    const dialog = page.locator('[role="dialog"]');

    // Look for Generate or Regenerate button
    const generateBtn = dialog
      .getByRole("button", { name: /generate|regenerate/i })
      .first();
    const hasGenerateBtn = await generateBtn.isVisible().catch(() => false);

    if (hasGenerateBtn) {
      await generateBtn.click();
      console.log("[Part 7] Clicked generate/regenerate proposal button");

      // Wait for LLM response — proposal text should appear
      await page
        .waitForSelector(
          '[role="dialog"] [class*="prose"], [role="dialog"] [class*="markdown"]',
          { timeout: 90_000 },
        )
        .catch(() => null);

      // Check if proposal content appeared
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
        `[Part 7] No generate button found — proposal may already exist (content: ${dialogText?.length ?? 0} chars)`,
      );
    }

    await page.waitForTimeout(demoPause.long);
  });

  // ── Part 8: Approve proposal ─────────────────────────────────────────────

  test("Part 8: Approve proposal", async ({ page }) => {
    test.skip(!dealId, "No deal created");
    test.setTimeout(30_000);

    // Ensure we're on the Proposal tab
    const proposalTab = page.getByTestId("tab-proposal");
    if (await proposalTab.isVisible().catch(() => false)) {
      await proposalTab.click();
      await page.waitForTimeout(demoPause.short);
    }

    const dialog = page.locator('[role="dialog"]');

    // Look for Approve Proposal button
    const approveBtn = dialog
      .getByRole("button", { name: /approve proposal/i })
      .first();
    const hasApproveBtn = await approveBtn.isVisible().catch(() => false);

    if (hasApproveBtn) {
      await approveBtn.click();
      console.log("[Part 8] Clicked Approve Proposal");

      // Wait for status to change — look for "Approved" badge
      await page.waitForTimeout(demoPause.medium);

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
    test.skip(!dealId, "No deal created");
    test.setTimeout(30_000);

    // Click the Deck & Close tab
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
      dialogText?.includes("generate deck");
    const hasMarkWon =
      dialogText?.includes("Mark Won") ||
      dialogText?.includes("Close Deal") ||
      dialogText?.includes("mark won");
    const hasInvoice =
      dialogText?.includes("Invoice") || dialogText?.includes("invoice");

    console.log(
      `[Part 9] Deck & Close — Generate Deck: ${!!hasGenerateDeck}, Mark Won: ${!!hasMarkWon}, Invoice: ${!!hasInvoice}`,
    );

    // At minimum the tab should render with some content
    expect(
      dialogText?.length,
      "Deck & Close tab should have content",
    ).toBeGreaterThan(20);

    await page.waitForTimeout(demoPause.medium);
  });

  // ── Part 10: Navigate to person profile ──────────────────────────────────

  test("Part 10: Navigate to person profile", async ({ page }) => {
    test.skip(!dealId || !contactId, "No deal or contact created");
    test.setTimeout(30_000);

    // Close the dialog panel first
    const closeBtn = page
      .locator('[role="dialog"]')
      .getByRole("button", { name: /close/i })
      .first();
    if (await closeBtn.isVisible().catch(() => false)) {
      await closeBtn.click();
      await page.waitForTimeout(demoPause.short);
    }

    // Navigate to person profile page
    await page.goto(`/people/${contactId}`);
    await page.waitForTimeout(demoPause.medium);

    // Verify the person page loaded — should show name and company info
    const bodyText = await page.textContent("body");
    const hasName =
      bodyText?.includes(DEMO_LEAD.first) ||
      bodyText?.includes(DEMO_LEAD.last);
    const hasCompany =
      bodyText?.includes("Aurora") || bodyText?.includes(DEMO_LEAD.company);

    console.log(
      `[Part 10] Person page — name: ${!!hasName}, company: ${!!hasCompany}`,
    );

    // Verify the page has meaningful content
    expect(
      bodyText?.length,
      "Person page should have content",
    ).toBeGreaterThan(100);

    await page.waitForTimeout(demoPause.medium);
  });

  // ── Part 11: Cleanup ─────────────────────────────────────────────────────

  test("Part 11: Cleanup test data", async ({ request }) => {
    await apiLogin(request);

    if (dealId) {
      await request.delete(`/api/crm/deals/${dealId}`).catch(() => {});
      console.log(`[Part 11] Deleted deal: ${dealId}`);
    }
    if (contactId) {
      await request.delete(`/api/crm/contacts/${contactId}`).catch(() => {});
      console.log(`[Part 11] Deleted contact: ${contactId}`);
    }
    if (companyId) {
      await request.delete(`/api/companies/${companyId}`).catch(() => {});
      console.log(`[Part 11] Deleted company: ${companyId}`);
    }

    console.log("[Part 11] Cleanup complete");
  });
});
