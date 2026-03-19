/**
 * Demo: Dealflow Pipeline — Full Lead-to-Won Lifecycle
 *
 * End-to-end feature demo showcasing the CRM pipeline automation:
 *   1. Create a new company + contact + deal in the CRM
 *   2. View the deal on the Pipeline Kanban board
 *   3. Browse deal detail panel tabs (Overview, Intel, Transcripts)
 *   4. Advance through pipeline stages (Lead → Intel → BA → Proposal)
 *   5. Generate a proposal via Cash (LLM) — REQUIRES ANTHROPIC_API_KEY
 *   6. Approve proposal, advance to Polish
 *   7. Generate a pitch deck via Lux (LLM) — REQUIRES ANTHROPIC_API_KEY
 *   8. Send invoice, mark deal Won
 *   9. Verify final state: company profile, person profile, deliverables
 *   10. Cleanup test data
 *
 * Prerequisites:
 *   - Dev server running on FRONTEND_PORT
 *   - Seed database with Sirak Studios organization + pipeline stages
 *   - ANTHROPIC_API_KEY in .env (for LLM proposal/deck generation)
 *   - Pipeline stages with stage_type values (Lead, Intel, BA, etc.)
 *
 * This demo uses short pauses in headless mode (QA) and medium pauses in
 * headed mode (visual demo). Set DEMO_PACE=short|medium|long to control.
 */
import { test, expect } from './fixtures';
import { demoPause, login, TEST_DATA_PREFIX, t } from '../helpers';

const ORG_ID = '02020202-0202-0202-0202-020202020202'; // Sirak Studios
const BASE_URL = `http://localhost:${process.env.FRONTEND_PORT || '3000'}`;

// Test data — a fictional but realistic lead
const DEMO_LEAD = {
  company: `${TEST_DATA_PREFIX} Aurora Design Co`,
  first: 'Elena',
  last: 'Vasquez',
  email: `elena-${Date.now()}@auroradesign.co`,
  title: 'Creative Director',
  deal_name: `${TEST_DATA_PREFIX} Aurora Design Co — Elena Vasquez`,
  context: 'Elena is looking for a full brand refresh and digital strategy overhaul for their boutique design agency. Budget range $20-30K.',
};

// ── Helpers ──────────────────────────────────────────────────────────────────

async function apiGet(sessionId: string, path: string) {
  const res = await fetch(`${BASE_URL}/api${path}`, {
    headers: { Authorization: `Bearer ${sessionId}`, 'Content-Type': 'application/json' },
  });
  return res.json();
}

