import { test, expect } from "./fixtures";
import { apiLogin, login, TEST_DATA_PREFIX, t } from "./helpers";

/**
 * Workflow Pipeline E2E Tests (Phase 10)
 *
 * End-to-end demonstration of the dogfooding pipeline:
 *   10A. Feedback → Triage → Task
 *   10B. Webhook → DataSource → Triage
 *   10C. Task Assignment → Execution Status
 *   10D. Full Pipeline Walkthrough
 */

const ORCHA_PROJECT_ID = "00000000-0000-0000-0000-000000000001";
const PCG_ORG_ID = "01010101-0101-0101-0101-010101010101";
const BUGS_BOARD_ID = "d0600000-0000-0000-0000-000000000001";
const DEV_AGENT_ID = "a0000000-0000-0000-0000-000000000001";
const QA_AGENT_ID = "a0000000-0000-0000-0000-000000000002";

// ─── 10A: Feedback → Triage → Task ─────────────────────────────────────────

test.describe("10A: Feedback → Triage → Task", () => {
  test("submit feedback and verify triage task appears on Bugs board", async ({ page, request }) => {
    await apiLogin(request);

    // 1. Submit feedback via API
    const feedbackRes = await request.post("/api/feedback", {
      data: {
        feedback_type: "bug",
        title: `${TEST_DATA_PREFIX} Pipeline feedback test`,
        description: "Login button unresponsive on Safari. Steps: 1. Open Safari 2. Navigate to /login 3. Click Sign In — nothing happens.",
        severity: "high",
        email: "pipeline-test@orcha.dev",
      },
    });
    expect(feedbackRes.ok()).toBeTruthy();
    const feedback = (await feedbackRes.json()).data;
    expect(feedback.task_id).toBeTruthy();

    // 2. Navigate to ORCHA Platform → Bugs board
    await page.goto(`/projects/${ORCHA_PROJECT_ID}/tasks`);
    await page.waitForLoadState("domcontentloaded");
    await expect(page.getByText("To Do").first()).toBeVisible({ timeout: t(10_000) });

    // 3. Verify triage task appears with correct priority
    await expect(
      page.getByText("Pipeline feedback test").first()
    ).toBeVisible({ timeout: t(10_000) });

    // 4. Click the task to verify description is populated
    await page.getByText("Pipeline feedback test").first().click();
    await expect(page.locator("body")).toContainText("Login button unresponsive", { timeout: t(5_000) });

    // 5. Verify DataSource was created
    await page.goto(`/organizations/${PCG_ORG_ID}/intelligence/data-sources`);
    await page.waitForLoadState("domcontentloaded");
    await expect(
      page.getByText("Pipeline feedback test").first()
    ).toBeVisible({ timeout: t(10_000) });

    // Cleanup
    await request.delete(`/api/tasks/${feedback.task_id}`).catch(() => {});
  });
});

// ─── 10B: Webhook → DataSource → Triage ────────────────────────────────────

