import { cn } from '@/lib/utils';
import { useProjectCapacity, type ProjectCapacity } from '@/hooks/useProjectCapacity';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { Cpu, Globe } from 'lucide-react';

interface SlotIndicatorProps {
  projectId: string;
  className?: string;
  showLabels?: boolean;
}

interface SlotBarProps {
  used: number;
  total: number;
  type: 'agent' | 'browser';
  showLabel?: boolean;
}

function SlotBar({ used, total, type, showLabel = false }: SlotBarProps) {
  const percentage = total > 0 ? (used / total) * 100 : 0;
  const Icon = type === 'agent' ? Cpu : Globe;
  const label = type === 'agent' ? 'Agent Slots' : 'Browser Slots';
  const color = type === 'agent' ? 'bg-blue-500' : 'bg-purple-500';
  const bgColor = type === 'agent' ? 'bg-blue-100 dark:bg-blue-900/30' : 'bg-purple-100 dark:bg-purple-900/30';

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <div className="flex items-center gap-2">
          <Icon className="h-4 w-4 text-muted-foreground" />
          {showLabel && (
            <span className="text-xs text-muted-foreground min-w-[80px]">{label}</span>
          )}
          <div className={cn('relative h-2 w-20 rounded-full overflow-hidden', bgColor)}>
            <div
              className={cn('absolute left-0 top-0 h-full rounded-full transition-all', color)}
              style={{ width: `${percentage}%` }}
            />
          </div>
          <span className="text-xs text-muted-foreground min-w-[32px]">
            {used}/{total}
          </span>
        </div>
      </TooltipTrigger>
      <TooltipContent>
        <p>{label}: {used} of {total} in use</p>
        <p className="text-xs text-muted-foreground">
          {total - used} available
        </p>
      </TooltipContent>
    </Tooltip>
  );
}

export function SlotIndicator({ projectId, className, showLabels = false }: SlotIndicatorProps) {
  const { data: capacity, isLoading } = useProjectCapacity(projectId);

  if (isLoading || !capacity) {
    return (
      <div className={cn('flex items-center gap-4', className)}>
        <div className="h-4 w-32 animate-pulse rounded bg-muted" />
      </div>
    );
  }

  return (
    <div className={cn('flex items-center gap-4', className)}>
      <SlotBar
        used={capacity.active_agent_slots}
        total={capacity.max_concurrent_agents}
        type="agent"
        showLabel={showLabels}
      />
      <SlotBar
        used={capacity.active_browser_slots}
        total={capacity.max_concurrent_browser_agents}
        type="browser"
        showLabel={showLabels}
      />
    </div>
  );
}

interface CompactSlotIndicatorProps {
  projectId: string;
  className?: string;
}

export function CompactSlotIndicator({ projectId, className }: CompactSlotIndicatorProps) {
  const { data: capacity, isLoading } = useProjectCapacity(projectId);

  if (isLoading || !capacity) {
    return null;
  }

  const totalUsed = capacity.active_agent_slots + capacity.active_browser_slots;
  const totalMax = capacity.max_concurrent_agents + capacity.max_concurrent_browser_agents;

  if (totalUsed === 0) {
    return null;
  }

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <div className={cn('flex items-center gap-1 text-xs', className)}>
          <div className="relative flex">
            {Array.from({ length: capacity.active_agent_slots }).map((_, i) => (
              <div
                key={`agent-${i}`}
                className="h-2 w-2 rounded-full bg-blue-500 -ml-0.5 first:ml-0 border border-background"
              />
            ))}
            {Array.from({ length: capacity.active_browser_slots }).map((_, i) => (
              <div
                key={`browser-${i}`}
                className="h-2 w-2 rounded-full bg-purple-500 -ml-0.5 first:ml-0 border border-background"
              />
            ))}
          </div>
          <span className="text-muted-foreground">
            {totalUsed}/{totalMax}
          </span>
        </div>
      </TooltipTrigger>
      <TooltipContent>
        <p>Active Executions</p>
        <p className="text-xs">Agent: {capacity.active_agent_slots}/{capacity.max_concurrent_agents}</p>
        <p className="text-xs">Browser: {capacity.active_browser_slots}/{capacity.max_concurrent_browser_agents}</p>
      </TooltipContent>
    </Tooltip>
  );
}

export function SlotUtilizationBadge({ capacity }: { capacity: ProjectCapacity }) {
  const totalUsed = capacity.active_agent_slots + capacity.active_browser_slots;
  const totalMax = capacity.max_concurrent_agents + capacity.max_concurrent_browser_agents;
  const percentage = totalMax > 0 ? (totalUsed / totalMax) * 100 : 0;

  let statusColor = 'bg-green-500';
  let statusText = 'Available';

  if (percentage >= 100) {
    statusColor = 'bg-red-500';
    statusText = 'Full';
  } else if (percentage >= 75) {
    statusColor = 'bg-yellow-500';
    statusText = 'Limited';
  } else if (percentage >= 50) {
    statusColor = 'bg-blue-500';
    statusText = 'In Use';
  }

  return (
    <div className="flex items-center gap-2">
      <div className={cn('h-2 w-2 rounded-full', statusColor)} />
      <span className="text-xs text-muted-foreground">{statusText}</span>
    </div>
  );
}
