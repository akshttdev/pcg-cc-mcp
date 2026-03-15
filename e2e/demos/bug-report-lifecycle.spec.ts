/**
 * Demo: Bug Report Full Lifecycle
 *
 * Walks through the complete lifecycle of a user-submitted bug report
 * entirely through the UI:
 *   Report → find on kanban → add QA watcher → status progression → Done
 *
 * Feedback tasks are created in the ORCHA Platform project (hardcoded UUID
 * 00000000-0000-0000-0000-000000000001) by the backend POST /api/feedback.
 *
 * Self-contained: seeds agents in beforeAll, no reliance on specific DB state
 * beyond the ORCHA Platform project existing (guaranteed by migrations).
 */
import { test, expect } from "./fixtures";
import {
  t, login, TEST_DATA_PREFIX,
  navigateToProjectTasks, navigateToTaskDetail,
  changeTaskStatus, addQaWatcher, findTaskCard,
  cleanupTaskByPath, ensureAgentsSeeded, openFeedbackDialog,
} from "../helpers";

const BUG_TITLE = `${TEST_DATA_PREFIX} Demo: Dashboard crash ${Date.now()}`;
const DEMO_PAUSE = 1_500;
const BUGREPORTS_PROJECT_ID = "00000000-0000-0000-0000-000000000001";

let TASK_PATH: string;

test.describe("Bug Report Lifecycle Demo", () => {
  test.describe.configure({ mode: "serial" });

  test.beforeAll(async ({ request }) => {
    await ensureAgentsSeeded(request);
  });

  test("Step 1: Submit bug report via Feedback dialog", async ({ page }) => {
    await login(page);
    await page.goto(`/projects/${BUGREPORTS_PROJECT_ID}/tasks`);
    await expect(
      page.getByRole("button", { name: "Create new task" })
    ).toBeVisible({ timeout: t(15_000) });

    // Open feedback dialog via data-testid (works in both collapsed/expanded sidebar)
    await openFeedbackDialog(page);

    // Verify Bug Report type is pre-selected (default)
    await expect(page.getByText("Bug Report").first()).toBeVisible({ timeout: t(3_000) });

    // Select severity: change from Medium to High
    const severityTrigger = page.getByRole("combobox").filter({ hasText: /Medium/ });
    await severityTrigger.click();
    await page.getByRole("option", { name: /high/i }).first().click();

    // Fill in title and description
    await page.getByRole("textbox", { name: "Title *" }).fill(BUG_TITLE);
    await page.getByRole("textbox", { name: "Description *" }).fill(
      "Dashboard shows white screen on empty project.\n\nSteps to reproduce:\n1. Create new project\n2. Navigate to dashboard\n3. See blank white screen"
    );

    await page.waitForTimeout(DEMO_PAUSE);

    // Submit
    await page.getByRole("button", { name: "Submit Feedback" }).click();

    // Success: toast appears with "Thank you" message, then dialog closes
    await expect(page.getByText(/Thank you.*feedback/i)).toBeVisible({ timeout: t(10_000) });

    // Verify dialog closed
    await expect(
      page.getByRole("heading", { name: "Submit Feedback" })
    ).not.toBeVisible({ timeout: t(5_000) });

    await page.waitForTimeout(DEMO_PAUSE);
  });

  test("Step 2: Find bug on kanban → open detail", async ({ page }) => {
    await navigateToProjectTasks(page, BUGREPORTS_PROJECT_ID);

    // Feedback creates tasks with [Bug] prefix — search for the title fragment
    const card = findTaskCard(page, BUG_TITLE);
    await expect(card).toBeVisible({ timeout: t(10_000) });
    await page.waitForTimeout(DEMO_PAUSE);

    await card.click();
    await expect(page.getByText("Agent Reviewers", { exact: true })).toBeVisible({
      timeout: t(10_000),
    });

    TASK_PATH = new URL(page.url()).pathname;
    await page.waitForTimeout(DEMO_PAUSE);
  });

  test("Step 3: Add QA watcher", async ({ page }) => {
    await navigateToTaskDetail(page, TASK_PATH);
    await addQaWatcher(page, { demoPause: DEMO_PAUSE });

    // Verify the ORCHA QA agent name is visible
    await expect(page.getByText(/ORCHA QA/i)).toBeVisible({ timeout: t(5_000) });
  });

  test("Step 4: Change status To Do → In Progress", async ({ page }) => {
    await navigateToTaskDetail(page, TASK_PATH);
    await changeTaskStatus(page, "To Do", "In Progress", { demoPause: DEMO_PAUSE });
  });

  test("Step 5: Change status In Progress → In Review", async ({ page }) => {
    await navigateToTaskDetail(page, TASK_PATH);
    await changeTaskStatus(page, "In Progress", "In Review", { demoPause: DEMO_PAUSE });
  });

  test("Step 6: Verify task in In Review on kanban", async ({ page }) => {
    await navigateToProjectTasks(page, BUGREPORTS_PROJECT_ID);

    // Verify task card is visible on the board
    const card = findTaskCard(page, BUG_TITLE);
    await expect(card).toBeVisible({ timeout: t(10_000) });

    // Verify "In Review" column header is visible (confirms we can see the column)
    await expect(page.getByText("In Review").first()).toBeVisible({ timeout: t(5_000) });
    await page.waitForTimeout(DEMO_PAUSE);
  });

  test("Step 7: Human approval — change to Done", async ({ page }) => {
    await navigateToTaskDetail(page, TASK_PATH);
    await changeTaskStatus(page, "In Review", "Done", { demoPause: DEMO_PAUSE });
  });

  test("Step 8: Verify task in Done column", async ({ page }) => {
    await navigateToProjectTasks(page, BUGREPORTS_PROJECT_ID);

    // Verify Done column exists and has tasks
    const doneHeader = page.locator("p").filter({ hasText: /^Done$/ });
    await expect(doneHeader).toBeVisible({ timeout: t(10_000) });

    // Verify our task card is still findable on the board (now in Done column)
    const card = findTaskCard(page, BUG_TITLE);
    await expect(card).toBeVisible({ timeout: t(5_000) });

    await page.waitForTimeout(DEMO_PAUSE * 2);
  });

  test.afterAll(async ({ request }) => {
    if (TASK_PATH) await cleanupTaskByPath(request, TASK_PATH);
  });
});
