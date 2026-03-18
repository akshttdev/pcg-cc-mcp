import { APIRequestContext, expect } from "@playwright/test";
import { apiLogin } from "../auth";
import { DEMO_REPO_CONFIG, type DemoRepoConfig } from "./config";

const GITHUB_API = "https://api.github.com";

/** Helper to make GitHub API calls with auth */
async function ghApi(
  request: APIRequestContext,
  method: "GET" | "POST" | "PUT" | "PATCH" | "DELETE",
  path: string,
  data?: unknown
) {
  const token = process.env.GITHUB_TOKEN;
  if (!token) throw new Error("GITHUB_TOKEN env var required for demo simulation");

  const url = path.startsWith("http") ? path : `${GITHUB_API}${path}`;
  const opts = {
    headers: {
      Authorization: `token ${token}`,
      Accept: "application/vnd.github.v3+json",
    },
    ...(data ? { data } : {}),
  };

  switch (method) {
    case "GET": return request.get(url, opts);
    case "POST": return request.post(url, opts);
    case "PUT": return request.put(url, opts);
    case "PATCH": return request.patch(url, opts);
    case "DELETE": return request.delete(url, opts);
  }
}

/**
 * Create a branch with a commit in the sandbox repo via GitHub API.
 * Returns the branch name.
 */
async function createBranchWithCommit(
  request: APIRequestContext,
  config: DemoRepoConfig,
  branchName: string,
  message: string
): Promise<string> {
  const { owner, name, defaultBranch } = config;

  // 1. Get the SHA of the default branch
  const refRes = await ghApi(
    request, "GET",
    `/repos/${owner}/${name}/git/refs/heads/${defaultBranch}`
  );
  expect(refRes.ok(), `Failed to get ref for ${defaultBranch}`).toBeTruthy();
  const refData = await refRes.json();
  const baseSha = refData.object.sha;

  // 2. Create a new branch
  const createRefRes = await ghApi(
    request, "POST",
    `/repos/${owner}/${name}/git/refs`,
    { ref: `refs/heads/${branchName}`, sha: baseSha }
  );
  expect(createRefRes.ok(), `Failed to create branch ${branchName}`).toBeTruthy();

  // 3. Create a file on the branch (the "agent's commit")
  const timestamp = new Date().toISOString();
  const content = Buffer.from(
    `// Agent simulation commit\n// Task: ${message}\n// Created: ${timestamp}\nexport const status = "completed";\n`
  ).toString("base64");

  const filePath = `src/agent-work-${Date.now()}.ts`;
  const createFileRes = await ghApi(
    request, "PUT",
    `/repos/${owner}/${name}/contents/${filePath}`,
    {
      message,
      content,
      branch: branchName,
    }
  );
  expect(createFileRes.ok(), `Failed to create file on ${branchName}`).toBeTruthy();

  return branchName;
}

/**
 * Create a PR in the sandbox repo via GitHub API.
 */
async function createPr(
  request: APIRequestContext,
  config: DemoRepoConfig,
  branchName: string,
  title: string,
  body: string
): Promise<{ prNumber: number; prUrl: string }> {
  const { owner, name, defaultBranch } = config;

  const prRes = await ghApi(
    request, "POST",
    `/repos/${owner}/${name}/pulls`,
    {
      title,
      body,
      head: branchName,
      base: defaultBranch,
    }
  );
  expect(prRes.ok(), "Failed to create PR").toBeTruthy();
  const prData = await prRes.json();

  return {
    prNumber: prData.number,
    prUrl: prData.html_url,
  };
}

/**
 * Get or create a task attempt for the given task.
 * If an attempt already exists, returns the latest one.
 * Otherwise creates a new one (execution will fail, but the record persists).
 */
