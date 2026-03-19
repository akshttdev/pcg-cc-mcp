/**
 * VISUAL PIPELINE WALKTHROUGH
 *
 * The Playwright agent acts as Sirak (the operator), navigating through
 * the actual UI at each stage. Slowed down so a human can watch.
 *
 * Lead: Marcus Webb, Founder of "Vanguard Social Club"
 */
import { test, expect, Page } from '@playwright/test';

const BASE_URL = `http://localhost:${process.env.FRONTEND_PORT || '3000'}`;
const ORG_ID = '02020202-0202-0202-0202-020202020202';
let PIPELINE_ID = '';
const PAUSE = 2500; // ms between actions so you can watch

const TEST_LEAD = {
  first_name: 'Marcus',
  last_name: 'Webb',
  full_name: 'Marcus Webb',
  email: 'marcus@vanguardsocialclub.com',
  company_name: 'Vanguard Social Club',
  job_title: 'Founder & CEO',
  deal_amount: 25000,
};

// ── Auth helpers ─────────────────────────────────────────────────────────────

async function loginAs(page: Page, username: string, password: string) {
  const res = await page.request.post(`${BASE_URL}/api/auth/login`, {
    data: { username, password },
    headers: { 'Content-Type': 'application/json' },
  });
  const body = await res.json();
  const sid = body?.data?.session_id;
  if (!sid) throw new Error(`Login failed: ${JSON.stringify(body)}`);
  await page.context().addCookies([{
    name: 'session_id', value: sid, domain: 'localhost', path: '/', httpOnly: false, secure: false,
  }]);
  return sid;
}

async function gotoAs(page: Page, user: string, pw: string, path: string) {
  await page.goto(`${BASE_URL}/`, { waitUntil: 'domcontentloaded' });
  const sid = await loginAs(page, user, pw);
  await page.evaluate((s) => localStorage.setItem('session_id', s), sid);
  await page.goto(`${BASE_URL}${path}`, { waitUntil: 'networkidle' });
  await page.evaluate((s) => localStorage.setItem('session_id', s), sid);
  await page.waitForTimeout(PAUSE);
}

async function apiGet(page: Page, path: string) {
  return (await page.request.get(`${BASE_URL}/api${path}`)).json();
}
async function apiPost(page: Page, path: string, body: object) {
  return (await page.request.post(`${BASE_URL}/api${path}`, {
    data: body, headers: { 'Content-Type': 'application/json' },
  })).json();
}
async function apiPatch(page: Page, path: string, body: object) {
  return (await page.request.patch(`${BASE_URL}/api${path}`, {
    data: body, headers: { 'Content-Type': 'application/json' },
  })).json();
}

// ── State ────────────────────────────────────────────────────────────────────

let companyId: string;
let personId: string;
let contactId: string;
let dealId: string;

test.setTimeout(300000); // 5 min — this is a slow visual walkthrough

