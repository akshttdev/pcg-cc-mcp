import { Page, APIRequestContext, expect } from "@playwright/test";

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

/** Set a view-as role override via localStorage and reload */
export async function setViewAsRole(page: Page, role: string) {
  await page.evaluate((r) => localStorage.setItem("pcg:view-as-role", r), role);
  await page.reload({ waitUntil: "domcontentloaded" });
  await page.waitForTimeout(2500);
}

/** Clear view-as override via localStorage and reload */
export async function clearViewAsRole(page: Page) {
  await page.evaluate(() => localStorage.removeItem("pcg:view-as-role"));
  await page.reload({ waitUntil: "domcontentloaded" });
  await page.waitForTimeout(2500);
}

/** Locate a settings scope tab button by exact label (scoped to settings aside) */
export function settingsTab(page: Page, label: string) {
  return page.locator("aside button", { hasText: new RegExp(`^${label}$`) });
}

/** Get the current view-as role from localStorage */
export async function getViewAsRole(page: Page): Promise<string | null> {
  return page.evaluate(() => localStorage.getItem("pcg:view-as-role"));
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
