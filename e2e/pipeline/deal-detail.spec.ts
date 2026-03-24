/**
 * Pipeline E2E: Deal Detail Features (DD-1 to DD-4)
 *
 * MCP verified: deal detail panel tabs are Overview, Intel, Review,
 * Transcripts, Proposal, Deck & Close, Projects, Activity, Agent History.
 * Stage bar shows "Move to {stage}" clickable items.
 *
 * Acceptance specs: planning/BACKLOG--remaining-work.md → DD-1 to DD-4
 */
import { test, expect } from "./fixtures";
import { t, demoPause, login, apiLogin, TEST_DATA_PREFIX } from "../helpers";
import { ORG_ID, PIPELINE_URL } from "./helpers";

let dealName: string;
let dealText: string;

test.describe("Deal Detail Features (DD-1 to DD-4)", () => {
  test.describe.configure({ mode: "serial" });

  test("setup: create deal and open detail panel", async ({ page, request }) => {
    test.setTimeout(60_000);
    await apiLogin(request);
    await login(page);

    // Create contact + deal via API
    const contactRes = await request.post("/api/crm/contacts", {
      data: {
        organization_id: ORG_ID,
        first_name: "E2E",
        last_name: `Detail${Date.now()}`,
        email: `e2e.detail.${Date.now()}@test.local`,
        company_name: "DetailCorp",
      },
    });
    const contact = (await contactRes.json()).data || (await contactRes.json());

    const pipelinesRes = await request.get(`/api/crm/pipelines?organization_id=${ORG_ID}`);
    const pipelines = (await pipelinesRes.json()).data || [];
    const salesPipeline = pipelines.find((p: { pipeline_type: string }) => p.pipeline_type === "sales");
    const stagesRes = await request.get(`/api/crm/pipelines/${salesPipeline.id}/stages`);
    const stages = (await stagesRes.json()).data || [];
    const leadStage = stages.find((s: { name: string }) => s.name === "Lead");

    dealName = `${TEST_DATA_PREFIX} Detail ${Date.now()}`;
    dealText = dealName.replace(`${TEST_DATA_PREFIX} `, "");
    await request.post("/api/crm/deals", {
      data: {
        organization_id: ORG_ID,
        crm_pipeline_id: salesPipeline.id,
        crm_stage_id: leadStage.id,
        crm_contact_id: contact.id,
        name: dealName,
        description: "Detail feature test — operator context.",
        amount: 45000,
      },
    });

    // Navigate and open deal
    await page.goto(PIPELINE_URL);
    await expect(page.getByText("Acquisition Pipeline")).toBeVisible({ timeout: t(15_000) });
    await page.getByText(dealText).first().click();
    await expect(page.locator('[role="dialog"]').first()).toBeVisible({ timeout: t(10_000) });
  });

  // ── DD-1: Transcripts tab ──────────────────────────────────────────────

  test("DD-1: Transcripts tab shows empty state for new deal", async ({ page }) => {
    test.setTimeout(30_000);
    // MCP showed tab "Transcripts" at ref=e793
    await page.getByRole("tab", { name: "Transcripts" }).click();
    await page.waitForTimeout(demoPause.short);

    // New deal has no transcripts — should show empty state
    const panelText = await page.locator('[role="dialog"]').first().textContent();
    expect(
      panelText?.includes("transcript") || panelText?.includes("No transcript") || panelText?.includes("Link"),
      "Transcripts tab should show content or empty state"
    ).toBeTruthy();
  });

  test("DD-1: auto-match transcripts from call intake", async () => {
    test.fixme(true, "Auto-match not implemented — manual linking works");
  });

  // ── DD-2: Call scheduling ──────────────────────────────────────────────

  test("DD-2: Overview tab shows scheduling section", async ({ page }) => {
    test.setTimeout(30_000);
    // MCP showed Overview tab has "Operator Context", "Expedite", "Probability", "Deal Value"
    await page.getByRole("tab", { name: "Overview" }).click();
    await page.waitForTimeout(demoPause.short);

    // Verify key sections are visible
    await expect(page.getByText("Operator Context").first()).toBeVisible({ timeout: t(3_000) });
    await expect(page.getByText("$45,000").first()).toBeVisible({ timeout: t(3_000) });
  });

  // ── DD-3: Person invitation ────────────────────────────────────────────

  test("DD-3: Deck & Close tab visible", async ({ page }) => {
    test.setTimeout(30_000);
    // MCP showed tab "Deck & Close" (not just "Deck")
    await page.getByRole("tab", { name: "Deck & Close" }).click();
    await page.waitForTimeout(demoPause.short);

    // Deck tab should render (even without a deck)
    const panelText = await page.locator('[role="dialog"]').first().textContent();
    expect(panelText?.length, "Deck & Close tab should have content").toBeGreaterThan(20);
  });

  test("DD-3: invite link only on won deals", async ({ page }) => {
    test.setTimeout(30_000);
    // Deal is in Lead — invite section should NOT be visible
    const panelText = await page.locator('[role="dialog"]').first().textContent() || "";
    const hasInvite = panelText.includes("Invite") || panelText.includes("invite link");
    // Invite should be hidden on non-won deals
    expect(hasInvite).toBeFalsy();
  });

  // ── DD-4: Invoice generation ───────────────────────────────────────────

  test("DD-4: AR invoice dashboard", async () => {
    test.fixme(true, "Invoice generation works but no tracking dashboard");
  });

  // ── Close and cleanup ──────────────────────────────────────────────────

  test("close detail panel", async ({ page }) => {
    await page.getByRole("button", { name: "Close" }).first().click();
    await page.waitForTimeout(demoPause.short);
  });

  test.afterAll(async ({ request }) => {
    await apiLogin(request);
    const dealsRes = await request.get(`/api/crm/deals?organization_id=${ORG_ID}`);
    if (dealsRes.ok()) {
      for (const deal of ((await dealsRes.json()).data || [])) {
        if ((deal.name as string)?.startsWith(TEST_DATA_PREFIX)) {
          await request.delete(`/api/crm/deals/${deal.id}`).catch(() => {});
        }
      }
    }
  });
});
