import { Page, expect } from "@playwright/test";
import { t } from "./timing";
import { TEST_USER } from "./auth";

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
  // Check if notification panel is already open by looking for the Notifications heading
  const heading = page.getByText("Notifications").first();
  const menu = page.locator('[role="menu"]').filter({ hasText: "Notifications" });
  if (await menu.isVisible().catch(() => false)) return;

  // Click the notification bell button
  await page.getByRole("button", { name: /notification/i }).click();

  // Wait for the dropdown menu to appear with the Activity tab
  // Use role-based selector since the tab text includes the count badge
  await expect(
    page.getByRole("button", { name: /^Activity/i })
  ).toBeVisible({ timeout: t(5_000) });
}
