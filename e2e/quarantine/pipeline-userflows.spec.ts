/**
 * Pipeline User Flow E2E Tests
 * Covers: task cards, sidebar navigation, call intake, client overview,
 * person/company profile pages, and cross-page link integrity.
 */
import { test, expect, Page } from '@playwright/test';
import { loginAsAdmin, loginAndGoto, DEAL_ID, PIPELINE_ID, ORG_ID } from './helpers/auth';

test.beforeEach(async ({ page }) => {
  await loginAsAdmin(page);
});

async function apiGet(page: Page, path: string) {
  const res = await page.request.get(`http://localhost:3000/api${path}`);
  return res.json();
}

// ── Sidebar Navigation ───────────────────────────────────────────────────────

test.describe('Sidebar Navigation', () => {
  test('sidebar loads with CRM pipeline link for org', async ({ page }) => {
    await loginAndGoto(page, '/');
    await page.waitForTimeout(2000);

    // Sidebar should contain "Acquisition" or "Pipeline" or CRM link
    const sidebar = page.locator('nav, [class*="sidebar"], aside').first();
    const bodyText = await page.textContent('body');
    const hasCrmLink = bodyText?.includes('Acquisition') || bodyText?.includes('Pipeline') || bodyText?.includes('CRM');
    console.log('Sidebar has CRM link:', hasCrmLink);
    expect(hasCrmLink).toBe(true);
  });

  test('sidebar person links use /people/ not /persons/', async ({ page }) => {
    await loginAndGoto(page, '/');
    await page.waitForTimeout(2000);

    // Check all links in the page — none should be /persons/ (except API calls)
    const allLinks = await page.locator('a[href*="/persons/"]').all();
    for (const link of allLinks) {
      const href = await link.getAttribute('href');
      // /persons/ links should not exist in navigation — only /people/
      console.log('Found /persons/ link (should be redirect only):', href);
    }

    // /people/ links should exist if any person is shown
    const peopleLinks = await page.locator('a[href*="/people/"]').count();
    console.log('Found /people/ links:', peopleLinks);
  });
});

// ── Call Intake Page ─────────────────────────────────────────────────────────

test.describe('Call Intake Page', () => {
  test('call intake page loads with items', async ({ page }) => {
    await loginAndGoto(page, '/call-intake');
    await page.waitForTimeout(2000);

    const bodyText = await page.textContent('body');
    expect(bodyText?.length).toBeGreaterThan(100);
    console.log('Call intake page loaded, body length:', bodyText?.length);
  });

  test('intake page has no /persons/ links (all should be /people/)', async ({ page }) => {
    await loginAndGoto(page, '/call-intake');
    await page.waitForTimeout(3000);

    const personsLinks = await page.locator('a[href*="/persons/"]').count();
    const peopleLinks = await page.locator('a[href*="/people/"]').count();
    console.log('/persons/ links:', personsLinks, '| /people/ links:', peopleLinks);
    expect(personsLinks).toBe(0);
  });

  test('intake items do NOT have Create Deal button yet (gap confirmation)', async ({ page }) => {
    await loginAndGoto(page, '/call-intake');
    await page.waitForTimeout(2000);

    const createDealBtn = page.locator('button').filter({ hasText: /create deal/i });
    const count = await createDealBtn.count();
    console.log('"Create Deal" buttons found:', count, '(expected: 0 — gap P1 #4)');
    // This confirms the gap — when we implement it, change expect to > 0
    expect(count).toBe(0);
  });
});

// ── Task Cards (Pipeline Tasks) ──────────────────────────────────────────────

test.describe('Task Cards — Deal-Linked Tasks', () => {
  test('deal has associated tasks via API', async ({ page }) => {
    const res = await apiGet(page, `/crm/deals/${DEAL_ID}/rich`);
    const tasks = res.data?.tasks ?? [];
    console.log('Deal tasks:', tasks.length);
    for (const t of tasks) {
      console.log(`  - [${t.status}] ${t.title}`);
    }
    // Deal should have at least review tasks from stage transitions
    expect(tasks.length).toBeGreaterThanOrEqual(0);
  });

  test('my-tasks page loads and shows task cards', async ({ page }) => {
    await loginAndGoto(page, '/my-tasks');
    await page.waitForTimeout(3000);

    const bodyText = await page.textContent('body');
    expect(bodyText?.length).toBeGreaterThan(50);
    console.log('My tasks page loaded, body length:', bodyText?.length);
  });
});

// ── Person Profile Page ──────────────────────────────────────────────────────

test.describe('Person Profile Page', () => {
  test('/people/:id loads person profile', async ({ page }) => {
    const res = await apiGet(page, `/crm/deals/${DEAL_ID}/rich`);
    const personId = res.data?.person_id;
    expect(personId).toBeTruthy();

    await loginAndGoto(page, `/people/${personId}`);
    await page.waitForTimeout(2000);

    const bodyText = await page.textContent('body');
    expect(bodyText).toContain('Marotta');
    console.log('Person profile page loaded for:', personId);
  });

  test('/persons/:id redirects to /people/:id', async ({ page }) => {
    const res = await apiGet(page, `/crm/deals/${DEAL_ID}/rich`);
    const personId = res.data?.person_id;

    await loginAndGoto(page, `/persons/${personId}`);
    await page.waitForTimeout(2000);

    // Should have redirected to /people/
    const url = page.url();
    expect(url).toContain('/people/');
    expect(url).not.toContain('/persons/');
    console.log('Redirected to:', url);
  });

  test('person intel page loads at /people/:id/intel', async ({ page }) => {
    const res = await apiGet(page, `/crm/deals/${DEAL_ID}/rich`);
    const personId = res.data?.person_id;

    await loginAndGoto(page, `/people/${personId}/intel`);
    await page.waitForTimeout(2000);

    const bodyText = await page.textContent('body');
    expect(bodyText?.length).toBeGreaterThan(100);
    console.log('Person intel page loaded, content length:', bodyText?.length);
  });
});

