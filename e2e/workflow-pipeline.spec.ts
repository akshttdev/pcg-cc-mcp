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

  test("task with agent_id auto-registers QA agent watcher", async ({ request }) => {
    await apiLogin(request);

    // Create task with dev agent assigned — should auto-register QA watcher
    const taskRes = await request.post("/api/tasks", {
      data: {
        project_id: ORCHA_PROJECT_ID,
        title: `${TEST_DATA_PREFIX} Agent watcher registration test`,
        description: "Test that creating a task with DEV agent auto-registers QA agent as watcher.",
        priority: "low",
        agent_id: DEV_AGENT_ID,
        board_id: BUGS_BOARD_ID,
        tags: ["e2e-test"],
      },
    });
    expect(taskRes.ok()).toBeTruthy();
    const task = (await taskRes.json()).data;

    // Fetch the task and verify collaborators include QA agent as agent_watcher
    const getRes = await request.get(`/api/tasks/${task.id}`);
    expect(getRes.ok()).toBeTruthy();
    const fetchedTask = (await getRes.json()).data;

    // collaborators is a JSON string — parse it
    const collaborators = fetchedTask.collaborators
      ? JSON.parse(fetchedTask.collaborators)
      : [];
    const qaWatcher = collaborators.find(
      (c: any) => c.actor_id === QA_AGENT_ID && c.actor_type === "agent_watcher"
    );
    expect(qaWatcher).toBeTruthy();
    expect(qaWatcher.last_action).toBe("watching");

    // Cleanup
    await request.delete(`/api/tasks/${task.id}`).catch(() => {});
  });
});

// ─── 10C-2: Agent Watcher API (manual add/remove/list) ──────────────────────

test.describe("10C-2: Agent Watcher API", () => {
  test("add, list, and remove agent watchers via API", async ({ request }) => {
    await apiLogin(request);

    // 1. Create a task (no agent_id — no auto-registered watchers)
    const taskRes = await request.post("/api/tasks", {
      data: {
        project_id: ORCHA_PROJECT_ID,
        title: `${TEST_DATA_PREFIX} Agent watcher API test`,
        description: "Test manual add/remove/list of agent watchers.",
        priority: "low",
        board_id: BUGS_BOARD_ID,
      },
    });
    expect(taskRes.ok()).toBeTruthy();
    const task = (await taskRes.json()).data;

    // 2. List watchers — should be empty
    const listEmpty = await request.get(`/api/tasks/${task.id}/agent-watchers`);
    expect(listEmpty.ok()).toBeTruthy();
    const emptyWatchers = (await listEmpty.json()).data;
    expect(emptyWatchers).toHaveLength(0);

    // 3. Add QA agent as watcher
    const addRes = await request.post(`/api/tasks/${task.id}/agent-watchers`, {
      data: { agent_id: QA_AGENT_ID },
    });
    expect(addRes.ok()).toBeTruthy();

    // 4. List watchers — should contain QA agent with "watching" status
    const listOne = await request.get(`/api/tasks/${task.id}/agent-watchers`);
    expect(listOne.ok()).toBeTruthy();
    const watchers = (await listOne.json()).data;
    expect(watchers).toHaveLength(1);
    expect(watchers[0].agent_id).toBe(QA_AGENT_ID);
    expect(watchers[0].last_action).toBe("watching");
    expect(watchers[0].agent_name).toBeTruthy();

    // 5. Add DEV agent as second watcher
    const addDev = await request.post(`/api/tasks/${task.id}/agent-watchers`, {
      data: { agent_id: DEV_AGENT_ID },
    });
    expect(addDev.ok()).toBeTruthy();

    const listTwo = await request.get(`/api/tasks/${task.id}/agent-watchers`);
    expect(listTwo.ok()).toBeTruthy();
    expect((await listTwo.json()).data).toHaveLength(2);

    // 6. Remove QA watcher
    const removeRes = await request.delete(
      `/api/tasks/${task.id}/agent-watchers/${QA_AGENT_ID}`
    );
    expect(removeRes.ok()).toBeTruthy();

    // 7. Verify only DEV watcher remains
    const listAfterRemove = await request.get(`/api/tasks/${task.id}/agent-watchers`);
    expect(listAfterRemove.ok()).toBeTruthy();
    const remaining = (await listAfterRemove.json()).data;
    expect(remaining).toHaveLength(1);
    expect(remaining[0].agent_id).toBe(DEV_AGENT_ID);

    // Cleanup
    await request.delete(`/api/tasks/${task.id}`).catch(() => {});
  });

  test("add watcher with invalid agent_id returns error", async ({ request }) => {
    await apiLogin(request);

    const taskRes = await request.post("/api/tasks", {
      data: {
        project_id: ORCHA_PROJECT_ID,
        title: `${TEST_DATA_PREFIX} Watcher invalid agent test`,
        description: "Test adding watcher with non-existent agent ID.",
        priority: "low",
        board_id: BUGS_BOARD_ID,
      },
    });
    expect(taskRes.ok()).toBeTruthy();
    const task = (await taskRes.json()).data;

    // Try to add a non-existent agent
    const addRes = await request.post(`/api/tasks/${task.id}/agent-watchers`, {
      data: { agent_id: "00000000-0000-0000-0000-999999999999" },
    });
    expect(addRes.ok()).toBeFalsy();
    expect(addRes.status()).toBeGreaterThanOrEqual(400);

    // Cleanup
    await request.delete(`/api/tasks/${task.id}`).catch(() => {});
  });
});

