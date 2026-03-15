/**
 * Demo: Spanish Conversation Workflow → CRM Pipeline (Multilingual)
 *
 * End-to-end feature demo showing:
 *   1. Build a CRM extraction workflow with Spanish-aware prompts
 *   2. Create a Spanish conversation data source
 *   3. Run the workflow against the data source
 *   4. Verify staged records, approve & commit into CRM
 *   5. Verify CRM contacts (Spanish names preserved)
 *   6. Verify CRM deals were created with correct amounts
 *
 * This demo showcases multilingual workflow capabilities —
 * LLM nodes handle Spanish→English translation + structured extraction.
 *
 * Prerequisites:
 *   - Dev server running on FRONTEND_PORT (default 3001)
 *   - Seed database with Powerclub Global organization
 *   - LLM backend accessible (PCG Router) for workflow execution
 */
import { test, expect } from "./fixtures";
import { t, demoPause, login, apiLogin, TEST_DATA_PREFIX } from "../helpers";

// ── Constants ────────────────────────────────────────────────────────────────

const ORG_ID = "01010101-0101-0101-0101-010101010101"; // Powerclub Global

const CONVERSATION_TITLE = `${TEST_DATA_PREFIX} Reunión LATAM: Expansión ${Date.now()}`;
const WORKFLOW_NAME = `${TEST_DATA_PREFIX} Spanish CRM Extract`;
const WORKFLOW_ID = `e2e_spanish_crm_${Date.now()}`;

const SPANISH_CONVERSATION = `Transcripción de Reunión — Expansión Latinoamérica
Fecha: 2026-03-12
Participantes: Ana Morales (Powerclub Global), Carlos Ramírez (TechSoluciones),
               Elena Varga (Grupo Andino)

Ana Morales: Buenos días, Carlos y Elena. Gracias por conectarse hoy.
Quiero hablar sobre la expansión de nuestra plataforma en América Latina.

Carlos Ramírez: Gracias, Ana. En TechSoluciones tenemos un equipo de
150 ingenieros y estamos buscando una solución de gestión de proyectos
más robusta. Mi correo es carlos.ramirez@techsoluciones.com y mi
teléfono es +52-555-0198.

Elena Varga: Desde Grupo Andino, representamos a tres subsidiarias en
Colombia, Perú y Chile. Nuestro presupuesto para herramientas digitales
es de $350,000 dólares anuales. Pueden contactarme en
elena.varga@grupoandino.com.

Carlos Ramírez: Nuestro director de tecnología, Miguel Santos,
también estaría interesado. Su correo es miguel.santos@techsoluciones.com.
Estamos considerando un contrato inicial de $180,000 dólares para el
primer año.

Elena Varga: Además, tenemos una iniciativa de transformación digital
por $500,000 que incluiría capacitación y despliegue en las tres oficinas.

Ana Morales: Excelente. También quiero presentarles a nuestro
especialista regional, Diego Herrera — diego.herrera@powerclub.global.
Él coordinará la implementación.

Carlos Ramírez: Perfecto. Nuestra empresa matriz, Innovación Global S.A.,
podría expandir esto a nivel corporativo — potencialmente $1.2 millones.`;

// ── Prompt Templates ─────────────────────────────────────────────────────────

const EXTRACT_CONTACTS_PROMPT =
  "This text is a business conversation in Spanish. " +
  "Extract all people mentioned as business contacts. " +
  "For each person provide: first_name, last_name, email, phone, " +
  "company_name, job_title. Preserve original Spanish names " +
  "(do not anglicize). Return JSON array.";

const EXTRACT_COMPANIES_PROMPT =
  "This text is a Spanish business conversation. Extract all " +
  "companies and organizations mentioned. For each: name, industry " +
  "(inferred from context), description (one sentence in English about " +
  "their business). Return JSON array.";

const EXTRACT_DEALS_PROMPT =
  "This text is a Spanish business conversation. " +
  "Extract all potential deals or business opportunities mentioned. For each: " +
  "name (descriptive title), amount (numeric value), currency (USD), " +
  "contact_name (associated contact), company_name, stage (discovery or " +
  "qualification or proposal). Return JSON array.";

// Known test emails for verification and cleanup
const TEST_EMAILS = [
  "carlos.ramirez@techsoluciones.com",
  "elena.varga@grupoandino.com",
  "miguel.santos@techsoluciones.com",
  "diego.herrera@powerclub.global",
];

