/**
 * Demo: Notification Center
 *
 * Exercises the in-app notification system:
 *   - Bell icon with unread badge
 *   - Dropdown with activity and inbox notifications
 *   - Mark all read
 *   - Click-to-navigate (deep linking)
 *
 * API calls are used only to seed prerequisite state (project creation).
 * All feature interactions go through the UI.
 *
 * Prerequisites:
 *   - Dev server running on FRONTEND_PORT (default 3001)
 *   - Seed database with Powerclub Global organization
 */
import { test, expect } from "./fixtures";
import {
  t, login, createDemoProject, TEST_DATA_PREFIX,
  navigateToProjectTasks, createTaskViaUI, openNotifications, cleanupProject,
} from "../helpers";

let PROJECT_ID: string;

const TASK_TITLE_1 = `${TEST_DATA_PREFIX} Notif Demo Task A ${Date.now()}`;
const TASK_TITLE_2 = `${TEST_DATA_PREFIX} Notif Demo Task B ${Date.now()}`;

/** Create a task via the UI, then navigate back to the kanban. */
async function createTaskAndReturnToKanban(
  page: import("@playwright/test").Page,
  title: string
) {
  await createTaskViaUI(page, title);

  // After creation, drawer auto-opens — navigate back to kanban view
  await page.waitForTimeout(2_000);
  await navigateToProjectTasks(page, PROJECT_ID);

  const shortTitle = title.replace(`${TEST_DATA_PREFIX} `, "");
  await expect(page.getByText(shortTitle).first()).toBeVisible({ timeout: t(10_000) });
}

test.describe("Notification Center Demo", () => {
  test.describe.configure({ mode: "serial" });

  test("Step 1: Create a task via UI to generate activity", async ({ page }) => {
    await login(page);
    PROJECT_ID = await createDemoProject(page.request, `${TEST_DATA_PREFIX} Notification Demo`);
    await navigateToProjectTasks(page, PROJECT_ID);
    await createTaskAndReturnToKanban(page, TASK_TITLE_1);
  });

  test("Step 2: Notification bell is visible with unread indicator", async ({ page }) => {
    await navigateToProjectTasks(page, PROJECT_ID);
    const bellButton = page.getByRole("button", { name: /notification/i });
    await expect(bellButton).toBeVisible({ timeout: t(5_000) });

    // Verify unread badge (blue dot) appears after task creation
    const unreadDot = bellButton.locator("span.rounded-full");
    await expect(unreadDot).toBeVisible({ timeout: t(5_000) });
  });

  test("Step 3: Open notification dropdown", async ({ page }) => {
    await openNotifications(page);

    // Verify the notification panel is open by checking for the heading
    await expect(page.getByText("Notifications").first()).toBeVisible({
      timeout: t(3_000),
    });
  });

  test("Step 4: Verify activity items contain task title", async ({ page }) => {
    await openNotifications(page);

    const timestamps = page.getByText(/(just now|\d+m ago|\d+h ago|\d+d ago)/);
    await expect(timestamps.first()).toBeVisible({ timeout: t(5_000) });

    // Notification text should contain the task title (formatAction includes it)
    const shortTitle = TASK_TITLE_1.replace(`${TEST_DATA_PREFIX} `, "");
    await expect(
      page.locator(".max-h-80").getByText(new RegExp(shortTitle)).first()
    ).toBeVisible({ timeout: t(5_000) });
  });

  test("Step 5: Mark all read and verify badge disappears", async ({ page }) => {
    await openNotifications(page);

    const markAllBtn = page.getByRole("button", { name: /mark all read/i });
    await expect(markAllBtn).toBeVisible({ timeout: t(5_000) });
    await markAllBtn.click();

    await page.waitForTimeout(1_000);

    // Close dropdown, then verify unread badge is gone
    await page.keyboard.press("Escape");
    await page.waitForTimeout(500);
    const bellButton = page.getByRole("button", { name: /notification/i });
    const unreadDot = bellButton.locator("span.rounded-full");
    await expect(unreadDot).not.toBeVisible({ timeout: t(5_000) });
  });

  test("Step 6: Create another task via UI to generate fresh notification", async ({ page }) => {
    await navigateToProjectTasks(page, PROJECT_ID);
    await createTaskAndReturnToKanban(page, TASK_TITLE_2);
  });

  test("Step 7: Click notification navigates to task", async ({ page }) => {
    await openNotifications(page);

    const firstItem = page.locator(".max-h-80 .cursor-pointer").first();
    await expect(firstItem).toBeVisible({ timeout: t(5_000) });
    await firstItem.click();

    await expect(page).not.toHaveURL(/about:blank/, { timeout: t(5_000) });
    const url = page.url();
    expect(
      url.includes("/tasks/") || url.includes("/my-tasks") || url.includes("/projects/")
    ).toBeTruthy();
  });

  test.afterAll(async ({ request }) => {
    if (PROJECT_ID) await cleanupProject(request, PROJECT_ID);
  });
});
