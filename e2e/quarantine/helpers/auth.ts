import { Page } from '@playwright/test';

const BASE_URL = `http://localhost:${process.env.FRONTEND_PORT || '3000'}`;

export const DEAL_ID = '3b5de595-b2dc-0792-2284-e0349788dfd7';
export const PIPELINE_ID = '138ff8ec-6d65-493e-b6a9-0f9fef409968';
export const ORG_ID = '02020202-0202-0202-0202-020202020202';

export async function loginAsAdmin(page: Page) {
  const res = await page.request.post(`${BASE_URL}/api/auth/login`, {
    data: { username: 'admin', password: 'admin123' },
    headers: { 'Content-Type': 'application/json' },
  });
  const body = await res.json();
  const sessionId = body?.data?.session_id;
  if (!sessionId) throw new Error(`Login failed: ${JSON.stringify(body)}`);

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
  await page.goto(`${BASE_URL}/`, { waitUntil: 'domcontentloaded' });
  const sessionId = await loginAsAdmin(page);

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
