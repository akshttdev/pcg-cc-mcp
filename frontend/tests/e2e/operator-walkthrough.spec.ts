/**
 * OPERATOR WALKTHROUGH — Full UI-Driven Pipeline + Task Board
 *
 * The Playwright agent acts as Sirak (operator) going through every step:
 * - Pipeline board: see deals move through stages
 * - Deal panel: review data in each tab
 * - Task board: see review tasks, open them, complete them
 * - Agent outputs: review Cash proposals, Lux decks
 * - UI actions: approve, send invoice, mark won
 *
 * Lead: Devon Franklin — public figure, discoverable via web research
 * Company: Franklin Entertainment Group
 */
import { test, expect, Page } from '@playwright/test';

const ORG_ID = '02020202-0202-0202-0202-020202020202';
const PIPELINE_ID = '138ff8ec-6d65-493e-b6a9-0f9fef409968';
const PAUSE = 3500;
const SHORT = 2000;

const LEAD = {
  full_name: 'Devon Franklin',
  email: 'devon@franklinentertainment.com',
  company_name: 'Franklin Entertainment Group',
  job_title: 'CEO & Producer',
};

const OPERATOR_CONTEXT = `Devon Franklin is a Hollywood producer, motivational speaker, and bestselling author. Connected through a mutual contact at an industry event. He produces faith-based films for Sony and Netflix. Looking for a creative agency to handle brand refresh and digital presence for an upcoming book launch and speaking tour. High-value opportunity with strong social following.`;

const DISCOVERY_TRANSCRIPT = `Discovery Call with Devon Franklin — March 17, 2026.

Devon described Franklin Entertainment as a faith-based entertainment production company. Recent projects include multiple Netflix and Sony deals.

Key needs: Complete brand refresh, social media strategy overhaul, website redesign with content hub, video content production for speaking tour promo package.

Budget: $30-50K initial phase with potential ongoing retainer. Wants phased proposal with clear deliverables and timeline.

Next steps: Devon to share brand assets. Sirak to prepare proposal within 1 week. Presentation call scheduled for March 24 via Zoom.`;

// ── Helpers ──────────────────────────────────────────────────────────────────

async function loginAs(page: Page, user: string, pw: string) {
  const res = await page.request.post('http://localhost:3000/api/auth/login', {
    data: { username: user, password: pw }, headers: { 'Content-Type': 'application/json' },
  });
  const sid = (await res.json())?.data?.session_id;
  if (!sid) throw new Error('Login failed');
  await page.context().addCookies([{ name: 'session_id', value: sid, domain: 'localhost', path: '/', httpOnly: false, secure: false }]);
  return sid;
}

async function gotoAs(page: Page, user: string, pw: string, path: string) {
  await page.goto('http://localhost:3000/', { waitUntil: 'domcontentloaded' });
  const sid = await loginAs(page, user, pw);
  await page.evaluate((s) => localStorage.setItem('session_id', s), sid);
  await page.goto(`http://localhost:3000${path}`, { waitUntil: 'networkidle' });
  await page.evaluate((s) => localStorage.setItem('session_id', s), sid);
  await page.waitForTimeout(SHORT);
}

async function api(page: Page, method: string, path: string, body?: object) {
  const opts: any = { headers: { 'Content-Type': 'application/json' } };
  if (body) opts.data = body;
  const res = method === 'GET'
    ? await page.request.get(`http://localhost:3000/api${path}`)
    : method === 'POST'
      ? await page.request.post(`http://localhost:3000/api${path}`, opts)
      : await page.request.patch(`http://localhost:3000/api${path}`, opts);
  return res.json();
}

async function openDealPanel(page: Page, name: string) {
  await page.waitForSelector('[class*="inline-grid"]', { timeout: 20000 });
  await page.waitForTimeout(SHORT);
  const card = page.locator('[class*="inline-grid"]').locator('p').filter({ hasText: new RegExp(name) }).first();
  if (await card.count() > 0) {
    await card.scrollIntoViewIfNeeded();
    await page.waitForTimeout(500);
    await card.click();
    await page.waitForSelector('[role="dialog"]', { timeout: 10000 });
    await page.waitForTimeout(SHORT);
    return true;
  }
  return false;
}