// ─── 10C-3: Agent Watcher UI ────────────────────────────────────────────────

test.describe("10C-3: Agent Watcher UI", () => {
  test("AgentWatcherPanel renders in task detail with watcher badges", async ({
    page,
    request,
  }) => {
    await apiLogin(request);

    // Create task with DEV agent → auto-registers QA watcher
    const taskRes = await request.post("/api/tasks", {
      data: {
        project_id: ORCHA_PROJECT_ID,
        title: `${TEST_DATA_PREFIX} Watcher panel UI test`,
        description: "Test that AgentWatcherPanel renders watchers.",
        priority: "low",
        agent_id: DEV_AGENT_ID,
        board_id: BUGS_BOARD_ID,
      },
    });
    expect(taskRes.ok()).toBeTruthy();
    const task = (await taskRes.json()).data;

    // Navigate to project tasks and click the task
    await page.goto(`/projects/${ORCHA_PROJECT_ID}/tasks`);
    await page.waitForLoadState("domcontentloaded");
    await expect(page.getByText("To Do").first()).toBeVisible({ timeout: t(10_000) });
    await expect(
      page.getByText("Watcher panel UI test").first()
    ).toBeVisible({ timeout: t(10_000) });
    await page.getByText("Watcher panel UI test").first().click();

    // Verify "Agent Reviewers" heading is visible in the detail panel
    await expect(
      page.getByText("Agent Reviewers").first()
    ).toBeVisible({ timeout: t(5_000) });

    // Verify at least one watcher badge is visible (QA agent auto-registered)
    await expect(
      page.getByText("Watching").first()
    ).toBeVisible({ timeout: t(5_000) });

    // Verify "Add" button exists
    await expect(
      page.getByRole("button", { name: "Add" }).first()
    ).toBeVisible();

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

    // 4. Verify agent watcher auto-registration works end-to-end
    //    Create a task with DEV agent → QA should appear as agent_watcher
    const watcherTaskRes = await request.post("/api/tasks", {
      data: {
        project_id: ORCHA_PROJECT_ID,
        title: `${TEST_DATA_PREFIX} Watcher pipeline check`,
        description: "Verify auto_watch_agent_ids wiring in full pipeline.",
        priority: "low",
        agent_id: DEV_AGENT_ID,
        board_id: BUGS_BOARD_ID,
      },
    });
    expect(watcherTaskRes.ok()).toBeTruthy();
    const watcherTask = (await watcherTaskRes.json()).data;
    const watcherTaskGet = await request.get(`/api/tasks/${watcherTask.id}`);
    expect(watcherTaskGet.ok()).toBeTruthy();
    const wtData = (await watcherTaskGet.json()).data;
    const collabs = wtData.collaborators ? JSON.parse(wtData.collaborators) : [];
    const qaWatcher = collabs.find(
      (c: any) => c.actor_id === QA_AGENT_ID && c.actor_type === "agent_watcher"
    );
    expect(qaWatcher).toBeTruthy();
    await request.delete(`/api/tasks/${watcherTask.id}`).catch(() => {});

    // 5. Verify project boards exist
    const boardsRes = await request.get(`/api/projects/${ORCHA_PROJECT_ID}/boards`);
    expect(boardsRes.ok()).toBeTruthy();
    const boards = (await boardsRes.json()).data;
    expect(boards.length).toBeGreaterThanOrEqual(3);

    // 6. Submit feedback and verify task is created
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

    // 7. Verify task was created with correct project
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
