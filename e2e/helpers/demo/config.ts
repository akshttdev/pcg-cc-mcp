/**
 * Sandbox repo configuration for E2E demos.
 *
 * The demo sandbox is a small, disposable GitHub repo used for
 * agent simulation — real commits, real PRs, no production impact.
 */

export interface DemoRepoConfig {
  owner: string;
  name: string;
  defaultBranch: string;
  /** Local clone path (for worktree creation). Set via E2E_DEMO_REPO_LOCAL_PATH env var. */
  localPath: string;
}

export const DEMO_REPO_CONFIG: DemoRepoConfig = {
  owner: process.env.E2E_DEMO_REPO_OWNER ?? "KingBodhi",
  name: process.env.E2E_DEMO_REPO_NAME ?? "e2e-demo-sandbox",
  defaultBranch: process.env.E2E_DEMO_REPO_DEFAULT_BRANCH ?? "main",
  localPath: process.env.E2E_DEMO_REPO_LOCAL_PATH ?? "/tmp/e2e-demo-sandbox",
};
