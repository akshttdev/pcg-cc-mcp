/**
 * Demo: Manual QA Trigger from Task Detail (with PR)
 *
 * Demonstrates the QA watcher flow when a user manually moves a task
 * to "In Review" status with a PR linked:
 *   Create task → add QA watcher → link PR → In Progress → In Review →
 *   watcher triggered → QA verdict → Human approval → Done
 *
 * The PR is created in the sandbox repo before the status change,
 * so when the task moves to In Review, spawn_watcher_reviews finds
 * the PR and triggers the QA watcher.
 *
 * Requires: GITHUB_TOKEN env var for sandbox repo operations.
 *
 * Self-contained: creates its own project and seeds agents in beforeAll.
 * No reliance on specific seed DB state beyond Powerclub Global org existing.
 */
import { test, expect } from "./fixtures";
import {
  t, login, createDemoProject, TEST_DATA_PREFIX, apiLogin,
  navigateToProjectTasks, navigateToTaskDetail, createTaskViaUI,
  changeTaskStatus, addQaWatcher, findTaskCard, cleanupProject,
  ensureAgentsSeeded,
} from "../helpers";
import {
  createPrForTask, simulateQaVerdict,
  cleanupDemoBranches, cleanupDemoPr,
} from "../helpers/demo";
import { waitForToast } from "../helpers/demo/assertions";

const TASK_TITLE = `${TEST_DATA_PREFIX} Manual QA Trigger ${Date.now()}`;
const QA_AGENT_ID = "a0000000-0000-0000-0000-000000000002";

let PROJECT_ID: string;
let TASK_PATH: string;
let TASK_ID: string;
let DEMO_BRANCH: string | undefined;
let DEMO_PR_NUMBER: number | undefined;

const DEMO_PAUSE = 1_500;

test.describe("Manual QA Trigger Demo", () => {
  test.describe.configure({ mode: "serial" });

  test.beforeAll(async ({ request }) => {
    await ensureAgentsSeeded(request);
  });

  test("Step 1: Create a task via the UI", async ({ page }) => {
    await login(page);
    PROJECT_ID = await createDemoProject(page.request, `${TEST_DATA_PREFIX} QA Trigger Demo`);
    await navigateToProjectTasks(page, PROJECT_ID);

    await createTaskViaUI(page, TASK_TITLE, {
      description: "Test task for manual QA trigger verification",
      demoPause: DEMO_PAUSE,
    });

    TASK_PATH = new URL(page.url()).pathname;
    TASK_ID = TASK_PATH.split("/tasks/")[1];
    await page.waitForTimeout(DEMO_PAUSE);
  });

  test("Step 2: Add QA watcher from task detail drawer", async ({ page }) => {
    await navigateToTaskDetail(page, TASK_PATH);
    await addQaWatcher(page, { demoPause: DEMO_PAUSE });

    // Verify the ORCHA QA agent name is visible alongside "Watching" status
    await expect(page.getByText(/ORCHA QA/i)).toBeVisible({ timeout: t(5_000) });
    await expect(page.getByText("Watching")).toBeVisible({ timeout: t(5_000) });
  });

  test("Step 3: Link a PR to the task (simulate prior dev work)", async ({ page, request }) => {
    await apiLogin(request);

    // Create a real PR in the sandbox repo and link it to the task attempt
    const result = await createPrForTask(request, TASK_ID);
    DEMO_BRANCH = result.branch;
    DEMO_PR_NUMBER = result.prNumber;

    await page.waitForTimeout(DEMO_PAUSE);
  });

  test("Step 4: Change status To Do → In Progress", async ({ page }) => {
    await navigateToTaskDetail(page, TASK_PATH);
    await changeTaskStatus(page, "To Do", "In Progress", { demoPause: DEMO_PAUSE });
  });

  test("Step 5: Change status In Progress → In Review (triggers watcher)", async ({ page }) => {
    await navigateToTaskDetail(page, TASK_PATH);
    await changeTaskStatus(page, "In Progress", "In Review", { demoPause: DEMO_PAUSE });

    // With a PR linked, the server's spawn_watcher_reviews should trigger.
    // Wait for the watcher to change from "Watching" to "Triggered"
    // Note: If the real QA agent executor is not available, the watcher may
    // stay in triggered state. We'll simulate the verdict in the next step.
    await page.waitForTimeout(DEMO_PAUSE * 2);
  });

  test("Step 6: QA watcher verdict — simulate QA pass", async ({ page, request }) => {
    await navigateToTaskDetail(page, TASK_PATH);
    await apiLogin(request);

    // Wait for watcher status badge to render (from AgentWatcherPanel API call).
    // UI renders: watching → "Watching", triggered → "Running"
    await expect(
      page.getByText("Watching").or(page.getByText("Running"))
    ).toBeVisible({ timeout: t(10_000) });

    // Allow WebSocket to connect and deliver initial snapshot, so the diff hook
    // has prev state before we trigger changes.
    await page.waitForTimeout(2_000);

    const runningVisible = await page.getByText("Running").isVisible().catch(() => false);
    if (!runningVisible) {
      // Watcher still in "Watching" state — trigger it manually
      await request.patch(`/api/tasks/${TASK_ID}/collaborators`, {
        data: { actor_id: QA_AGENT_ID, actor_type: "agent_watcher", action: "triggered" },
      });
      await page.waitForTimeout(DEMO_PAUSE);
    }

    // Simulate QA verdict: PASS
    await simulateQaVerdict(request, TASK_ID, QA_AGENT_ID, "qa_pass");

    // Wait for "QA verdict: PASS" toast
    await waitForToast(page, /QA verdict.*PASS/i, { timeout: t(15_000) });
    await page.waitForTimeout(DEMO_PAUSE);
  });

  test("Step 7: Verify task in In Review on kanban", async ({ page }) => {
    await navigateToProjectTasks(page, PROJECT_ID);

    // Verify the task card is visible on the board
    const card = findTaskCard(page, TASK_TITLE);
    await expect(card).toBeVisible({ timeout: t(10_000) });

    // Verify "In Review" column header is visible
    await expect(page.getByText("In Review").first()).toBeVisible({ timeout: t(5_000) });
    await page.waitForTimeout(DEMO_PAUSE);
  });

  test("Step 8: Human approval — change to Done", async ({ page }) => {
    await navigateToTaskDetail(page, TASK_PATH);
    await changeTaskStatus(page, "In Review", "Done", { demoPause: DEMO_PAUSE * 2 });
  });

  test.afterAll(async ({ request }) => {
    if (PROJECT_ID) await cleanupProject(request, PROJECT_ID);
    if (DEMO_BRANCH) {
      await cleanupDemoBranches(request, [DEMO_BRANCH]);
    }
    if (DEMO_PR_NUMBER) {
      await cleanupDemoPr(request, DEMO_PR_NUMBER);
    }
  });
});