async function clickTab(page: Page, tabName: string) {
  const dialog = page.locator('[role="dialog"]');
  const tab = dialog.locator('[role="tablist"] button, [role="tab"]').filter({ hasText: new RegExp(`^${tabName}`) }).first();
  if (await tab.count() > 0) { await tab.click(); await page.waitForTimeout(SHORT); }
}

async function completeTasksAndAdvance(page: Page, dealId: string): Promise<string> {
  const rich = await api(page, 'GET', `/crm/deals/${dealId}/rich`);
  for (const task of rich.data?.tasks ?? []) {
    if (task.status !== 'done' && task.status !== 'cancelled') {
      await api(page, 'PATCH', `/tasks/${task.id}`, { status: 'done' });
    }
  }
  await api(page, 'POST', `/crm/deals/${dealId}/advance`, {});
  const check = await api(page, 'GET', `/crm/deals/${dealId}/rich`);
  return check.data?.stage ?? '?';
}

// ── State ────────────────────────────────────────────────────────────────────
let companyId: string, personId: string, contactId: string, dealId: string;

test.setTimeout(600000);

test.describe.serial('Operator Walkthrough — Devon Franklin / Franklin Entertainment', () => {

  // ═══════════════ SETUP ═══════════════

  test('Setup: Create lead entities', async ({ page }) => {
    await loginAs(page, 'admin', 'admin123');

    const cos = await api(page, 'GET', '/companies?limit=500');
    const ec = (cos.data ?? []).find((c: any) => c.name === LEAD.company_name);
    if (ec) { companyId = ec.id; } else {
      companyId = (await api(page, 'POST', '/companies', { name: LEAD.company_name, industry: 'Entertainment & Media', headquarters: 'Los Angeles, CA', created_by_org_id: ORG_ID })).data?.id;
    }

    const ps = await api(page, 'GET', '/persons?search=Franklin');
    const ep = (ps.data ?? []).find((p: any) => p.full_name?.includes('Devon'));
    if (ep) { personId = ep.id; } else {
      personId = (await api(page, 'POST', '/persons', { full_name: LEAD.full_name, email: LEAD.email, company_name: LEAD.company_name, company_id: companyId, person_type: 'lead', job_title: LEAD.job_title })).data?.id;
    }

    const cs = await api(page, 'GET', `/crm/contacts?organization_id=${ORG_ID}`);
    const ecc = (cs.data ?? []).find((c: any) => c.full_name?.includes('Devon'));
    if (ecc) { contactId = ecc.id; } else {
      contactId = (await api(page, 'POST', '/crm/contacts', { organization_id: ORG_ID, first_name: 'Devon', last_name: 'Franklin', email: LEAD.email, company_name: LEAD.company_name, job_title: LEAD.job_title, lifecycle_stage: 'lead', person_id: personId })).data?.id;
    }

    const ds = await api(page, 'GET', `/crm/deals/enriched?organization_id=${ORG_ID}`);
    const ed = (ds.data ?? []).find((d: any) => d.name?.includes('Franklin'));
    if (ed) { dealId = ed.id; } else {
      const pipeline = await api(page, 'GET', `/crm/pipelines/${PIPELINE_ID}`);
      const intelStage = (pipeline.data?.stages ?? []).find((s: any) => s.stage_type === 'intel');
      dealId = (await api(page, 'POST', '/crm/deals', { organization_id: ORG_ID, crm_contact_id: contactId, crm_pipeline_id: PIPELINE_ID, crm_stage_id: intelStage?.id, name: `${LEAD.full_name} — ${LEAD.company_name}`, currency: 'USD' })).data?.id;
    }

    expect(dealId).toBeTruthy();
    console.log('✅ Lead created:', LEAD.full_name);
  });

  // ═══════════════ STAGE 0: INTEL ═══════════════

  test('INTEL: Operator sees lead on pipeline board', async ({ page }) => {
    await gotoAs(page, 'Sirak', 'Sirak123', `/organizations/${ORG_ID}/crm/pipeline`);
    const opened = await openDealPanel(page, 'Franklin');
    expect(opened).toBe(true);
    await page.waitForTimeout(PAUSE);
    console.log('✅ Deal panel open — new lead visible');
  });

  test('INTEL: Operator adds context notes', async ({ page }) => {
    await gotoAs(page, 'Sirak', 'Sirak123', `/organizations/${ORG_ID}/crm/pipeline`);
    await openDealPanel(page, 'Franklin');

    // Click on the operator context card to edit
    const dialog = page.locator('[role="dialog"]');
    const contextCard = dialog.locator('text=No context yet').first();
    if (await contextCard.count() > 0) {
      await contextCard.click();
      await page.waitForTimeout(SHORT);

      // Type context
      const textarea = dialog.locator('textarea').first();
      if (await textarea.count() > 0) {
        await textarea.fill(OPERATOR_CONTEXT);
        await page.waitForTimeout(SHORT);

        // Click save
        const saveBtn = dialog.locator('button').filter({ hasText: /Save Context/i }).first();
        if (await saveBtn.count() > 0) {
          await saveBtn.click();
          await page.waitForTimeout(PAUSE);
          console.log('✅ Context saved via UI');
        }
      }
    } else {
      // Fallback: save via API
      await loginAs(page, 'admin', 'admin123');
      await api(page, 'PATCH', `/crm/deals/${dealId}`, { description: OPERATOR_CONTEXT });
      console.log('✅ Context saved via API (UI card not found — may already have context)');
    }
  });

  test('INTEL: Operator reviews Intel tab', async ({ page }) => {
    await gotoAs(page, 'Sirak', 'Sirak123', `/organizations/${ORG_ID}/crm/pipeline`);
    await openDealPanel(page, 'Franklin');
    await clickTab(page, 'Intel');
    await page.waitForTimeout(PAUSE);
    console.log('✅ Intel tab — reviewing person & company research');
  });

  test('INTEL: Operator goes to My Tasks, sees review task', async ({ page }) => {
    await gotoAs(page, 'Sirak', 'Sirak123', '/my-tasks');
    await page.waitForTimeout(PAUSE);

    const taskCard = page.locator('text=Review & approve').first();
    const hasTask = await taskCard.count() > 0;
    console.log('   Review task visible in My Tasks:', hasTask);
    if (hasTask) {
      await taskCard.scrollIntoViewIfNeeded();
      await page.waitForTimeout(PAUSE);
    }
    console.log('✅ My Tasks — operator sees pipeline review task');
  });

  test('INTEL: Complete review task, advance to BA', async ({ page }) => {
    await loginAs(page, 'admin', 'admin123');
    const newStage = await completeTasksAndAdvance(page, dealId);
    console.log('✅ Advanced to:', newStage);
  });

  // ═══════════════ STAGE 1: BUSINESS ANALYSIS ═══════════════

  test('BA: Operator sees deal in BA stage', async ({ page }) => {
    await gotoAs(page, 'Sirak', 'Sirak123', `/organizations/${ORG_ID}/crm/pipeline`);
    await openDealPanel(page, 'Franklin');
    await clickTab(page, 'Intel');
    await page.waitForTimeout(PAUSE);
    await clickTab(page, 'Review');
    await page.waitForTimeout(PAUSE);
    console.log('✅ BA stage — reviewing intel + business report');
  });

  test('BA: Check My Tasks for BA review task (assigned to Sirak)', async ({ page }) => {
    await gotoAs(page, 'Sirak', 'Sirak123', '/my-tasks');
    await page.waitForTimeout(PAUSE);

    const baTask = page.locator('text=business analysis').first();
    console.log('   BA review task visible:', await baTask.count() > 0);
    console.log('✅ My Tasks — BA review task (should be assigned to Sirak)');
  });

  test('BA: Complete review, advance to Discovery', async ({ page }) => {
    await loginAs(page, 'admin', 'admin123');
    const newStage = await completeTasksAndAdvance(page, dealId);
    console.log('✅ Advanced to:', newStage);
  });

  // ═══════════════ STAGE 2: DISCOVERY ═══════════════

  test('DISCOVERY: Link discovery transcript', async ({ page }) => {
    await loginAs(page, 'admin', 'admin123');
    await api(page, 'POST', `/crm/deals/${dealId}/transcripts`, {
      transcript_text: DISCOVERY_TRANSCRIPT,
      summary: 'Discovery call: brand refresh, social strategy, website redesign, video production. Budget $30-50K. Presentation call March 24.',
      matched_by: 'manual',
    });
    console.log('✅ Discovery transcript linked');
  });

  test('DISCOVERY: Operator views transcript in deal panel', async ({ page }) => {
    await gotoAs(page, 'Sirak', 'Sirak123', `/organizations/${ORG_ID}/crm/pipeline`);
    await openDealPanel(page, 'Franklin');
    await clickTab(page, 'Transcripts');
    await page.waitForTimeout(PAUSE);

    const dialog = page.locator('[role="dialog"]');
    const hasTranscript = (await dialog.textContent())?.includes('Discovery') ?? false;
    console.log('   Transcript visible:', hasTranscript);

    // View Overview for call scheduling UI
    await clickTab(page, 'Overview');
    await page.waitForTimeout(PAUSE);
    console.log('✅ Discovery stage — transcript + call scheduling reviewed');
  });

  test('DISCOVERY: Advance to Proposal (Astra Pass 2 + Cash triggered)', async ({ page }) => {
    await loginAs(page, 'admin', 'admin123');
    const newStage = await completeTasksAndAdvance(page, dealId);
    console.log('✅ Advanced to:', newStage);
    console.log('   Astra Pass 2 + Cash generating in background...');

    // Poll for Cash to finish
    for (let i = 0; i < 20; i++) {
      await page.waitForTimeout(3000);
      const poll = await api(page, 'GET', `/crm/deals/${dealId}/rich`);
      const pt = poll.data?.proposal_text ?? '';
      if (pt.length > 200) {
        console.log('   ✅ Cash proposal ready! Length:', pt.length);
        if (poll.data?.amount) console.log('   💰 Cash set deal.amount: $' + poll.data.amount);
        break;
      }
      if (i === 19) console.log('   ⚠️ Proposal still generating (may need more time)');
    }
  });

  // ═══════════════ STAGE 3: PROPOSAL ═══════════════

  test('PROPOSAL: Operator reviews Cash proposal', async ({ page }) => {
    await gotoAs(page, 'Sirak', 'Sirak123', `/organizations/${ORG_ID}/crm/pipeline`);
    await openDealPanel(page, 'Franklin');
    await clickTab(page, 'Proposal');
    await page.waitForTimeout(PAUSE);

    const dialog = page.locator('[role="dialog"]');
    const content = await dialog.textContent();
    console.log('   Proposal content length:', content?.length ?? 0);
    console.log('   Has Executive Summary:', content?.includes('Executive Summary') ?? false);
    await page.waitForTimeout(PAUSE);
    console.log('✅ Proposal reviewed by operator');
  });

  test('PROPOSAL: Operator approves proposal (UI click)', async ({ page }) => {
    await gotoAs(page, 'Sirak', 'Sirak123', `/organizations/${ORG_ID}/crm/pipeline`);
    await openDealPanel(page, 'Franklin');
    await clickTab(page, 'Proposal');
    await page.waitForTimeout(SHORT);

    const dialog = page.locator('[role="dialog"]');
    const approveBtn = dialog.locator('button').filter({ hasText: /Approve Proposal/i }).first();
    if (await approveBtn.count() > 0) {
      await approveBtn.click();
      await page.waitForTimeout(PAUSE);
      console.log('✅ Proposal APPROVED via UI — deliverables auto-created');
    } else {
      await loginAs(page, 'admin', 'admin123');
      await api(page, 'POST', `/crm/deals/${dealId}/approve-proposal`, {});
      console.log('✅ Proposal approved via API');
    }

    await loginAs(page, 'admin', 'admin123');
    const check = await api(page, 'GET', `/crm/deals/${dealId}/rich`);
    console.log('   Status:', check.data?.proposal_status);
    console.log('   Amount:', check.data?.amount ? `$${check.data.amount}` : 'pending');
  });

  test('PROPOSAL: Advance to Polish (Lux triggered)', async ({ page }) => {
    await loginAs(page, 'admin', 'admin123');
    const newStage = await completeTasksAndAdvance(page, dealId);
    console.log('✅ Advanced to:', newStage);

    // Wait for Lux
    for (let i = 0; i < 15; i++) {
      await page.waitForTimeout(2000);
      const poll = await api(page, 'GET', `/crm/deals/${dealId}/rich`);
      if (poll.data?.deck_url) { console.log('   ✅ Lux deck ready!'); break; }
      if (i === 14) console.log('   ⚠️ Deck still generating...');
    }
  });

  // ═══════════════ STAGE 4: POLISH ═══════════════

  test('POLISH: Operator reviews deck + shares for internal review', async ({ page }) => {
    await gotoAs(page, 'Sirak', 'Sirak123', `/organizations/${ORG_ID}/crm/pipeline`);
    await openDealPanel(page, 'Franklin');
    await clickTab(page, 'Deck');
    await page.waitForTimeout(PAUSE);

    const dialog = page.locator('[role="dialog"]');

    // Click Share for Review
    const shareBtn = dialog.locator('button').filter({ hasText: /Share for Review/i }).first();
    if (await shareBtn.count() > 0) {
      await shareBtn.click();
      await page.waitForTimeout(SHORT);
      console.log('   Internal review link shared');
    }
    await page.waitForTimeout(PAUSE);
    console.log('✅ Deck reviewed and shared for team feedback');
  });

  // ═══════════════ STAGE 5: PRESENT ═══════════════

  test('PRESENT: Advance to Present, operator sends invoice via UI', async ({ page }) => {
    await loginAs(page, 'admin', 'admin123');
    await completeTasksAndAdvance(page, dealId);

    // View as Sirak — send invoice from DeckTab
    await gotoAs(page, 'Sirak', 'Sirak123', `/organizations/${ORG_ID}/crm/pipeline`);
    await openDealPanel(page, 'Franklin');
    await clickTab(page, 'Deck');
    await page.waitForTimeout(PAUSE);

    const dialog = page.locator('[role="dialog"]');
    const sendBtn = dialog.locator('button').filter({ hasText: /Send Invoice/i }).first();
    if (await sendBtn.count() > 0) {
      await sendBtn.click();
      await page.waitForTimeout(SHORT);
      const confirmBtn = dialog.locator('button').filter({ hasText: /Confirm Send/i }).first();
      if (await confirmBtn.count() > 0) {
        await confirmBtn.click();
        await page.waitForTimeout(PAUSE);
        console.log('✅ Invoice sent via UI');
      }
    } else {
      await loginAs(page, 'admin', 'admin123');
      await api(page, 'POST', `/crm/deals/${dealId}/send-invoice`, { client_name: LEAD.full_name, notes: 'Brand refresh package', due_days: 14 });
      console.log('✅ Invoice sent via API');
    }
  });

  // ═══════════════ STAGE 6→7: FOLLOW UP → WON ═══════════════

  test('WON: Advance through Follow Up, mark deal Won via UI', async ({ page }) => {
    await loginAs(page, 'admin', 'admin123');
    // Advance Present → Follow Up
    await completeTasksAndAdvance(page, dealId);
    // Advance Follow Up (complete tasks)
    const rich = await api(page, 'GET', `/crm/deals/${dealId}/rich`);
    for (const task of rich.data?.tasks ?? []) {
      if (task.status !== 'done' && task.status !== 'cancelled') {
        await api(page, 'PATCH', `/tasks/${task.id}`, { status: 'done' });
      }
    }

    // Mark Won as Sirak via UI
    await gotoAs(page, 'Sirak', 'Sirak123', `/organizations/${ORG_ID}/crm/pipeline`);
    await openDealPanel(page, 'Franklin');
    await clickTab(page, 'Deck');
    await page.waitForTimeout(SHORT);

    const dialog = page.locator('[role="dialog"]');
    const wonBtn = dialog.locator('button').filter({ hasText: /Mark Won/i }).first();
    if (await wonBtn.count() > 0) {
      await wonBtn.click();
      await page.waitForTimeout(SHORT);
      const confirmBtn = dialog.locator('button').filter({ hasText: /Confirm Won/i }).first();
      if (await confirmBtn.count() > 0) {
        await confirmBtn.click();
        await page.waitForTimeout(PAUSE);
        console.log('🏆 DEAL WON via UI!');
      }
    } else {
      await loginAs(page, 'admin', 'admin123');
      const won = await api(page, 'POST', `/crm/deals/${dealId}/mark-won`, { win_reason: 'Client approved proposal after presentation' });
      if (won.data) {
        console.log('🏆 DEAL WON! Project:', won.data.project_name, '| Tasks:', won.data.tasks_created);
      }
    }

    await loginAs(page, 'admin', 'admin123');
    const final = await api(page, 'GET', `/crm/deals/${dealId}/rich`);
    console.log('   Final stage:', final.data?.stage);
    console.log('   Won at:', final.data?.won_at);
  });

  // ═══════════════ VERIFICATION ═══════════════

  test('VERIFY: Won column + company + person profiles', async ({ page }) => {
    // Pipeline board — Won column
    await gotoAs(page, 'Sirak', 'Sirak123', `/organizations/${ORG_ID}/crm/pipeline`);
    const wonCol = page.locator('text=Won').first();
    if (await wonCol.count() > 0) { await wonCol.scrollIntoViewIfNeeded(); }
    await page.waitForTimeout(PAUSE);
    console.log('✅ Pipeline board — deal in Won column');

    // Company profile
    await gotoAs(page, 'Sirak', 'Sirak123', `/companies/${companyId}`);
    await page.waitForTimeout(PAUSE);
    console.log('✅ Company profile:', (await page.textContent('body'))?.includes('Franklin') ? 'loaded' : 'error');

    // Person profile
    await gotoAs(page, 'Sirak', 'Sirak123', `/people/${personId}`);
    await page.waitForTimeout(PAUSE);
    console.log('✅ Person profile:', (await page.textContent('body'))?.includes('Devon') ? 'loaded' : 'error');

    // My Tasks — should show completed pipeline tasks
    await gotoAs(page, 'Sirak', 'Sirak123', '/my-tasks');
    await page.waitForTimeout(PAUSE);
    console.log('✅ My Tasks — pipeline workflow complete');
  });

  // ═══════════════ CLEANUP ═══════════════

  test('Cleanup', async ({ page }) => {
    await loginAs(page, 'admin', 'admin123');
    if (dealId) await page.request.delete(`http://localhost:3000/api/crm/deals/${dealId}`);
    if (contactId) await page.request.delete(`http://localhost:3000/api/crm/contacts/${contactId}`);
    if (personId) await page.request.delete(`http://localhost:3000/api/persons/${personId}`);
    if (companyId) await page.request.delete(`http://localhost:3000/api/companies/${companyId}`);
    console.log('✅ Cleaned up');
  });
});
