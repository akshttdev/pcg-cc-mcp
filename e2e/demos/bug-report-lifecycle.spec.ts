/**
 * Demo: Bug Report Full Lifecycle
 *
 * Walks through the complete lifecycle of a user-submitted bug report
 * entirely through the UI — no API calls:
 *   Report → find on kanban → add QA watcher → status progression → Done
 *
 * Feedback tasks are created in the Bug Reports project (hardcoded UUID
 * 00000000-0000-0000-0000-000000000001), not in the current project.
 *
 * Prerequisites:
 *   - Dev server running (FRONTEND_PORT, default 3001)
 *   - Seed database with Bug Reports project and at least one agent
 */
import { test, expect } from "./fixtures";
import {
  t, login, TEST_DATA_PREFIX,
  navigateToProjectTasks, navigateToTaskDetail,
  changeTaskStatus, addQaWatcher, findTaskCard, cleanupTaskByPath,
} from "../helpers";

const BUG_TITLE = `${TEST_DATA_PREFIX} Demo: Dashboard crash ${Date.now()}`;
const DEMO_PAUSE = 1_500;
const BUGREPORTS_PROJECT_ID = "00000000-0000-0000-0000-000000000001";

let TASK_PATH: string;

test.describe("Bug Report Lifecycle Demo", () => {
  test.describe.configure({ mode: "serial" });

  test("Step 1: Submit bug report via Feedback dialog", async ({ page }) => {
    await login(page);
    await page.goto(`/projects/${BUGREPORTS_PROJECT_ID}/tasks`);
    await expect(
      page.getByRole("button", { name: "Feedback & Support" })
    ).toBeVisible({ timeout: t(10_000) });

    await page.getByRole("button", { name: "Feedback & Support" }).click();
    await expect(page.getByRole("heading", { name: "Submit Feedback" })).toBeVisible({
      timeout: t(5_000),
    });

    await expect(page.getByText("Bug Report").first()).toBeVisible({ timeout: t(3_000) });

    // Select severity: change from Medium to High
    const severityTrigger = page.getByRole("combobox").filter({ hasText: /Medium/ });
    await severityTrigger.click();
    await page.getByRole("option", { name: /high/i }).first().click();

    await page.getByRole("textbox", { name: "Title *" }).fill(BUG_TITLE);
    await page.getByRole("textbox", { name: "Description *" }).fill(
      "Dashboard shows white screen on empty project.\n\nSteps to reproduce:\n1. Create new project\n2. Navigate to dashboard\n3. See blank white screen"
    );

    await page.waitForTimeout(DEMO_PAUSE);

    await page.getByRole("button", { name: "Submit Feedback" }).click();
    await expect(page.getByText("Thank You!")).toBeVisible({ timeout: t(10_000) });
    await page.waitForTimeout(DEMO_PAUSE);
  });

  test("Step 2: Find bug on kanban → open detail → add QA watcher", async ({ page }) => {
    await navigateToProjectTasks(page, BUGREPORTS_PROJECT_ID);

    const card = findTaskCard(page, BUG_TITLE);
    await expect(card).toBeVisible({ timeout: t(10_000) });
    await page.waitForTimeout(DEMO_PAUSE);

    await card.click();
    await expect(page.getByText("Agent Reviewers", { exact: true })).toBeVisible({
      timeout: t(10_000),
    });

    TASK_PATH = new URL(page.url()).pathname;
    await page.waitForTimeout(DEMO_PAUSE);

    await addQaWatcher(page, { demoPause: DEMO_PAUSE });
  });

  test("Step 3: Change status To Do → In Progress", async ({ page }) => {
    await navigateToTaskDetail(page, TASK_PATH);
    await changeTaskStatus(page, "To Do", "In Progress", { demoPause: DEMO_PAUSE });
  });

  test("Step 4: Change status In Progress → In Review", async ({ page }) => {
    await navigateToTaskDetail(page, TASK_PATH);
    await changeTaskStatus(page, "In Progress", "In Review", { demoPause: DEMO_PAUSE });
  });

  test("Step 5: Verify task in In Review on kanban", async ({ page }) => {
    await navigateToProjectTasks(page, BUGREPORTS_PROJECT_ID);
    await expect(page.getByText("In Review").first()).toBeVisible({ timeout: t(10_000) });
    await expect(findTaskCard(page, BUG_TITLE)).toBeVisible({ timeout: t(5_000) });
    await page.waitForTimeout(DEMO_PAUSE);
  });

  test("Step 6: Human approval — change to Done", async ({ page }) => {
    await navigateToTaskDetail(page, TASK_PATH);
    await changeTaskStatus(page, "In Review", "Done", { demoPause: DEMO_PAUSE });
  });

  test("Step 7: Verify task in Done column", async ({ page }) => {
    await page.goto(`/projects/${BUGREPORTS_PROJECT_ID}/tasks`);

    const doneHeader = page.getByText("Done").first();
    await expect(doneHeader).toBeVisible({ timeout: t(10_000) });

    const doneCount = doneHeader.locator("..").getByText(/\d+/);
    await expect(doneCount).toBeVisible();

    await page.waitForTimeout(DEMO_PAUSE * 2);
  });

  test.afterAll(async ({ request }) => {
    if (TASK_PATH) await cleanupTaskByPath(request, TASK_PATH);
  });
});
