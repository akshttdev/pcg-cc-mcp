import { Page, APIRequestContext, expect } from "@playwright/test";

const headed = process.env.E2E_HEADED === "true";

/**
 * Scale a timeout for headed mode (slowMo makes everything take longer).
 * Usage: `await expect(locator).toBeVisible({ timeout: t(5_000) })`
 */
export function t(baseMs: number, headedMs?: number): number {
  if (!headed) return baseMs;
  return headedMs ?? baseMs * 2;
}

/**
 * Ensure page is authenticated. Call this in beforeEach when using shared context.
 * If the page ended up on /login (e.g. after cookie loss), re-authenticates.
 */
export async function ensureAuthenticated(page: Page) {
  await page.goto("/");
  await page.waitForLoadState("domcontentloaded");
  // Check if we got redirected to login
  try {
    await page.waitForURL(/\/login/, { timeout: 1_000 });
    // We're on login — need to re-auth
    await login(page);
  } catch {
    // Not on login — we're authenticated, good
  }
}

/** Default test credentials */
export const TEST_USER = {
  username: "admin",
  password: "admin123",
};

/** Login and wait for dashboard to load */
export async function login(page: Page) {
  await page.goto("/login");
  await page.getByRole("textbox", { name: "Username or Email" }).fill(TEST_USER.username);
  await page.getByRole("textbox", { name: "Password" }).fill(TEST_USER.password);
  await page.getByRole("button", { name: "Sign in" }).click();

  // Wait for redirect away from login
  await expect(page).not.toHaveURL(/\/login/, { timeout: 10_000 });
}

/** Login via API and return authenticated request context */
export async function apiLogin(request: APIRequestContext) {
  const res = await request.post("/api/auth/login", {
    data: { username: TEST_USER.username, password: TEST_USER.password },
  });
  expect(res.ok()).toBeTruthy();
  return res;
}

/** Navigate to a known project's task board */
export async function navigateToFirstProjectTasks(page: Page) {
  // Authenticate via API, then navigate directly to project tasks
  await page.request.post("/api/auth/login", {
    data: { username: TEST_USER.username, password: TEST_USER.password },
  });

  const projectsRes = await page.request.get("/api/projects");
  const projects = (await projectsRes.json()).data;
  if (!projects || projects.length === 0) {
    throw new Error("No projects found in seed data");
  }
  const projectId = projects[0].id;
  await page.goto(`/projects/${projectId}/tasks`);
  await page.waitForLoadState("domcontentloaded");
  return projectId;
}

/** Prefix for all test-created data — makes cleanup easy */
export const TEST_DATA_PREFIX = "[E2E]";

// ─── View-As Helpers ────────────────────────────────────────────────────────

/** Ensure page is on the app (not about:blank) so localStorage is accessible */
async function ensureOnApp(page: Page) {
  if (page.url() === "about:blank") {
    await page.goto("/");
    await page.waitForLoadState("domcontentloaded");
  }
}

/** Wait for the app to finish rendering after navigation/reload */
async function waitForAppReady(page: Page) {
  // Wait for network to settle first, then check for a known DOM landmark
  await page.waitForLoadState("networkidle");
}

/** Set a view-as role override via localStorage and reload */
export async function setViewAsRole(page: Page, role: string) {
  await ensureOnApp(page);
  await page.evaluate((r) => localStorage.setItem("pcg:view-as-role", r), role);
  await page.reload({ waitUntil: "domcontentloaded" });
  await waitForAppReady(page);
}

/** Clear view-as override via localStorage and reload */
export async function clearViewAsRole(page: Page) {
  await ensureOnApp(page);
  await page.evaluate(() => localStorage.removeItem("pcg:view-as-role"));
  await page.reload({ waitUntil: "domcontentloaded" });
  await waitForAppReady(page);
}

/** Locate a settings scope tab button by exact label (scoped to settings aside) */
export function settingsTab(page: Page, label: string) {
  return page.locator("aside button", { hasText: new RegExp(`^${label}$`) });
}

/** Get the current view-as role from localStorage */
export async function getViewAsRole(page: Page): Promise<string | null> {
  return page.evaluate(() => localStorage.getItem("pcg:view-as-role"));
}

/** Wait for the settings page to be ready (scope tabs mounted in aside) */
export async function waitForSettingsReady(page: Page) {
  await expect(page.locator("aside button").first()).toBeVisible({ timeout: 10_000 });
}

// ─── Cleanup ────────────────────────────────────────────────────────────────

/** Clean up test data created during E2E runs */
export async function cleanupTestData(request: APIRequestContext) {
  await apiLogin(request);

  // Get all projects
  const projectsRes = await request.get("/api/projects");
  if (!projectsRes.ok()) return;
  const projects = (await projectsRes.json()).data || [];

  // For each project, delete tasks with E2E prefix
  for (const project of projects) {
    const tasksRes = await request.get(`/api/projects/${project.id}/tasks`);
    if (!tasksRes.ok()) continue;
    // Guard against HTML responses (SPA fallback)
    const contentType = tasksRes.headers()["content-type"] || "";
    if (!contentType.includes("application/json")) continue;
    let responseData;
    try { responseData = await tasksRes.json(); } catch { continue; }
    const tasks = responseData.data || responseData || [];
    if (!Array.isArray(tasks)) continue;

    for (const task of tasks) {
      if (task.title?.startsWith(TEST_DATA_PREFIX)) {
        await request.delete(`/api/tasks/${task.id}`).catch(() => {});
      }
    }
  }

  // Delete test projects
  for (const project of projects) {
    if (project.name?.startsWith(TEST_DATA_PREFIX)) {
      await request.delete(`/api/projects/${project.id}`).catch(() => {});
    }
  }
}
