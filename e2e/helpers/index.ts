/**
 * E2E Helpers — barrel export
 *
 * Submodules:
 *   ./auth       — login, apiLogin, ensureAuthenticated
 *   ./timing     — t() timeout scaler
 *   ./navigation — page navigation helpers
 *   ./tasks      — task/project CRUD, kanban interactions
 *   ./agents     — agent seeding
 *   ./ui         — feedback dialog, view-as, settings
 *   ./cleanup    — test data cleanup
 *   ./demo/      — Sprint 2 agent simulation helpers
 */

export { TEST_USER, login, apiLogin, ensureAuthenticated } from "./auth";
export { t } from "./timing";
export {
  navigateToFirstProjectTasks,
  navigateToProjectTasks,
  navigateToTaskDetail,
  openNotifications,
} from "./navigation";
export {
  TEST_DATA_PREFIX,
  createDemoProject,
  createTaskViaUI,
  changeTaskStatus,
  addQaWatcher,
  findTaskCard,
} from "./tasks";
export { ensureAgentsSeeded } from "./agents";
export {
  openFeedbackDialog,
  setViewAsRole,
  clearViewAsRole,
  settingsTab,
  getViewAsRole,
  waitForSettingsReady,
} from "./ui";
export {
  cleanupProject,
  cleanupTaskByPath,
  cleanupTestData,
  cleanupE2eDataSources,
} from "./cleanup";
