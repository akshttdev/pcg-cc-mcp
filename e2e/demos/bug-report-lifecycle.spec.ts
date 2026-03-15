/**
 * Demo: Bug Report Full Lifecycle (Agent-Driven)
 *
 * Walks through the complete lifecycle of a user-submitted bug report
 * with real agent interactions:
 *   Report → find on kanban → add QA watcher → agent starts working →
 *   PR created → QA review triggered → QA verdict → Human approval → Done
 *
 * Agent interactions are simulated via API (real GitHub branches, PRs,
 * and merge records) — no stub executors in production code.
 *
 * Requires: GITHUB_TOKEN env var for sandbox repo operations.
 *
 * Self-contained: seeds agents in beforeAll, no reliance on specific DB state
 * beyond the ORCHA Platform project existing (guaranteed by migrations).
 */
import { test, expect } from "./fixtures";
import {
  t, login, TEST_DATA_PREFIX, apiLogin,
  navigateToProjectTasks, navigateToTaskDetail,
  changeTaskStatus, addQaWatcher, findTaskCard,
  cleanupTaskByPath, cleanupE2eDataSources, ensureAgentsSeeded, openFeedbackDialog,
} from "../helpers";
import {
  simulateDevAgentWork, simulateQaVerdict,
  cleanupDemoBranches, cleanupDemoPr,
} from "../helpers/demo";
import { waitForToast } from "../helpers/demo/assertions";

const BUG_TITLE = `${TEST_DATA_PREFIX} Demo: Dashboard crash ${Date.now()}`;
const DEMO_PAUSE = 1_500;
const BUGREPORTS_PROJECT_ID = "00000000-0000-0000-0000-000000000001";
const QA_AGENT_ID = "a0000000-0000-0000-0000-000000000002";

let TASK_PATH: string;
let TASK_ID: string;
let DEMO_BRANCH: string | undefined;
let DEMO_PR_NUMBER: number | undefined;

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
    TASK_ID = TASK_PATH.split("/tasks/")[1];
    await page.waitForTimeout(DEMO_PAUSE);
  });

  test("Step 3: Add QA watcher", async ({ page }) => {
    await navigateToTaskDetail(page, TASK_PATH);
    await addQaWatcher(page, { demoPause: DEMO_PAUSE });

    // Verify the ORCHA QA agent name is visible
    await expect(page.getByText(/ORCHA QA/i)).toBeVisible({ timeout: t(5_000) });
  });

  test("Step 4: Agent starts working — simulate dev agent", async ({ page, request }) => {
    await navigateToTaskDetail(page, TASK_PATH);
    await apiLogin(request);

    // Simulate dev agent: creates real branch + commit + PR in sandbox repo,
    // links PR to task attempt, updates status to inreview
    const result = await simulateDevAgentWork(request, TASK_ID);
    DEMO_BRANCH = result.branch;
    DEMO_PR_NUMBER = result.prNumber;

    // Wait for toast showing the task moved to In Review
    await waitForToast(page, /moved to In Review|In Review/i, { timeout: t(15_000) });
    await page.waitForTimeout(DEMO_PAUSE);

    // The backend's spawn_watcher_reviews triggers the QA watcher automatically
    // when a task moves to InReview with a linked PR. Wait for that toast too.
    await waitForToast(page, /QA review started/i, { timeout: t(15_000) });
    await page.waitForTimeout(DEMO_PAUSE);
  });

  test("Step 5: Verify task in In Review on kanban", async ({ page }) => {
    await navigateToProjectTasks(page, BUGREPORTS_PROJECT_ID);

    // Verify task card is visible on the board
    const card = findTaskCard(page, BUG_TITLE);
    await expect(card).toBeVisible({ timeout: t(10_000) });

    // Verify "In Review" column header is visible (confirms we can see the column)
    await expect(page.getByText("In Review").first()).toBeVisible({ timeout: t(5_000) });
    await page.waitForTimeout(DEMO_PAUSE);
  });

  test("Step 6: QA watcher verdict — simulate QA pass", async ({ page, request }) => {
    await navigateToTaskDetail(page, TASK_PATH);
    await apiLogin(request);

    // Wait for page to fully load and WebSocket to deliver current task state.
    // Watcher should already be in "Running" state from Step 4's spawn_watcher_reviews.
    // (UI renders triggered → "Running", watching → "Watching")
    await expect(
      page.getByText("Running").or(page.getByText("Watching"))
    ).toBeVisible({ timeout: t(10_000) });

    // Allow WebSocket to connect and deliver initial snapshot, so the diff hook
    // has prev state before we trigger the verdict change.
    await page.waitForTimeout(2_000);

    // Simulate QA verdict: PASS
    await simulateQaVerdict(request, TASK_ID, QA_AGENT_ID, "qa_pass");

    // Wait for "QA verdict: PASS" toast
    await waitForToast(page, /QA verdict.*PASS/i, { timeout: t(15_000) });
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
    if (DEMO_BRANCH) {
      await cleanupDemoBranches(request, [DEMO_BRANCH]);
    }
    if (DEMO_PR_NUMBER) {
      await cleanupDemoPr(request, DEMO_PR_NUMBER);
    }
    // Feedback dialog creates data sources as side effect — clean them up.
    // Data sources are org-scoped; ORCHA Platform is in the default org.
    await cleanupE2eDataSources(request, "01010101-0101-0101-0101-010101010101");
  });
});
