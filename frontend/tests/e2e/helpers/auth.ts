import { Page, BrowserContext } from '@playwright/test';

export const DEAL_ID = '3b5de595-b2dc-0792-2284-e0349788dfd7';
export const PIPELINE_ID = '138ff8ec-6d65-493e-b6a9-0f9fef409968';
export const ORG_ID = '02020202-0202-0202-0202-020202020202';

export async function loginAsAdmin(page: Page) {
  // Use API login to get a session_id cookie
  const res = await page.request.post('http://localhost:3000/api/auth/login', {
    data: { username: 'admin', password: 'admin123' },
    headers: { 'Content-Type': 'application/json' },
  });
  const body = await res.json();
  const sessionId = body?.data?.session_id;
  if (!sessionId) throw new Error(`Login failed: ${JSON.stringify(body)}`);

  // Set the session cookie
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

export async function loginAndGoto(page: Page, path: string) {
  // Navigate to base first to establish domain for cookies
  await page.goto('http://localhost:3000/', { waitUntil: 'domcontentloaded' });
  const sessionId = await loginAsAdmin(page);

  // Also set localStorage so AuthContext picks up session via either mechanism
  await page.evaluate((sid) => localStorage.setItem('session_id', sid), sessionId);

  // Navigate to the target path
  await page.goto(`http://localhost:3000${path}`, { waitUntil: 'networkidle' });

  // Re-inject after navigation in case SPA cleared storage
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
