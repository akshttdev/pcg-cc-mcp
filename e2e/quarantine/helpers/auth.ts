import { Page } from '@playwright/test';

const BASE_URL = `http://localhost:${process.env.FRONTEND_PORT || '3000'}`;

export const DEAL_ID = '3b5de595-b2dc-0792-2284-e0349788dfd7';
export const ORG_ID = '02020202-0202-0202-0202-020202020202';
export const HUDSON_COMPANY_ID = '5b3d9e7c-8d05-454d-a73f-b8e659396072';

// Legacy constant — prefer discoverAcquisitionPipelineId() for dynamic lookup
export const PIPELINE_ID = 'eb837dc4-d803-471a-be9b-c59902e90b0e';

/**
 * Discover the Acquisition pipeline ID dynamically by querying the API.
 * Falls back to PIPELINE_ID constant if the query fails.
 */
export async function discoverAcquisitionPipelineId(page: Page): Promise<string> {
  try {
    const res = await page.request.get(
      `${BASE_URL}/api/crm/pipelines?organization_id=${ORG_ID}`,
    );
    const body = await res.json();
    const pipelines = body.data ?? [];
    const acquisition = pipelines.find(
      (p: { name: string; pipeline_type: string }) =>
        p.name === 'Acquisition' || p.pipeline_type === 'sales',
    );
    return acquisition?.id ?? PIPELINE_ID;
  } catch {
    return PIPELINE_ID;
  }
}

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
