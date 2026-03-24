import { Bot, User } from 'lucide-react';
import type { TaskCollaborator, AgentWithParsedFields } from 'shared/types';
import type { UserListItem } from '@/lib/api';

interface CollaboratorAvatarsProps {
  collaborators: TaskCollaborator[];
  usersMap?: Map<string, UserListItem>;
  agentsMap?: Map<string, AgentWithParsedFields>;
  maxVisible?: number;
}

export function CollaboratorAvatars({
  collaborators,
  usersMap,
  agentsMap,
  maxVisible = 3,
}: CollaboratorAvatarsProps) {
  if (!collaborators.length) return null;

  const tooltipText = collaborators.map(c => {
    const isAgent = c.actor_type === 'agent' || c.actor_type === 'agent_watcher';
    const name = isAgent
      ? agentsMap?.get(c.actor_id)?.short_name
      : usersMap?.get(c.actor_id)?.full_name;
    return name ? `${name} (${c.actor_type})` : `${c.actor_id.slice(0, 8)} (${c.actor_type})`;
  }).join(', ');

  return (
    <div className="flex items-center gap-1" title={tooltipText}>
      <div className="flex -space-x-1">
        {collaborators.slice(0, maxVisible).map((collaborator, idx) => {
          const isAgent = collaborator.actor_type === 'agent' || collaborator.actor_type === 'agent_watcher';
          const agentName = isAgent ? agentsMap?.get(collaborator.actor_id)?.short_name : undefined;
          return (
            <div
              key={`${collaborator.actor_id}-${idx}`}
              className={`h-4 w-4 rounded-full flex items-center justify-center text-[8px] font-medium border border-background ${
                isAgent
                  ? 'bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300'
                  : 'bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300'
              }`}
              title={agentName ?? collaborator.actor_id.slice(0, 8)}
            >
              {isAgent
                ? <Bot className="h-2.5 w-2.5" />
                : <User className="h-2.5 w-2.5" />
              }
            </div>
          );
        })}
        {collaborators.length > maxVisible && (
          <div className="h-4 w-4 rounded-full flex items-center justify-center text-[8px] font-medium border border-background bg-muted text-muted-foreground">
            +{collaborators.length - maxVisible}
          </div>
        )}
      </div>
      {/* Agent name labels */}
      <AgentNameLabels collaborators={collaborators} agentsMap={agentsMap} />
    </div>
  );
}

function AgentNameLabels({
  collaborators,
  agentsMap,
}: {
  collaborators: TaskCollaborator[];
  agentsMap?: Map<string, AgentWithParsedFields>;
}) {
  const agentCollabs = collaborators.filter(
    c => c.actor_type === 'agent' || c.actor_type === 'agent_watcher'
  );
  if (agentCollabs.length === 0) return null;

  const names = agentCollabs
    .map(c => agentsMap?.get(c.actor_id)?.short_name)
    .filter(Boolean);
  if (names.length === 0) return null;

  return (
    <span className="text-xs text-blue-600 dark:text-blue-400 truncate max-w-[80px]">
      {names.join(', ')}
    </span>
  );
}