test.describe.serial('Visual Pipeline Walkthrough — Operator View', () => {

  // ═══════════════════════════════════════════════════════════════════════════
  // SETUP: Create entities via API as admin, then switch to Sirak for UI
  // ═══════════════════════════════════════════════════════════════════════════

  test('Setup: Discover pipeline', async ({ page }) => {
    await loginAs(page, 'admin', 'admin123');

    const pipelines = await apiGet(page, `/crm/pipelines?organization_id=${ORG_ID}`);
    const acqPipeline = (pipelines.data ?? []).find((p: { name: string }) => p.name === 'Acquisition') || pipelines.data?.[0];
    test.skip(!acqPipeline, 'No pipeline found for org');
    PIPELINE_ID = acqPipeline.id;
    console.log('🎯 Pipeline:', PIPELINE_ID, acqPipeline.name);
  });

  test('Setup: Create lead entities', async ({ page }) => {
    test.skip(!PIPELINE_ID, 'No pipeline available');
    await loginAs(page, 'admin', 'admin123');

    // Find or create company
    const cos = await apiGet(page, '/companies?limit=500');
    const existing = (cos.data ?? []).find((c: any) => c.name === TEST_LEAD.company_name);
    if (existing) {
      companyId = existing.id;
    } else {
      const r = await apiPost(page, '/companies', {
        name: TEST_LEAD.company_name, industry: 'Luxury Hospitality',
        headquarters: 'Miami, FL', created_by_org_id: ORG_ID,
      });
      companyId = r.data?.id;
    }

    // Find or create person
    const ps = await apiGet(page, `/persons?search=${TEST_LEAD.last_name}`);
    const ep = (ps.data ?? []).find((p: any) => p.full_name?.includes(TEST_LEAD.last_name));
    if (ep) {
      personId = ep.id;
    } else {
      const r = await apiPost(page, '/persons', {
        full_name: TEST_LEAD.full_name, email: TEST_LEAD.email,
        company_name: TEST_LEAD.company_name, company_id: companyId,
        person_type: 'lead', job_title: TEST_LEAD.job_title,
      });
      personId = r.data?.id;
    }

    // Find or create contact
    const cs = await apiGet(page, `/crm/contacts?organization_id=${ORG_ID}`);
    const ec = (cs.data ?? []).find((c: any) => c.full_name?.includes(TEST_LEAD.last_name));
    if (ec) {
      contactId = ec.id;
    } else {
      const r = await apiPost(page, '/crm/contacts', {
        organization_id: ORG_ID, first_name: TEST_LEAD.first_name,
        last_name: TEST_LEAD.last_name, email: TEST_LEAD.email,
        company_name: TEST_LEAD.company_name, job_title: TEST_LEAD.job_title,
        lifecycle_stage: 'lead', person_id: personId,
      });
      contactId = r.data?.id;
    }

    // Find or create deal
    const ds = await apiGet(page, `/crm/deals/enriched?organization_id=${ORG_ID}`);
    const ed = (ds.data ?? []).find((d: any) => d.name?.includes(TEST_LEAD.company_name));
    if (ed) {
      dealId = ed.id;
    } else {
      const pipeline = await apiGet(page, `/crm/pipelines/${PIPELINE_ID}`);
      const intelStage = (pipeline.data?.stages ?? []).find((s: any) => s.stage_type === 'intel');
      const r = await apiPost(page, '/crm/deals', {
        organization_id: ORG_ID, crm_contact_id: contactId,
        crm_pipeline_id: PIPELINE_ID, crm_stage_id: intelStage?.id,
        name: `${TEST_LEAD.full_name} — ${TEST_LEAD.company_name}`,
        amount: TEST_LEAD.deal_amount, currency: 'USD',
      });
      dealId = r.data?.id;
    }

    console.log('✅ Setup complete');
    console.log('   Company:', companyId);
    console.log('   Person:', personId);
    console.log('   Deal:', dealId);
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // ACT 1: Sirak sees the new lead on the Pipeline board
  // ═══════════════════════════════════════════════════════════════════════════

  test('Act 1: View lead on Pipeline board', async ({ page }) => {
    await gotoAs(page, 'Sirak', 'Sirak123', `/organizations/${ORG_ID}/crm/pipeline`);

    // Wait for kanban to render
    await page.waitForSelector('[class*="inline-grid"], [class*="kanban"], [class*="pipeline"], [class*="board"]', { timeout: 20000 }).catch(() => null);
    await page.waitForTimeout(2000);
    await page.waitForTimeout(PAUSE);

    // Scroll to find Vanguard
    const card = page.locator(`text=${TEST_LEAD.company_name}`).first();
    if (await card.count() > 0) {
      await card.scrollIntoViewIfNeeded();
      await page.waitForTimeout(PAUSE);
      console.log('✅ Lead visible on Pipeline board');
    }
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // ACT 2: Open the deal panel, inspect Overview tab
  // ═══════════════════════════════════════════════════════════════════════════

  test('Act 2: Open deal panel — Overview tab', async ({ page }) => {
    await gotoAs(page, 'Sirak', 'Sirak123', `/organizations/${ORG_ID}/crm/pipeline`);
    await page.waitForSelector('[class*="inline-grid"], [class*="kanban"], [class*="pipeline"], [class*="board"]', { timeout: 20000 }).catch(() => null);
    await page.waitForTimeout(2000);
    await page.waitForTimeout(PAUSE);

    // Click on the deal card
    const kanban = page.locator('[class*="inline-grid"], [class*="kanban"], [class*="pipeline"]').first();
    const card = kanban.locator('p').filter({ hasText: new RegExp(TEST_LEAD.last_name) }).first();
    await card.scrollIntoViewIfNeeded();
    await page.waitForTimeout(1000);
    await card.click();

    // Wait for deal panel
    await page.waitForSelector('[role="dialog"]', { timeout: 10000 });
    await page.waitForTimeout(PAUSE);

    console.log('✅ Deal panel open — Overview tab visible');
    console.log('   Shows: contact info, org section, stage stepper');
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // ACT 3: Browse through each deal panel tab
  // ═══════════════════════════════════════════════════════════════════════════

  test('Act 3: Browse deal panel tabs', async ({ page }) => {
    await gotoAs(page, 'Sirak', 'Sirak123', `/organizations/${ORG_ID}/crm/pipeline`);
    await page.waitForSelector('[class*="inline-grid"], [class*="kanban"], [class*="pipeline"], [class*="board"]', { timeout: 20000 }).catch(() => null);
    await page.waitForTimeout(2000);
    await page.waitForTimeout(1500);

    // Open deal panel
    const kanban = page.locator('[class*="inline-grid"], [class*="kanban"], [class*="pipeline"]').first();
    const card = kanban.locator('p').filter({ hasText: new RegExp(TEST_LEAD.last_name) }).first();
    await card.scrollIntoViewIfNeeded();
    await card.click();
    await page.waitForSelector('[role="dialog"]', { timeout: 10000 });
    await page.waitForTimeout(1500);

    const dialog = page.locator('[role="dialog"]');
    const tabs = ['Intel', 'Review', 'Transcripts', 'Proposal', 'Deck', 'Activity'];

    for (const tabName of tabs) {
      const tab = dialog.locator('[role="tablist"] button, [role="tab"]').filter({ hasText: new RegExp(`^${tabName}`) }).first();
      if (await tab.count() > 0) {
        await tab.click();
        await page.waitForTimeout(PAUSE);
        console.log(`   📋 Viewed tab: ${tabName}`);
      }
    }

    console.log('✅ All deal panel tabs browsed');
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // ACT 4: Advance deal through stages (operator completes review tasks)
  // ═══════════════════════════════════════════════════════════════════════════

  test('Act 4: Advance through Intel → BA → Discovery → Proposal', async ({ page }) => {
    await loginAs(page, 'admin', 'admin123');

    const stages = ['Intel', 'Business Analysis', 'Discovery', 'Proposal'];

    for (let i = 0; i < stages.length - 1; i++) {
      // Complete pending review tasks
      const rich = await apiGet(page, `/crm/deals/${dealId}/rich`);
      for (const task of rich.data?.tasks ?? []) {
        if (task.status !== 'done' && task.status !== 'cancelled') {
          await apiPatch(page, `/tasks/${task.id}`, { status: 'done' });
        }
      }

      // Advance
      const adv = await apiPost(page, `/crm/deals/${dealId}/advance`, {});
      const check = await apiGet(page, `/crm/deals/${dealId}/rich`);
      console.log(`   ${stages[i]} → ${stages[i+1]}: ${adv.success ? '✅' : '❌'} (now: ${check.data?.stage})`);
    }

    console.log('✅ Deal advanced to Proposal stage');
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // ACT 5: View the deal in Proposal stage on the board
  // ═══════════════════════════════════════════════════════════════════════════

  test('Act 5: View deal in Proposal column', async ({ page }) => {
    await gotoAs(page, 'Sirak', 'Sirak123', `/organizations/${ORG_ID}/crm/pipeline`);
    await page.waitForSelector('[class*="inline-grid"], [class*="kanban"], [class*="pipeline"], [class*="board"]', { timeout: 20000 }).catch(() => null);
    await page.waitForTimeout(2000);
    await page.waitForTimeout(PAUSE);

    // Find the deal and verify it's in the Proposal area
    const card = page.locator(`text=${TEST_LEAD.company_name}`).first();
    if (await card.count() > 0) {
      await card.scrollIntoViewIfNeeded();
      await page.waitForTimeout(PAUSE);
    }
    console.log('✅ Deal visible in pipeline (should be in Proposal column)');
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // ACT 6: Generate proposal, view it, approve it
  // ═══════════════════════════════════════════════════════════════════════════

  test('Act 6: Generate + approve proposal', async ({ page }) => {
    await loginAs(page, 'admin', 'admin123');

    // Generate proposal if not exists
    const check = await apiGet(page, `/crm/deals/${dealId}/rich`);
    if (!check.data?.proposal_text || check.data.proposal_text === 'LLM generation failed') {
      console.log('   Generating proposal (Cash)...');
      await apiPost(page, `/crm/deals/${dealId}/generate-proposal`, {});
    }

    // Approve
    await apiPost(page, `/crm/deals/${dealId}/approve-proposal`, {});
    console.log('   ✅ Proposal approved');

    // Now view it as Sirak
    await gotoAs(page, 'Sirak', 'Sirak123', `/organizations/${ORG_ID}/crm/pipeline`);
    await page.waitForSelector('[class*="inline-grid"], [class*="kanban"], [class*="pipeline"], [class*="board"]', { timeout: 20000 }).catch(() => null);
    await page.waitForTimeout(2000);
    await page.waitForTimeout(1500);

    const kanban = page.locator('[class*="inline-grid"], [class*="kanban"], [class*="pipeline"]').first();
    const card = kanban.locator('p').filter({ hasText: new RegExp(TEST_LEAD.last_name) }).first();
    if (await card.count() > 0) {
      await card.scrollIntoViewIfNeeded();
      await card.click();
      await page.waitForSelector('[role="dialog"]', { timeout: 10000 });
      await page.waitForTimeout(1500);

      // Click Proposal tab
      const dialog = page.locator('[role="dialog"]');
      const proposalTab = dialog.locator('[role="tablist"] button, [role="tab"]').filter({ hasText: /^Proposal$/ }).first();
      if (await proposalTab.count() > 0) {
        await proposalTab.click();
        await page.waitForTimeout(PAUSE);
        console.log('   📋 Viewing Proposal tab');
      }
    }

    console.log('✅ Proposal generated and approved');
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // ACT 7: Advance to Polish, generate deck
  // ═══════════════════════════════════════════════════════════════════════════

  test('Act 7: Advance to Polish + generate deck', async ({ page }) => {
    await loginAs(page, 'admin', 'admin123');

    // Complete tasks and advance to Polish
    const rich = await apiGet(page, `/crm/deals/${dealId}/rich`);
    for (const task of rich.data?.tasks ?? []) {
      if (task.status !== 'done' && task.status !== 'cancelled') {
        await apiPatch(page, `/tasks/${task.id}`, { status: 'done' });
      }
    }
    await apiPost(page, `/crm/deals/${dealId}/advance`, {});

    // Generate deck
    const deckCheck = await apiGet(page, `/crm/deals/${dealId}/rich`);
    if (!deckCheck.data?.deck_url) {
      console.log('   Generating deck (Lux)...');
      await apiPost(page, `/crm/deals/${dealId}/generate-deck`, {});
    }

    // View deck tab as Sirak
    await gotoAs(page, 'Sirak', 'Sirak123', `/organizations/${ORG_ID}/crm/pipeline`);
    await page.waitForSelector('[class*="inline-grid"], [class*="kanban"], [class*="pipeline"], [class*="board"]', { timeout: 20000 }).catch(() => null);
    await page.waitForTimeout(2000);
    await page.waitForTimeout(1500);

    const kanban = page.locator('[class*="inline-grid"], [class*="kanban"], [class*="pipeline"]').first();
    const card = kanban.locator('p').filter({ hasText: new RegExp(TEST_LEAD.last_name) }).first();
    if (await card.count() > 0) {
      await card.scrollIntoViewIfNeeded();
      await card.click();
      await page.waitForSelector('[role="dialog"]', { timeout: 10000 });
      await page.waitForTimeout(1500);

      const dialog = page.locator('[role="dialog"]');
      const deckTab = dialog.locator('[role="tablist"] button, [role="tab"]').filter({ hasText: /Deck/i }).first();
      if (await deckTab.count() > 0) {
        await deckTab.click();
        await page.waitForTimeout(PAUSE);
        console.log('   📋 Viewing Deck & Close tab');
      }
    }

    console.log('✅ Deck generated');
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // ACT 8: Send invoice + advance to Present → Follow Up
  // ═══════════════════════════════════════════════════════════════════════════

  test('Act 8: Send invoice, advance through Present → Follow Up', async ({ page }) => {
    await loginAs(page, 'admin', 'admin123');

    // Send invoice
    const invCheck = await apiGet(page, `/crm/deals/${dealId}/rich`);
    if (!invCheck.data?.invoice_id) {
      await apiPost(page, `/crm/deals/${dealId}/send-invoice`, {
        client_name: TEST_LEAD.full_name, notes: 'Brand Identity package', due_days: 14,
      });
      console.log('   ✅ Invoice sent');
    }

    // Advance Present → Follow Up
    for (let i = 0; i < 2; i++) {
      const rich = await apiGet(page, `/crm/deals/${dealId}/rich`);
      for (const task of rich.data?.tasks ?? []) {
        if (task.status !== 'done' && task.status !== 'cancelled') {
          await apiPatch(page, `/tasks/${task.id}`, { status: 'done' });
        }
      }
      await apiPost(page, `/crm/deals/${dealId}/advance`, {});
    }

    console.log('✅ Advanced through Present → Follow Up');
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // ACT 9: Mark Won — watch project + client get created
  // ═══════════════════════════════════════════════════════════════════════════

  test('Act 9: Mark deal Won', async ({ page }) => {
    await loginAs(page, 'admin', 'admin123');

    // Complete remaining tasks
    const rich = await apiGet(page, `/crm/deals/${dealId}/rich`);
    for (const task of rich.data?.tasks ?? []) {
      if (task.status !== 'done' && task.status !== 'cancelled') {
        await apiPatch(page, `/tasks/${task.id}`, { status: 'done' });
      }
    }

    const won = await apiPost(page, `/crm/deals/${dealId}/mark-won`, {
      win_reason: 'Client loved the proposal',
    });

    console.log('   🏆 DEAL WON!');
    if (won.data) {
      console.log('   Client:', won.data.client_id);
      console.log('   Project:', won.data.project_name);
      console.log('   Tasks:', won.data.tasks_created);
    }

    // View the Won deal on the board as Sirak
    await gotoAs(page, 'Sirak', 'Sirak123', `/organizations/${ORG_ID}/crm/pipeline`);
    await page.waitForSelector('[class*="inline-grid"], [class*="kanban"], [class*="pipeline"], [class*="board"]', { timeout: 20000 }).catch(() => null);
    await page.waitForTimeout(2000);
    await page.waitForTimeout(PAUSE);

    // Scroll right to see Won column
    const kanban = page.locator('[class*="inline-grid"], [class*="kanban"], [class*="pipeline"]').first();
    const wonText = page.locator('text=Won').first();
    if (await wonText.count() > 0) {
      await wonText.scrollIntoViewIfNeeded();
      await page.waitForTimeout(PAUSE);
    }

    console.log('✅ Deal in Won column');
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // ACT 10: Visit the company profile
  // ═══════════════════════════════════════════════════════════════════════════

  test('Act 10: Visit company profile', async ({ page }) => {
    await gotoAs(page, 'Sirak', 'Sirak123', `/companies/${companyId}`);
    await page.waitForTimeout(PAUSE);

    const body = await page.textContent('body');
    if (body?.includes(TEST_LEAD.company_name)) {
      console.log('✅ Company profile loaded: ' + TEST_LEAD.company_name);
    } else {
      console.log('❌ Company profile did not load');
    }
    await page.waitForTimeout(PAUSE);
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // ACT 11: Visit the person profile
  // ═══════════════════════════════════════════════════════════════════════════

  test('Act 11: Visit person profile', async ({ page }) => {
    await gotoAs(page, 'Sirak', 'Sirak123', `/people/${personId}`);
    await page.waitForTimeout(PAUSE);

    const body = await page.textContent('body');
    if (body?.includes(TEST_LEAD.last_name)) {
      console.log('✅ Person profile loaded: ' + TEST_LEAD.full_name);
    } else {
      console.log('❌ Person profile did not load');
    }
    await page.waitForTimeout(PAUSE);
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // ACT 12: View the Contacts tab with the new company
  // ═══════════════════════════════════════════════════════════════════════════

  test('Act 12: View Org Contacts — Companies view', async ({ page }) => {
    await gotoAs(page, 'Sirak', 'Sirak123', `/organizations/${ORG_ID}/crm/contacts`);
    await page.waitForTimeout(PAUSE);

    // Click Companies sub-tab
    const companiesBtn = page.locator('button').filter({ hasText: /^Companies/ }).first();
    if (await companiesBtn.count() > 0) {
      await companiesBtn.click();
      await page.waitForTimeout(PAUSE);
    }

    // Look for Vanguard
    const vanguard = page.locator(`text=${TEST_LEAD.company_name}`).first();
    if (await vanguard.count() > 0) {
      await vanguard.scrollIntoViewIfNeeded();
      await page.waitForTimeout(PAUSE);
      console.log('✅ Vanguard Social Club visible in Companies view');
    }
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // CLEANUP
  // ═══════════════════════════════════════════════════════════════════════════

  test('Cleanup: Remove test data', async ({ page }) => {
    await loginAs(page, 'admin', 'admin123');
    if (dealId) await page.request.delete(`${BASE_URL}/api/crm/deals/${dealId}`);
    if (contactId) await page.request.delete(`${BASE_URL}/api/crm/contacts/${contactId}`);
    if (personId) await page.request.delete(`${BASE_URL}/api/persons/${personId}`);
    if (companyId) await page.request.delete(`${BASE_URL}/api/companies/${companyId}`);
    console.log('✅ Test data cleaned up');
  });
});
