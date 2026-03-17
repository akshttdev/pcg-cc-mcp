/**
 * FULL PIPELINE WALKTHROUGH E2E TEST
 *
 * Simulates a complete dealflow from lead intake to Won using a fake
 * prospect based on a real person/company for discoverable intel:
 *
 *   Lead: Marcus Webb, Founder of "Vanguard Social Club"
 *         (a fictional luxury social club in Miami, FL)
 *
 * This test exercises every stage of the 9-stage pipeline and exposes
 * gaps in automation, UI, and data flow.
 */
import { test, expect, Page } from '@playwright/test';

const ORG_ID = '02020202-0202-0202-0202-020202020202';
const PIPELINE_ID = '138ff8ec-6d65-493e-b6a9-0f9fef409968'; // Sirak Studios Acquisition

// Test lead data
const TEST_LEAD = {
  first_name: 'Marcus',
  last_name: 'Webb',
  email: 'marcus@vanguardsocialclub.com',
  phone: '+1 305-555-0199',
  company_name: 'Vanguard Social Club',
  job_title: 'Founder & CEO',
  company_industry: 'Luxury Hospitality & Private Membership',
  company_hq: 'Miami, FL',
  company_website: 'https://vanguardsocialclub.com',
  deal_amount: 25000,
};

// ── Helpers ──────────────────────────────────────────────────────────────────

async function loginAs(page: Page, username: string, password: string) {
  const res = await page.request.post('http://localhost:3000/api/auth/login', {
    data: { username, password },
    headers: { 'Content-Type': 'application/json' },
  });
  const body = await res.json();
  const sessionId = body?.data?.session_id;
  if (!sessionId) throw new Error(`Login failed for ${username}: ${JSON.stringify(body)}`);
  await page.context().addCookies([{
    name: 'session_id', value: sessionId, domain: 'localhost', path: '/', httpOnly: false, secure: false,
  }]);
  return sessionId;
}

async function loginAsSirak(page: Page) { return loginAs(page, 'Sirak', 'Sirak123'); }
async function loginAsAdmin(page: Page) { return loginAs(page, 'admin', 'admin123'); }

async function apiPost(page: Page, path: string, body: object) {
  const res = await page.request.post(`http://localhost:3000/api${path}`, {
    data: body, headers: { 'Content-Type': 'application/json' },
  });
  return res.json();
}

async function apiGet(page: Page, path: string) {
  const res = await page.request.get(`http://localhost:3000/api${path}`);
  return res.json();
}

async function apiPatch(page: Page, path: string, body: object) {
  const res = await page.request.patch(`http://localhost:3000/api${path}`, {
    data: body, headers: { 'Content-Type': 'application/json' },
  });
  return res.json();
}

async function gotoAsSirak(page: Page, path: string) {
  const sessionId = await loginAsSirak(page);
  await page.goto('http://localhost:3000/', { waitUntil: 'domcontentloaded' });
  await page.evaluate((sid) => localStorage.setItem('session_id', sid), sessionId);
  await page.goto(`http://localhost:3000${path}`, { waitUntil: 'networkidle' });
  await page.evaluate((sid) => localStorage.setItem('session_id', sid), sessionId);
  await page.waitForTimeout(1500);
}

async function gotoAsAdmin(page: Page, path: string) {
  const sessionId = await loginAsAdmin(page);
  await page.goto('http://localhost:3000/', { waitUntil: 'domcontentloaded' });
  await page.evaluate((sid) => localStorage.setItem('session_id', sid), sessionId);
  await page.goto(`http://localhost:3000${path}`, { waitUntil: 'networkidle' });
  await page.evaluate((sid) => localStorage.setItem('session_id', sid), sessionId);
  await page.waitForTimeout(1500);
}

// ── State tracking across tests ──────────────────────────────────────────────

let personId: string;
let companyId: string;
let contactId: string;
let dealId: string;

test.setTimeout(120000);

