/**
 * Topsi Chat API E2E Tests
 * Tests the Topsi chat endpoint for basic response behavior, video status
 * queries, unknown commands, and auth enforcement.
 */
import { expect, Page, test } from '@playwright/test';

import { loginAsAdmin } from './helpers/auth';

// ── API helper ────────────────────────────────────────────────────────────────

async function chatPost(page: Page, body: object) {
  const res = await page.request.post('http://localhost:3000/api/topsi/chat', {
    data: body,
    headers: { 'Content-Type': 'application/json' },
  });
  return { status: res.status(), body: await res.json().catch(() => null) };
}

// ── Topsi Chat API ────────────────────────────────────────────────────────────

test.describe('Topsi Chat API', () => {
  test.beforeEach(async ({ page }) => {
    await loginAsAdmin(page);
  });

  test('responds to a simple greeting', async ({ page }) => {
    const { status, body } = await chatPost(page, {
      message: 'Hello, what can you do?',
      sessionId: 'e2e-test-greeting-001',
    });

    // Accept success or a response containing a message
    expect([200, 201]).toContain(status);
    expect(body).toBeTruthy();

    // The response should have a success flag or a message field
    const hasMessage =
      body?.message !== undefined ||
      body?.data?.message !== undefined ||
      body?.reply !== undefined ||
      body?.response !== undefined;

    expect(body?.success === true || hasMessage).toBe(true);

    const messageText =
      body?.message ??
      body?.data?.message ??
      body?.reply ??
      body?.response ??
      '';
    if (messageText) {
      expect(typeof messageText).toBe('string');
      console.log(
        '✅ Topsi greeting response (first 100 chars):',
        String(messageText).slice(0, 100)
      );
    }
  });

  test('handles video status query', async ({ page }) => {
    // LLM call may take up to 60s — override default 30s timeout
    test.setTimeout(90_000);

    const { status, body } = await chatPost(page, {
      message: 'What is the status of our video pipeline?',
      sessionId: 'e2e-test-video-status-001',
    });

    expect([200, 201]).toContain(status);
    expect(body).toBeTruthy();
    // No error field present (or error is falsy)
    expect(body?.error).toBeFalsy();

    const messageText =
      body?.message ?? body?.data?.message ?? body?.reply ?? body?.response;
    expect(messageText).toBeTruthy();
    console.log(
      '✅ Video status response (first 100 chars):',
      String(messageText ?? '').slice(0, 100)
    );
  });

  test('does not crash on unknown commands', async ({ page }) => {
    const { status, body } = await chatPost(page, {
      message: 'xkcd random nonsense command 12345',
      sessionId: 'e2e-test-unknown-001',
    });

    // Must not return 500
    expect(status).not.toBe(500);
    expect(body).toBeTruthy();

    const messageText =
      body?.message ?? body?.data?.message ?? body?.reply ?? body?.response;
    expect(messageText).toBeTruthy();
    console.log('✅ Unknown command handled gracefully, status:', status);
  });
});

// ── Rate Limits / Auth ────────────────────────────────────────────────────────

test.describe('Topsi Rate Limits / Auth', () => {
  // No beforeEach here — we intentionally skip login for the auth test

  test('rejects unauthenticated requests', async ({ page }) => {
    // Do NOT call loginAsAdmin — make request without auth
    const res = await page.request.post(
      'http://localhost:3000/api/topsi/chat',
      {
        data: {
          message: 'Hello without auth',
          sessionId: 'e2e-test-noauth-001',
        },
        headers: { 'Content-Type': 'application/json' },
      }
    );

    const status = res.status();
    const body = await res.json().catch(() => null);

    // Should be 401 Unauthorized or have success: false
    const isRejected =
      status === 401 ||
      status === 403 ||
      body?.success === false ||
      body?.error !== undefined;

    expect(isRejected).toBe(true);
    console.log(
      '✅ Unauthenticated request rejected — status:',
      status,
      'success:',
      body?.success
    );
  });
});
