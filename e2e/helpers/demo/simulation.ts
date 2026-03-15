import { APIRequestContext, expect } from "@playwright/test";
import { apiLogin } from "../auth";
import { DEMO_REPO_CONFIG, type DemoRepoConfig } from "./config";

/**
 * Simulate dev agent work via API calls.
 *
 * This does what a real dev agent completion triggers:
 * 1. Creates a branch + commit in the sandbox repo (via gh CLI)
 * 2. Creates a PR via gh CLI
 * 3. Links the PR to the task via merge record API
 * 4. Updates task status to `inreview` (triggers spawn_watcher_reviews)
 *
 * @returns { branch, prNumber, prUrl } for cleanup
 */
export async function simulateDevAgentWork(
  request: APIRequestContext,
  taskId: string,
  repo?: Partial<DemoRepoConfig>
): Promise<{ branch: string; prNumber: number; prUrl: string }> {
  const config = { ...DEMO_REPO_CONFIG, ...repo };
  await apiLogin(request);

  // TODO Phase 1: Implement when sandbox repo is created
  // 1. gh api to create branch from default branch
  // 2. gh api to create/update a file on that branch (the "agent's commit")
  // 3. gh pr create
  // 4. POST /api/merges to link PR to task attempt
  // 5. PATCH /api/tasks/:id { status: "inreview" }

  throw new Error(
    "simulateDevAgentWork not yet implemented — needs sandbox repo (Phase 0)"
  );
}

/**
 * Simulate a QA watcher verdict via API calls.
 *
 * This does what a real QA agent's finalize_review triggers:
 * 1. Creates an execution artifact with verdict JSON
 * 2. Updates the watcher's collaborator action to the verdict
 *
 * @param verdict - "qa_pass" | "qa_needs_changes" | "qa_fail"
 */
export async function simulateQaVerdict(
  request: APIRequestContext,
  taskId: string,
  verdict: "qa_pass" | "qa_needs_changes" | "qa_fail"
): Promise<void> {
  await apiLogin(request);

  // TODO Phase 1: Implement
  // 1. Find the watcher's task attempt (GET /api/tasks/:id should include collaborators)
  // 2. Create execution artifact with { verdict: "PASS"|"NEEDS_CHANGES"|"FAIL" }
  // 3. Update collaborator last_action to verdict

  throw new Error(
    "simulateQaVerdict not yet implemented — needs API investigation (Phase 1)"
  );
}

/**
 * Create a PR in the sandbox repo and link it to a task.
 *
 * Used when the demo needs a PR to exist before triggering watchers
 * (e.g., Manual QA Trigger demo).
 */
export async function createPrForTask(
  request: APIRequestContext,
  taskId: string,
  branch: string,
  repo?: Partial<DemoRepoConfig>
): Promise<{ prNumber: number; prUrl: string }> {
  const config = { ...DEMO_REPO_CONFIG, ...repo };
  await apiLogin(request);

  // TODO Phase 1: Implement
  // 1. gh pr create on the branch
  // 2. POST /api/merges to link PR to task attempt

  throw new Error(
    "createPrForTask not yet implemented — needs sandbox repo (Phase 0)"
  );
}
