/**
 * E2E Test Seed Helpers
 *
 * API-based entity creation for test setup. Tests should create their own data
 * in beforeAll/beforeEach using these helpers, prefixed with [E2E] for cleanup.
 *
 * All helpers return the created entity for assertions.
 */

import type { APIRequestContext } from '@playwright/test';

const TEST_PREFIX = '[E2E]';
const BASE_URL = process.env.FRONTEND_URL || `http://localhost:${process.env.FRONTEND_PORT || 3000}`;

/** Auth headers from stored session */
function authHeaders(sessionId: string) {
  return {
    'Content-Type': 'application/json',
    Authorization: `Bearer ${sessionId}`,
  };
}

/** Get session ID from auth storage file */
export async function getSessionId(): Promise<string> {
  const fs = await import('fs');
  const path = await import('path');
  const authFile = path.resolve(__dirname, '../.auth/user.json');
  if (!fs.existsSync(authFile)) {
    throw new Error('Auth file not found — run auth.setup.ts first');
  }
  const auth = JSON.parse(fs.readFileSync(authFile, 'utf-8'));
  const sessionCookie = auth.cookies?.find(
    (c: { name: string }) => c.name === 'session_id'
  );
  const sessionStorage = auth.origins?.[0]?.localStorage?.find(
    (item: { name: string }) => item.name === 'session_id'
  );
  return sessionCookie?.value || sessionStorage?.value || '';
}

/** Create a test organization */
export async function createTestOrg(
  request: APIRequestContext,
  sessionId: string,
  name?: string
) {
  const orgName = name || `${TEST_PREFIX} Test Org ${Date.now()}`;
  const resp = await request.post(`${BASE_URL}/api/organizations`, {
    headers: authHeaders(sessionId),
    data: { name: orgName, slug: orgName.toLowerCase().replace(/\W+/g, '-') },
  });
  if (!resp.ok()) {
    throw new Error(`Failed to create org: ${resp.status()} ${await resp.text()}`);
  }
  const body = await resp.json();
  return body.data;
}

/** Create a test CRM contact */
export async function createTestContact(
  request: APIRequestContext,
  sessionId: string,
  organizationId: string,
  overrides?: { first_name?: string; last_name?: string; email?: string }
) {
  const data = {
    organization_id: organizationId,
    first_name: overrides?.first_name || `${TEST_PREFIX} Contact`,
    last_name: overrides?.last_name || `${Date.now()}`,
    email: overrides?.email || `e2e-${Date.now()}@test.local`,
  };
  const resp = await request.post(`${BASE_URL}/api/crm/contacts`, {
    headers: authHeaders(sessionId),
    data,
  });
  if (!resp.ok()) {
    throw new Error(`Failed to create contact: ${resp.status()} ${await resp.text()}`);
  }
  const body = await resp.json();
  return body.data;
}

/** Create a test CRM deal */
export async function createTestDeal(
  request: APIRequestContext,
  sessionId: string,
  organizationId: string,
  pipelineId: string,
  overrides?: { name?: string; crm_contact_id?: string; crm_stage_id?: string }
) {
  const data = {
    organization_id: organizationId,
    pipeline_id: pipelineId,
    name: overrides?.name || `${TEST_PREFIX} Deal ${Date.now()}`,
    crm_contact_id: overrides?.crm_contact_id,
    crm_stage_id: overrides?.crm_stage_id,
  };
  const resp = await request.post(`${BASE_URL}/api/crm/deals`, {
    headers: authHeaders(sessionId),
    data,
  });
  if (!resp.ok()) {
    throw new Error(`Failed to create deal: ${resp.status()} ${await resp.text()}`);
  }
  const body = await resp.json();
  return body.data;
}

/** Create a test company */
export async function createTestCompany(
  request: APIRequestContext,
  sessionId: string,
  organizationId: string,
  overrides?: { name?: string; website?: string }
) {
  const data = {
    organization_id: organizationId,
    name: overrides?.name || `${TEST_PREFIX} Company ${Date.now()}`,
    website: overrides?.website,
  };
  const resp = await request.post(`${BASE_URL}/api/companies`, {
    headers: authHeaders(sessionId),
    data,
  });
  if (!resp.ok()) {
    throw new Error(`Failed to create company: ${resp.status()} ${await resp.text()}`);
  }
  const body = await resp.json();
  return body.data;
}

/** Clean up all entities with [E2E] prefix */
export async function cleanupTestData(
  request: APIRequestContext,
  sessionId: string
) {
  const headers = authHeaders(sessionId);

  // Clean deals
  try {
    const dealsResp = await request.get(`${BASE_URL}/api/crm/deals`, { headers });
    if (dealsResp.ok()) {
      const deals = (await dealsResp.json()).data || [];
      for (const deal of deals) {
        if (deal.name?.startsWith(TEST_PREFIX)) {
          await request.delete(`${BASE_URL}/api/crm/deals/${deal.id}`, { headers });
        }
      }
    }
  } catch {
    // Best effort cleanup
  }

  // Clean contacts
  try {
    const contactsResp = await request.get(`${BASE_URL}/api/crm/contacts`, { headers });
    if (contactsResp.ok()) {
      const contacts = (await contactsResp.json()).data || [];
      for (const contact of contacts) {
        const name = `${contact.first_name} ${contact.last_name}`;
        if (name.startsWith(TEST_PREFIX)) {
          await request.delete(`${BASE_URL}/api/crm/contacts/${contact.id}`, { headers });
        }
      }
    }
  } catch {
    // Best effort cleanup
  }
}

/** Well-known test constants */
export const TEST_CONSTANTS = {
  ORG_ID: '02020202-0202-0202-0202-020202020202',
  ORG_NAME: 'Sirak Studios',
  PIPELINE_ID: '138ff8ec-6d65-493e-b6a9-0f9fef409968',
  PIPELINE_NAME: 'Sirak Studios Acquisition',
  ADMIN_USERNAME: 'admin',
  ADMIN_PASSWORD: 'admin123',
  STAGES: {
    LEAD: 'a1000001-0000-0000-0000-000000000001',
    INTEL: 'a1000002-0000-0000-0000-000000000002',
    BUSINESS_ANALYSIS: 'a1000003-0000-0000-0000-000000000003',
    PROPOSAL: 'a1000004-0000-0000-0000-000000000004',
    POLISH: 'a1000005-0000-0000-0000-000000000005',
    INVOICE: 'a1000006-0000-0000-0000-000000000006',
    NEGOTIATION: 'a1000007-0000-0000-0000-000000000007',
    WON: 'a1000008-0000-0000-0000-000000000008',
    LOST: 'a1000009-0000-0000-0000-000000000009',
  },
} as const;