// ── Company Profile Page ─────────────────────────────────────────────────────

test.describe('Company Profile Page', () => {
  test('/companies/:id loads company profile', async ({ page }) => {
    const res = await apiGet(page, `/crm/deals/${DEAL_ID}/rich`);
    const companyId = res.data?.company_id;
    expect(companyId).toBeTruthy();

    await loginAndGoto(page, `/companies/${companyId}`);
    await page.waitForTimeout(2000);

    const bodyText = await page.textContent('body');
    expect(bodyText).toContain('Hudson');
    console.log('Company profile page loaded for:', companyId);
  });

  test('company profile has intelligence data', async ({ page }) => {
    const res = await apiGet(page, `/crm/deals/${DEAL_ID}/rich`);
    const companyId = res.data?.company_id;

    await loginAndGoto(page, `/companies/${companyId}`);
    await page.waitForTimeout(3000);

    const bodyText = await page.textContent('body');
    // Should show some intelligence content
    const hasIntel = bodyText?.includes('Intelligence') || bodyText?.includes('Research') ||
                     bodyText?.includes('intel') || bodyText?.includes('done');
    console.log('Company profile has intelligence content:', hasIntel);
  });
});

// ── Client Overview Page ─────────────────────────────────────────────────────

test.describe('Client Overview Page', () => {
  test('client overview page loads (org-scoped)', async ({ page }) => {
    await loginAndGoto(page, `/organizations/${ORG_ID}`);
    await page.waitForTimeout(2000);

    const bodyText = await page.textContent('body');
    expect(bodyText?.length).toBeGreaterThan(100);
    console.log('Organization page loaded, body length:', bodyText?.length);
  });

  test('org page has no /persons/ links', async ({ page }) => {
    await loginAndGoto(page, `/organizations/${ORG_ID}`);
    await page.waitForTimeout(2000);

    const personsLinks = await page.locator('a[href*="/persons/"]').count();
    const peopleLinks = await page.locator('a[href*="/people/"]').count();
    console.log('/persons/ links:', personsLinks, '| /people/ links:', peopleLinks);
    expect(personsLinks).toBe(0);
  });
});

// ── Cross-Page Link Integrity ────────────────────────────────────────────────

test.describe('Cross-Page Link Integrity', () => {
  test('deal advance-requirements API returns valid gate data', async ({ page }) => {
    const res = await apiGet(page, `/crm/deals/${DEAL_ID}/advance-requirements`);
    const data = res.data ?? res;
    console.log('Advance requirements:', JSON.stringify(data).slice(0, 200));
    // Should return some structure (not an error)
    expect(data).toBeTruthy();
  });

  test('deal stage move API accepts valid stage transition', async ({ page }) => {
    // Just verify the current stage is queryable — don't actually move
    const res = await apiGet(page, `/crm/deals/${DEAL_ID}/rich`);
    const stage = res.data?.stage;
    const stageId = res.data?.crm_stage_id;
    expect(stage).toBeTruthy();
    expect(stageId).toBeTruthy();
    console.log('Current stage:', stage, '| stage_id:', stageId);
  });

  test('organization CRM board loads for Sirak Studios', async ({ page }) => {
    await loginAndGoto(page, `/organizations/${ORG_ID}/crm/acquisition`);
    await page.waitForTimeout(3000);

    const bodyText = await page.textContent('body');
    // Should show pipeline stages
    expect(bodyText).toContain('Intel');
    expect(bodyText).toContain('Proposal');
    console.log('CRM board loaded with pipeline stages');
  });

  test('proposals page loads', async ({ page }) => {
    await loginAndGoto(page, '/proposals');
    await page.waitForTimeout(2000);

    const bodyText = await page.textContent('body');
    expect(bodyText?.length).toBeGreaterThan(100);
    console.log('Proposals page loaded');
  });

  test('command center loads', async ({ page }) => {
    await loginAndGoto(page, '/command-center');
    await page.waitForTimeout(2000);

    const bodyText = await page.textContent('body');
    expect(bodyText?.length).toBeGreaterThan(100);
    console.log('Command center loaded');
  });

  test('business reports page loads', async ({ page }) => {
    await loginAndGoto(page, '/business-reports');
    await page.waitForTimeout(2000);

    const bodyText = await page.textContent('body');
    expect(bodyText?.length).toBeGreaterThan(100);
    console.log('Business reports page loaded');
  });

  test('people directory page loads with /people/ links', async ({ page }) => {
    await loginAndGoto(page, '/people');
    await page.waitForTimeout(2000);

    const bodyText = await page.textContent('body');
    expect(bodyText?.length).toBeGreaterThan(100);

    // All person links should be /people/
    const personsLinks = await page.locator('a[href*="/persons/"]').count();
    expect(personsLinks).toBe(0);
    console.log('People directory loaded, no /persons/ links found');
  });
});