async function ensureTaskAttempt(
  request: APIRequestContext,
  taskId: string
): Promise<string> {
  // Check for existing attempts
  const attemptsRes = await request.get(`/api/task-attempts?task_id=${taskId}`);
  expect(attemptsRes.ok()).toBeTruthy();
  const attemptsData = await attemptsRes.json();
  const attempts = attemptsData.data ?? attemptsData;

  if (Array.isArray(attempts) && attempts.length > 0) {
    return attempts[0].id;
  }

  // Create a bare attempt record (no execution started)
  const createRes = await request.post("/api/task-attempts/create-record", {
    data: {
      task_id: taskId,
      executor: "CLAUDE_CODE",
      base_branch: "main",
    },
  });
  if (!createRes.ok()) {
    const errBody = await createRes.text().catch(() => "(no body)");
    throw new Error(`Failed to create task attempt record for task ${taskId}: ${createRes.status()} ${errBody}`);
  }
  const createData = await createRes.json();
  return createData.data?.id ?? createData.id;
}

/**
 * Simulate dev agent work via API calls.
 *
 * This does what a real dev agent completion triggers:
 * 1. Creates a branch + commit in the sandbox repo (via GitHub API)
 * 2. Creates a PR via GitHub API
 * 3. Creates a task attempt (if none exists)
 * 4. Links the PR to the task attempt via POST /api/task-attempts/:id/link-pr
 * 5. Updates task status to `inreview` (triggers spawn_watcher_reviews)
 *
 * @returns { branch, prNumber, prUrl, attemptId } for cleanup
 */
export async function simulateDevAgentWork(
  request: APIRequestContext,
  taskId: string,
  repo?: Partial<DemoRepoConfig>
): Promise<{ branch: string; prNumber: number; prUrl: string; attemptId: string }> {
  const config = { ...DEMO_REPO_CONFIG, ...repo };
  await apiLogin(request);

  // 1. Create branch + commit in sandbox repo
  const branchName = `e2e-demo/${taskId.slice(0, 8)}-${Date.now()}`;
  await createBranchWithCommit(
    request, config, branchName,
    `feat: agent work for task ${taskId.slice(0, 8)}`
  );

  // 2. Create PR
  const { prNumber, prUrl } = await createPr(
    request, config, branchName,
    `[E2E Demo] Agent work for task ${taskId.slice(0, 8)}`,
    `Automated agent simulation commit.\n\nTask: ${taskId}`
  );

  // 3. Ensure task attempt exists
  const attemptId = await ensureTaskAttempt(request, taskId);

  // 4. Link PR to task attempt
  const linkRes = await request.post(`/api/task-attempts/${attemptId}/link-pr`, {
    data: {
      pr_number: prNumber,
      pr_url: prUrl,
      target_branch: config.defaultBranch,
    },
  });
  if (!linkRes.ok()) {
    const errBody = await linkRes.text().catch(() => "(no body)");
    throw new Error(`Failed to link PR to attempt ${attemptId}: ${linkRes.status()} ${errBody}`);
  }

  // 5. Update task status to inreview (triggers watcher spawn)
  const updateRes = await request.put(`/api/tasks/${taskId}`, {
    data: { status: "inreview" },
  });
  if (!updateRes.ok()) {
    const errBody = await updateRes.text().catch(() => "(no body)");
    throw new Error(`Failed to update task status to inreview: ${updateRes.status()} ${errBody}`);
  }

  return { branch: branchName, prNumber, prUrl, attemptId };
}

/**
 * Simulate a QA watcher verdict via API calls.
 *
 * This does what a real QA agent's finalize_review triggers:
 * 1. Updates the watcher's collaborator action to the verdict
 * 2. Broadcasts the change via WebSocket (server-side)
 *
 * @param agentId - The agent watcher's ID
 * @param verdict - "qa_pass" | "qa_needs_changes" | "qa_fail"
 */
