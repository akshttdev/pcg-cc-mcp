/**
 * Test company navigation as Sirak (non-admin user).
 * User reports redirect to org page when clicking company Profile/Intel.
 */
import { test, expect, Page } from '@playwright/test';

const BASE_URL = `http://localhost:${process.env.FRONTEND_PORT || '3000'}`;
const ORG_ID = '02020202-0202-0202-0202-020202020202';

async function loginAsSirak(page: Page) {
  const res = await page.request.post(`${BASE_URL}/api/auth/login`, {
    data: { username: 'Sirak', password: 'Sirak123' },
    headers: { 'Content-Type': 'application/json' },
  });
  const body = await res.json();
  const sessionId = body?.data?.session_id;
  if (!sessionId) throw new Error(`Sirak login failed: ${JSON.stringify(body)}`);

  await page.context().addCookies([{
    name: 'session_id',
    value: sessionId,
    domain: 'localhost',
    path: '/',
    httpOnly: false,
    secure: false,
  }]);

  return sessionId;
}

async function loginAndGotoAsSirak(page: Page, path: string) {
  await page.goto(`${BASE_URL}/`, { waitUntil: 'domcontentloaded' });
  const sessionId = await loginAsSirak(page);
  await page.evaluate((sid) => localStorage.setItem('session_id', sid), sessionId);
  await page.goto(`${BASE_URL}${path}`, { waitUntil: 'networkidle' });
  await page.evaluate((sid) => localStorage.setItem('session_id', sid), sessionId);
  await page.context().addCookies([{
    name: 'session_id',
    value: sessionId,
    domain: 'localhost',
    path: '/',
    httpOnly: false,
    secure: false,
  }]);
  await page.waitForTimeout(1500);
}

test.setTimeout(60000);

// ── Test 1: Direct URL to company ────────────────────────────────────────────

test('Sirak: Direct /companies/:id URL', async ({ page }) => {
  const navLog: string[] = [];
  page.on('framenavigated', (frame) => {
    if (frame === page.mainFrame()) navLog.push(frame.url());
  });

  await loginAndGotoAsSirak(page, '/companies/5b3d9e7c-8d05-454d-a73f-b8e659396072');
  await page.waitForTimeout(4000);

  console.log('FINAL URL:', page.url());
  console.log('Nav log:', navLog.filter(u => !u.includes('about:blank')));

  const body = await page.textContent('body');
  console.log('Body preview:', body?.slice(0, 200));

  if (page.url().includes('/organizations/')) {
    console.log('BUG: Sirak gets redirected to org page from direct company URL!');
    console.log('This is likely a RoleRoute permission issue — Sirak lacks platform_member role');
  }
});

// ── Test 2: CRM Contacts > Companies > Hudson Profile ────────────────────────

test('Sirak: CRM Contacts > Companies > Hudson Profile', async ({ page }) => {
  const navLog: string[] = [];
  page.on('framenavigated', (frame) => {
    if (frame === page.mainFrame()) navLog.push(frame.url());
  });

  await loginAndGotoAsSirak(page, `/organizations/${ORG_ID}/crm/contacts`);
  await page.waitForTimeout(3000);

  console.log('Step 1 URL:', page.url());

  // Click Companies sub-tab
  const companiesTab = page.locator('button').filter({ hasText: /^Companies/ }).first();
  if (await companiesTab.count() > 0) {
    await companiesTab.click();
    await page.waitForTimeout(2000);
  }

  // Find Hudson Profile link
  const hudsonProfile = page.locator('a[href*="/companies/5b3d9e7c"]').first();
  if (await hudsonProfile.count() > 0) {
    const href = await hudsonProfile.getAttribute('href');
    console.log('Hudson link href:', href);
    await hudsonProfile.click();
    await page.waitForTimeout(5000);

    console.log('FINAL URL:', page.url());
    console.log('Nav log:', navLog.filter(u => !u.includes('about:blank')));

    if (page.url().includes('/organizations/')) {
      console.log('BUG CONFIRMED: Sirak Profile click redirects to org page!');
    } else {
      console.log('OK: Navigated to company page');
    }
  } else {
    console.log('Hudson company link not found');
  }
});

// ── Test 3: Pipeline > Deal panel > IntelTab > PROFILE ───────────────────────

test('Sirak: Pipeline > Deal > IntelTab > PROFILE', async ({ page }) => {
  const navLog: string[] = [];
  page.on('framenavigated', (frame) => {
    if (frame === page.mainFrame()) navLog.push(frame.url());
  });

  await loginAndGotoAsSirak(page, `/organizations/${ORG_ID}/crm/acquisition`);
  await page.waitForSelector('[class*="inline-grid"]', { timeout: 20000 });
  await page.waitForTimeout(2000);

  const kanban = page.locator('[class*="inline-grid"]');
  const dealCard = kanban.locator('p').filter({ hasText: /Joshua Marotta/ }).first();
  if (await dealCard.count() > 0) {
    await dealCard.scrollIntoViewIfNeeded();
    await dealCard.click();
    await page.waitForSelector('[role="dialog"]', { timeout: 10000 });
    await page.waitForTimeout(1000);

    const dialog = page.locator('[role="dialog"]');
    await dialog.locator('[role="tablist"] button, [role="tab"]').filter({ hasText: /^Intel$/ }).first().click();
    await page.waitForTimeout(1500);

    const profileLink = dialog.locator('a[href*="/companies/"]').first();
    if (await profileLink.count() > 0) {
      const href = await profileLink.getAttribute('href');
      console.log('PROFILE href:', href);
      await profileLink.click();
      await page.waitForTimeout(5000);

      console.log('FINAL URL:', page.url());
      console.log('Nav log:', navLog.filter(u => !u.includes('about:blank')));

      if (page.url().includes('/organizations/')) {
        console.log('BUG CONFIRMED: Sirak IntelTab PROFILE redirects to org page!');
      }
    }
  }
});

// ── Test 4: Check Sirak's effective role ─────────────────────────────────────

test('Sirak: Check role and permissions', async ({ page }) => {
  await loginAndGotoAsSirak(page, '/');
  await page.waitForTimeout(3000);

  console.log('Default landing URL for Sirak:', page.url());

  // Check what role Sirak has
  const roleInfo = await page.evaluate(() => {
    const stored = localStorage.getItem('session_id');
    return { sessionId: stored?.slice(0, 8), url: window.location.href };
  });
  console.log('Session info:', roleInfo);

  // Try navigating to /companies/... directly
  await page.goto(`${BASE_URL}/companies/5b3d9e7c-8d05-454d-a73f-b8e659396072`, { waitUntil: 'networkidle' });
  await page.waitForTimeout(3000);

  console.log('After direct company nav URL:', page.url());
  const body = await page.textContent('body');
  console.log('Body:', body?.slice(0, 200));

  if (page.url().includes('/organizations/') || page.url().includes('/login')) {
    console.log('ROOT CAUSE: Sirak does not have permission to access /companies/:id route');
    console.log('The route requires RoleRoute minRole="platform_member"');
    console.log('Sirak is likely org_member or lower, so RoleRoute redirects to /');
    console.log('And / redirects to the org page');
  }
});