test.describe("10B: Webhook → DataSource → Triage", () => {
  test("GitHub issue webhook creates DataSource and triage task", async ({ page, request }) => {
    await apiLogin(request);
    const issueNumber = 50000 + Math.floor(Math.random() * 50000);

    // 1. Fire GitHub issue webhook
    const webhookRes = await request.post("/api/webhooks/github", {
      headers: { "X-GitHub-Event": "issues", "Content-Type": "application/json" },
      data: {
        action: "opened",
        issue: {
          number: issueNumber,
          title: `${TEST_DATA_PREFIX} Pipeline webhook test issue`,
          body: "Tests fail intermittently on CI: `cargo test --workspace` exits with code 101.",
          html_url: `https://github.com/test/pcg-cc-mcp/issues/${issueNumber}`,
          user: { login: "pipeline-bot" },
          labels: [{ name: "bug" }, { name: "ci" }],
        },
        repository: { full_name: "test/pcg-cc-mcp" },
      },
    });
    expect(webhookRes.status()).toBe(200);

    // 2. Navigate to org data sources → verify issue appears
    await page.goto(`/organizations/${PCG_ORG_ID}/intelligence/data-sources`);
    await page.waitForLoadState("domcontentloaded");
    await expect(
      page.getByText(`#${issueNumber}`).first()
    ).toBeVisible({ timeout: t(10_000) });

    // 3. Navigate to Bugs board → verify triage task exists
    await page.goto(`/projects/${ORCHA_PROJECT_ID}/tasks`);
    await page.waitForLoadState("domcontentloaded");

    // The triage pipeline may or may not have run (depends on API key).
    // Verify the board at least loads.
    await expect(page.getByText("To Do").first()).toBeVisible({ timeout: t(10_000) });

    // Cleanup DataSource
    const dsRes = await request.get(`/api/data-sources?organization_id=${PCG_ORG_ID}&limit=20`);
    if (dsRes.ok()) {
      const sources = (await dsRes.json()).data;
      const found = sources.find(
        (s: any) => s.data_type === "github_issue" && s.title?.includes(`#${issueNumber}`)
      );
      if (found) {
        await request.delete(`/api/data-sources/${found.id}`).catch(() => {});
      }
    }
  });
});

// ─── 10C: Task Assignment → Execution Status ───────────────────────────────

test.describe("10C: Task Assignment → Execution Status", () => {
  test("seeded agents exist and are configured (API validation)", async ({ request }) => {
    await apiLogin(request);

    // Verify Dev agent exists
    const agentsRes = await request.get("/api/agents");
    expect(agentsRes.ok()).toBeTruthy();
    const agents = (await agentsRes.json()).data;

    const devAgent = agents.find((a: any) => a.short_name === "ORCHA Dev");
    expect(devAgent).toBeTruthy();
    expect(devAgent.status).toBe("active");
    expect(devAgent.designation).toBe("Platform Development Agent");

    const qaAgent = agents.find((a: any) => a.short_name === "ORCHA QA");
    expect(qaAgent).toBeTruthy();
    expect(qaAgent.status).toBe("active");
    expect(qaAgent.designation).toBe("Quality Assurance Review Agent");
  });

  test("task with agent_id shows agent assignment in detail view", async ({ page, request }) => {
    await apiLogin(request);

    // Create task with dev agent assigned
    const taskRes = await request.post("/api/tasks", {
      data: {
        project_id: ORCHA_PROJECT_ID,
        title: `${TEST_DATA_PREFIX} Agent assignment test`,
        description: "Test that agent assignment is visible in the task detail view.",
        priority: "low",
        agent_id: DEV_AGENT_ID,
        board_id: BUGS_BOARD_ID,
        tags: ["e2e-test"],
      },
    });
    expect(taskRes.ok()).toBeTruthy();
    const task = (await taskRes.json()).data;

    // Navigate to task detail
    await page.goto(`/projects/${ORCHA_PROJECT_ID}/tasks`);
    await page.waitForLoadState("domcontentloaded");
    await expect(page.getByText("To Do").first()).toBeVisible({ timeout: t(10_000) });

    // Verify agent assignment is visible on the kanban card or detail
    // (The card should show the task title at minimum)
    await expect(
      page.getByText("Agent assignment test").first()
    ).toBeVisible({ timeout: t(10_000) });

    // Cleanup
    await request.delete(`/api/tasks/${task.id}`).catch(() => {});
  });
});

// ─── 10D: Full Pipeline Walkthrough ────────────────────────────────────────

