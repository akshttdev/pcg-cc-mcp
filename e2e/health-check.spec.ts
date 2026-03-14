import { test, expect } from "./fixtures";
import { login, apiLogin, navigateToFirstProjectTasks, cleanupTestData, TEST_DATA_PREFIX, TEST_USER } from "./helpers";

// Most tests use storageState from auth.setup.ts — no per-test login needed.
// Only the Authentication describe block tests login/logout flows directly.

/**
 * ORCHA Dashboard — E2E Health Check Suite
 *
 * Covers core workflows at integration level:
 *   1. Authentication (login/logout)
 *   2. Sidebar & navigation
 *   3. Project lifecycle (list, create, view)
 *   4. Task lifecycle (create, view on kanban, detail panel)
 *   5. Organization pages
 *   6. Settings pages
 *   7. Workflows, Mission Control, My Tasks
 *   8. API health (UUID format verification)
 *
 * All test data is prefixed with "[E2E]" and cleaned up after the suite runs.
 */

// Clean up any leftover test data before and after the suite
test.beforeAll(async ({ request }) => {
  await cleanupTestData(request);
});

test.afterAll(async ({ request }) => {
  await cleanupTestData(request);
});

// ─── Auth ────────────────────────────────────────────────────────────────────

test.describe("Authentication", () => {
  // These tests exercise the login flow itself — clear auth state before each
  test.beforeEach(async ({ page }) => {
    await page.context().clearCookies();
    await page.evaluate(() => localStorage.clear()).catch(() => {});
  });

  test("login with valid credentials redirects to dashboard", async ({ page }) => {
    await login(page);
    await expect(page.getByText("My Workspace")).toBeVisible({ timeout: 10_000 });
  });

  test("login with invalid credentials shows error", async ({ page }) => {
    await page.goto("/login");
    await page.getByRole("textbox", { name: "Username or Email" }).fill("bad_user");
    await page.getByRole("textbox", { name: "Password" }).fill("bad_pass");
    await page.getByRole("button", { name: "Sign in" }).click();

    await expect(page.getByRole("alert")).toBeVisible({ timeout: 5_000 });
  });

  test("unauthenticated access redirects to login", async ({ page }) => {
    await page.goto("/projects");
    await expect(page).toHaveURL(/\/login/);
  });

  // Re-login after auth tests so the shared context has valid cookies
  test("restore auth state", async ({ page }) => {
    await login(page);
  });
});

// ─── Sidebar & Navigation ────────────────────────────────────────────────────

test.describe("Sidebar & Navigation", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("domcontentloaded");
  });

  test("sidebar shows workspace sections", async ({ page }) => {
    await expect(page.getByText("My Workspace")).toBeVisible();
    await expect(page.getByRole("link", { name: "My Projects" })).toBeVisible();
    await expect(page.getByRole("link", { name: "My Tasks" })).toBeVisible();
    await expect(page.getByRole("link", { name: "My Workflows" })).toBeVisible();
  });

  test("sidebar shows organizations label", async ({ page }) => {
    await expect(page.getByText("Organizations", { exact: true })).toBeVisible();
  });

  test("My Projects link navigates to projects page", async ({ page }) => {
    await page.getByRole("link", { name: "My Projects" }).click();
    await expect(page).toHaveURL("/projects");
    await expect(page.getByRole("heading", { name: "Projects", level: 1 })).toBeVisible();
  });

  test("My Tasks link navigates to tasks page", async ({ page }) => {
    await page.getByRole("link", { name: "My Tasks" }).click();
    await expect(page).toHaveURL("/my-tasks");
  });

  test("Settings link navigates to settings", async ({ page }) => {
    await page.getByRole("link", { name: "Settings" }).click();
    await expect(page).toHaveURL(/\/settings/);
  });
});

// ─── Projects ────────────────────────────────────────────────────────────────

