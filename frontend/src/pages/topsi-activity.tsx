import { Network } from 'lucide-react';
import { ActivityFeed } from '@/components/activity/ActivityFeed';
import type { ActivityType } from '@/types/activity';

const AGENT_ACTIVITY_TYPES: ActivityType[] = [
  'agent_tool_call',
  'agent_workflow_triggered',
  'agent_workflow_completed',
];

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

      <ActivityFeed filterTypes={AGENT_ACTIVITY_TYPES} limit={200} />
    </div>
  );
}
