import { cn } from '@/lib/utils';
import { differenceInMinutes, format } from 'date-fns';
import type { ActiveExecutionInfo } from '@/hooks/useMissionControl';

interface ExecutionTimelineProps {
  executions: ActiveExecutionInfo[];
  className?: string;
}

interface TimelineBarProps {
  execution: ActiveExecutionInfo;
  startTime: Date;
  totalMinutes: number;
}

function getExecutorColor(executor: string): string {
  // Generate consistent color based on executor name
  const colors = [
    'bg-blue-500',
    'bg-green-500',
    'bg-purple-500',
    'bg-orange-500',
    'bg-pink-500',
    'bg-cyan-500',
    'bg-yellow-500',
    'bg-red-500',
  ];
  const hash = executor.split('').reduce((acc, char) => acc + char.charCodeAt(0), 0);
  return colors[hash % colors.length];
}

function TimelineBar({ execution, startTime, totalMinutes }: TimelineBarProps) {
  const execStart = new Date(execution.process.started_at);
  const now = new Date();

  // Calculate position and width
  const startOffset = differenceInMinutes(execStart, startTime);
  const duration = differenceInMinutes(now, execStart);

  const left = totalMinutes > 0 ? (startOffset / totalMinutes) * 100 : 0;
  const width = totalMinutes > 0 ? Math.min((duration / totalMinutes) * 100, 100 - left) : 0;

  const color = getExecutorColor(execution.executor);

  return (
    <div className="group relative h-8 flex items-center">
      {/* Label */}
      <div className="w-32 shrink-0 pr-2 text-right">
        <span className="text-xs font-medium truncate block" title={execution.task_title}>
          {execution.task_title.length > 20
            ? `${execution.task_title.slice(0, 20)}...`
            : execution.task_title}
        </span>
        <span className="text-[10px] text-muted-foreground">{execution.executor}</span>
      </div>

      {/* Timeline bar container */}
      <div className="flex-1 h-6 bg-muted/30 rounded relative overflow-hidden">
        {/* Execution bar */}
        <div
          className={cn(
            'absolute h-full rounded transition-all',
            color,
            execution.process.status === 'running' && 'animate-pulse'
          )}
          style={{
            left: `${left}%`,
            width: `${Math.max(width, 1)}%`,
          }}
        />

        {/* Hover tooltip */}
        <div
          className="absolute inset-0 opacity-0 group-hover:opacity-100 transition-opacity"
          style={{
            left: `${left}%`,
            width: `${Math.max(width, 10)}%`,
          }}
        >
          <div className="absolute -top-8 left-1/2 -translate-x-1/2 bg-popover text-popover-foreground text-xs px-2 py-1 rounded shadow-md whitespace-nowrap z-10">
            {format(execStart, 'HH:mm')} - {duration}min
          </div>
        </div>
      </div>
    </div>
  );
}

export function ExecutionTimeline({ executions, className }: ExecutionTimelineProps) {
  if (executions.length === 0) {
    return (
      <div className={cn('text-center text-muted-foreground py-8', className)}>
        No active executions
      </div>
    );
  }

  // Calculate time range (last 2 hours or from earliest execution)
  const now = new Date();
  const twoHoursAgo = new Date(now.getTime() - 2 * 60 * 60 * 1000);

  const earliestStart = executions.reduce((earliest, exec) => {
    const start = new Date(exec.process.started_at);
    return start < earliest ? start : earliest;
  }, now);

  const startTime = earliestStart < twoHoursAgo ? earliestStart : twoHoursAgo;
  const totalMinutes = differenceInMinutes(now, startTime);

  // Generate time markers
  const markers: Date[] = [];
  const interval = totalMinutes > 60 ? 30 : 15; // 30 min or 15 min intervals
  let markerTime = new Date(startTime);
  markerTime.setMinutes(Math.ceil(markerTime.getMinutes() / interval) * interval, 0, 0);

  while (markerTime <= now) {
    markers.push(new Date(markerTime));
    markerTime = new Date(markerTime.getTime() + interval * 60 * 1000);
  }

  return (
    <div className={cn('space-y-2', className)}>
      {/* Time axis */}
      <div className="flex items-center">
        <div className="w-32 shrink-0" />
        <div className="flex-1 relative h-6">
          {markers.map((marker, i) => {
            const offset = differenceInMinutes(marker, startTime);
            const left = totalMinutes > 0 ? (offset / totalMinutes) * 100 : 0;

            return (
              <div
                key={i}
                className="absolute text-[10px] text-muted-foreground -translate-x-1/2"
                style={{ left: `${left}%` }}
              >
                {format(marker, 'HH:mm')}
              </div>
            );
          })}
        </div>
      </div>

      {/* Timeline bars */}
      <div className="space-y-1">
        {executions.map((execution) => (
          <TimelineBar
            key={execution.process.id}
            execution={execution}
            startTime={startTime}
            totalMinutes={totalMinutes}
          />
        ))}
      </div>

      {/* Legend */}
      <div className="flex items-center gap-4 mt-4 pt-2 border-t text-xs text-muted-foreground">
        <div className="flex items-center gap-1">
          <div className="w-3 h-3 rounded bg-blue-500" />
          <span>Agent Active</span>
        </div>
        <div className="flex items-center gap-1">
          <div className="w-3 h-3 rounded bg-blue-500 animate-pulse" />
          <span>Running</span>
        </div>
      </div>
    </div>
  );
}
