/**
 * Demo: Bug Report Full Lifecycle (Agent-Driven)
 *
 * Walks through the complete lifecycle of a user-submitted bug report
 * with real agent interactions, alternating between the app and GitHub:
 *
 *   Report → find on kanban → add QA watcher →
 *   agent starts working → [GitHub: see PR + dev summary comment] →
 *   [App: task in In Review] →
 *   QA review triggered → [GitHub: see QA review comment] →
 *   [App: QA Passed badge visible] →
 *   Human approval (only after agent approval) → Done
 *
 * Agent interactions are simulated via API (real GitHub branches, PRs,
 * and review comments) — no stub executors in production code.
 *
 * Requires: GITHUB_TOKEN env var for sandbox repo operations.
 */
import { test, expect } from "./fixtures";
import {
  t, demoPause, login, TEST_DATA_PREFIX, apiLogin,
  navigateToProjectTasks, navigateToTaskDetail,
  changeTaskStatus, addQaWatcher, findTaskCard,
  cleanupTaskByPath, cleanupE2eDataSources, ensureAgentsSeeded, openFeedbackDialog,
} from "../helpers";
import {
  simulateDevAgentWork, simulateQaVerdict,
  cleanupDemoBranches, cleanupDemoPr,
  postDevAgentSummaryComment, postQaReviewComment, fetchPrComments, getPrUrl,
} from "../helpers/demo";


