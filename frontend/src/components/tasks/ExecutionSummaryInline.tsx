import { Badge } from '@/components/ui/badge';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import {
  FileEdit,
  FilePlus,
  FileX,
  Terminal,
  AlertTriangle,
  Clock,
  Wrench,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { ExecutionSummary, ExecutionSummaryBrief } from 'shared/types';

type SummaryVariant = ExecutionSummary | ExecutionSummaryBrief;

interface ExecutionSummaryInlineProps {
  summary: SummaryVariant;
  compact?: boolean;
  className?: string;
}

const isFullSummary = (summary: SummaryVariant): summary is ExecutionSummary =>
  'files_deleted' in summary;

const toNumber = (value?: number | bigint | null): number =>
  typeof value === 'bigint' ? Number(value) : value ?? 0;

const formatDuration = (ms?: number | bigint | null) => {
  const value = toNumber(ms);
  if (!value) return '--';
  if (value < 1000) return `${value}ms`;
  if (value < 60000) return `${(value / 1000).toFixed(1)}s`;
  const mins = Math.floor(value / 60000);
  const secs = Math.floor((value % 60000) / 1000);
  return `${mins}m ${secs}s`;
};

const extractTools = (summary: SummaryVariant): string[] => {
  const raw = summary.tools_used;
  if (!raw) return [];
  if (Array.isArray(raw)) return raw.filter(Boolean);
  try {
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
};

export function ExecutionSummaryInline({
  summary,
  compact = false,
  className,
}: ExecutionSummaryInlineProps) {
  const filesModified = toNumber(summary.files_modified);
  const filesCreated = toNumber(summary.files_created);
  const filesDeleted = isFullSummary(summary)
    ? toNumber(summary.files_deleted)
    : 0;
  const totalFiles = filesModified + filesCreated + filesDeleted;
  const commandsRun = isFullSummary(summary)
    ? toNumber(summary.commands_run)
    : 0;
  const commandsFailed = toNumber(summary.commands_failed);
  const tools = extractTools(summary);
  const duration = toNumber(summary.execution_time_ms);
  const errorSummary = isFullSummary(summary)
    ? summary.error_summary
    : null;

  if (compact) {
    return (
      <TooltipProvider>
        <div className={cn('flex items-center gap-1.5 text-[10px] text-muted-foreground', className)}>
          {totalFiles > 0 && (
            <Tooltip>
              <TooltipTrigger asChild>
                <span className="flex items-center gap-0.5">
                  <FileEdit className="h-3 w-3" />
                  {totalFiles}
                </span>
              </TooltipTrigger>
              <TooltipContent side="top" className="text-xs">
                <div className="space-y-0.5">
                  {filesModified > 0 && <p>{filesModified} modified</p>}
                  {filesCreated > 0 && <p>{filesCreated} created</p>}
                  {filesDeleted > 0 && <p>{filesDeleted} deleted</p>}
                </div>
              </TooltipContent>
            </Tooltip>
          )}

          {commandsFailed > 0 && (
            <Tooltip>
              <TooltipTrigger asChild>
                <span className="flex items-center gap-0.5 text-destructive">
                  <AlertTriangle className="h-3 w-3" />
                  {commandsFailed}
                </span>
              </TooltipTrigger>
              <TooltipContent side="top" className="text-xs">
                {commandsFailed} command{commandsFailed !== 1 ? 's' : ''} failed
                {errorSummary && (
                  <p className="mt-1 max-w-[200px] truncate text-muted-foreground">
                    {errorSummary}
                  </p>
                )}
              </TooltipContent>
            </Tooltip>
          )}

          {duration > 0 && (
            <Tooltip>
              <TooltipTrigger asChild>
                <span className="flex items-center gap-0.5">
                  <Clock className="h-3 w-3" />
                  {formatDuration(duration)}
                </span>
              </TooltipTrigger>
              <TooltipContent side="top" className="text-xs">
                Execution time
              </TooltipContent>
            </Tooltip>
          )}
        </div>
      </TooltipProvider>
    );
  }

  return (
    <TooltipProvider>
      <div className={cn('flex flex-wrap items-center gap-1.5', className)}>
        {totalFiles > 0 && (
          <Tooltip>
            <TooltipTrigger asChild>
              <Badge variant="outline" className="text-[10px] h-5 px-1.5 gap-1">
                {filesModified > 0 && (
                  <span className="flex items-center gap-0.5">
                    <FileEdit className="h-2.5 w-2.5" />
                    {filesModified}
                  </span>
                )}
                {filesCreated > 0 && (
                  <span className="flex items-center gap-0.5 text-green-600">
                    <FilePlus className="h-2.5 w-2.5" />
                    {filesCreated}
                  </span>
                )}
                {filesDeleted > 0 && (
                  <span className="flex items-center gap-0.5 text-red-600">
                    <FileX className="h-2.5 w-2.5" />
                    {filesDeleted}
                  </span>
                )}
              </Badge>
            </TooltipTrigger>
            <TooltipContent side="top" className="text-xs">
              <div className="space-y-0.5">
                {filesModified > 0 && <p>{filesModified} files modified</p>}
                {filesCreated > 0 && <p>{filesCreated} files created</p>}
                {filesDeleted > 0 && <p>{filesDeleted} files deleted</p>}
              </div>
            </TooltipContent>
          </Tooltip>
        )}

        {commandsRun > 0 && (
          <Tooltip>
            <TooltipTrigger asChild>
              <Badge
                variant="outline"
                className={cn(
                  'text-[10px] h-5 px-1.5 gap-1',
                  commandsFailed > 0 && 'border-destructive text-destructive'
                )}
              >
                <Terminal className="h-2.5 w-2.5" />
                {commandsRun}
                {commandsFailed > 0 && (
                  <span className="text-destructive">
                    ({commandsFailed} failed)
                  </span>
                )}
              </Badge>
            </TooltipTrigger>
            <TooltipContent side="top" className="text-xs">
              {commandsRun} commands run
              {commandsFailed > 0 && (
                <p className="text-destructive">{commandsFailed} failed</p>
              )}
            </TooltipContent>
          </Tooltip>
        )}

        {tools.length > 0 && (
          <Tooltip>
            <TooltipTrigger asChild>
              <Badge variant="outline" className="text-[10px] h-5 px-1.5 gap-1">
                <Wrench className="h-2.5 w-2.5" />
                {tools.slice(0, 2).join(', ')}
                {tools.length > 2 && <span>+{tools.length - 2}</span>}
              </Badge>
            </TooltipTrigger>
            <TooltipContent side="top" className="text-xs">
              {tools.join(', ')}
            </TooltipContent>
          </Tooltip>
        )}

        {commandsFailed > 0 && (
          <Tooltip>
            <TooltipTrigger asChild>
              <Badge variant="outline" className="text-[10px] h-5 px-1.5 gap-1 text-destructive">
                <AlertTriangle className="h-2.5 w-2.5" />
                {commandsFailed}
              </Badge>
            </TooltipTrigger>
            <TooltipContent side="top" className="text-xs">
              {commandsFailed} command{commandsFailed !== 1 ? 's' : ''} failed
              {errorSummary && (
                <p className="mt-1 max-w-[220px] text-muted-foreground">
                  {errorSummary}
                </p>
              )}
            </TooltipContent>
          </Tooltip>
        )}

        {duration > 0 && (
          <Tooltip>
            <TooltipTrigger asChild>
              <Badge variant="outline" className="text-[10px] h-5 px-1.5 gap-1">
                <Clock className="h-2.5 w-2.5" />
                {formatDuration(duration)}
              </Badge>
            </TooltipTrigger>
            <TooltipContent side="top" className="text-xs">
              Execution time
            </TooltipContent>
          </Tooltip>
        )}
      </div>
    </TooltipProvider>
  );
}
