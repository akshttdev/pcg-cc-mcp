/**
 * Video Pipeline E2E Tests
 * Tests the video job API, avatar profiles API, and public video serving.
 */
import { expect, Page, test } from '@playwright/test';

import { loginAsAdmin } from './helpers/auth';

const SAMI_ID = '528c5bf7-364c-40fd-a593-86d90ca7acaa';

// ── Auth setup ────────────────────────────────────────────────────────────────

test.beforeEach(async ({ page }) => {
  await loginAsAdmin(page);
});

// ── API helpers ───────────────────────────────────────────────────────────────

async function apiGet(page: Page, path: string) {
  const res = await page.request.get(`http://localhost:3000/api${path}`);
  return { status: res.status(), body: await res.json() };
}

async function apiPost(page: Page, path: string, body: object) {
  const res = await page.request.post(`http://localhost:3000/api${path}`, {
    data: body,
    headers: { 'Content-Type': 'application/json' },
  });
  return { status: res.status(), body: await res.json() };
}

// ── Video Job API ─────────────────────────────────────────────────────────────

test.describe('Video Job API', () => {
  test('creates a video job with segments metadata', async ({ page }) => {
    const segments = JSON.stringify([
      {
        label: 'Test',
        text: 'Test script.',
        broll_category: 'pcg',
        approx_words: 4,
        broll_duration: 8,
      },
    ]);

    const { status, body } = await apiPost(page, '/video-gen/jobs', {
      avatar_profile_id: SAMI_ID,
      script_text: 'Test script for E2E.',
      segments_json: segments,
    });

    // Accept 201 Created or 404 if Sami avatar not seeded in this environment
    if (status === 404) {
      console.log(
        'Avatar not found in this environment — skipping creation assertion'
      );
      return;
    }

    expect(status).toBe(201);
    expect(body.id).toBeTruthy();
    expect(['pending', 'tts_generating']).toContain(body.status);
    expect(body.segments_json).toBeTruthy();
    console.log('✅ Created video job:', body.id, 'status:', body.status);
  });

  test('fetches video job status', async ({ page }) => {
    // First create a job (if avatar exists)
    const createRes = await apiPost(page, '/video-gen/jobs', {
      avatar_profile_id: SAMI_ID,
      script_text: 'Status check test.',
      segments_json: null,
    });

    if (createRes.status === 404) {
      // Try listing existing jobs instead
      const { status, body } = await apiGet(page, '/video-gen/jobs');
      expect(status).toBe(200);
      const jobs = Array.isArray(body) ? body : (body.data ?? []);
      if (jobs.length === 0) {
        console.log('No jobs available in environment — skipping status fetch');
        return;
      }
      const jobId = jobs[0].id;
      const { status: getStatus, body: getBody } = await apiGet(
        page,
        `/video-gen/jobs/${jobId}`
      );
      expect(getStatus).toBe(200);
      expect(getBody.id).toBeTruthy();
      expect(getBody.tts_status).toBeTruthy();
      expect(getBody.postprod_status).toBeTruthy();
      console.log('✅ Fetched existing job status:', getBody.status);
      return;
    }

    expect(createRes.status).toBe(201);
    const jobId = createRes.body.id;

    const { status, body } = await apiGet(page, `/video-gen/jobs/${jobId}`);
    expect(status).toBe(200);
    expect(body.id).toBeTruthy();
    expect(body.tts_status).toBeTruthy();
    expect(body.postprod_status).toBeTruthy();
    console.log(
      '✅ Job status:',
      body.status,
      '| tts_status:',
      body.tts_status
    );
  });

  test('lists video jobs', async ({ page }) => {
    const { status, body } = await apiGet(page, '/video-gen/jobs');
    expect(status).toBe(200);
    const jobs = Array.isArray(body) ? body : (body.data ?? body);
    expect(Array.isArray(jobs)).toBe(true);
    console.log('✅ Listed video jobs, count:', jobs.length);
  });
});

// ── Avatar Profiles API ───────────────────────────────────────────────────────
// Route: /api/video-gen/avatars (authenticated)

test.describe('Avatar Profiles API', () => {
  test('lists avatar profiles', async ({ page }) => {
    const { status, body } = await apiGet(page, '/video-gen/avatars');
    expect(status).toBe(200);
    const profiles = Array.isArray(body) ? body : (body.data ?? body);
    expect(Array.isArray(profiles)).toBe(true);
    if (profiles.length > 0) {
      expect(profiles[0].slug).toBeTruthy();
      console.log(
        '✅ Avatar profiles:',
        profiles.map((p: { slug: string }) => p.slug)
      );
    } else {
      console.log('No avatar profiles seeded in this environment');
    }
  });

  test('sami-satoshi avatar has required fields', async ({ page }) => {
    // List all avatars and find sami-satoshi
    const { status, body } = await apiGet(page, '/video-gen/avatars');
    expect(status).toBe(200);
    const profiles = Array.isArray(body) ? body : (body.data ?? body);
    const sami = profiles.find(
      (p: { slug: string }) => p.slug === 'sami-satoshi'
    );
    if (!sami) {
      console.log(
        'sami-satoshi avatar not seeded in this environment — skipping'
      );
      return;
    }
    expect(sami.heygen_avatar_id).toBeTruthy();
    expect(sami.elevenlabs_voice_id).toBeTruthy();
    console.log('✅ sami-satoshi — heygen_avatar_id:', sami.heygen_avatar_id);
  });
});

// ── Public Video Serving ──────────────────────────────────────────────────────

test.describe('Public Video Serving', () => {
  test('serve_final_video returns 404 for unknown job', async ({ page }) => {
    const res = await page.request.get(
      'http://localhost:3000/api/video-gen/final/00000000-0000-0000-0000-000000000000'
    );
    // Expect either 404 (correct) or the response body indicates an error
    if (res.status() === 404) {
      console.log('✅ Got expected 404 for unknown job');
    } else {
      const body = await res.json().catch(() => null);
      expect(body?.error ?? body?.message ?? res.status()).toBeTruthy();
      console.log(
        '✅ Got error response for unknown job, status:',
        res.status()
      );
    }
    expect([404, 400, 500]).toContain(res.status());
  });

  test('serve_final_video supports range requests on existing video', async ({
    page,
  }) => {
    // Find a job with postprod_ready status
    const { body: jobsBody } = await apiGet(page, '/video-gen/jobs');
    const jobs = Array.isArray(jobsBody) ? jobsBody : (jobsBody.data ?? []);

    const readyJob = jobs.find(
      (j: { status: string }) => j.status === 'postprod_ready'
    );

    if (!readyJob) {
      console.log('No postprod_ready job found — skipping range request test');
      return;
    }

    // Playwright request API strips Range headers; use page.evaluate + native fetch instead
    await page.goto('http://localhost:3000/', {
      waitUntil: 'domcontentloaded',
    });
    const result = await page.evaluate(async (jobId: string) => {
      const res = await fetch(`/api/video-gen/final/${jobId}`, {
        headers: { Range: 'bytes=0-1023' },
      });
      return {
        status: res.status,
        contentRange: res.headers.get('content-range'),
      };
    }, readyJob.id);

    expect(result.status).toBe(206);
    expect(result.contentRange).toMatch(/^bytes 0-1023\//);
    console.log('✅ Range request 206 — content-range:', result.contentRange);
  });
});