export async function simulateQaVerdict(
  request: APIRequestContext,
  taskId: string,
  agentId: string,
  verdict: "qa_pass" | "qa_needs_changes" | "qa_fail"
): Promise<void> {
  await apiLogin(request);

  const res = await request.patch(`/api/tasks/${taskId}/collaborators`, {
    data: {
      actor_id: agentId,
      actor_type: "agent_watcher",
      action: verdict,
    },
  });
  expect(res.ok(), `Failed to set QA verdict ${verdict} for agent ${agentId}`).toBeTruthy();
}

/**
 * Create a PR in the sandbox repo and link it to a task.
 *
 * Used when the demo needs a PR to exist before triggering watchers
 * (e.g., Manual QA Trigger demo).
 */
// ── PR Comment Helpers ───────────────────────────────────────────────────────

/**
 * Post a dev agent work summary comment on a PR.
 *
 * Mirrors what `post_dev_agent_pr_comment()` does in the backend:
 * posts an audit trail comment identifying the agent and summarizing work done.
 */
export async function postDevAgentSummaryComment(
  request: APIRequestContext,
  prNumber: number,
  taskId: string,
  taskTitle: string,
  repo?: Partial<DemoRepoConfig>
): Promise<void> {
  const config = { ...DEMO_REPO_CONFIG, ...repo };

  const comment = [
    `## Dev Agent Work Summary`,
    ``,
    `**Task**: \`${taskId.slice(0, 8)}\` — ${taskTitle}`,
    `**Agent**: ORCHA Dev Agent (CLAUDE_CODE)`,
    ``,
    `### Changes Made`,
    `- Analyzed task requirements and completion criteria`,
    `- Implemented fix for the reported issue`,
    `- Added relevant test coverage`,
    `- Verified build passes locally`,
    ``,
    `### Files Changed`,
    `| File | Change |`,
    `|---|---|`,
    `| \`src/agent-work-*.ts\` | New: agent implementation |`,
    ``,
    `---`,
    `*Created by **ORCHA Dev Agent** — Automated by ORCHA Platform*`,
  ].join("\n");

  const res = await ghApi(
    request, "POST",
    `/repos/${config.owner}/${config.name}/issues/${prNumber}/comments`,
    { body: comment }
  );
  expect(res.ok(), `Failed to post dev agent summary comment on PR #${prNumber}`).toBeTruthy();
}

/**
 * Post a QA review comment on a PR.
 *
 * Mirrors what `finalize_review()` → `build_review_comment()` does in the backend:
 * posts a structured review with verdict, criteria checks, and issues table.
 */
export async function postQaReviewComment(
  request: APIRequestContext,
  prNumber: number,
  taskId: string,
  verdict: "pass" | "needs_changes" | "fail",
  opts?: {
    summary?: string;
    criteriaChecks?: Array<{ criterion: string; met: boolean; notes: string }>;
    issues?: Array<{ file: string; line: number; severity: string; description: string }>;
    iteration?: number;
  },
  repo?: Partial<DemoRepoConfig>
): Promise<void> {
  const config = { ...DEMO_REPO_CONFIG, ...repo };
  const iteration = opts?.iteration ?? 1;

  const verdictEmoji = verdict === "pass" ? "\u2705" : verdict === "needs_changes" ? "\u26a0\ufe0f" : "\u274c";
  const verdictLabel = verdict === "pass" ? "Pass" : verdict === "needs_changes" ? "Needs Changes" : "Fail";
  const summary = opts?.summary ?? (verdict === "pass"
    ? "All completion criteria met. Code changes are clean and well-structured."
    : "Some issues found that need to be addressed before merging.");

  const checks = opts?.criteriaChecks ?? [
    { criterion: "Bug fix addresses reported issue", met: verdict === "pass", notes: verdict === "pass" ? "Fix correctly handles the edge case" : "Partial fix — edge case not covered" },
    { criterion: "No regressions introduced", met: true, notes: "Existing tests pass" },
    { criterion: "Code follows project conventions", met: true, notes: "Consistent style and patterns" },
  ];

  const issues = opts?.issues ?? (verdict !== "pass" ? [
    { file: "src/agent-work.ts", line: 12, severity: "warning", description: "Missing error handling for null input" },
  ] : []);

  let comment = `## QA Review — Iteration ${iteration}\n\n**Verdict**: ${verdictEmoji} ${verdictLabel}\n\n`;

  comment += "### Completion Criteria\n";
  for (const check of checks) {
    const checkbox = check.met ? "[x]" : "[ ]";
    comment += `- ${checkbox} ${check.criterion} — ${check.notes}\n`;
  }
  comment += "\n";

  if (issues.length > 0) {
    comment += "### Issues\n| File | Line | Severity | Description |\n|---|---|---|---|\n";
    for (const issue of issues) {
      comment += `| \`${issue.file}\` | ${issue.line} | ${issue.severity} | ${issue.description} |\n`;
    }
    comment += "\n";
  }

  comment += `### Summary\n${summary}\n\n`;
  comment += `---\n*ORCHA QA Agent • Task ${taskId.slice(0, 8)} • Iteration ${iteration}/2*`;

  const res = await ghApi(
    request, "POST",
    `/repos/${config.owner}/${config.name}/issues/${prNumber}/comments`,
    { body: comment }
  );
  expect(res.ok(), `Failed to post QA review comment on PR #${prNumber}`).toBeTruthy();
}

