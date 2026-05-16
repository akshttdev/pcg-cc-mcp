import { test, expect } from "@playwright/test";

/**
 * Avatar Engine — API-level smoke + access control.
 *
 * Pure-API tests: no browser auth setup required. We assert the multi-tenant
 * safety property at the API boundary (401 without credentials, 400 on
 * path traversal, retired init-talking-head route is gone).
 *
 * Out of scope for this spec (track separately):
 *   - Authenticated happy path (page load, upload, generate) — requires
 *     auth.setup.ts to be working in this worktree
 *   - Cross-tenant 403 between two real users — requires a non-admin user
 *     fixture, no helper exists yet (`createTestUser` in helpers/seed.ts)
 */

// Don't inherit storageState — every request is unauthenticated.
test.use({ storageState: { cookies: [], origins: [] } });

const ANY_UUID = "00000000-0000-0000-0000-000000000000";

test.describe("Avatar Engine — access control", () => {
  test("unauthenticated GET /api/video-gen/avatars returns 401", async ({ request }) => {
    const resp = await request.get("/api/video-gen/avatars");
    expect(resp.status()).toBe(401);
  });

  test("unauthenticated GET single avatar returns 401", async ({ request }) => {
    const resp = await request.get(`/api/video-gen/avatars/${ANY_UUID}`);
    expect(resp.status()).toBe(401);
  });

  test("unauthenticated POST generate-profile returns 401", async ({ request }) => {
    const resp = await request.post(
      `/api/video-gen/avatars/${ANY_UUID}/generate-profile`
    );
    expect(resp.status()).toBe(401);
  });

  test("unauthenticated POST render-motion returns 401", async ({ request }) => {
    const resp = await request.post(
      `/api/video-gen/avatars/${ANY_UUID}/render-motion`,
      { data: { script_text: "hello" } }
    );
    expect(resp.status()).toBe(401);
  });

  test("unauthenticated GET /api/video-gen/jobs returns 401", async ({ request }) => {
    const resp = await request.get("/api/video-gen/jobs");
    expect(resp.status()).toBe(401);
  });
});

test.describe("Avatar Engine — defense in depth (public routes)", () => {
  test("path traversal on motion clip filename is refused", async ({ request }) => {
    // Public route: no auth required, but the regex must reject `..` in the
    // clip filename. Avatar UUID is real-shaped so we get past Path<Uuid>.
    const resp = await request.get(
      `/api/video-gen/avatars/${ANY_UUID}/motion/bad..clip.mp4`
    );
    expect(resp.status()).toBe(400);
  });

  test("path with slash in clip filename is refused", async ({ request }) => {
    // The route extracts the filename as a single segment so a `/` in `clip`
    // would normally be impossible — but URL-encoded `%2f` could slip through.
    // Either the regex rejects it (preferred) or the router decodes and
    // routes it elsewhere (also fine, since the regex catches what reaches us).
    const resp = await request.get(
      `/api/video-gen/avatars/${ANY_UUID}/motion/bad%2Fclip.mp4`
    );
    expect([400, 404]).toContain(resp.status());
  });

  test("path-traversal regex accepts valid clip names (still 404 since file is absent)", async ({
    request,
  }) => {
    const resp = await request.get(
      `/api/video-gen/avatars/${ANY_UUID}/motion/intro_v1.mp4`
    );
    // Valid name passes the regex; file doesn't exist → 404.
    expect(resp.status()).toBe(404);
  });
});

test.describe("Avatar Engine — retired surface", () => {
  test("init-talking-head route is removed", async ({ request }) => {
    const resp = await request.post(
      `/api/video-gen/avatars/${ANY_UUID}/init-talking-head`
    );
    // Either 404 (clean miss) or 200 HTML (frontend catch-all on unknown
    // backend routes). Both confirm the API handler is gone.
    expect([200, 404]).toContain(resp.status());
    if (resp.status() === 200) {
      const body = await resp.text();
      expect(body.toLowerCase()).toContain("html");
    }
  });
});
