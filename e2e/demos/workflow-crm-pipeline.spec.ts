/**
 * Demo: Workflow Builder + CRM Pipeline
 *
 * End-to-end feature demo showing:
 *   1. Build a CRM extraction workflow (contacts, companies, deals)
 *   2. Create a "conversation" data source via the org intelligence page
 *   3. Run the workflow against the data source
 *   4. Verify the workflow successfully parsed the data source — MUST produce staged records
 *
 * All feature interactions go through the UI. API calls are used for
 * prerequisite state (auth) and post-test cleanup.
 *
 * Prerequisites:
 *   - Dev server running on FRONTEND_PORT (default 3001)
 *   - Seed database with Powerclub Global organization
 *   - LLM backend accessible (PCG Router) for workflow execution
 */
import { test, expect } from "./fixtures";
import { t, login, apiLogin, TEST_DATA_PREFIX } from "../helpers";

// ── Constants ────────────────────────────────────────────────────────────────

const ORG_ID = "01010101-0101-0101-0101-010101010101"; // Powerclub Global

const CONVERSATION_TITLE = `${TEST_DATA_PREFIX} Client Meeting: Acme Corp ${Date.now()}`;
const WORKFLOW_NAME = `${TEST_DATA_PREFIX} CRM Extract`;
const WORKFLOW_ID = `e2e_crm_extract_${Date.now()}`;

const DEMO_PAUSE = 1_500;

const CONVERSATION_CONTENT = `Meeting Transcript — Acme Corp Partnership Discussion
Date: 2026-03-10

Sarah Chen: Thanks for joining us today, Marcus and Lisa.

Marcus Webb: Our engineering team of about 200 people currently spends too much time on manual deployment. My email is marcus.webb@acmecorp.com, phone +1-555-0142.

Lisa Park: From procurement, we're looking at $450,000 annually. I'm lisa.park@acmecorp.com.

Marcus Webb: Our head of DevOps is Raj Patel — raj.patel@acmecorp.com.

Lisa Park: We also have a $200,000 analytics initiative.

Marcus Webb: Our parent company GlobalTech Industries might be interested too — potentially $2M+.`;

const EXTRACT_CONTACTS_PROMPT =
  "Extract all people mentioned. For each: first_name, last_name, email, phone, company_name, job_title. Return JSON array.";

const EXTRACT_COMPANIES_PROMPT =
  "Extract all companies mentioned. For each: name, industry, employee_count, parent_company. Return JSON array.";

const EXTRACT_DEALS_PROMPT =
  "Extract all potential deals/opportunities. For each: title, value, currency, contact_name, company_name, stage. Return JSON array.";

// ── Workflow Builder Helpers ─────────────────────────────────────────────────

/** Add an LLM Extract node: open picker, click LLM Extract, rename, fill prompt, connect to input. */
async function addExtractNode(
  page: import("@playwright/test").Page,
  name: string,
  prompt: string,
  connectTo: string
) {
  await page.getByRole("button", { name: "Add Node" }).click();
  await page.getByRole("button", { name: /^LLM Extract Extract/ }).click();
  await expect(page.getByText("Node Name")).toBeVisible({ timeout: t(5_000) });

  const nameInput = page.getByText("Node Name", { exact: true }).locator("..").getByRole("textbox");
  await nameInput.fill(name);

  // Fill the prompt template textarea (under "Prompt Template" label)
  const promptTextarea = page.getByRole("textbox", { name: /extraction instructions|Analyze the following/ });
  await expect(promptTextarea).toBeVisible({ timeout: t(5_000) });
  await promptTextarea.fill(prompt);

  await page.getByRole("combobox").filter({ hasText: /Add input connection/ }).click();
  await page.getByRole("option", { name: new RegExp(connectTo) }).first().click();

  await expect(page.getByText(`from: ${connectTo}`).last()).toBeVisible({ timeout: t(5_000) });
  await page.waitForTimeout(DEMO_PAUSE);
}

