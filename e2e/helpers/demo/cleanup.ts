import { APIRequestContext } from "@playwright/test";
import { DEMO_REPO_CONFIG, type DemoRepoConfig } from "./config";

/**
 * Delete remote branches created during demo runs.
 * Uses the GitHub API via the test request context.
 */
export async function cleanupDemoBranches(
  request: APIRequestContext,
  branches: string[],
  repo?: Partial<DemoRepoConfig>
): Promise<void> {
  const config = { ...DEMO_REPO_CONFIG, ...repo };

  for (const branch of branches) {
    // DELETE /repos/:owner/:repo/git/refs/heads/:branch
    await request
      .delete(
        `https://api.github.com/repos/${config.owner}/${config.name}/git/refs/heads/${branch}`,
        {
          headers: {
            Authorization: `token ${process.env.GITHUB_TOKEN}`,
            Accept: "application/vnd.github.v3+json",
          },
        }
      )
      .catch(() => {
        // Branch may already be deleted (auto-delete on PR merge)
      });
  }
}

/**
 * Close a PR created during demo runs.
 */
export async function cleanupDemoPr(
  request: APIRequestContext,
  prNumber: number,
  repo?: Partial<DemoRepoConfig>
): Promise<void> {
  const config = { ...DEMO_REPO_CONFIG, ...repo };

  await request
    .patch(
      `https://api.github.com/repos/${config.owner}/${config.name}/pulls/${prNumber}`,
      {
        headers: {
          Authorization: `token ${process.env.GITHUB_TOKEN}`,
          Accept: "application/vnd.github.v3+json",
        },
        data: { state: "closed" },
      }
    )
    .catch(() => {
      // PR may already be closed
    });
}
