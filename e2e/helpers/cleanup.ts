import { APIRequestContext } from "@playwright/test";
import { apiLogin } from "./auth";
import { TEST_DATA_PREFIX } from "./tasks";

/** Delete a project via API (for afterAll). */
export async function cleanupProject(request: APIRequestContext, projectId: string) {
  await apiLogin(request);
  await request.delete(`/api/projects/${projectId}`).catch(() => {});
}

/** Delete a task by extracting its ID from a task detail URL path. */
export async function cleanupTaskByPath(request: APIRequestContext, taskPath: string) {
  await apiLogin(request);
  const taskId = taskPath.split("/tasks/")[1];
  if (taskId) {
    await request.delete(`/api/tasks/${taskId}`).catch(() => {});
  }
}

/** Clean up test data created during E2E runs */
export async function cleanupTestData(request: APIRequestContext) {
  await apiLogin(request);

  // Get all projects
  const projectsRes = await request.get("/api/projects");
  if (!projectsRes.ok()) return;
  const projects = (await projectsRes.json()).data || [];

  // For each project, delete tasks with E2E prefix
  for (const project of projects) {
    const tasksRes = await request.get(`/api/projects/${project.id}/tasks`);
    if (!tasksRes.ok()) continue;
    // Guard against HTML responses (SPA fallback)
    const contentType = tasksRes.headers()["content-type"] || "";
    if (!contentType.includes("application/json")) continue;
    let responseData;
    try { responseData = await tasksRes.json(); } catch { continue; }
    const tasks = responseData.data || responseData || [];
    if (!Array.isArray(tasks)) continue;

    for (const task of tasks) {
      if (task.title?.startsWith(TEST_DATA_PREFIX)) {
        await request.delete(`/api/tasks/${task.id}`).catch(() => {});
      }
    }
  }

  // Delete test projects
  for (const project of projects) {
    if (project.name?.startsWith(TEST_DATA_PREFIX)) {
      await request.delete(`/api/projects/${project.id}`).catch(() => {});
    }
  }
}
