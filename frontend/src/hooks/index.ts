import { useState, useEffect } from 'react';

// Simple debounced value hook
export function useDebouncedValue<T>(value: T, delay: number): T {
  const [debouncedValue, setDebouncedValue] = useState<T>(value);

  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedValue(value);
    }, delay);

    return () => {
      clearTimeout(timer);
    };
  }, [value, delay]);

  return debouncedValue;
}

export { useBranchStatus } from './useBranchStatus';
export { useAttemptExecution } from './useAttemptExecution';
export { useOpenInEditor } from './useOpenInEditor';
export { useDevServer } from './useDevServer';
export { useRebase } from './useRebase';
export { useMerge } from './useMerge';
export { usePush } from './usePush';
export { useKeyboardShortcut } from './useKeyboardShortcut';
export { useExecutionSummary, useUpdateExecutionSummaryFeedback } from './useExecutionSummary';
export {
  useAgents,
  useActiveAgents,
  useAgent,
  useAgentByName,
  useCreateAgent,
  useUpdateAgent,
  useDeleteAgent,
  useSeedCoreAgents,
  useUpdateAgentStatus,
  useAssignAgentWallet,
} from './useAgents';
export {
  useProjectCapacity,
  useActiveSlots,
  useActiveExecutions,
  canStartExecution,
} from './useProjectCapacity';
export {
  useMissionControlDashboard,
  useExecutionArtifacts,
  useExecutionPlan,
  useActivePlans,
  parsePlanSteps,
} from './useMissionControl';
export type {
  AgentTaskPlan,
  ExecutionArtifact,
  ActiveExecutionInfo,
  ProjectExecutionSummary,
  MissionControlDashboard,
} from './useMissionControl';
export {
  useBowserSummary,
  useActiveSessions,
  useSession,
  useSessionDetails,
  useScreenshots,
  useScreenshotsWithDiffs,
  useActions,
  useAllowlist,
  useCheckUrl,
  useStartSession,
  useCloseSession,
  useNavigate,
  useAddToAllowlist,
  useRemoveFromAllowlist,
} from './useBowser';
export type {
  BrowserType,
  SessionStatus,
  ActionType,
  ActionResult,
  PatternType,
  BrowserSession,
  BrowserScreenshot,
  BrowserAction,
  BrowserAllowlist,
  BrowserSessionDetails,
  BowserSummary,
} from './useBowser';
export {
  useCollaborationState,
  usePauseHistory,
  useHandoffs,
  useInjections,
  usePendingInjections,
  usePauseExecution,
  useResumeExecution,
  useTakeoverExecution,
  useReturnControl,
  useInjectContext,
  useAcknowledgeInjection,
  useAcknowledgeAllInjections,
} from './useCollaboration';
export type {
  ControlState,
  ActorType,
  HandoffType,
  InjectionType,
  PauseAction,
  ContextInjection,
  ExecutionHandoff,
  ExecutionPauseHistory,
  ActorInfo,
  CollaborationState,
} from './useCollaboration';
export {
  useTaskAutonomyMode,
  useCheckpointDefinitions,
  useExecutionCheckpoints,
  usePendingCheckpoints,
  useProjectGates,
  usePendingGates,
  usePendingApprovalsSummary,
  useCanProceed,
  useSetTaskAutonomyMode,
  useCreateCheckpointDefinition,
  useUpdateCheckpointDefinition,
  useDeleteCheckpointDefinition,
  useReviewCheckpoint,
  useSkipCheckpoint,
  useCreateApprovalGate,
  useDeleteApprovalGate,
  useSubmitGateApproval,
  useBypassGate,
} from './useAutonomy';
export type {
  AutonomyMode,
  CheckpointType,
  CheckpointStatus,
  GateType,
  ApprovalDecision,
  PendingGateStatus,
  CheckpointDefinition,
  ExecutionCheckpoint,
  ApprovalGate,
  PendingGate,
  GateApproval,
  PendingApprovalsSummary,
} from './useAutonomy';
export {
  useTaskCardData,
  useTasksCardData,
} from './useTaskCardData';
export {
  useWorkflowEventStream,
  useTasksWorkflowEventPolling,
  useExecutionEventStream,
} from './useWorkflowEventStream';
