/**
 * Demo: Manual QA Trigger from Task Detail (with PR)
 *
 * Demonstrates the QA watcher flow when a user manually moves a task
 * to "In Review" status with a PR linked, alternating between app and GitHub:
 *
 *   Create task → add QA watcher → link PR →
 *   [GitHub: see PR with dev comment] →
 *   [App: In Progress → In Review] → watcher triggered →
 *   [GitHub: see QA review comment] →
 *   [App: QA Passed] → Human approval → Done
 *
 * The PR is created before the status change, so when the task moves to
 * In Review, spawn_watcher_reviews finds the PR and triggers the QA watcher.
 *
 * Requires: GITHUB_TOKEN env var for sandbox repo operations.
 */
import { test, expect } from "./fixtures";
import {
  t, demoPause, login, createDemoProject, TEST_DATA_PREFIX, apiLogin,
  navigateToProjectTasks, navigateToTaskDetail, createTaskViaUI,
  changeTaskStatus, addQaWatcher, findTaskCard, cleanupProject,
  ensureAgentsSeeded,
} from "../helpers";
import {
  createPrForTask, simulateQaVerdict,
  cleanupDemoBranches, cleanupDemoPr,
  postDevAgentSummaryComment, postQaReviewComment, fetchPrComments, getPrUrl,
} from "../helpers/demo";
import { waitForToast } from "../helpers/demo/assertions";

const TASK_TITLE = `${TEST_DATA_PREFIX} Manual QA Trigger ${Date.now()}`;
const QA_AGENT_ID = "a0000000-0000-0000-0000-000000000002";

