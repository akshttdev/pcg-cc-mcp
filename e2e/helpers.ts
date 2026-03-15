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

/** Login and wait for dashboard to load. Skips if already authenticated. */
export async function login(page: Page) {
  // If we're already on an app page (not login, not about:blank), skip login
  const url = page.url();
  if (url && !url.includes("/login") && url !== "about:blank") {
    return;
  }

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

// ─── Demo Project Helpers ───────────────────────────────────────────────────

const DEMO_ORG_ID = "01010101-0101-0101-0101-010101010101"; // Powerclub Global

/**
 * Create a fresh, empty project via API for demo use.
 * Returns the project ID. The project belongs to the Powerclub Global org
 * so it appears in the sidebar.
 */
export async function createDemoProject(
  request: APIRequestContext,
  name?: string
): Promise<string> {
  await apiLogin(request);
  const projectName = name ?? `${TEST_DATA_PREFIX} Demo Project ${Date.now()}`;
  const uniquePath = `/tmp/e2e-demo-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
  const res = await request.post("/api/projects", {
    data: {
      name: projectName,
      git_repo_path: uniquePath,
      use_existing_repo: false,
      organization_id: DEMO_ORG_ID,
    },
  });
  expect(res.ok()).toBeTruthy();
  const body = await res.json();
  return body.data?.id ?? body.id;
}

// ─── Navigation Helpers ──────────────────────────────────────────────────────

/** Navigate to a project's kanban board and wait for it to load. */
export async function navigateToProjectTasks(page: Page, projectId: string) {
  await page.goto(`/projects/${projectId}/tasks`);
  await expect(
    page.getByRole("button", { name: "Create new task" })
  ).toBeVisible({ timeout: t(10_000) });
}

/** Navigate to task detail drawer. Skips if already showing the drawer. */
export async function navigateToTaskDetail(page: Page, taskPath: string) {
  const reviewers = page.getByText("Agent Reviewers", { exact: true });
  if (await reviewers.isVisible().catch(() => false)) return;

  await page.goto(taskPath);
  await expect(reviewers).toBeVisible({ timeout: t(10_000) });
}

/** Open the notification dropdown. Skips if already open. */
export async function openNotifications(page: Page) {
  const activity = page.getByText("Activity", { exact: true });
  if (await activity.isVisible().catch(() => false)) return;

  await page.getByRole("button", { name: /notification/i }).click();
  await expect(activity).toBeVisible({ timeout: t(5_000) });
}

// ─── UI Interaction Helpers ──────────────────────────────────────────────────

/**
 * Create a task via the UI. Assumes the kanban page is already loaded.
 * Returns after the task detail drawer opens (URL contains the new task ID).
 */
export async function createTaskViaUI(
  page: Page,
  title: string,
  opts?: { description?: string; demoPause?: number }
) {
  await page.getByRole("button", { name: "Create new task" }).click();
  await expect(page.getByRole("textbox", { name: "Title" })).toBeVisible({ timeout: t(5_000) });

  await page.getByRole("textbox", { name: "Title" }).fill(title);

  if (opts?.description) {
    const descArea = page.getByRole("textbox", { name: /Supports Markdown/i });
    await descArea.fill(opts.description);
  }

  if (opts?.demoPause) await page.waitForTimeout(opts.demoPause);

  await page.getByRole("button", { name: "Create Task", exact: true }).click();

  // After creation, drawer auto-opens with Agent Reviewers section
  await expect(page.getByText("Agent Reviewers", { exact: true })).toBeVisible({
    timeout: t(10_000),
  });
}

/**
 * Change task status via the combobox in the task detail drawer.
 * Expects the drawer to already be open.
 */
export async function changeTaskStatus(
  page: Page,
  from: string,
  to: string,
  opts?: { demoPause?: number }
) {
  const statusTrigger = page.getByRole("combobox").filter({ hasText: from });
  await expect(statusTrigger).toBeVisible({ timeout: t(10_000) });

  await statusTrigger.click();
  await page.getByRole("option", { name: to }).click();

  await expect(page.getByText(`Status changed to ${to}`)).toBeVisible({
    timeout: t(5_000),
  });

  if (opts?.demoPause) await page.waitForTimeout(opts.demoPause);
}

/** Add a QA watcher from the task detail drawer. */
export async function addQaWatcher(page: Page, opts?: { demoPause?: number }) {
  await page.getByRole("button", { name: "Add", exact: true }).click();
  await page.getByRole("button", { name: /ORCHA QA/i }).first().click();
  await expect(page.getByText("Watching")).toBeVisible({ timeout: t(5_000) });
  if (opts?.demoPause) await page.waitForTimeout(opts.demoPause);
}

/**
 * Find a task card on the kanban board by its title.
 * Strips the TEST_DATA_PREFIX and matches as a regex.
 */
export function findTaskCard(page: Page, title: string) {
  const titleFragment = title
    .replace(`${TEST_DATA_PREFIX} `, "")
    .replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return page.getByRole("button", { name: new RegExp(titleFragment) }).first();
}

// ─── Cleanup Helpers ─────────────────────────────────────────────────────────

/** Delete a project via API (for afterAll). */
export async function cleanupProject(request: APIRequestContext, projectId: string) {
  await apiLogin(request);
  await request.delete(`/api/projects/${projectId}`).catch(() => {});
}

/** Delete a task by extracting its ID from a task detail URL path. */
export async function cleanupTaskByPath(request: APIRequestContext, taskPath: string) {
  await apiLogin(request);
  const taskId = taskPath.split("/tasks/")[1];
  if (taskId) {
    await request.delete(`/api/tasks/${taskId}`).catch(() => {});
  }
}

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
