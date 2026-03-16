import type {
  TaskWithAttemptStatus,
  ExecutionArtifact,
  FlowEventType,
} from 'shared/types';
import type { TaskCardMode } from '../../EnhancedTaskCard';

// Valid flow event types for normalization
export const FLOW_EVENT_TYPES: FlowEventType[] = [
  'phase_started',
  'phase_completed',
  'artifact_created',
  'artifact_updated',
  'approval_requested',
  'approval_decision',
  'wide_research_started',
  'subagent_progress',
  'wide_research_completed',
  'agent_handoff',
  'flow_paused',
  'flow_resumed',
  'flow_failed',
  'flow_completed',
];

export function normalizeFlowEventType(value: string): FlowEventType {
  return FLOW_EVENT_TYPES.includes(value as FlowEventType)
    ? (value as FlowEventType)
    : 'flow_completed';
}

export function detectCardMode(task: TaskWithAttemptStatus, artifacts: ExecutionArtifact[]): TaskCardMode {
  const tags = task.tags?.toLowerCase() || '';

  // Check tags first
  if (tags.includes('visual') || tags.includes('design') || tags.includes('image')) {
    return 'visual';
  }
  if (tags.includes('document') || tags.includes('writing') || tags.includes('content')) {
    return 'document';
  }
  if (tags.includes('media') || tags.includes('video') || tags.includes('recording')) {
    return 'media';
  }
  if (tags.includes('code') || tags.includes('development') || tags.includes('programming')) {
    return 'terminal';
  }

  // Check artifacts - use actual ArtifactType values
  const artifactTypes = artifacts.map((a) => a.artifact_type);
  if (artifactTypes.some((t) => t === 'visual_brief' || t === 'screenshot')) {
    return 'visual';
  }
  if (
    artifactTypes.some(
      (t) =>
        t === 'content_draft' ||
        t === 'research_report' ||
        t === 'strategy_document' ||
        t === 'content_calendar' ||
        t === 'competitor_analysis'
    )
  ) {
    return 'document';
  }
  if (artifactTypes.some((t) => t === 'browser_recording' || t === 'walkthrough')) {
    return 'media';
  }
  if (artifactTypes.some((t) => t === 'diff_summary' || t === 'test_result' || t === 'checkpoint')) {
    return 'terminal';
  }

  // Default based on assigned agent
  const agentName = task.assigned_agent?.toLowerCase() || '';
  if (agentName.includes('design') || agentName.includes('visual')) return 'visual';
  if (agentName.includes('writer') || agentName.includes('content')) return 'document';

  return 'terminal';
}