/**
 * Fetch comments on a PR from GitHub API.
 * Returns array of { body, user } objects.
 */
export async function fetchPrComments(
  request: APIRequestContext,
  prNumber: number,
  repo?: Partial<DemoRepoConfig>
): Promise<Array<{ body: string; user: string }>> {
  const config = { ...DEMO_REPO_CONFIG, ...repo };

  const res = await ghApi(
    request, "GET",
    `/repos/${config.owner}/${config.name}/issues/${prNumber}/comments`
  );
  expect(res.ok(), `Failed to fetch comments for PR #${prNumber}`).toBeTruthy();
  const comments = await res.json();

  return (comments as Array<{ body: string; user: { login: string } }>).map((c) => ({
    body: c.body,
    user: c.user.login,
  }));
}

/**
 * Get the HTML URL for a PR in the sandbox repo.
 */
export function getPrUrl(prNumber: number, repo?: Partial<DemoRepoConfig>): string {
  const config = { ...DEMO_REPO_CONFIG, ...repo };
  return `https://github.com/${config.owner}/${config.name}/pull/${prNumber}`;
}

// ── PR Creation Helpers ──────────────────────────────────────────────────────

export async function createPrForTask(
  request: APIRequestContext,
  taskId: string,
  repo?: Partial<DemoRepoConfig>
): Promise<{ branch: string; prNumber: number; prUrl: string; attemptId: string }> {
  const config = { ...DEMO_REPO_CONFIG, ...repo };
  await apiLogin(request);

  // Create branch + commit
  const branchName = `e2e-demo/${taskId.slice(0, 8)}-${Date.now()}`;
  await createBranchWithCommit(
    request, config, branchName,
    `feat: stub PR for task ${taskId.slice(0, 8)}`
  );

  // Create PR
  const { prNumber, prUrl } = await createPr(
    request, config, branchName,
    `[E2E Demo] Stub PR for task ${taskId.slice(0, 8)}`,
    `Stub PR for manual QA trigger demo.\n\nTask: ${taskId}`
  );

  // Ensure task attempt + link PR
  const attemptId = await ensureTaskAttempt(request, taskId);
  const linkRes = await request.post(`/api/task-attempts/${attemptId}/link-pr`, {
    data: {
      pr_number: prNumber,
      pr_url: prUrl,
      target_branch: config.defaultBranch,
    },
  });
  expect(linkRes.ok(), `Failed to link PR to attempt ${attemptId}`).toBeTruthy();

  return { branch: branchName, prNumber, prUrl, attemptId };
}