test.describe("Projects", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/projects");
    await page.waitForLoadState("domcontentloaded");
  });

  test("projects list page renders with create button", async ({ page }) => {
    await expect(page.getByRole("heading", { name: "Projects" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Create Project" })).toBeVisible();
  });

  test("project cards are visible from seed data", async ({ page }) => {
    // Seed data has projects — at least one "Active" badge should appear
    await expect(page.getByText("Active").first()).toBeVisible({ timeout: 10_000 });
  });

  test("create project dialog opens", async ({ page }) => {
    await page.getByRole("button", { name: "Create Project" }).click();
    await expect(page.getByRole("heading", { name: /Create.*Project/i })).toBeVisible({ timeout: 5_000 });
  });

  test("clicking a project navigates to project detail", async ({ page }) => {
    // Project cards are clickable divs, not anchor links — click the first card heading
    const projectCard = page.getByText("Active").first();
    await expect(projectCard).toBeVisible({ timeout: 10_000 });
    await projectCard.click();
    await expect(page).toHaveURL(/\/projects\/[0-9a-f-]{36}/, { timeout: 10_000 });
  });
});

// ─── Tasks & Kanban ──────────────────────────────────────────────────────────

test.describe("Tasks & Kanban", () => {

  test("kanban board renders with status columns", async ({ page }) => {
    await navigateToFirstProjectTasks(page);

    // Kanban columns should be visible
    await expect(page.getByText("To Do").first()).toBeVisible({ timeout: 10_000 });
    await expect(page.getByText("In Progress").first()).toBeVisible();
    await expect(page.getByText("Done").first()).toBeVisible();
  });

  test("create task dialog has all core fields", async ({ page }) => {
    await navigateToFirstProjectTasks(page);

    await page.getByRole("button", { name: "Create new task" }).click();
    await expect(page.getByRole("heading", { name: "Create New Task" })).toBeVisible({ timeout: 5_000 });

    // Core fields
    await expect(page.getByRole("textbox", { name: "Title" })).toBeVisible();
    await expect(page.getByText("Priority").first()).toBeVisible();
    await expect(page.getByText("Completion Criteria")).toBeVisible();
    await expect(page.getByText("Output Format")).toBeVisible();
  });

  test("create task and verify it appears on kanban", async ({ page }) => {
    await navigateToFirstProjectTasks(page);

    await page.getByRole("button", { name: "Create new task" }).click();
    await expect(page.getByRole("heading", { name: "Create New Task" })).toBeVisible({ timeout: 5_000 });

    const taskTitle = `${TEST_DATA_PREFIX} Kanban Test ${Date.now()}`;
    await page.getByRole("textbox", { name: "Title" }).fill(taskTitle);
    await page.getByRole("button", { name: "Create Task" }).click();

    // Task title should appear somewhere on the page (kanban or detail panel)
    await expect(page.getByText(taskTitle).first()).toBeVisible({ timeout: 10_000 });
  });

  test("task detail panel shows overview tab", async ({ page }) => {
    await navigateToFirstProjectTasks(page);

    // Find and click a task card (button with heading level 4 inside)
    const taskButton = page.locator("h4").first();
    if (await taskButton.isVisible({ timeout: 5_000 }).catch(() => false)) {
      await taskButton.click();

      // Detail panel should show tabs
      await expect(page.getByRole("tab", { name: "Overview" })).toBeVisible({ timeout: 5_000 });
      await expect(page.getByRole("tab", { name: "Artifacts" })).toBeVisible();
    }
  });
});

// ─── Organizations ───────────────────────────────────────────────────────────

test.describe("Organizations", () => {
  test("organization profile page loads", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("domcontentloaded");
    // Navigate to first org from sidebar link
    const orgLink = page.locator("a[href*='/organizations/']").first();
    await expect(orgLink).toBeVisible({ timeout: 10_000 });
    await orgLink.click();
    await expect(page).toHaveURL(/\/organizations\/[0-9a-f-]{36}/);
  });
});

// ─── Settings ────────────────────────────────────────────────────────────────

