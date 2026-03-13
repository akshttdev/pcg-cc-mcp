import { Network } from 'lucide-react';
import { ActivityFeed } from '@/components/activity/ActivityFeed';
import type { ActivityType } from '@/types/activity';

const AGENT_ACTIVITY_TYPES: ActivityType[] = [
  'agent_tool_call',
  'agent_workflow_triggered',
  'agent_workflow_completed',
];

const EMPTY_STATE = (
  <>
    <Network className="h-12 w-12 mx-auto mb-4 opacity-30" />
    <p>No agent activity yet</p>
    <p className="text-sm mt-1">
      Activity will appear here when Topsi executes tool calls or triggers workflows.
    </p>
  </>
);

export function TopsiActivityPage() {
  return (
    <div className="container mx-auto px-4 py-8 max-w-4xl">
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-3">
          <Network className="h-6 w-6 text-cyan-600" />
          <div>
            <h1 className="text-2xl font-bold">Topsi Activity</h1>
            <p className="text-sm text-muted-foreground">
              Tool calls, workflow triggers, and agent actions
            </p>
          </div>
        </div>
      </div>

      <ActivityFeed
        filterTypes={AGENT_ACTIVITY_TYPES}
        emptyMessage={EMPTY_STATE}
        limit={200}
      />
    </div>
  );
}