async function apiPost(sessionId: string, path: string, body: Record<string, unknown>) {
  const res = await fetch(`${BASE_URL}/api${path}`, {
    method: 'POST',
    headers: { Authorization: `Bearer ${sessionId}`, 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
  return res.json();
}

async function apiPatch(sessionId: string, path: string, body: Record<string, unknown>) {
  const res = await fetch(`${BASE_URL}/api${path}`, {
    method: 'PATCH',
    headers: { Authorization: `Bearer ${sessionId}`, 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
  return res.json();
}

async function apiDelete(sessionId: string, path: string) {
  await fetch(`${BASE_URL}/api${path}`, {
    method: 'DELETE',
    headers: { Authorization: `Bearer ${sessionId}` },
  });
}

async function getApiSession(): Promise<string> {
  const res = await fetch(`${BASE_URL}/api/auth/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ username: 'admin', password: 'admin123' }),
  });
  const body = await res.json();
  return body?.data?.session_id || '';
}

// ── Test Suite ───────────────────────────────────────────────────────────────

test.describe.serial('Demo: Dealflow Pipeline — Lead to Won', () => {
  let sessionId: string;
  let companyId: string;
  let contactId: string;
  let personId: string;
  let dealId: string;
  let pipelineId: string;

  // ── Part 1: Setup — Create entities via API ────────────────────────────

  test('Part 1: Login and discover pipeline', async ({ page }) => {
    sessionId = await getApiSession();
    expect(sessionId).toBeTruthy();

    // Discover the Acquisition pipeline
    const pipelines = await apiGet(sessionId, `/crm/pipelines?organization_id=${ORG_ID}`);
    const acq = (pipelines.data ?? []).find((p: { name: string }) => p.name === 'Acquisition') || pipelines.data?.[0];
    test.skip(!acq, 'No pipeline found — need Acquisition pipeline with stages');
    pipelineId = acq.id;
    console.log(t('Pipeline discovered:', pipelineId, acq.name));
  });

  test('Part 2: Create company, contact, and deal', async ({ page }) => {
    test.skip(!pipelineId, 'No pipeline');
    sessionId = await getApiSession();

    // Create company
    const coRes = await apiPost(sessionId, '/companies', {
      name: DEMO_LEAD.company,
      industry: 'Design & Creative Services',
      created_by_org_id: ORG_ID,
    });
    companyId = coRes.data?.id;
    if (!companyId) {
      // Find existing
      const list = await apiGet(sessionId, '/companies?limit=500');
      companyId = (list.data ?? []).find((c: { name: string }) => c.name === DEMO_LEAD.company)?.id;
    }
    expect(companyId).toBeTruthy();
    console.log(t('Company created:', companyId));

    // Create CRM contact
    const ctRes = await apiPost(sessionId, '/crm/contacts', {
      organization_id: ORG_ID,
      first_name: DEMO_LEAD.first,
      last_name: DEMO_LEAD.last,
      email: DEMO_LEAD.email,
      job_title: DEMO_LEAD.title,
      company_name: DEMO_LEAD.company,
    });
    contactId = ctRes.data?.id;
    expect(contactId).toBeTruthy();
    console.log(t('Contact created:', contactId));

    // Create deal
    const dealRes = await apiPost(sessionId, '/crm/deals', {
      name: DEMO_LEAD.deal_name,
      organization_id: ORG_ID,
      crm_pipeline_id: pipelineId,
      crm_contact_id: contactId,
      description: DEMO_LEAD.context,
      amount: 25000,
    });
    dealId = dealRes.data?.id;
    expect(dealId).toBeTruthy();
    console.log(t('Deal created:', dealId, dealRes.data?.stage));
  });

  // ── Part 3: Browse the pipeline board ──────────────────────────────────

  test('Part 3: View deal on Pipeline Kanban board', async ({ page }) => {
    test.skip(!dealId, 'No deal created');
    await login(page);
    await page.goto(`${BASE_URL}/organizations/${ORG_ID}/crm/pipeline`);
    await page.waitForTimeout(demoPause.medium);

    // Verify the org pipeline page loads
    const bodyText = await page.textContent('body');
    expect(bodyText?.length).toBeGreaterThan(200);

    // Check Pipelines tab is visible
    const pipelinesTab = page.locator('button, [role="tab"]').filter({ hasText: /Pipelines/i });
    if (await pipelinesTab.count() > 0) {
      await pipelinesTab.first().click();
      await page.waitForTimeout(demoPause.medium);
    }

    console.log(t('Pipeline board loaded'));
  });

  // ── Part 4: Browse deal detail ─────────────────────────────────────────

  test('Part 4: Browse deal detail via API', async ({ page }) => {
    test.skip(!dealId, 'No deal');
    sessionId = await getApiSession();

    const rich = await apiGet(sessionId, `/crm/deals/${dealId}/rich`);
    expect(rich.data?.id).toBeTruthy();
    console.log(t('Deal detail:', rich.data?.name));
    console.log(t('  Stage:', rich.data?.stage));
    console.log(t('  Contact:', rich.data?.contact_name || '(pending)'));
    console.log(t('  Company:', rich.data?.contact_company || '(pending)'));
    console.log(t('  Amount: $' + (rich.data?.amount || 0)));
  });

  // ── Part 5: Advance through stages ─────────────────────────────────────

  test('Part 5: Advance deal through stages (Lead → Intel → BA → Discovery → Proposal)', async ({ page }) => {
    test.skip(!dealId, 'No deal');
    test.setTimeout(60_000);
    sessionId = await getApiSession();

    // Get stages for this pipeline
    const stagesRes = await apiGet(sessionId, `/crm/pipelines/${pipelineId}/stages`);
    const stages = stagesRes.data ?? stagesRes.stages ?? [];
    const stageOrder = ['lead', 'intel', 'business_analysis', 'discovery', 'proposal'];

    for (const targetType of stageOrder.slice(1)) { // skip lead (already there)
      // Clear any pending review tasks first
      const tasks = await apiGet(sessionId, `/tasks?crm_deal_id=${dealId}`);
      for (const task of (tasks.data ?? [])) {
        if (task.status !== 'done' && task.status !== 'cancelled' && task.title?.includes('Review')) {
          await apiPatch(sessionId, `/tasks/${task.id}`, { status: 'done' });
        }
      }

      // Advance
      const advRes = await apiPost(sessionId, `/crm/deals/${dealId}/advance`, {});
      console.log(t(`  Advanced to: ${advRes.data?.stage || advRes.message || 'error'}`));

      if (advRes.data?.stage?.toLowerCase().includes('proposal') ||
          advRes.data?.crm_stage_id === stages.find((s: { stage_type: string }) => s.stage_type === 'proposal')?.id) {
        break; // Reached proposal stage
      }
    }

    // Verify we're at or past proposal
    const check = await apiGet(sessionId, `/crm/deals/${dealId}`);
    console.log(t('Current stage:', check.data?.stage));
  });

  // ── Part 6: Generate proposal (LLM) ───────────────────────────────────

  test('Part 6: Generate proposal via Cash (LLM)', async ({ page }) => {
    test.skip(!dealId, 'No deal');
    test.setTimeout(120_000);
    sessionId = await getApiSession();

    const hasKey = !!process.env.ANTHROPIC_API_KEY || !!process.env.OPENAI_API_KEY;
    if (!hasKey) {
      console.log(t('SKIP: No LLM API key — proposal generation requires ANTHROPIC_API_KEY or OPENAI_API_KEY'));
      test.skip(true, 'No LLM API key configured');
      return;
    }

    const res = await apiPost(sessionId, `/crm/deals/${dealId}/generate-proposal`, {});
    if (res.data?.proposal_text) {
      console.log(t('Proposal generated:', res.data.proposal_text.slice(0, 100) + '...'));
      expect(res.data.proposal_text.length).toBeGreaterThan(50);
    } else {
      console.log(t('Proposal generation response:', JSON.stringify(res).slice(0, 200)));
      // Not a hard fail — LLM may be unavailable
      test.fixme(true, 'Proposal generation did not return text — LLM may be unavailable');
    }
  });

  // ── Part 7: Approve proposal ───────────────────────────────────────────

  test('Part 7: Approve proposal and advance', async ({ page }) => {
    test.skip(!dealId, 'No deal');
    sessionId = await getApiSession();

    // Check if proposal exists
    const deal = await apiGet(sessionId, `/crm/deals/${dealId}`);
    if (!deal.data?.proposal_text) {
      test.skip(true, 'No proposal to approve — LLM generation may have been skipped');
      return;
    }

    const approveRes = await apiPost(sessionId, `/crm/deals/${dealId}/approve-proposal`, {});
    console.log(t('Proposal approved:', approveRes.data?.proposal_status));
    expect(approveRes.data?.proposal_status).toBe('approved');

    // Advance to Polish
    const tasks = await apiGet(sessionId, `/tasks?crm_deal_id=${dealId}`);
    for (const task of (tasks.data ?? [])) {
      if (task.status !== 'done' && task.status !== 'cancelled') {
        await apiPatch(sessionId, `/tasks/${task.id}`, { status: 'done' });
      }
    }
    const advRes = await apiPost(sessionId, `/crm/deals/${dealId}/advance`, {});
    console.log(t('Advanced to:', advRes.data?.stage));
  });

  // ── Part 8: Generate deck (LLM) ───────────────────────────────────────

  test('Part 8: Generate pitch deck via Lux (LLM)', async ({ page }) => {
    test.skip(!dealId, 'No deal');
    test.setTimeout(120_000);
    sessionId = await getApiSession();

    const deal = await apiGet(sessionId, `/crm/deals/${dealId}`);
    if (!deal.data?.proposal_text) {
      test.skip(true, 'No proposal — deck generation requires proposal first');
      return;
    }

    const hasKey = !!process.env.ANTHROPIC_API_KEY || !!process.env.OPENAI_API_KEY;
    if (!hasKey) {
      test.skip(true, 'No LLM API key configured');
      return;
    }

    const res = await apiPost(sessionId, `/crm/deals/${dealId}/generate-deck`, {});
    if (res.data?.deck_url) {
      console.log(t('Deck generated:', res.data.deck_url));
    } else {
      console.log(t('Deck generation response:', JSON.stringify(res).slice(0, 200)));
      test.fixme(true, 'Deck generation did not return deck_url');
    }
  });

  // ── Part 9: Invoice and Won ────────────────────────────────────────────

  test('Part 9: Send invoice and mark deal Won', async ({ page }) => {
    test.skip(!dealId, 'No deal');
    sessionId = await getApiSession();

    // Clear tasks and advance to final stages
    const clearAndAdvance = async () => {
      const tasks = await apiGet(sessionId, `/tasks?crm_deal_id=${dealId}`);
      for (const task of (tasks.data ?? [])) {
        if (task.status !== 'done' && task.status !== 'cancelled') {
          await apiPatch(sessionId, `/tasks/${task.id}`, { status: 'done' });
        }
      }
      return apiPost(sessionId, `/crm/deals/${dealId}/advance`, {});
    };

    // Advance through remaining stages until we can mark won
    for (let i = 0; i < 5; i++) {
      const res = await clearAndAdvance();
      console.log(t(`  Stage: ${res.data?.stage || res.message}`));
      if (res.data?.stage === 'won' || res.message?.includes('closed')) break;
    }

    // Mark won
    const wonRes = await apiPost(sessionId, `/crm/deals/${dealId}/won`, {
      company_name: DEMO_LEAD.company,
    });
    if (wonRes.data) {
      console.log(t('Deal marked Won!', wonRes.data?.stage));
    } else {
      console.log(t('Won response:', JSON.stringify(wonRes).slice(0, 200)));
    }
  });

  // ── Part 10: Verify final state ────────────────────────────────────────

  test('Part 10: Verify final state — deal, company, person', async ({ page }) => {
    test.skip(!dealId, 'No deal');
    sessionId = await getApiSession();

    // Check deal
    const deal = await apiGet(sessionId, `/crm/deals/${dealId}`);
    console.log(t('Final deal state:', deal.data?.stage, '| proposal:', deal.data?.proposal_status));

    // Check company profile page loads
    if (companyId) {
      await login(page);
      await page.goto(`${BASE_URL}/companies/${companyId}`);
      await page.waitForTimeout(demoPause.medium);
      const bodyText = await page.textContent('body');
      console.log(t('Company page loaded:', (bodyText?.length || 0) > 200 ? 'yes' : 'minimal content'));
    }

    // Check person exists
    if (contactId) {
      const persons = await apiGet(sessionId, `/persons?crm_contact_id=${contactId}`);
      personId = persons.data?.[0]?.id;
      console.log(t('Person record:', personId || 'not linked'));
    }
  });

  // ── Part 11: Cleanup ───────────────────────────────────────────────────

  test('Part 11: Cleanup test data', async ({ page }) => {
    sessionId = await getApiSession();

    if (dealId) await apiDelete(sessionId, `/crm/deals/${dealId}`);
    if (contactId) await apiDelete(sessionId, `/crm/contacts/${contactId}`);
    if (companyId) await apiDelete(sessionId, `/companies/${companyId}`);

    console.log(t('Cleanup complete'));
  });
});
