import { Page, APIRequestContext, expect } from "@playwright/test";
import { t } from "./timing";
import { apiLogin } from "./auth";

/** Prefix for all test-created data — makes cleanup easy */
export const TEST_DATA_PREFIX = "[E2E]";

const DEMO_ORG_ID = "01010101-0101-0101-0101-010101010101"; // Powerclub Global

/**
 * Create a fresh, empty project via API for demo use.
 * Returns the project ID. The project belongs to the Powerclub Global org
 * so it appears in the sidebar.
 */
export async function createDemoProject(
  request: APIRequestContext,
  name?: string
): Promise<string> {
  await apiLogin(request);
  const projectName = name ?? `${TEST_DATA_PREFIX} Demo Project ${Date.now()}`;
  const uniquePath = `/tmp/e2e-demo-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
  const res = await request.post("/api/projects", {
    data: {
      name: projectName,
      git_repo_path: uniquePath,
      use_existing_repo: false,
      organization_id: DEMO_ORG_ID,
    },
  });
  expect(res.ok()).toBeTruthy();
  const body = await res.json();
  return body.data?.id ?? body.id;
}

/**
 * Create a task via the UI. Assumes the kanban page is already loaded.
 * Returns after the task detail drawer opens (URL contains the new task ID).
 */
export async function createTaskViaUI(
  page: Page,
  title: string,
  opts?: { description?: string; demoPause?: number }
) {
  await page.getByRole("button", { name: "Create new task" }).click();
  await expect(page.getByRole("textbox", { name: "Title" })).toBeVisible({ timeout: t(5_000) });

  await page.getByRole("textbox", { name: "Title" }).fill(title);

  if (opts?.description) {
    const descArea = page.getByRole("textbox", { name: /Supports Markdown/i });
    await descArea.fill(opts.description);
  }

  if (opts?.demoPause) await page.waitForTimeout(opts.demoPause);

  await page.getByRole("button", { name: "Create Task", exact: true }).click();

  // After creation, drawer auto-opens with Agent Reviewers section
  await expect(page.getByText("Agent Reviewers", { exact: true })).toBeVisible({
    timeout: t(10_000),
  });
}

/**
 * Change task status via the combobox in the task detail drawer.
 * Expects the drawer to already be open.
 */
export async function changeTaskStatus(
  page: Page,
  from: string,
  to: string,
  opts?: { demoPause?: number }
) {
  const statusTrigger = page.getByRole("combobox").filter({ hasText: from });
  await expect(statusTrigger).toBeVisible({ timeout: t(10_000) });

  await statusTrigger.click();
  await page.getByRole("option", { name: to }).click();

  await expect(page.getByText(`Status changed to ${to}`)).toBeVisible({
    timeout: t(5_000),
  });

  if (opts?.demoPause) await page.waitForTimeout(opts.demoPause);
}

/** Add a QA watcher from the task detail drawer. */
export async function addQaWatcher(page: Page, opts?: { demoPause?: number }) {
  await page.getByRole("button", { name: "Add", exact: true }).click();
  await page.getByRole("button", { name: /ORCHA QA/i }).first().click();
  await expect(page.getByText("Watching")).toBeVisible({ timeout: t(5_000) });
  if (opts?.demoPause) await page.waitForTimeout(opts.demoPause);
}

/**
 * Find a task card on the kanban board by its title.
 * Strips the TEST_DATA_PREFIX and matches as a regex.
 */
export function findTaskCard(page: Page, title: string) {
  const titleFragment = title
    .replace(`${TEST_DATA_PREFIX} `, "")
    .replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return page.getByRole("button", { name: new RegExp(titleFragment) }).first();
}
