import { useCallback, useMemo } from 'react';
import {
  KanbanProvider,
  KanbanBoard,
  KanbanHeader,
  KanbanCards,
  KanbanCard,
} from '@/components/ui/shadcn-io/kanban';
import { AgentFlowCard } from './AgentFlowCard';
import { Badge } from '@/components/ui/badge';
import { Loader } from '@/components/ui/loader';
import type { AgentFlow, FlowStatus } from 'shared/types';
import type { DragEndEvent } from '@dnd-kit/core';

interface AgentFlowBoardProps {
  flows: AgentFlow[];
  isLoading?: boolean;
  onFlowClick?: (flow: AgentFlow) => void;
  onFlowApprove?: (flow: AgentFlow) => void;
  onFlowReject?: (flow: AgentFlow) => void;
  onStatusChange?: (flowId: string, newStatus: FlowStatus) => void;
}

const columns: { id: FlowStatus; name: string; color: string }[] = [
  { id: 'planning', name: 'Planning', color: '--blue-500' },
  { id: 'executing', name: 'Executing', color: '--yellow-500' },
  { id: 'verifying', name: 'Verifying', color: '--purple-500' },
  { id: 'awaiting_approval', name: 'Awaiting Approval', color: '--orange-500' },
  { id: 'completed', name: 'Completed', color: '--green-500' },
];

export function AgentFlowBoard({
  flows,
  isLoading,
  onFlowClick,
  onFlowApprove,
  onFlowReject,
  onStatusChange,
}: AgentFlowBoardProps) {
  // Group flows by status
  const flowsByStatus = useMemo(() => {
    const grouped: Record<FlowStatus, AgentFlow[]> = {
      planning: [],
      executing: [],
      verifying: [],
      awaiting_approval: [],
      completed: [],
      failed: [],
      paused: [],
    };

    flows.forEach((flow) => {
      if (grouped[flow.status]) {
        grouped[flow.status].push(flow);
      }
    });

    return grouped;
  }, [flows]);

  const handleDragEnd = useCallback(
    (event: DragEndEvent) => {
      const { active, over } = event;

      if (!over || active.id === over.id) return;

      const flowId = active.id as string;
      const newStatus = over.id as FlowStatus;

      // Find the flow
      const flow = flows.find((f) => f.id === flowId);
      if (!flow) return;

      // Prevent invalid transitions
      const validTransitions: Record<FlowStatus, FlowStatus[]> = {
        planning: ['executing', 'failed', 'paused'],
        executing: ['verifying', 'failed', 'paused'],
        verifying: ['awaiting_approval', 'completed', 'failed'],
        awaiting_approval: ['completed', 'failed'],
        completed: [],
        failed: ['planning'],
        paused: ['planning', 'executing'],
      };

      if (!validTransitions[flow.status]?.includes(newStatus)) {
        return;
      }

      onStatusChange?.(flowId, newStatus);
    },
    [flows, onStatusChange]
  );

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader />
      </div>
    );
  }

  return (
    <KanbanProvider onDragEnd={handleDragEnd} className="h-full">
      {columns.map((column) => {
        const columnFlows = flowsByStatus[column.id] || [];
        return (
          <KanbanBoard key={column.id} id={column.id}>
            <KanbanHeader
              name={column.name}
              color={column.color}
              className="justify-between"
            >
              <div className="flex items-center gap-2">
                <div
                  className="h-2 w-2 rounded-full"
                  style={{ backgroundColor: `var(${column.color})` }}
                />
                <span className="text-sm font-medium">{column.name}</span>
                <Badge variant="secondary" className="text-xs">
                  {columnFlows.length}
                </Badge>
              </div>
            </KanbanHeader>
            <KanbanCards className="p-2 gap-2 overflow-y-auto">
              {columnFlows.map((flow, index) => (
                <KanbanCard
                  key={flow.id}
                  id={flow.id}
                  name={flow.flow_type}
                  index={index}
                  parent={column.id}
                >
                  <AgentFlowCard
                    flow={flow}
                    onClick={() => onFlowClick?.(flow)}
                    onApprove={() => onFlowApprove?.(flow)}
                    onReject={() => onFlowReject?.(flow)}
                  />
                </KanbanCard>
              ))}
            </KanbanCards>
          </KanbanBoard>
        );
      })}
    </KanbanProvider>
  );
}