const BUG_TITLE = `${TEST_DATA_PREFIX} Demo: Dashboard crash ${Date.now()}`;
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

    await openFeedbackDialog(page);
    await expect(page.getByText("Bug Report").first()).toBeVisible({ timeout: t(3_000) });

    // Select severity: High
    const severityTrigger = page.getByRole("combobox").filter({ hasText: /Medium/ });
    await severityTrigger.click();
    await page.getByRole("option", { name: /high/i }).first().click();

    await page.getByRole("textbox", { name: "Title *" }).fill(BUG_TITLE);
    await page.getByRole("textbox", { name: "Description *" }).fill(
      "Dashboard shows white screen on empty project.\n\nSteps to reproduce:\n1. Create new project\n2. Navigate to dashboard\n3. See blank white screen"
    );

    await page.waitForTimeout(demoPause.medium);
    await page.getByRole("button", { name: "Submit Feedback" }).click();
    await expect(page.getByText(/Thank you.*feedback/i)).toBeVisible({ timeout: t(10_000) });
    await expect(
      page.getByRole("heading", { name: "Submit Feedback" })
    ).not.toBeVisible({ timeout: t(5_000) });
    await page.waitForTimeout(demoPause.medium);
  });

  test("Step 2: Find bug on kanban → open detail", async ({ page }) => {
    await navigateToProjectTasks(page, BUGREPORTS_PROJECT_ID);

    const card = findTaskCard(page, BUG_TITLE);
    await expect(card).toBeVisible({ timeout: t(10_000) });
    await page.waitForTimeout(demoPause.medium);

    await card.click();
    await expect(page.getByText("Agent Reviewers", { exact: true })).toBeVisible({
      timeout: t(10_000),
    });

    TASK_PATH = new URL(page.url()).pathname;
    TASK_ID = TASK_PATH.split("/tasks/")[1];
    await page.waitForTimeout(demoPause.medium);
  });

  test("Step 3: Add QA watcher", async ({ page }) => {
    await navigateToTaskDetail(page, TASK_PATH);
    await addQaWatcher(page, { demoPause: demoPause.medium });
    await expect(page.getByText(/ORCHA QA/i)).toBeVisible({ timeout: t(5_000) });
  });

  test("Step 4: Dev agent completes work → visit GitHub PR with summary", async ({ page, request }) => {
    test.setTimeout(60_000);
    await navigateToTaskDetail(page, TASK_PATH);
    await apiLogin(request);

    // Simulate dev agent: branch + commit + PR + link + status change
    const result = await simulateDevAgentWork(request, TASK_ID);
    DEMO_BRANCH = result.branch;
    DEMO_PR_NUMBER = result.prNumber;

    // Dev agent posts work summary on the PR (mirrors post_dev_agent_pr_comment)
    await postDevAgentSummaryComment(request, DEMO_PR_NUMBER, TASK_ID, BUG_TITLE);

    // Verify dev agent comment was posted via API
    const comments = await fetchPrComments(request, DEMO_PR_NUMBER);
    const devComment = comments.find((c) => c.body.includes("Dev Agent Work Summary"));
    expect(devComment, "Dev agent should have posted a work summary comment on the PR").toBeTruthy();

    // --- Visit GitHub: show the PR with the dev agent's summary comment ---
    const prUrl = getPrUrl(DEMO_PR_NUMBER);
    await page.goto(prUrl);
    await page.waitForLoadState("domcontentloaded");

    // Verify we're on the PR page — PR title visible
    await expect(page.locator("body")).toContainText(
      `#${DEMO_PR_NUMBER}`,
      { timeout: t(15_000) }
    );
    // Pause so viewer can see the PR page header
    await page.waitForTimeout(demoPause.medium);

    // Scroll down to comments section to ensure they render
    await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));
    await page.waitForTimeout(demoPause.medium);

    // Try to find dev agent comment in the rendered page (GitHub may lazy-load)
    const summaryVisible = await page.getByText("Dev Agent Work Summary").first()
      .isVisible().catch(() => false);
    if (summaryVisible) {
      await expect(page.getByText("ORCHA Dev Agent").first()).toBeVisible({ timeout: t(5_000) });
    }
    // Comment existence already verified via API above — visual check is best-effort

    // Hold on the GitHub PR page so viewer can read the comment
    await page.waitForTimeout(demoPause.long);
  });

  test("Step 5: Return to app — verify task In Review + QA triggered", async ({ page }) => {
    // Navigate back to the app to show the task moved to In Review
    await navigateToProjectTasks(page, BUGREPORTS_PROJECT_ID);
    const card = findTaskCard(page, BUG_TITLE);
    await expect(card).toBeVisible({ timeout: t(10_000) });
    await expect(page.getByText("In Review").first()).toBeVisible({ timeout: t(5_000) });
    await page.waitForTimeout(demoPause.medium);
  });

  test("Step 6: QA reviews → visit GitHub PR with QA review comment", async ({ page, request }) => {
    test.skip(!DEMO_PR_NUMBER, "Skipping — Step 4 did not create a PR");
    test.setTimeout(60_000);
    await navigateToTaskDetail(page, TASK_PATH);
    await apiLogin(request);

    // Wait for watcher state
    await expect(
      page.getByText("Running").or(page.getByText("Watching"))
    ).toBeVisible({ timeout: t(10_000) });
    await page.waitForTimeout(2_000);

    // QA agent renders verdict: PASS
    await simulateQaVerdict(request, TASK_ID, QA_AGENT_ID, "qa_pass");

    // QA agent posts structured review comment on the PR
    await postQaReviewComment(request, DEMO_PR_NUMBER!, TASK_ID, "pass", {
      summary: "All completion criteria met. The dashboard crash fix correctly handles empty project state.",
      criteriaChecks: [
        { criterion: "Bug fix addresses reported issue", met: true, notes: "Null check added for empty project data" },
        { criterion: "No regressions introduced", met: true, notes: "All existing tests pass" },
        { criterion: "Code follows project conventions", met: true, notes: "Consistent error handling pattern" },
      ],
    });

    // Verify both comments exist via API (reliable — not affected by GitHub lazy-loading)
    const comments = await fetchPrComments(request, DEMO_PR_NUMBER!);
    expect(comments.find((c) => c.body.includes("Dev Agent Work Summary")),
      "Dev agent should have posted a work summary comment").toBeTruthy();
    expect(comments.find((c) => c.body.includes("QA Review")),
      "QA agent should have posted a review comment").toBeTruthy();

    // --- Visit GitHub: show QA review comment alongside dev summary ---
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

  test("Step 7: Return to app — QA Passed badge visible", async ({ page }) => {
    // Navigate back to task detail to show QA verdict in the app
    await navigateToTaskDetail(page, TASK_PATH);

    await expect(
      page.getByText("Agent Reviewers", { exact: true })
    ).toBeVisible({ timeout: t(10_000) });

    // Verify "Passed" badge is visible — agent has approved
    await expect(
      page.getByText(/Passed|qa_pass/i)
    ).toBeVisible({ timeout: t(10_000) });
    await page.waitForTimeout(demoPause.medium);
  });

  test("Step 8: Human approval — change to Done (after agent approval)", async ({ page }) => {
    await navigateToTaskDetail(page, TASK_PATH);

    // Confirm agent approval badge before human acts
    await expect(
      page.getByText(/Passed|qa_pass/i)
    ).toBeVisible({ timeout: t(10_000) });

    await changeTaskStatus(page, "In Review", "Done", { demoPause: demoPause.medium });
  });

  test("Step 9: Verify task in Done column", async ({ page }) => {
    await navigateToProjectTasks(page, BUGREPORTS_PROJECT_ID);

    const doneHeader = page.locator("p").filter({ hasText: /^Done$/ });
    await expect(doneHeader).toBeVisible({ timeout: t(10_000) });

    const card = findTaskCard(page, BUG_TITLE);
    await expect(card).toBeVisible({ timeout: t(5_000) });

    await page.waitForTimeout(demoPause.long);
  });

  // ── Future: Agent Iteration Flow ──────────────────────────────────────────

  test.fixme("Future: QA finds issues → dev agent iterates with fixes", async () => {
    // Flow:
    // 1. QA verdict "needs_changes" with issues table
    // 2. Backend (finalize_review) sends task back to InProgress
    // 3. Dev agent reads QA feedback from PR comments
    // 4. Dev agent posts resolution comment for each issue
    // 5. Dev pushes fix commit, task returns to InReview
    // 6. QA re-reviews (iteration 2/2)
    // 7. [GitHub: show iteration 2 review comment]
    // 8. [App: QA Passed on second attempt]
    //
    // Blocked: needs real agent execution (LLM backend) to read QA feedback
    // and respond with code changes. Backend flow exists in finalize_review().
  });

  test.fixme("Future: PR gated — human cannot merge without agent approval", async () => {
    // When QA verdict is "needs_changes" or watcher is still "Running":
    // 1. UI should show warning when human tries to move to Done
    // 2. "Done" status change blocked or shows confirmation dialog
    // 3. PR on GitHub shows "Changes Requested" review status
    //
    // Blocked: needs frontend UI gating on watcher approval status.
    // Backend handles iteration (sends back to InProgress on needs_changes)
    // but frontend allows manual status override without warning.
  });

  test.fixme("Future: Dev agent resolution comments address each QA issue", async () => {
    // After QA finds issues:
    // 1. Dev agent reads issues from QA review comment
    // 2. For each issue, dev posts resolution reply with file/line/fix
    // 3. Resolution summary references all QA issues addressed
    //
    // Blocked: needs agent orchestration to parse QA comments and respond.
    // GitHub comment API exists, agent iteration exists, but the
    // "read feedback → respond with resolutions" loop is not built.
  });

  test.afterAll(async ({ request }) => {
    if (TASK_PATH) await cleanupTaskByPath(request, TASK_PATH);
    if (DEMO_BRANCH) {
      await cleanupDemoBranches(request, [DEMO_BRANCH]);
    }
    if (DEMO_PR_NUMBER) {
      await cleanupDemoPr(request, DEMO_PR_NUMBER);
    }
    await cleanupE2eDataSources(request, "01010101-0101-0101-0101-010101010101");
  });
});
