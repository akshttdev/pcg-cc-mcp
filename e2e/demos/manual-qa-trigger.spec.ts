/**
 * Demo: Manual QA Trigger from Task Detail
 *
 * Demonstrates the QA watcher flow when a user manually moves a task
 * to "In Review" status:
 *   Create task → add QA watcher → To Do → In Progress → In Review → Done
 *
 * Prerequisites:
 *   - Dev server running (FRONTEND_PORT, default 3001)
 *   - Seed database with ORCHA Platform project and QA agent
 */
import { test, expect } from "./fixtures";
import {
  t, login, createDemoProject, TEST_DATA_PREFIX,
  navigateToProjectTasks, navigateToTaskDetail, createTaskViaUI,
  changeTaskStatus, addQaWatcher, findTaskCard, cleanupProject,
} from "../helpers";

const TASK_TITLE = `${TEST_DATA_PREFIX} Manual QA Trigger ${Date.now()}`;

let PROJECT_ID: string;
let TASK_PATH: string;

const DEMO_PAUSE = 1_500;

test.describe("Manual QA Trigger Demo", () => {
  test.describe.configure({ mode: "serial" });

  test("Step 1: Create a task via the UI", async ({ page }) => {
    await login(page);
    PROJECT_ID = await createDemoProject(page.request, `${TEST_DATA_PREFIX} QA Trigger Demo`);
    await navigateToProjectTasks(page, PROJECT_ID);

    await createTaskViaUI(page, TASK_TITLE, {
      description: "Test task for manual QA trigger verification",
      demoPause: DEMO_PAUSE,
    });

    TASK_PATH = new URL(page.url()).pathname;
    await page.waitForTimeout(DEMO_PAUSE);
  });

  test("Step 2: Add QA watcher from task detail drawer", async ({ page }) => {
    await navigateToTaskDetail(page, TASK_PATH);
    await addQaWatcher(page, { demoPause: DEMO_PAUSE });
  });

  test("Step 3: Change status To Do → In Progress", async ({ page }) => {
    await navigateToTaskDetail(page, TASK_PATH);
    await changeTaskStatus(page, "To Do", "In Progress", { demoPause: DEMO_PAUSE });
  });

  test("Step 4: Change status In Progress → In Review (triggers watcher check)", async ({
    page,
  }) => {
    await navigateToTaskDetail(page, TASK_PATH);
    await changeTaskStatus(page, "In Progress", "In Review", { demoPause: DEMO_PAUSE });
  });

  test("Step 5: Verify watcher stayed in Watching (no PR)", async ({ page }) => {
    await navigateToTaskDetail(page, TASK_PATH);
    await expect(page.getByText("Watching")).toBeVisible({ timeout: t(5_000) });
    await page.waitForTimeout(DEMO_PAUSE);
  });

  test("Step 6: Verify task in In Review on kanban", async ({ page }) => {
    await navigateToProjectTasks(page, PROJECT_ID);
    await expect(page.getByText("In Review").first()).toBeVisible({ timeout: t(10_000) });
    await expect(findTaskCard(page, TASK_TITLE)).toBeVisible({ timeout: t(5_000) });
    await page.waitForTimeout(DEMO_PAUSE);
  });

  test("Step 7: Human approval — change to Done", async ({ page }) => {
    await navigateToTaskDetail(page, TASK_PATH);
    await changeTaskStatus(page, "In Review", "Done", { demoPause: DEMO_PAUSE * 2 });
  });

  test.afterAll(async ({ request }) => {
    if (PROJECT_ID) await cleanupProject(request, PROJECT_ID);
  });
});
