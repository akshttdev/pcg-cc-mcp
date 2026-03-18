/**
 * Demo Simulation Helpers
 *
 * API-driven agent simulation for Sprint 2 demos.
 * These helpers call the same APIs that real agent completions trigger,
 * without injecting any test artifacts into the production codebase.
 *
 * Submodules:
 *   ./config     — sandbox repo configuration
 *   ./simulation — dev agent work + QA verdict simulation
 *   ./cleanup    — branch/PR cleanup after demos
 */

export { DEMO_REPO_CONFIG, type DemoRepoConfig } from "./config";
export {
  simulateDevAgentWork,
  simulateQaVerdict,
  createPrForTask,
  postDevAgentSummaryComment,
  postQaReviewComment,
  fetchPrComments,
  getPrUrl,
} from "./simulation";
export { cleanupDemoBranches, cleanupDemoPr } from "./cleanup";

// Re-export toast assertion helpers
export { waitForToast } from "./assertions";