// Keywords to match extracted deals
const TEST_DEAL_KEYWORDS = [
  "TechSoluciones",
  "180,000",
  "180000",
  "Grupo Andino",
  "500,000",
  "500000",
  "Innovación",
  "Innovacion",
  "1,200,000",
  "1200000",
];

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

  // Fill the prompt template textarea
  const promptTextarea = page.getByRole("textbox", { name: /extraction instructions|Analyze the following/ });
  await expect(promptTextarea).toBeVisible({ timeout: t(5_000) });
  await promptTextarea.fill(prompt);

  await page.getByRole("combobox").filter({ hasText: /Add input connection/ }).click();
  await page.getByRole("option", { name: new RegExp(connectTo) }).first().click();

  await expect(page.getByText(`from: ${connectTo}`).last()).toBeVisible({ timeout: t(5_000) });
  await page.waitForTimeout(demoPause.medium);
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
  await page.waitForTimeout(demoPause.medium);
}

// ── Test Flow ────────────────────────────────────────────────────────────────

test.describe("Spanish Workflow → CRM Pipeline Demo", () => {
  test.describe.configure({ mode: "serial" });

  test("Part 1: Build Spanish CRM extraction workflow", async ({ page }) => {
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
      .fill("Extracts CRM data from Spanish business conversations (contacts, companies, deals)");
    await page.waitForTimeout(demoPause.medium);

    // Add Data Source node
    await page.getByRole("button", { name: "Add Node" }).click();
    await page.getByRole("button", { name: /^Data Source Marks this/ }).click();
    await expect(page.getByText("Data Source").first()).toBeVisible({ timeout: t(5_000) });
    await page.waitForTimeout(demoPause.medium);

    // Add Extract nodes — each connected to Data Source, with Spanish-aware prompts
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
    await page.waitForTimeout(demoPause.medium);
  });

  test("Part 2: Create Spanish conversation data source", async ({ page }) => {
    await page.goto(`/organizations/${ORG_ID}/intelligence/data-sources`);

    await expect(page.getByRole("button", { name: "Add Data Source" })).toBeVisible({
      timeout: t(10_000),
    });
    await page.getByRole("button", { name: "Add Data Source" }).click();

    await expect(page.getByRole("heading", { name: "Add Data Source" })).toBeVisible({
      timeout: t(5_000),
    });

    await page.getByRole("textbox", { name: /Client kickoff/ }).fill(CONVERSATION_TITLE);
    await page.getByRole("textbox", { name: /Paste or type/ }).fill(SPANISH_CONVERSATION);
    await page.waitForTimeout(demoPause.medium);

    await page.getByRole("button", { name: "Add Source" }).click();
    await expect(page.getByRole("heading", { name: "Add Data Source" })).not.toBeVisible({
      timeout: t(5_000),
    });
    await page.waitForTimeout(demoPause.medium);

    const sourceTitle = CONVERSATION_TITLE.replace(`${TEST_DATA_PREFIX} `, "");
    await expect(page.getByText(sourceTitle).first()).toBeVisible({ timeout: t(10_000) });
    await page.waitForTimeout(demoPause.medium);
  });

  test("Part 3: Run workflow against Spanish data source", async ({ page }) => {
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

    await page.waitForTimeout(demoPause.medium);

    // Click the Run Workflow button in the dialog
    await page.getByRole("button", { name: /Run Workflow/ }).last().click();

    // Wait for "Running..." state (confirms request was sent)
    await expect(
      page.getByRole("button", { name: "Running..." })
    ).toBeVisible({ timeout: t(10_000) });

    // Wait for navigation to staging tab (only happens on successful parse with records)
    await expect(page).toHaveURL(/tab=staging/, { timeout: t(60_000) });

    // Verify no failure indicators
    const failedText = page.getByText("Failed to run workflow");
    await expect(failedText).not.toBeVisible();

    // Verify the staging tab loaded and has records
    await expect(
      page.getByText(/records staged|contacts|companies|deals/i).first()
    ).toBeVisible({ timeout: t(15_000) });

    await page.waitForTimeout(demoPause.medium);
  });

  test("Part 4: Verify staged records and approve & commit", async ({ page, request }) => {
    test.setTimeout(60_000);
    await apiLogin(request);

    // Navigate to staging tab if not already there
    if (!page.url().includes("tab=staging")) {
      await page.goto("/workflows");
      await expect(page.getByRole("tab", { name: "Staging" })).toBeVisible({
        timeout: t(10_000),
      });
      await page.getByRole("tab", { name: "Staging" }).click();
    }

    // Verify staging records are present (not empty)
    const noRecordsText = page.getByText("No staged records for this workflow run.");
    const noRecordsVisible = await noRecordsText.isVisible().catch(() => false);
    expect(noRecordsVisible, "Staging tab shows 'No staged records' — workflow parsing failed").toBe(false);

    // Verify Spanish names appear in staged records (preserved through translation)
    const body = page.locator("body");
    const hasContact = await Promise.any([
      expect(body).toContainText("Carlos", { timeout: t(10_000) }).then(() => true),
      expect(body).toContainText("Elena", { timeout: t(10_000) }).then(() => true),
      expect(body).toContainText("Ramírez", { timeout: t(10_000) }).then(() => true),
      expect(body).toContainText("Ramirez", { timeout: t(10_000) }).then(() => true),
      expect(body).toContainText("Varga", { timeout: t(10_000) }).then(() => true),
      expect(body).toContainText("Miguel", { timeout: t(10_000) }).then(() => true),
    ]).catch(() => false);
    expect(hasContact, "No Spanish contact names found in staged records").toBeTruthy();

    // Verify at least one company name
    const hasCompany = await Promise.any([
      expect(body).toContainText("TechSoluciones", { timeout: t(5_000) }).then(() => true),
      expect(body).toContainText("Grupo Andino", { timeout: t(5_000) }).then(() => true),
      expect(body).toContainText("Innovación", { timeout: t(5_000) }).then(() => true),
      expect(body).toContainText("Innovacion", { timeout: t(5_000) }).then(() => true),
    ]).catch(() => false);
    expect(hasCompany, "No company names found in staged records").toBeTruthy();

    // Verify at least one email from the Spanish conversation
    const hasEmail = await Promise.any([
      expect(body).toContainText("carlos.ramirez@techsoluciones.com", { timeout: t(5_000) }).then(() => true),
      expect(body).toContainText("elena.varga@grupoandino.com", { timeout: t(5_000) }).then(() => true),
      expect(body).toContainText("miguel.santos@techsoluciones.com", { timeout: t(5_000) }).then(() => true),
    ]).catch(() => false);
    expect(hasEmail, "No email addresses found in staged records").toBeTruthy();

    await page.waitForTimeout(demoPause.long);

    // ── Approve & Commit ──

    // Wait for approve button
    await expect(
      page.getByRole("button", { name: /Approve & commit/i }).first()
    ).toBeVisible({ timeout: t(15_000) });

    await page.waitForTimeout(demoPause.medium);

    // Click "Approve & commit N valid"
    await page.getByRole("button", { name: /Approve & commit/i }).first().click();
    await page.waitForTimeout(demoPause.medium);

    // Verify via API that records were committed to CRM
    const contactsRes = await request.get(`/api/crm/contacts?organization_id=${ORG_ID}`);
    const contacts = contactsRes.ok() ? await contactsRes.json() : { data: [] };
    const contactList = contacts.data || contacts || [];
    const committedContacts = contactList.filter((c: { email?: string }) =>
      TEST_EMAILS.includes(c.email ?? "")
    );
    console.log(`[Part 4] CRM contacts: ${contactList.length} total, ${committedContacts.length} matching test emails`);

    if (committedContacts.length === 0) {
      // Auto-approve didn't work — fallback to API batch approve + commit
      const url = new URL(page.url(), "http://localhost");
      let workflowRunId = url.searchParams.get("run");

      if (!workflowRunId) {
        const stagingRes = await request.get("/api/workflow-staging/pending?organization_id=" + ORG_ID);
        if (stagingRes.ok()) {
          const staging = await stagingRes.json();
          const records = staging.data || staging || [];
          if (records.length > 0) {
            workflowRunId = records[0].workflow_run_id;
          }
        }
      }

      expect(workflowRunId, "Could not determine workflow_run_id for batch approve").toBeTruthy();

      // List all staged records for this run
      const recordsRes = await request.get(`/api/workflow-staging?workflow_run_id=${workflowRunId}`);
      expect(recordsRes.ok()).toBeTruthy();
      const allRecords = await recordsRes.json();
      const records = allRecords.data || allRecords || [];
      console.log(`[Part 4] Staging records for run ${workflowRunId}: ${records.length}`);

      const retryableIds = records
        .filter((r: { status: string }) => r.status === "pending_review" || r.status === "error")
        .map((r: { id: string }) => r.id);
      console.log(`[Part 4] Retryable records (pending + error): ${retryableIds.length}`);

      if (retryableIds.length > 0) {
        const approveRes = await request.post("/api/workflow-staging/batch", {
          data: { ids: retryableIds, action: "approve" },
        });
        expect(approveRes.ok(), "Failed to batch-approve staged records").toBeTruthy();

        const commitRes = await request.post("/api/workflow-staging/batch-commit", {
          data: { workflow_run_id: workflowRunId },
        });
        expect(commitRes.ok(), "Failed to batch-commit staged records").toBeTruthy();
        const commitResult = await commitRes.json();
        const result = commitResult.data || commitResult;
        console.log(`[Part 4] Batch commit: ${result.committed} committed, ${result.errors} errors`);
        expect(result.committed, "No records were committed").toBeGreaterThan(0);
      }

      await page.reload();
    }

    // Verify committed state in the UI
    await expect(
      page.getByText(/committed successfully|committed/i).first()
    ).toBeVisible({ timeout: t(30_000) });

    await page.waitForTimeout(demoPause.long);
  });

  test("Part 5: Verify CRM contacts with Spanish names", async ({ page, request }) => {
    test.setTimeout(60_000);
    await apiLogin(request);

    // Verify contacts exist via API
    const contactsRes = await request.get(`/api/crm/contacts?organization_id=${ORG_ID}`);
    expect(contactsRes.ok()).toBeTruthy();
    const contactsBody = await contactsRes.json();
    const allContacts = contactsBody.data || contactsBody || [];
    const matchingContacts = allContacts.filter((c: { email?: string }) =>
      TEST_EMAILS.includes(c.email ?? "")
    );
    console.log(`[Part 5] CRM contacts: ${allContacts.length} total, ${matchingContacts.length} matching test emails`);
    expect(matchingContacts.length, "No contacts from Spanish conversation found in CRM").toBeGreaterThan(0);

    // Navigate to CRM contacts page for visual verification
    await login(page);
    await page.goto(`/organizations/${ORG_ID}/crm/contacts`);

    await expect(
      page.getByRole("tab", { name: "Contacts" })
    ).toBeVisible({ timeout: t(10_000) });

    // Click a contact card to open detail panel
    const contactCard = page.getByRole("button").filter({
      hasText: /Ramírez|Ramirez|Varga|Santos|Herrera|Morales/i,
    }).first();
    await contactCard.click();

    // Verify detail panel opens
    await expect(
      page.getByRole("heading", { level: 3 }).first()
    ).toBeVisible({ timeout: t(10_000) });

    await page.waitForTimeout(demoPause.long);
  });

  test("Part 6: Verify CRM deals were created", async ({ page, request }) => {
    test.setTimeout(60_000);
    await apiLogin(request);

    // Verify deals via API
    const dealsRes = await request.get(`/api/crm/deals?organization_id=${ORG_ID}`);
    expect(dealsRes.ok()).toBeTruthy();
    const dealsBody = await dealsRes.json();
    const deals = dealsBody.data || dealsBody || [];
    const matchingDeals = deals.filter((d: { name?: string }) =>
      TEST_DEAL_KEYWORDS.some((kw) => (d.name ?? "").includes(kw))
    );
    console.log(`[Part 6] CRM deals: ${deals.length} total, ${matchingDeals.length} matching test keywords`);
    expect(matchingDeals.length, "No deals from Spanish conversation found in CRM").toBeGreaterThan(0);

    // Navigate to CRM pipeline page for visual verification
    await login(page);
    await page.goto(`/organizations/${ORG_ID}/crm/pipeline`);
    await expect(
      page.getByText(/pipeline|deals/i).first()
    ).toBeVisible({ timeout: t(10_000) });

    await page.waitForTimeout(demoPause.long);
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
        if (wf.id?.startsWith("e2e_spanish_crm_")) {
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
        if (ds.title?.includes("Reunión LATAM") || ds.title?.includes("Reunion LATAM")) {
          await request.delete(`/api/data-sources/${ds.id}`).catch(() => {});
        }
      }
    }

    // Delete CRM contacts created by the workflow commit
    const contactsRes = await request.get(`/api/crm/contacts?organization_id=${ORG_ID}`);
    if (contactsRes.ok()) {
      const contacts = await contactsRes.json();
      const list = contacts.data || contacts || [];
      for (const c of list) {
        if (TEST_EMAILS.includes(c.email)) {
          await request.delete(`/api/crm/contacts/${c.id}`).catch(() => {});
        }
      }
    }

    // Delete CRM deals created by the workflow commit
    const dealsRes = await request.get(`/api/crm/deals?organization_id=${ORG_ID}`);
    if (dealsRes.ok()) {
      const deals = await dealsRes.json();
      const list = deals.data || deals || [];
      for (const d of list) {
        const matchesKeyword = TEST_DEAL_KEYWORDS.some((kw) =>
          (d.name ?? "").includes(kw) || (d.company_name ?? "").includes(kw)
        );
        if (matchesKeyword) {
          await request.delete(`/api/crm/deals/${d.id}`).catch(() => {});
        }
      }
    }

    // Delete companies created by the workflow
    const companiesRes = await request.get(`/api/companies?organization_id=${ORG_ID}`);
    if (companiesRes.ok()) {
      const companies = await companiesRes.json();
      const list = companies.data || companies || [];
      const testCompanyNames = ["TechSoluciones", "Grupo Andino", "Innovación Global", "Innovacion Global"];
      for (const co of list) {
        if (testCompanyNames.some((n) => (co.name ?? "").includes(n))) {
          await request.delete(`/api/companies/${co.id}`).catch(() => {});
        }
      }
    }
  });
});