let PROJECT_ID: string;
let TASK_PATH: string;
let TASK_ID: string;
let DEMO_BRANCH: string | undefined;
let DEMO_PR_NUMBER: number | undefined;

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
      demoPause: demoPause.medium,
    });

    TASK_PATH = new URL(page.url()).pathname;
    TASK_ID = TASK_PATH.split("/tasks/")[1];
    await page.waitForTimeout(demoPause.medium);
  });

  test("Step 2: Add QA watcher from task detail drawer", async ({ page }) => {
    await navigateToTaskDetail(page, TASK_PATH);
    await addQaWatcher(page, { demoPause: demoPause.medium });

    await expect(page.getByText(/ORCHA QA/i)).toBeVisible({ timeout: t(5_000) });
    await expect(page.getByText("Watching")).toBeVisible({ timeout: t(5_000) });
  });

  test("Step 3: Link PR → visit GitHub to see PR + dev summary", async ({ page, request }) => {
    test.setTimeout(60_000);
    await apiLogin(request);

    // Create a real PR in the sandbox repo and link it to the task attempt
    const result = await createPrForTask(request, TASK_ID);
    DEMO_BRANCH = result.branch;
    DEMO_PR_NUMBER = result.prNumber;

    // Dev agent posts work summary on the PR
    await postDevAgentSummaryComment(request, DEMO_PR_NUMBER, TASK_ID, TASK_TITLE);

    // Verify dev agent comment was posted via API (reliable)
    const comments = await fetchPrComments(request, DEMO_PR_NUMBER);
    const devComment = comments.find((c) => c.body.includes("Dev Agent Work Summary"));
    expect(devComment, "Dev agent should have posted a work summary comment on the PR").toBeTruthy();

    // --- Visit GitHub: show the PR with dev agent's summary ---
    const prUrl = getPrUrl(DEMO_PR_NUMBER);
    await page.goto(prUrl);
    await page.waitForLoadState("domcontentloaded");

    await expect(page.locator("body")).toContainText(
      `#${DEMO_PR_NUMBER}`,
      { timeout: t(15_000) }
    );
    // Pause so viewer can see the PR page header
    await page.waitForTimeout(demoPause.medium);

    // Scroll to load lazy content (comments section)
    await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));
    await page.waitForTimeout(demoPause.medium);

    // Best-effort visual check (GitHub may lazy-load comments)
    const summaryVisible = await page.getByText("Dev Agent Work Summary").first()
      .isVisible().catch(() => false);
    if (summaryVisible) {
      await expect(page.getByText("ORCHA Dev Agent").first()).toBeVisible({ timeout: t(5_000) });
    }
    // Comment existence already verified via API above

    // Hold on the GitHub PR page so viewer can read the dev summary
    await page.waitForTimeout(demoPause.long);
  });

  test("Step 4: Return to app — To Do → In Progress", async ({ page }) => {
    await navigateToTaskDetail(page, TASK_PATH);
    await changeTaskStatus(page, "To Do", "In Progress", { demoPause: demoPause.medium });
  });

  test("Step 5: In Progress → In Review (triggers watcher)", async ({ page }) => {
    await navigateToTaskDetail(page, TASK_PATH);
    await changeTaskStatus(page, "In Progress", "In Review", { demoPause: demoPause.medium });

    // With a PR linked, the server's spawn_watcher_reviews should trigger
    await waitForToast(page, /QA review started|watcher.*trigger/i, { timeout: t(15_000) });
    await page.waitForTimeout(demoPause.medium);
  });

  test("Step 6: QA verdict → visit GitHub to see review comment", async ({ page, request }) => {
    test.setTimeout(60_000);
    await navigateToTaskDetail(page, TASK_PATH);
    await apiLogin(request);

    // Wait for watcher state
    await expect(
      page.getByText("Watching").or(page.getByText("Running"))
    ).toBeVisible({ timeout: t(10_000) });
    await page.waitForTimeout(2_000);

    const runningVisible = await page.getByText("Running").isVisible().catch(() => false);
    if (!runningVisible) {
      await request.patch(`/api/tasks/${TASK_ID}/collaborators`, {
        data: { actor_id: QA_AGENT_ID, actor_type: "agent_watcher", action: "triggered" },
      });
      await page.waitForTimeout(demoPause.medium);
    }

    // Simulate QA verdict: PASS
    await simulateQaVerdict(request, TASK_ID, QA_AGENT_ID, "qa_pass");

    // QA agent posts review comment on the PR
    await postQaReviewComment(request, DEMO_PR_NUMBER!, TASK_ID, "pass", {
      summary: "All criteria met. Task implementation is clean and complete.",
      criteriaChecks: [
        { criterion: "Implementation matches requirements", met: true, notes: "Verified against task description" },
        { criterion: "Tests pass", met: true, notes: "No regressions" },
        { criterion: "Code quality", met: true, notes: "Follows project patterns" },
      ],
    });

    // Verify both comments exist via API (reliable)
    const comments = await fetchPrComments(request, DEMO_PR_NUMBER!);
    expect(comments.find((c) => c.body.includes("Dev Agent Work Summary")),
      "Dev agent should have posted a work summary").toBeTruthy();
    expect(comments.find((c) => c.body.includes("QA Review")),
      "QA agent should have posted a review").toBeTruthy();

    // --- Visit GitHub: show both dev summary and QA review comments ---
    const prUrl = getPrUrl(DEMO_PR_NUMBER!);
    await page.goto(prUrl);
    await page.waitForLoadState("domcontentloaded");

    // Verify we're on the PR page
    await expect(page.locator("body")).toContainText(
      `#${DEMO_PR_NUMBER}`,
      { timeout: t(15_000) }
    );
    // Pause so viewer can see the PR page header
    await page.waitForTimeout(demoPause.medium);

    // Scroll to load lazy content (comments section)
    await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));
    await page.waitForTimeout(demoPause.medium);

    // Best-effort visual check (GitHub may lazy-load comments)
    const qaVisible = await page.getByText("QA Review").first()
      .isVisible().catch(() => false);
    if (qaVisible) {
      await expect(page.getByText("Completion Criteria").first()).toBeVisible({ timeout: t(5_000) });
    }
    // Comment existence already verified via API above

    // Hold on the GitHub PR page so viewer can read the QA review
    await page.waitForTimeout(demoPause.long);
  });

  test("Step 7: Return to app — verify QA Passed + task In Review", async ({ page }) => {
    // Navigate back to app to show QA verdict
    await navigateToTaskDetail(page, TASK_PATH);

    await expect(
      page.getByText("Agent Reviewers", { exact: true })
    ).toBeVisible({ timeout: t(10_000) });

    await expect(
      page.getByText(/Passed|qa_pass/i)
    ).toBeVisible({ timeout: t(10_000) });

    // Verify on kanban
    await navigateToProjectTasks(page, PROJECT_ID);
    const card = findTaskCard(page, TASK_TITLE);
    await expect(card).toBeVisible({ timeout: t(10_000) });
    await expect(page.getByText("In Review").first()).toBeVisible({ timeout: t(5_000) });
    await page.waitForTimeout(demoPause.medium);
  });

  test("Step 8: Human approval — change to Done (after agent approval)", async ({ page }) => {
    await navigateToTaskDetail(page, TASK_PATH);

    // Confirm agent has approved
    await expect(
      page.getByText(/Passed|qa_pass/i)
    ).toBeVisible({ timeout: t(10_000) });

    await changeTaskStatus(page, "In Review", "Done", { demoPause: demoPause.long });
  });

  // ── Future: Agent Iteration + PR Gating ────────────────────────────────────

  test.fixme("Future: QA needs_changes → task returns to InProgress automatically", async () => {
    // Flow:
    // 1. QA posts "needs_changes" verdict with issues
    // 2. Backend finalize_review() resets watcher to "watching" and sets task → InProgress
    // 3. Verify task moved back to InProgress on kanban
    // 4. [GitHub: QA review shows "Needs Changes" with issues table]
    // 5. Dev agent picks up again, pushes fix, task returns to InReview
    // 6. QA re-reviews (iteration 2/2)
    //
    // Blocked: needs agent execution to iterate on QA feedback.
  });

  test.fixme("Future: Dev agent posts resolution for each QA-found issue", async () => {
    // After QA feedback:
    // 1. Dev reads issues from QA comment
    // 2. Posts resolution comment: "Fixed: [issue] — [what was changed]"
    // 3. Pushes fix commit referencing the QA review
    //
    // Blocked: needs agent orchestration for feedback-response loop.
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