/** Add an output node and connect it to the specified upstream node. */
async function addOutputNode(
  page: import("@playwright/test").Page,
  outputType: string,
  connectTo: string
) {
  await page.getByRole("button", { name: "Add Node" }).click();
  await page.getByRole("button", { name: new RegExp(`Output: ${outputType}`) }).click();

  await expect(page.getByText("Input Connections")).toBeVisible({ timeout: t(5_000) });

  const connectionCombo = page.getByRole("combobox").filter({ hasText: /Add input connection/ });
  await expect(connectionCombo).toBeVisible({ timeout: t(5_000) });
  await connectionCombo.click();
  await page.getByRole("option", { name: new RegExp(connectTo) }).first().click();

  await expect(page.getByText(`from: ${connectTo}`).last()).toBeVisible({ timeout: t(5_000) });
  await page.waitForTimeout(DEMO_PAUSE);
}

// ── Test Flow ────────────────────────────────────────────────────────────────

test.describe("Workflow → CRM Pipeline Demo", () => {
  test.describe.configure({ mode: "serial" });

  test("Part 1: Build CRM extraction workflow", async ({ page }) => {
    test.setTimeout(120_000);
    await login(page);
    await page.goto("/workflows");
    await expect(page.getByRole("button", { name: "New Workflow" })).toBeVisible({
      timeout: t(10_000),
    });

    await page.getByRole("button", { name: "New Workflow" }).click();
    await expect(page.getByText("Workflow ID")).toBeVisible({ timeout: t(5_000) });

    await page.getByRole("textbox", { name: "my_workflow" }).fill(WORKFLOW_ID);
    await page.getByRole("textbox", { name: "My Workflow" }).fill(WORKFLOW_NAME);
    await page
      .getByRole("textbox", { name: "What does this workflow do?" })
      .fill("Extracts contacts, companies, and deals from client conversations");
    await page.waitForTimeout(DEMO_PAUSE);

    // Add Data Source node
    await page.getByRole("button", { name: "Add Node" }).click();
    await page.getByRole("button", { name: /^Data Source Marks this/ }).click();
    await expect(page.getByText("Data Source").first()).toBeVisible({ timeout: t(5_000) });
    await page.waitForTimeout(DEMO_PAUSE);

    // Add Extract nodes — each connected to Data Source
    await addExtractNode(page, "Extract Contacts", EXTRACT_CONTACTS_PROMPT, "Data Source");
    await addExtractNode(page, "Extract Companies", EXTRACT_COMPANIES_PROMPT, "Data Source");
    await addExtractNode(page, "Extract Deals", EXTRACT_DEALS_PROMPT, "Data Source");

    // Add output nodes — each connected to its extract stage
    await addOutputNode(page, "CRM Contacts", "Extract Contacts");
    await addOutputNode(page, "Companies", "Extract Companies");
    await addOutputNode(page, "CRM Deals", "Extract Deals");

    // Save the workflow
    await page.getByRole("button", { name: "Save Workflow" }).click();
    await expect(page.getByText(WORKFLOW_NAME).first()).toBeVisible({ timeout: t(10_000) });
    await page.waitForTimeout(DEMO_PAUSE);
  });

  test("Part 2: Create conversation data source", async ({ page }) => {
    await page.goto(`/organizations/${ORG_ID}/intelligence/data-sources`);

    await expect(page.getByRole("button", { name: "Add Data Source" })).toBeVisible({
      timeout: t(10_000),
    });
    await page.getByRole("button", { name: "Add Data Source" }).click();

    await expect(page.getByRole("heading", { name: "Add Data Source" })).toBeVisible({
      timeout: t(5_000),
    });

    await page.getByRole("textbox", { name: /Client kickoff/ }).fill(CONVERSATION_TITLE);
    await page.getByRole("textbox", { name: /Paste or type/ }).fill(CONVERSATION_CONTENT);
    await page.waitForTimeout(DEMO_PAUSE);

    await page.getByRole("button", { name: "Add Source" }).click();
    await expect(page.getByRole("heading", { name: "Add Data Source" })).not.toBeVisible({
      timeout: t(5_000),
    });
    await page.waitForTimeout(DEMO_PAUSE);

    const sourceTitle = CONVERSATION_TITLE.replace(`${TEST_DATA_PREFIX} `, "");
    await expect(page.getByText(sourceTitle).first()).toBeVisible({ timeout: t(10_000) });
    await page.waitForTimeout(DEMO_PAUSE);
  });

  test("Part 3: Run workflow and verify data source was parsed", async ({ page }) => {
    test.setTimeout(120_000);
    await page.goto(`/organizations/${ORG_ID}/data-sources`);
    await expect(page.getByText("Data Sources").first()).toBeVisible({ timeout: t(10_000) });

    const sourceTitle = CONVERSATION_TITLE.replace(`${TEST_DATA_PREFIX} `, "");
    await page.getByText(sourceTitle).first().click();

    const runBtn = page.getByRole("button", { name: /Run Workflow/ }).first();
    await expect(runBtn).toBeVisible({ timeout: t(5_000) });
    await runBtn.click();

    await expect(page.getByText("Select a workflow...")).toBeVisible({ timeout: t(5_000) });
    await page.getByRole("combobox").click();

    const workflowName = WORKFLOW_NAME.replace(`${TEST_DATA_PREFIX} `, "");
    await page
      .getByRole("option", { name: new RegExp(workflowName) })
      .first()
      .click();

    await page.waitForTimeout(DEMO_PAUSE);

    // Click the Run Workflow button in the dialog
    await page.getByRole("button", { name: /Run Workflow/ }).last().click();

    // Wait for the workflow to complete — the dialog should change to "Running..."
    // then close on success, redirecting to the staging tab.
    //
    // SUCCESS path: dialog closes → navigates to /workflows?tab=staging&run=...
    // FAILURE path: toast shows "Failed to run workflow" and dialog stays open
    //
    // We MUST end up on the staging tab — anything else means parsing failed.

    // First, wait for the "Running..." state to appear (confirms the request was sent)
    await expect(
      page.getByRole("button", { name: "Running..." })
    ).toBeVisible({ timeout: t(10_000) });

    // Now wait for navigation to staging tab (only happens on successful parse with records)
    await expect(page).toHaveURL(/tab=staging/, { timeout: t(60_000) });

    // Verify we're NOT seeing failure indicators
    const failedText = page.getByText("Failed to run workflow");
    await expect(failedText).not.toBeVisible();

    // Verify the staging tab loaded and has records
    // (the URL includes &run=<id> which auto-selects the run)
    await expect(
      page.getByText(/records staged|contacts|companies|deals/i).first()
    ).toBeVisible({ timeout: t(15_000) });

    await page.waitForTimeout(DEMO_PAUSE);
  });

  test("Part 4: Verify staged records contain parsed CRM data", async ({ page }) => {
    test.setTimeout(60_000);

    // If Part 3 navigated us to staging, we might already be there.
    // Otherwise, go to workflows staging tab directly.
    if (!page.url().includes("tab=staging")) {
      await page.goto("/workflows");
      await expect(page.getByRole("tab", { name: "Staging" })).toBeVisible({
        timeout: t(10_000),
      });
      await page.getByRole("tab", { name: "Staging" }).click();
    }

    // Wait for staging records to load — must NOT show "No staged records"
    const noRecordsText = page.getByText("No staged records for this workflow run.");
    const noRecordsVisible = await noRecordsText.isVisible().catch(() => false);
    expect(noRecordsVisible, "Staging tab shows 'No staged records' — workflow parsing failed").toBe(false);

    // Verify the staging panel contains actual parsed data from the conversation.
    // The conversation mentions: Marcus Webb, Lisa Park, Raj Patel, Sarah Chen,
    // Acme Corp, GlobalTech Industries, and multiple deals.
    //
    // We need at least ONE of these to appear in the staged records to prove
    // the workflow actually parsed the data source content.
    const body = page.locator("body");

    // Check for at least one contact name from the conversation
    const hasContact = await Promise.any([
      expect(body).toContainText("Marcus", { timeout: t(10_000) }).then(() => true),
      expect(body).toContainText("Lisa", { timeout: t(10_000) }).then(() => true),
      expect(body).toContainText("Webb", { timeout: t(10_000) }).then(() => true),
      expect(body).toContainText("Park", { timeout: t(10_000) }).then(() => true),
      expect(body).toContainText("Raj", { timeout: t(10_000) }).then(() => true),
      expect(body).toContainText("Sarah", { timeout: t(10_000) }).then(() => true),
    ]).catch(() => false);

    expect(hasContact).toBeTruthy();

    // Check for at least one company name
    const hasCompany = await Promise.any([
      expect(body).toContainText("Acme", { timeout: t(5_000) }).then(() => true),
      expect(body).toContainText("GlobalTech", { timeout: t(5_000) }).then(() => true),
    ]).catch(() => false);

    expect(hasCompany).toBeTruthy();

    // Verify record type indicators are present (from TARGET_TYPE_CONFIG labels)
    // At minimum we should see contacts and companies extracted
    const pageText = await page.textContent("body");
    const hasRecordTypes =
      pageText?.includes("Contacts") || pageText?.includes("contact") ||
      pageText?.includes("Companies") || pageText?.includes("company");
    expect(hasRecordTypes).toBeTruthy();

    // Verify specific extracted field values (emails from transcript)
    const hasEmail = await Promise.any([
      expect(body).toContainText("marcus.webb@acmecorp.com", { timeout: t(5_000) }).then(() => true),
      expect(body).toContainText("lisa.park@acmecorp.com", { timeout: t(5_000) }).then(() => true),
      expect(body).toContainText("raj.patel@acmecorp.com", { timeout: t(5_000) }).then(() => true),
    ]).catch(() => false);
    expect(hasEmail, "No email addresses found in staged records").toBeTruthy();

    // Verify deal amounts were extracted
    const hasAmount = await Promise.any([
      expect(body).toContainText("450000", { timeout: t(5_000) }).then(() => true),
      expect(body).toContainText("450,000", { timeout: t(5_000) }).then(() => true),
      expect(body).toContainText("200000", { timeout: t(5_000) }).then(() => true),
      expect(body).toContainText("200,000", { timeout: t(5_000) }).then(() => true),
    ]).catch(() => false);
    expect(hasAmount, "No deal amounts found in staged records").toBeTruthy();

    // Verify staging actions are available (approve/commit buttons)
    const approveBtn = page.getByRole("button", { name: /approve|commit/i }).first();
    await expect(approveBtn).toBeVisible({ timeout: t(5_000) });

    await page.waitForTimeout(DEMO_PAUSE * 2);
  });

  // ─── Cleanup ───────────────────────────────────────────────────────────

  test.afterAll(async ({ request }) => {
    await apiLogin(request);

    // Delete test workflow
    const workflowsRes = await request.get("/api/workflows/definitions");
    if (workflowsRes.ok()) {
      const workflows = await workflowsRes.json();
      const list = workflows.data || workflows || [];
      for (const wf of list) {
        if (wf.name?.startsWith(TEST_DATA_PREFIX) || wf.id?.startsWith("e2e_")) {
          await request.delete(`/api/workflows/definitions/${wf.id}`).catch(() => {});
        }
      }
    }

    // Delete test data sources
    const dsRes = await request.get(`/api/organizations/${ORG_ID}/data-sources`);
    if (dsRes.ok()) {
      const sources = await dsRes.json();
      const list = sources.data || sources || [];
      for (const ds of list) {
        if (ds.title?.startsWith(TEST_DATA_PREFIX)) {
          await request.delete(`/api/data-sources/${ds.id}`).catch(() => {});
        }
      }
    }
  });
});