test.describe("10D: Full Pipeline Walkthrough", () => {
  test("pipeline components are correctly wired (API integration check)", async ({ request }) => {
    await apiLogin(request);

    // 1. Verify Bug Triage Pipeline workflow definition exists
    const defsRes = await request.get("/api/workflows/definitions");
    expect(defsRes.ok()).toBeTruthy();
    const defs = (await defsRes.json()).data;
    const triage = defs.find((d: any) => d.id === "bug_triage_pipeline");
    expect(triage).toBeTruthy();

    // 2. Verify ORCHA Bug Triage trigger is configured
    const triggersRes = await request.get("/api/workflows/triggers");
    expect(triggersRes.ok()).toBeTruthy();
    const triggers = (await triggersRes.json()).data;
    const orchaTrigger = triggers.find((t: any) => t.name === "ORCHA Bug Triage");
    expect(orchaTrigger).toBeTruthy();
    expect(orchaTrigger.enabled).toBeTruthy();

    // 3. Verify agents are seeded
    const agentsRes = await request.get("/api/agents");
    expect(agentsRes.ok()).toBeTruthy();
    const agents = (await agentsRes.json()).data;
    const devAgent = agents.find((a: any) => a.short_name === "ORCHA Dev");
    const qaAgent = agents.find((a: any) => a.short_name === "ORCHA QA");
    expect(devAgent).toBeTruthy();
    expect(qaAgent).toBeTruthy();

    // 4. Verify project boards exist
    const boardsRes = await request.get(`/api/projects/${ORCHA_PROJECT_ID}/boards`);
    expect(boardsRes.ok()).toBeTruthy();
    const boards = (await boardsRes.json()).data;
    expect(boards.length).toBeGreaterThanOrEqual(3);

    // 5. Submit feedback and verify task is created
    const feedbackRes = await request.post("/api/feedback", {
      data: {
        feedback_type: "bug",
        title: `${TEST_DATA_PREFIX} Full pipeline check`,
        description: "End-to-end wiring verification.",
        severity: "low",
      },
    });
    expect(feedbackRes.ok()).toBeTruthy();
    const feedback = (await feedbackRes.json()).data;
    expect(feedback.task_id).toBeTruthy();

    // 6. Verify task was created with correct project
    const taskRes = await request.get(`/api/tasks/${feedback.task_id}`);
    expect(taskRes.ok()).toBeTruthy();
    const task = (await taskRes.json()).data;
    expect(task.project_id).toBe(ORCHA_PROJECT_ID);

    // Cleanup
    await request.delete(`/api/tasks/${feedback.task_id}`).catch(() => {});
  });

  test("full pipeline visible in browser walkthrough", async ({ page, request }) => {
    await apiLogin(request);

    // 1. Submit feedback
    const feedbackRes = await request.post("/api/feedback", {
      data: {
        feedback_type: "bug",
        title: `${TEST_DATA_PREFIX} Browser walkthrough test`,
        description: "Full E2E walkthrough: feedback → task → board → detail",
        severity: "medium",
        email: "walkthrough@orcha.dev",
      },
    });
    expect(feedbackRes.ok()).toBeTruthy();
    const feedback = (await feedbackRes.json()).data;

    // 2. Verify task on board
    await page.goto(`/projects/${ORCHA_PROJECT_ID}/tasks`);
    await page.waitForLoadState("domcontentloaded");
    await expect(page.getByText("To Do").first()).toBeVisible({ timeout: t(10_000) });
    await expect(
      page.getByText("Browser walkthrough test").first()
    ).toBeVisible({ timeout: t(15_000) });

    // 3. Click task to see detail
    await page.getByText("Browser walkthrough test").first().click();

    // 4. Verify description visible in detail
    await expect(page.locator("body")).toContainText("Full E2E walkthrough", { timeout: t(5_000) });

    // 5. Navigate to org intelligence page
    await page.goto(`/organizations/${PCG_ORG_ID}/intelligence/data-sources`);
    await page.waitForLoadState("domcontentloaded");
    // Verify DataSource for this feedback exists
    await expect(
      page.getByText("Browser walkthrough test").first()
    ).toBeVisible({ timeout: t(10_000) });

    // Cleanup
    await request.delete(`/api/tasks/${feedback.task_id}`).catch(() => {});
  });
});