test.describe("Settings", () => {
  test("settings page loads", async ({ page }) => {
    await page.goto("/settings");
    await expect(page).toHaveURL(/\/settings/);
  });

  test("general settings page loads", async ({ page }) => {
    await page.goto("/settings/general");
    await page.waitForLoadState("domcontentloaded");
    await expect(page.locator("body")).not.toBeEmpty();
  });

  test("agents settings page loads", async ({ page }) => {
    await page.goto("/settings/agents");
    await page.waitForLoadState("domcontentloaded");
    await expect(page.locator("body")).not.toBeEmpty();
  });

  test("MCP settings page loads", async ({ page }) => {
    await page.goto("/settings/mcp");
    await page.waitForLoadState("domcontentloaded");
    await expect(page.locator("body")).not.toBeEmpty();
  });

  test("Topsi admin settings page loads", async ({ page }) => {
    await page.goto("/settings/topsi");
    await page.waitForLoadState("domcontentloaded");
    // Wait for loading spinner to disappear (API fetch for admin prompt)
    await expect(page.locator('[class*="animate-spin"]')).not.toBeVisible({ timeout: 20_000 }).catch(() => {});
    await expect(page.getByText("Topsi Configuration")).toBeVisible({ timeout: 20_000 });
    await expect(page.getByText("System Prompt")).toBeVisible({ timeout: 10_000 });
  });

  test("Topsi user preferences page loads", async ({ page }) => {
    await page.goto("/settings/topsi-preferences");
    await page.waitForLoadState("domcontentloaded");
    // Wait for loading spinner to disappear (API fetch for user preferences)
    await expect(page.locator('[class*="animate-spin"]')).not.toBeVisible({ timeout: 20_000 }).catch(() => {});
    await expect(page.getByText("Topsi Preferences")).toBeVisible({ timeout: 20_000 });
    await expect(page.getByText("Confirmation Mode")).toBeVisible({ timeout: 10_000 });
  });
});

// ─── Workflows ───────────────────────────────────────────────────────────────

test.describe("Workflows", () => {
  test("workflows page loads", async ({ page }) => {
    await page.goto("/workflows");
    await page.waitForLoadState("domcontentloaded");
    await expect(page.locator("body")).not.toBeEmpty();
  });
});

// ─── Topsi UI (Phase 5) ─────────────────────────────────────────────────────

test.describe("Topsi UI", () => {
  test("Topsi Activity page loads for admin", async ({ page }) => {
    await page.goto("/topsi-activity");
    await page.waitForLoadState("domcontentloaded");
    await expect(page.getByText("Topsi Activity")).toBeVisible({ timeout: 10_000 });
  });

  test("sidebar shows Topsi Activity link for admin", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle");
    // Expand the Admin Platforms collapsible section first
    const adminTrigger = page.getByText("Admin Platforms", { exact: true });
    await expect(adminTrigger).toBeVisible({ timeout: 10_000 });
    await adminTrigger.click();
    await expect(page.getByRole("link", { name: "Topsi Activity" })).toBeVisible({ timeout: 10_000 });
  });

  test("AskTopsiButton visible on project tasks page", async ({ page }) => {
    await navigateToFirstProjectTasks(page);
    await expect(page.getByRole("button", { name: "Ask Topsi" })).toBeVisible({ timeout: 10_000 });
  });

  test("AskTopsiButton opens popover with suggested questions", async ({ page }) => {
    await navigateToFirstProjectTasks(page);
    await page.getByRole("button", { name: "Ask Topsi" }).click();
    await expect(page.getByText("Ask Topsi about this project")).toBeVisible({ timeout: 5_000 });
    // Verify at least one suggestion is rendered
    await expect(page.getByText("What tasks are blocked?")).toBeVisible();
  });

  test("Topsi admin prompt API returns data", async ({ request }) => {
    await apiLogin(request);
    const res = await request.get("/api/topsi/admin/prompt");
    expect(res.ok()).toBeTruthy();
    const data = await res.json();
    // mode should be a string (standard or sudolang)
    expect(typeof data.mode).toBe("string");
  });

  test("Topsi user settings API returns defaults", async ({ request }) => {
    await apiLogin(request);
    const res = await request.get("/api/topsi/user-settings");
    expect(res.ok()).toBeTruthy();
    const data = await res.json();
    expect(data.default_confirmation_mode).toBeTruthy();
  });

  test("Topsi tools API returns risk-grouped tools", async ({ request }) => {
    await apiLogin(request);
    const res = await request.get("/api/topsi/tools");
    expect(res.ok()).toBeTruthy();
    const data = await res.json();
    // Should have red, yellow, green groups with tool arrays
    expect(Array.isArray(data.red.tools)).toBe(true);
    expect(Array.isArray(data.yellow.tools)).toBe(true);
    expect(Array.isArray(data.green.tools)).toBe(true);
    // Known red tools
    expect(data.red.tools).toContain("delete_task");
    // Known yellow tools
    expect(data.yellow.tools).toContain("create_task");
    // Green should have read-only tools
    expect(data.green.tools.length).toBeGreaterThan(0);
  });

  test("Topsi user settings API accepts PUT", async ({ request }) => {
    await apiLogin(request);
    const res = await request.put("/api/topsi/user-settings", {
      data: {
        default_confirmation_mode: "confirm_destructive",
        auto_approve_timeout_minutes: 5,
      },
    });
    expect(res.ok()).toBeTruthy();

    // Verify the update persisted
    const getRes = await request.get("/api/topsi/user-settings");
    expect(getRes.ok()).toBeTruthy();
    const data = await getRes.json();
    expect(data.default_confirmation_mode).toBe("confirm_destructive");
    expect(data.auto_approve_timeout_minutes).toBe(5);
  });
});