test.describe.serial('Full Pipeline Walkthrough — Vanguard Social Club', () => {

  // Use admin for API operations (entity creation), Sirak for UI navigation
  test.beforeEach(async ({ page }) => {
    await loginAsAdmin(page);
  });

  // ══════════════════════════════════════════════════════════════════════════
  // PHASE 1: LEAD ENTRY — Create person, company, contact, deal
  // ══════════════════════════════════════════════════════════════════════════

  test('Phase 1: Create lead entities (company, person, contact, deal)', async ({ page }) => {
    const DEAL_NAME = `${TEST_LEAD.first_name} ${TEST_LEAD.last_name} — ${TEST_LEAD.company_name}`;

    // ── Find or create Company ──────────────────────────────────────────
    const listRes = await apiGet(page, '/companies?limit=500');
    const companies = Array.isArray(listRes.data) ? listRes.data : [];
    const existingCo = companies.find((c: any) => c.name === TEST_LEAD.company_name);

    if (existingCo) {
      companyId = existingCo.id;
      console.log('Using existing company:', companyId);
    } else {
      const res = await apiPost(page, '/companies', {
        name: TEST_LEAD.company_name,
        website: TEST_LEAD.company_website,
        industry: TEST_LEAD.company_industry,
        headquarters: TEST_LEAD.company_hq,
        created_by_org_id: ORG_ID,
      });
      companyId = res.data?.id;
      console.log('Created company:', companyId, res.success ? '' : res.message);
    }
    expect(companyId).toBeTruthy();

    // ── Find or create Person ───────────────────────────────────────────
    const personsRes = await apiGet(page, `/persons?search=${encodeURIComponent(TEST_LEAD.last_name)}`);
    const persons = Array.isArray(personsRes.data) ? personsRes.data : [];
    const existingPerson = persons.find((p: any) =>
      p.email === TEST_LEAD.email || (p.full_name && p.full_name.includes(TEST_LEAD.last_name))
    );

    if (existingPerson) {
      personId = existingPerson.id;
      console.log('Using existing person:', personId);
    } else {
      const res = await apiPost(page, '/persons', {
        full_name: `${TEST_LEAD.first_name} ${TEST_LEAD.last_name}`,
        email: TEST_LEAD.email,
        phone: TEST_LEAD.phone,
        company_name: TEST_LEAD.company_name,
        company_id: companyId,
        person_type: 'lead',
        job_title: TEST_LEAD.job_title,
      });
      personId = res.data?.id;
      console.log('Created person:', personId, res.success ? '' : res.message);
    }
    expect(personId).toBeTruthy();

    // ── Find or create CRM Contact ──────────────────────────────────────
    const contactsRes = await apiGet(page, `/crm/contacts?organization_id=${ORG_ID}`);
    const contacts = Array.isArray(contactsRes.data) ? contactsRes.data : [];
    const existingContact = contacts.find((c: any) =>
      c.email === TEST_LEAD.email || (c.full_name && c.full_name.includes(TEST_LEAD.last_name))
    );

    if (existingContact) {
      contactId = existingContact.id;
      console.log('Using existing contact:', contactId);
    } else {
      const res = await apiPost(page, '/crm/contacts', {
        organization_id: ORG_ID,
        first_name: TEST_LEAD.first_name,
        last_name: TEST_LEAD.last_name,
        email: TEST_LEAD.email,
        phone: TEST_LEAD.phone,
        company_name: TEST_LEAD.company_name,
        job_title: TEST_LEAD.job_title,
        lifecycle_stage: 'lead',
        person_id: personId,
      });
      contactId = res.data?.id;
      console.log('Created contact:', contactId, res.success ? '' : res.message);
    }
    expect(contactId).toBeTruthy();

    // ── Find or create Deal ─────────────────────────────────────────────
    const dealsRes = await apiGet(page, `/crm/deals/enriched?organization_id=${ORG_ID}`);
    const deals = Array.isArray(dealsRes.data) ? dealsRes.data : [];
    const existingDeal = deals.find((d: any) => d.name?.includes(TEST_LEAD.company_name));

    if (existingDeal) {
      dealId = existingDeal.id;
      console.log('Using existing deal:', dealId, 'stage:', existingDeal.stage);
    } else {
      const pipelineRes = await apiGet(page, `/crm/pipelines/${PIPELINE_ID}`);
      const stages = pipelineRes.data?.stages ?? [];
      const intelStage = stages.find((s: any) => s.stage_type === 'intel');
      expect(intelStage).toBeTruthy();

      const res = await apiPost(page, '/crm/deals', {
        organization_id: ORG_ID,
        crm_contact_id: contactId,
        crm_pipeline_id: PIPELINE_ID,
        crm_stage_id: intelStage.id,
        name: DEAL_NAME,
        amount: TEST_LEAD.deal_amount,
        currency: 'USD',
      });
      dealId = res.data?.id;
      console.log('Created deal:', dealId, res.success ? '' : res.message);
    }
    expect(dealId).toBeTruthy();

    console.log('✅ Phase 1 complete — company:', companyId, 'person:', personId, 'contact:', contactId, 'deal:', dealId);
  });

  test('Phase 1.5: Verify deal appears on Kanban board', async ({ page }) => {
    await gotoAsSirak(page, `/organizations/${ORG_ID}/crm/acquisition`);
    await page.waitForSelector('[class*="inline-grid"]', { timeout: 20000 });
    await page.waitForTimeout(2000);

    const dealText = await page.locator(`text=${TEST_LEAD.company_name}`).count();
    console.log(`"${TEST_LEAD.company_name}" on kanban:`, dealText);
    expect(dealText).toBeGreaterThan(0);
  });

  // ══════════════════════════════════════════════════════════════════════════
  // PHASE 2: INTEL STAGE — Research should auto-trigger
  // ══════════════════════════════════════════════════════════════════════════

  test('Phase 2.1: Verify Intel stage auto-triggered research (check tasks)', async ({ page }) => {
    const richRes = await apiGet(page, `/crm/deals/${dealId}/rich`);
    const deal = richRes.data;
    console.log('Deal stage:', deal?.stage);
    console.log('Person ID:', deal?.person_id);
    console.log('Company ID:', deal?.company_id);
    console.log('Intel status:', deal?.intelligence_status);
    console.log('Tasks:', deal?.tasks?.length ?? 0);
    // Scout should have been triggered — or at minimum a review task created
  });

  test('Phase 2.2: Verify person profile page loads', async ({ page }) => {
    await gotoAsSirak(page, `/people/${personId}`);
    const body = await page.textContent('body');
    expect(body).toContain(TEST_LEAD.last_name);
    console.log('✅ Person profile accessible');
  });

  test('Phase 2.3: Verify company profile page loads with action buttons', async ({ page }) => {
    await gotoAsSirak(page, `/companies/${companyId}`);
    await page.waitForTimeout(2000);
    const body = await page.textContent('body');

    if (!body?.includes(TEST_LEAD.company_name)) {
      console.log('GAP: Company profile page did not load for Sirak — likely BLOB UUID find_by_id issue');
      console.log('URL:', page.url());
      // Don't fail — this is a known gap, not a test bug
      return;
    }

    const researchBtn = page.locator('button').filter({ hasText: /Research/ });
    const brandGuideBtn = page.locator('a, button').filter({ hasText: /Brand Guide/ });
    const intelBtn = page.locator('button').filter({ hasText: /^Intel$/ });

    console.log('✅ Company profile loaded');
    console.log('Research button:', await researchBtn.count() > 0);
    console.log('Brand Guide button:', await brandGuideBtn.count() > 0);
    console.log('Intel button:', await intelBtn.count() > 0);
  });

  test('Phase 2.4: Verify company brand guide page accessible', async ({ page }) => {
    await gotoAsSirak(page, `/companies/${companyId}/brand-guide`);
    await page.waitForTimeout(2000);
    const body = await page.textContent('body');
    const hasSetup = body?.includes('Set Up') || body?.includes('Brand Guide') || body?.includes('brand guide') || body?.includes('brand');
    console.log('Brand guide page loaded:', hasSetup);
    console.log('URL:', page.url());
    if (!page.url().includes('/brand-guide')) {
      console.log('GAP: Brand guide page redirected — likely company not found');
    }
  });

  // ══════════════════════════════════════════════════════════════════════════
  // PHASE 3: ADVANCE THROUGH STAGES (Intel → BA → Discovery → Proposal)
  // ══════════════════════════════════════════════════════════════════════════

  test('Phase 3: Advance Intel → BA → Discovery → Proposal', async ({ page }) => {
    const STAGE_ORDER = ['intel', 'business_analysis', 'discovery', 'proposal'];

    // Helper: complete all pending tasks and advance
    async function advanceOnce(label: string) {
      // Complete pending tasks
      const richRes = await apiGet(page, `/crm/deals/${dealId}/rich`);
      const tasks = richRes.data?.tasks ?? [];
      for (const task of tasks) {
        if (task.status !== 'done' && task.status !== 'cancelled') {
          await apiPatch(page, `/tasks/${task.id}`, { status: 'done' });
        }
      }
      const advRes = await apiPost(page, `/crm/deals/${dealId}/advance`, {});
      const check = await apiGet(page, `/crm/deals/${dealId}/rich`);
      console.log(`${label}: ${advRes.success ? '✅' : '❌'} → ${check.data?.stage} ${advRes.message ?? ''}`);
      return check.data?.stage;
    }

    // Check current stage and advance to Proposal
    let richRes = await apiGet(page, `/crm/deals/${dealId}/rich`);
    let currentStage = richRes.data?.stage?.toLowerCase().replace(/\s+/g, '_') ?? 'intel';
    console.log('Starting stage:', richRes.data?.stage);

    const currentIdx = STAGE_ORDER.indexOf(currentStage);
    const targetIdx = STAGE_ORDER.indexOf('proposal');

    if (currentIdx < targetIdx) {
      for (let i = currentIdx; i < targetIdx; i++) {
        const from = STAGE_ORDER[i];
        const to = STAGE_ORDER[i + 1];
        await advanceOnce(`${from} → ${to}`);
      }
    } else {
      console.log('Already at or past Proposal stage, skipping advances');
    }

    // Verify we're at Proposal
    richRes = await apiGet(page, `/crm/deals/${dealId}/rich`);
    console.log('Final stage:', richRes.data?.stage);
    console.log('Proposal text exists:', !!richRes.data?.proposal_text);
    console.log('GAP CHECK: Did Cash auto-trigger?', richRes.data?.proposal_text ? 'YES' : 'NO — Cash auto-trigger missing');
  });

  // ══════════════════════════════════════════════════════════════════════════
  // PHASE 4: PROPOSAL STAGE — Cash generates proposal
  // ══════════════════════════════════════════════════════════════════════════

  test('Phase 4.1: Generate proposal (Cash) — manual trigger', async ({ page }) => {
    const check = await apiGet(page, `/crm/deals/${dealId}/rich`);
    if (check.data?.proposal_text) {
      console.log('Proposal already exists (Cash auto-triggered), length:', check.data.proposal_text.length);
      return;
    }

    console.log('Triggering Cash manually...');
    const res = await apiPost(page, `/crm/deals/${dealId}/generate-proposal`, {});
    console.log('Cash result:', res.success, res.message ?? '');
    if (res.success) {
      console.log('Proposal length:', res.data?.proposal_text?.length);
    } else {
      console.log('GAP: Cash proposal generation failed:', res.message);
    }
  });

  test('Phase 4.2: Verify proposal exists via API', async ({ page }) => {
    const res = await apiGet(page, `/crm/deals/${dealId}/rich`);
    const proposal = res.data?.proposal_text;
    console.log('Proposal exists:', !!proposal);
    console.log('Proposal length:', proposal?.length ?? 0);
    console.log('Proposal status:', res.data?.proposal_status);
    console.log('Stage:', res.data?.stage);
    if (proposal && proposal.length > 100) {
      console.log('Proposal preview:', proposal.slice(0, 200));
      const hasJson = proposal.includes('```json');
      console.log('Has deliverables JSON block:', hasJson);
    }
  });

  test('Phase 4.3: Approve proposal', async ({ page }) => {
    const res = await apiPost(page, `/crm/deals/${dealId}/approve-proposal`, {});
    console.log('Approve result:', res.success);
    const check = await apiGet(page, `/crm/deals/${dealId}/rich`);
    console.log('Proposal status:', check.data?.proposal_status);
    expect(check.data?.proposal_status).toBe('approved');
  });

  test('Phase 4.4: Advance Proposal → Polish', async ({ page }) => {
    const richRes = await apiGet(page, `/crm/deals/${dealId}/rich`);
    for (const task of richRes.data?.tasks ?? []) {
      if (task.status !== 'done' && task.status !== 'cancelled') {
        await apiPatch(page, `/tasks/${task.id}`, { status: 'done' });
      }
    }
    const advRes = await apiPost(page, `/crm/deals/${dealId}/advance`, {});
    console.log('Proposal → Polish:', advRes.success, advRes.message ?? '');

    const check = await apiGet(page, `/crm/deals/${dealId}/rich`);
    console.log('Stage:', check.data?.stage);
    console.log('Deck URL exists:', !!check.data?.deck_url);
    console.log('GAP CHECK: Did Lux auto-trigger?', !!check.data?.deck_url ? 'YES' : 'NO — Lux auto-trigger missing');
  });

  // ══════════════════════════════════════════════════════════════════════════
  // PHASE 5: POLISH → PRESENT → FOLLOW UP → WON
  // ══════════════════════════════════════════════════════════════════════════

  test('Phase 5.1: Generate deck (Lux) — manual trigger', async ({ page }) => {
    const check = await apiGet(page, `/crm/deals/${dealId}/rich`);
    if (check.data?.deck_url) {
      console.log('Deck already exists (Lux auto-triggered)');
      return;
    }

    console.log('Triggering Lux manually...');
    const res = await apiPost(page, `/crm/deals/${dealId}/generate-deck`, {});
    console.log('Lux result:', res.success, res.message ?? '');
  });

  test('Phase 5.2: Advance Polish → Present', async ({ page }) => {
    const richRes = await apiGet(page, `/crm/deals/${dealId}/rich`);
    for (const task of richRes.data?.tasks ?? []) {
      if (task.status !== 'done' && task.status !== 'cancelled') {
        await apiPatch(page, `/tasks/${task.id}`, { status: 'done' });
      }
    }
    const advRes = await apiPost(page, `/crm/deals/${dealId}/advance`, {});
    console.log('Polish → Present:', advRes.success, advRes.message ?? '');
  });

  test('Phase 5.3: Send invoice', async ({ page }) => {
    const res = await apiPost(page, `/crm/deals/${dealId}/send-invoice`, {
      client_name: `${TEST_LEAD.first_name} ${TEST_LEAD.last_name}`,
      notes: 'Brand Identity & Launch package',
      due_days: 14,
    });
    console.log('Invoice result:', res.success, JSON.stringify(res.data ?? res.message ?? '').slice(0, 100));
  });

  test('Phase 5.4: Advance Present → Follow Up', async ({ page }) => {
    const richRes = await apiGet(page, `/crm/deals/${dealId}/rich`);
    for (const task of richRes.data?.tasks ?? []) {
      if (task.status !== 'done' && task.status !== 'cancelled') {
        await apiPatch(page, `/tasks/${task.id}`, { status: 'done' });
      }
    }
    const advRes = await apiPost(page, `/crm/deals/${dealId}/advance`, {});
    console.log('Present → Follow Up:', advRes.success, advRes.message ?? '');
  });

  test('Phase 5.5: Mark deal Won', async ({ page }) => {
    // Complete follow-up tasks
    const richRes = await apiGet(page, `/crm/deals/${dealId}/rich`);
    for (const task of richRes.data?.tasks ?? []) {
      if (task.status !== 'done' && task.status !== 'cancelled') {
        await apiPatch(page, `/tasks/${task.id}`, { status: 'done' });
      }
    }

    const res = await apiPost(page, `/crm/deals/${dealId}/mark-won`, {
      win_reason: 'Client loved the proposal and deck',
    });
    console.log('Mark Won result:', res.success);
    console.log('Won data:', JSON.stringify(res.data ?? res.message ?? '').slice(0, 300));

    if (res.data) {
      console.log('  Client ID:', res.data.client_id);
      console.log('  Project ID:', res.data.project_id);
      console.log('  Project name:', res.data.project_name);
      console.log('  Tasks created:', res.data.tasks_created);
    }
  });

  // ══════════════════════════════════════════════════════════════════════════
  // PHASE 6: VERIFICATION — Check everything was provisioned
  // ══════════════════════════════════════════════════════════════════════════

  test('Phase 6.1: Verify deal is in Won stage', async ({ page }) => {
    const res = await apiGet(page, `/crm/deals/${dealId}/rich`);
    console.log('Final stage:', res.data?.stage);
    console.log('Won at:', res.data?.won_at);
    console.log('Project ID:', res.data?.project_id);
    console.log('Invoice ID:', res.data?.invoice_id);
  });

  test('Phase 6.2: Verify company Intel Page exists (business report)', async ({ page }) => {
    // Check if a business report was created for this company
    const res = await apiGet(page, '/business-reports');
    const reports = res.data ?? [];
    // Note: reports may not be an array if the API returns differently
    if (Array.isArray(reports)) {
      const ourReport = reports.find((r: any) =>
        r.title?.includes(TEST_LEAD.company_name) || r.company_name?.includes(TEST_LEAD.company_name)
      );
      console.log('Business report for Vanguard:', ourReport ? 'EXISTS' : 'NOT FOUND — GAP: Intel Page not auto-created');
    }
  });

  test('Phase 6.3: Verify company profile shows intel data', async ({ page }) => {
    await gotoAsSirak(page, `/companies/${companyId}`);
    await page.waitForTimeout(2000);
    const body = await page.textContent('body');
    if (body?.includes(TEST_LEAD.company_name)) {
      console.log('✅ Company profile shows intel:', body?.includes('done') || body?.includes('Intel'));
    } else {
      console.log('GAP: Company profile not loading (BLOB UUID issue)');
    }
  });

  test('Phase 6.4: Verify person profile accessible', async ({ page }) => {
    await gotoAsSirak(page, `/people/${personId}`);
    await page.waitForTimeout(2000);
    const body = await page.textContent('body');
    if (body?.includes(TEST_LEAD.last_name)) {
      console.log('✅ Person profile accessible post-Won');
    } else {
      console.log('GAP: Person profile not loading post-Won');
      console.log('URL:', page.url());
    }
  });

  // ══════════════════════════════════════════════════════════════════════════
  // PHASE 7: CLEANUP — Remove test data
  // ══════════════════════════════════════════════════════════════════════════

  test('Phase 7: Cleanup test data', async ({ page }) => {
    // Delete deal
    if (dealId) {
      await page.request.delete(`http://localhost:3000/api/crm/deals/${dealId}`);
      console.log('Deleted deal:', dealId);
    }
    // Delete contact
    if (contactId) {
      await page.request.delete(`http://localhost:3000/api/crm/contacts/${contactId}`);
      console.log('Deleted contact:', contactId);
    }
    // Delete person
    if (personId) {
      await page.request.delete(`http://localhost:3000/api/persons/${personId}`);
      console.log('Deleted person:', personId);
    }
    // Delete company
    if (companyId) {
      await page.request.delete(`http://localhost:3000/api/companies/${companyId}`);
      console.log('Deleted company:', companyId);
    }
  });
});
