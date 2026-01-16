import { useQuery } from '@tanstack/react-query';
import { taskArtifactsApi, agentFlowsApi } from '@/lib/api';
import type { ExecutionArtifact, AgentFlowEvent, ArtifactType, FlowEventType } from 'shared/types';

// Valid flow event types for normalization
const FLOW_EVENT_TYPES: FlowEventType[] = [
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

function normalizeFlowEventType(value: string): FlowEventType {
  return FLOW_EVENT_TYPES.includes(value as FlowEventType)
    ? (value as FlowEventType)
    : 'flow_completed';
}

// Normalize API events to shared types
function normalizeEvents(events: Awaited<ReturnType<typeof agentFlowsApi.getEvents>>): AgentFlowEvent[] {
  return events.map((event) => ({
    ...event,
    event_type: normalizeFlowEventType(event.event_type),
    event_data: event.event_data ?? '',
  }));
}

interface TaskCardData {
  artifacts: ExecutionArtifact[];
  primaryArtifact: ExecutionArtifact | undefined;
  workflowEvents: AgentFlowEvent[];
}

/**
 * Fetch enriched data for a single task card
 * Includes artifacts and workflow events for enhanced card display
 */
export function useTaskCardData(taskId: string | undefined) {
  // Fetch artifacts for this task
  const {
    data: artifactsData,
    isLoading: artifactsLoading,
  } = useQuery({
    queryKey: ['taskArtifacts', taskId],
    queryFn: async () => {
      if (!taskId) return [];
      const result = await taskArtifactsApi.list(taskId);
      return result
        .map((item) => item.artifact)
        .filter((a): a is NonNullable<typeof a> => Boolean(a))
        .map((artifact) => ({
          id: artifact.id,
          execution_process_id: artifact.execution_process_id ?? '',
          artifact_type: artifact.artifact_type as ArtifactType,
          title: artifact.title ?? 'Untitled',
          content: artifact.content ?? null,
          file_path: artifact.file_path ?? null,
          metadata: artifact.metadata ?? null,
          created_at: artifact.created_at,
        }));
    },
    enabled: !!taskId,
    staleTime: 60 * 1000, // 1 minute
  });

  // Fetch workflow events for this task
  const {
    data: workflowData,
    isLoading: workflowLoading,
  } = useQuery({
    queryKey: ['taskWorkflowEvents', taskId],
    queryFn: async () => {
      if (!taskId) return [];
      const flows = await agentFlowsApi.list({ task_id: taskId });
      if (flows.length === 0) return [];
      // Get events from the most recent flow
      const latestFlow = flows.sort(
        (a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
      )[0];
      const events = await agentFlowsApi.getEvents(latestFlow.id);
      return normalizeEvents(events);
    },
    enabled: !!taskId,
    staleTime: 30 * 1000, // 30 seconds
  });

  const artifacts = artifactsData ?? [];
  const workflowEvents = workflowData ?? [];

  // Determine primary artifact (first visual/document artifact, or first overall)
  const primaryArtifact = findPrimaryArtifact(artifacts);

  return {
    artifacts,
    primaryArtifact,
    workflowEvents,
    isLoading: artifactsLoading || workflowLoading,
  };
}

/**
 * Batch fetch card data for multiple tasks
 * More efficient for kanban board views
 */
export function useTasksCardData(taskIds: string[]) {
  // Batch fetch all artifacts
  const {
    data: allArtifacts,
    isLoading: artifactsLoading,
  } = useQuery({
    queryKey: ['tasksArtifacts', taskIds.sort().join(',')],
    queryFn: async () => {
      if (taskIds.length === 0) return new Map<string, ExecutionArtifact[]>();

      const artifactsByTask = new Map<string, ExecutionArtifact[]>();

      // Fetch artifacts for each task (could be optimized with a batch endpoint)
      await Promise.all(
        taskIds.map(async (taskId) => {
          try {
            const result = await taskArtifactsApi.list(taskId);
            const artifacts = result
              .map((item) => item.artifact)
              .filter((a): a is NonNullable<typeof a> => Boolean(a))
              .map((artifact) => ({
                id: artifact.id,
                execution_process_id: artifact.execution_process_id ?? '',
                artifact_type: artifact.artifact_type as ArtifactType,
                title: artifact.title ?? 'Untitled',
                content: artifact.content ?? null,
                file_path: artifact.file_path ?? null,
                metadata: artifact.metadata ?? null,
                created_at: artifact.created_at,
              }));
            artifactsByTask.set(taskId, artifacts);
          } catch {
            artifactsByTask.set(taskId, []);
          }
        })
      );

      return artifactsByTask;
    },
    enabled: taskIds.length > 0,
    staleTime: 60 * 1000,
  });

  // Batch fetch all workflow events
  const {
    data: allWorkflowEvents,
    isLoading: workflowLoading,
  } = useQuery({
    queryKey: ['tasksWorkflowEvents', taskIds.sort().join(',')],
    queryFn: async () => {
      if (taskIds.length === 0) return new Map<string, AgentFlowEvent[]>();

      const eventsByTask = new Map<string, AgentFlowEvent[]>();

      // Fetch all flows first
      const allFlows = await agentFlowsApi.list();
      const flowsByTask = new Map<string, typeof allFlows[0]>();

      // Get latest flow for each task
      for (const flow of allFlows) {
        if (!taskIds.includes(flow.task_id)) continue;
        const existing = flowsByTask.get(flow.task_id);
        if (!existing || new Date(flow.created_at) > new Date(existing.created_at)) {
          flowsByTask.set(flow.task_id, flow);
        }
      }

      // Fetch events for each task's flow
      await Promise.all(
        Array.from(flowsByTask.entries()).map(async ([taskId, flow]) => {
          try {
            const events = await agentFlowsApi.getEvents(flow.id);
            eventsByTask.set(taskId, normalizeEvents(events));
          } catch {
            eventsByTask.set(taskId, []);
          }
        })
      );

      // Initialize empty arrays for tasks without flows
      for (const taskId of taskIds) {
        if (!eventsByTask.has(taskId)) {
          eventsByTask.set(taskId, []);
        }
      }

      return eventsByTask;
    },
    enabled: taskIds.length > 0,
    staleTime: 30 * 1000,
  });

  // Build enriched data map
  const cardDataMap = new Map<string, TaskCardData>();

  for (const taskId of taskIds) {
    const artifacts = allArtifacts?.get(taskId) ?? [];
    const workflowEvents = allWorkflowEvents?.get(taskId) ?? [];
    const primaryArtifact = findPrimaryArtifact(artifacts);

    cardDataMap.set(taskId, {
      artifacts,
      primaryArtifact,
      workflowEvents,
    });
  }

  return {
    cardDataMap,
    isLoading: artifactsLoading || workflowLoading,
  };
}

/**
 * Find the primary artifact for card preview
 * Priority: visual > document > media > any
 */
function findPrimaryArtifact(artifacts: ExecutionArtifact[]): ExecutionArtifact | undefined {
  if (artifacts.length === 0) return undefined;

  const visualTypes: ArtifactType[] = ['screenshot', 'visual_brief', 'platform_screenshot'];
  const documentTypes: ArtifactType[] = ['research_report', 'strategy_document', 'content_draft', 'content_calendar', 'competitor_analysis'];
  const mediaTypes: ArtifactType[] = ['walkthrough', 'browser_recording'];

  // Try to find visual artifact first
  const visual = artifacts.find((a) => visualTypes.includes(a.artifact_type));
  if (visual) return visual;

  // Then document
  const document = artifacts.find((a) => documentTypes.includes(a.artifact_type));
  if (document) return document;

  // Then media
  const media = artifacts.find((a) => mediaTypes.includes(a.artifact_type));
  if (media) return media;

  // Fallback to first artifact
  return artifacts[0];
}