// ─── Mission Control ─────────────────────────────────────────────────────────

test.describe("Mission Control", () => {
  test("mission control page loads", async ({ page }) => {
    await page.goto("/mission-control");
    await page.waitForLoadState("domcontentloaded");

    await expect(page.getByRole("heading", { name: "Mission Control" })).toBeVisible({ timeout: 10_000 });
  });
});

// ─── My Tasks ────────────────────────────────────────────────────────────────

test.describe("My Tasks", () => {
  test("my tasks page loads", async ({ page }) => {
    await page.goto("/my-tasks");
    await page.waitForLoadState("domcontentloaded");
    await expect(page.locator("body")).not.toBeEmpty();
  });
});

// ─── API Health ──────────────────────────────────────────────────────────────

test.describe("API Health", () => {
  test("login API returns session", async ({ request }) => {
    const res = await request.post("/api/auth/login", {
      data: { username: TEST_USER.username, password: TEST_USER.password },
    });
    expect(res.ok()).toBeTruthy();
    const data = await res.json();
    expect(data.success).toBe(true);
    expect(data.data.session_id).toBeTruthy();
  });

  test("projects API returns TEXT UUIDs", async ({ request }) => {
    await apiLogin(request);

    const res = await request.get("/api/projects");
    expect(res.ok()).toBeTruthy();
    const data = await res.json();
    expect(data.success).toBe(true);
    expect(Array.isArray(data.data)).toBe(true);

    if (data.data.length > 0) {
      expect(data.data[0].id).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/);
    }
  });

  test("organizations API returns data with TEXT UUIDs", async ({ request }) => {
    await apiLogin(request);

    const res = await request.get("/api/organizations");
    expect(res.ok()).toBeTruthy();
    const data = await res.json();
    expect(data.success).toBe(true);
    expect(data.data.length).toBeGreaterThan(0);
    expect(data.data[0].id).toMatch(/^[0-9a-f]{8}-/);
  });

  test("task CRUD works end-to-end", async ({ request }) => {
    await apiLogin(request);

    // Get a project
    const projects = (await (await request.get("/api/projects")).json()).data;
    expect(projects.length).toBeGreaterThan(0);
    const projectId = projects[0].id;

    // Create
    const taskTitle = `${TEST_DATA_PREFIX} API CRUD ${Date.now()}`;
    const createRes = await request.post("/api/tasks", {
      data: {
        project_id: projectId,
        title: taskTitle,
        status: "todo",
        priority: "low",
        created_by: "admin",
      },
    });
    expect(createRes.ok()).toBeTruthy();
    const task = (await createRes.json()).data;
    expect(task.id).toMatch(/^[0-9a-f]{8}-/);
    expect(task.project_id).toBe(projectId);

    // Read
    const getRes = await request.get(`/api/tasks/${task.id}`);
    expect(getRes.ok()).toBeTruthy();
    const fetched = (await getRes.json()).data;
    expect(fetched.title).toBe(taskTitle);

    // Update
    const updateRes = await request.put(`/api/tasks/${task.id}`, {
      data: { status: "inprogress" },
    });
    expect(updateRes.ok()).toBeTruthy();

    // Delete (cleanup)
    const deleteRes = await request.delete(`/api/tasks/${task.id}`);
    expect(deleteRes.ok()).toBeTruthy();
  });
});
