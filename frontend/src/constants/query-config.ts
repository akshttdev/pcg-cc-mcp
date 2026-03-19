/** Centralized React Query timing configuration.
 *
 * Import these instead of hardcoding staleTime / refetchInterval
 * so all dashboards share consistent cache behavior.
 */

/** Data that changes infrequently (org settings, pipeline stages) */
export const STALE_SLOW = 10 * 60 * 1000; // 10 minutes

/** Default for most data (tasks, deals, costs) */
export const STALE_DEFAULT = 5 * 60 * 1000; // 5 minutes

/** Data that should feel near-real-time (agent status, process logs) */
export const STALE_FAST = 30 * 1000; // 30 seconds

/** Polling interval for live dashboards */
export const REFETCH_LIVE = 15 * 1000; // 15 seconds

/** Polling interval for background data */
export const REFETCH_BACKGROUND = 60 * 1000; // 60 seconds
