import { useCallback, useEffect, useRef, useMemo } from 'react';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Badge } from '@/components/ui/badge';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { KanbanCard } from '@/components/ui/shadcn-io/kanban';
import {
  CheckCircle,
  Copy,
  Edit,
  Loader2,
  MoreHorizontal,
  Trash2,
  XCircle,
  Bot,
  User,
  Terminal,
  FileText,
  Image,
  Video,
  Code,
  Play,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { AgentFlowBadges } from './AgentFlowBadges';
import { ExecutionSummaryInline } from './ExecutionSummaryInline';
import type { TaskWithAttemptStatus, ExecutionArtifact, ArtifactType, AgentFlowEvent } from 'shared/types';
import type { AgentFlow } from '@/lib/api';

type Task = TaskWithAttemptStatus;

// Task card display modes based on primary artifact/content type
export type TaskCardMode =
  | 'terminal'    // Code execution, diffs, logs (default for coding tasks)
  | 'visual'      // Images, screenshots, visual_brief
  | 'document'    // content_draft, research_report, strategy_document
  | 'media'       // video, walkthrough, browser_recording
  | 'compact';    // Minimal card for quick tasks

interface EnhancedTaskCardProps {
  task: Task;
  index: number;
  status: string;
  onEdit: (task: Task) => void;
  onDelete: (taskId: string) => void;
  onDuplicate?: (task: Task) => void;
  onViewDetails: (task: Task) => void;
  isOpen?: boolean;
  selectionMode?: boolean;
  isSelected?: boolean;
  onToggleSelection?: (taskId: string) => void;
  agentFlow?: AgentFlow;
  // New props for enhanced functionality
  primaryArtifact?: ExecutionArtifact;
  artifacts?: ExecutionArtifact[];
  workflowEvents?: AgentFlowEvent[];
  onSendMessage?: (message: string, agentName?: string) => Promise<string>;
  defaultMode?: TaskCardMode;
  showSessionLayer?: boolean;
}

// Derive card mode from task or primary artifact
function deriveCardMode(
  task: Task,
  primaryArtifact?: ExecutionArtifact,
  defaultMode?: TaskCardMode
): TaskCardMode {
  if (defaultMode) return defaultMode;

  // Check primary artifact type
  if (primaryArtifact) {
    const visualTypes: ArtifactType[] = ['screenshot', 'visual_brief', 'platform_screenshot'];
    const documentTypes: ArtifactType[] = ['research_report', 'strategy_document', 'content_draft', 'content_calendar', 'competitor_analysis'];
    const mediaTypes: ArtifactType[] = ['walkthrough', 'browser_recording'];

    if (visualTypes.includes(primaryArtifact.artifact_type)) return 'visual';
    if (documentTypes.includes(primaryArtifact.artifact_type)) return 'document';
    if (mediaTypes.includes(primaryArtifact.artifact_type)) return 'media';
  }

  // Check task tags or custom properties for hints
  const tags = task.tags?.toLowerCase() || '';
  if (tags.includes('design') || tags.includes('visual') || tags.includes('image')) return 'visual';
  if (tags.includes('content') || tags.includes('blog') || tags.includes('document') || tags.includes('research')) return 'document';
  if (tags.includes('video') || tags.includes('recording')) return 'media';

  // Default to terminal for coding tasks
  return 'terminal';
}

// Mode icon mapping
const modeIcons: Record<TaskCardMode, React.ReactNode> = {
  terminal: <Terminal className="h-3 w-3" />,
  visual: <Image className="h-3 w-3" />,
  document: <FileText className="h-3 w-3" />,
  media: <Video className="h-3 w-3" />,
  compact: <Code className="h-3 w-3" />,
};

// Artifact preview component
function ArtifactPreview({
  artifact,
  mode,
}: {
  artifact?: ExecutionArtifact;
  mode: TaskCardMode;
}) {
  if (!artifact) return null;

  const content = artifact.content;
  const metadata = artifact.metadata ? JSON.parse(artifact.metadata) : {};

  switch (mode) {
    case 'visual':
      return (
        <div className="relative w-full h-20 bg-gray-100 dark:bg-gray-800 rounded-md overflow-hidden">
          {artifact.file_path ? (
            <img
              src={`/api/files/${encodeURIComponent(artifact.file_path)}`}
              alt={artifact.title}
              className="w-full h-full object-cover"
            />
          ) : (
            <div className="flex items-center justify-center h-full">
              <Image className="h-8 w-8 text-gray-400" />
            </div>
          )}
          <div className="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/60 to-transparent p-1">
            <span className="text-[10px] text-white font-medium truncate block">
              {artifact.title}
            </span>
          </div>
        </div>
      );

    case 'document':
      return (
        <div className="bg-gray-50 dark:bg-gray-900 rounded-md p-2 max-h-20 overflow-hidden">
          <div className="text-[10px] font-medium text-gray-700 dark:text-gray-300 mb-1 truncate">
            {artifact.title}
          </div>
          {content && (
            <div className="text-[9px] text-gray-500 dark:text-gray-400 line-clamp-3 leading-relaxed">
              {content.substring(0, 200)}...
            </div>
          )}
        </div>
      );

    case 'media':
      return (
        <div className="relative w-full h-20 bg-gray-900 rounded-md overflow-hidden flex items-center justify-center">
          {artifact.file_path ? (
            <>
              <div className="absolute inset-0 bg-black/40" />
              <Play className="h-8 w-8 text-white/80" />
            </>
          ) : (
            <Video className="h-8 w-8 text-gray-400" />
          )}
          <div className="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/60 to-transparent p-1">
            <span className="text-[10px] text-white font-medium truncate block">
              {artifact.title}
            </span>
            {metadata.duration_seconds && (
              <span className="text-[9px] text-white/70">
                {Math.floor(metadata.duration_seconds / 60)}:{String(metadata.duration_seconds % 60).padStart(2, '0')}
              </span>
            )}
          </div>
        </div>
      );

    default:
      return null;
  }
}

export function EnhancedTaskCard({
  task,
  index,
  status,
  onEdit,
  onDelete,
  onDuplicate,
  onViewDetails,
  isOpen,
  selectionMode,
  isSelected,
  onToggleSelection,
  agentFlow,
  primaryArtifact,
  artifacts = [],
  workflowEvents = [],
  defaultMode,
}: EnhancedTaskCardProps) {
  const cardMode = deriveCardMode(task, primaryArtifact, defaultMode);
  const isAgentActive = task.has_in_progress_attempt;

  // Extract agent name from workflow events or task
  const agentName = useMemo(() => {
    if (task.assigned_agent) return task.assigned_agent;
    for (const event of workflowEvents) {
      try {
        const data = JSON.parse(event.event_data || '{}');
        if (data.agent_name) return data.agent_name;
      } catch {
        // Skip
      }
    }
    return undefined;
  }, [task.assigned_agent, workflowEvents]);

  const handleClick = useCallback(() => {
    if (selectionMode && onToggleSelection) {
      onToggleSelection(task.id);
    } else {
      onViewDetails(task);
    }
  }, [task, onViewDetails, selectionMode, onToggleSelection]);

  const localRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!isOpen || !localRef.current) return;
    const el = localRef.current;
    requestAnimationFrame(() => {
      el.scrollIntoView({
        block: 'center',
        inline: 'nearest',
        behavior: 'smooth',
      });
    });
  }, [isOpen]);

  return (
    <KanbanCard
      key={task.id}
      id={task.id}
      name={task.title}
      index={index}
      parent={status}
      onClick={handleClick}
      isOpen={isOpen}
      forwardedRef={localRef}
    >
      {/* Header row */}
      <div className="flex flex-1 gap-2 items-center min-w-0">
        {/* Checkbox for selection mode */}
        {selectionMode && (
          <div
            onClick={(e) => {
              e.stopPropagation();
              onToggleSelection?.(task.id);
            }}
          >
            <Checkbox
              checked={isSelected}
              onCheckedChange={() => onToggleSelection?.(task.id)}
            />
          </div>
        )}

        {/* Mode indicator */}
        <Badge variant="outline" className="h-5 px-1.5 gap-1">
          {modeIcons[cardMode]}
        </Badge>

        <h4 className="flex-1 min-w-0 line-clamp-2 font-light text-sm">
          {task.title}
        </h4>

        <div className="flex items-center space-x-1">
          {/* In Progress Spinner */}
          {task.has_in_progress_attempt && (
            <Loader2 className="h-3 w-3 animate-spin text-blue-500" />
          )}
          {/* Merged Indicator */}
          {task.has_merged_attempt && (
            <CheckCircle className="h-3 w-3 text-green-500" />
          )}
          {/* Failed Indicator */}
          {task.last_attempt_failed && !task.has_merged_attempt && (
            <XCircle className="h-3 w-3 text-destructive" />
          )}
          {/* Agent Flow Status */}
          {agentFlow && (
            <AgentFlowBadges flow={agentFlow} compact />
          )}
          {/* Execution Summary */}
          {task.last_execution_summary && (
            <ExecutionSummaryInline summary={task.last_execution_summary} compact />
          )}
          {/* Collaborator Avatars */}
          {task.collaborators && task.collaborators.length > 0 && (
            <div className="flex -space-x-1" title={task.collaborators.map(c => `${c.actor_id} (${c.actor_type})`).join(', ')}>
              {task.collaborators.slice(0, 3).map((collaborator, idx) => (
                <div
                  key={`${collaborator.actor_id}-${idx}`}
                  className={cn(
                    "h-4 w-4 rounded-full flex items-center justify-center text-[8px] font-medium border border-background",
                    collaborator.actor_type === 'agent'
                      ? 'bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300'
                      : 'bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300'
                  )}
                >
                  {collaborator.actor_type === 'agent'
                    ? <Bot className="h-2.5 w-2.5" />
                    : <User className="h-2.5 w-2.5" />
                  }
                </div>
              ))}
              {task.collaborators.length > 3 && (
                <div className="h-4 w-4 rounded-full flex items-center justify-center text-[8px] font-medium border border-background bg-muted text-muted-foreground">
                  +{task.collaborators.length - 3}
                </div>
              )}
            </div>
          )}
          {/* Actions Menu */}
          <div
            onPointerDown={(e) => e.stopPropagation()}
            onMouseDown={(e) => e.stopPropagation()}
            onClick={(e) => e.stopPropagation()}
          >
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-6 w-6 p-0 hover:bg-muted"
                >
                  <MoreHorizontal className="h-3 w-3" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem onClick={() => onEdit(task)}>
                  <Edit className="h-4 w-4 mr-2" />
                  Edit
                </DropdownMenuItem>
                {onDuplicate && (
                  <DropdownMenuItem onClick={() => onDuplicate(task)}>
                    <Copy className="h-4 w-4 mr-2" />
                    Duplicate
                  </DropdownMenuItem>
                )}
                <DropdownMenuItem
                  onClick={() => onDelete(task.id)}
                  className="text-destructive"
                >
                  <Trash2 className="h-4 w-4 mr-2" />
                  Delete
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>
      </div>

      {/* Description */}
      {task.description && (
        <p className="text-sm text-secondary-foreground break-words line-clamp-2 mt-1">
          {task.description}
        </p>
      )}

      {/* Artifact Preview Layer */}
      {primaryArtifact && cardMode !== 'terminal' && cardMode !== 'compact' && (
        <div className="mt-2">
          <ArtifactPreview artifact={primaryArtifact} mode={cardMode} />
        </div>
      )}

      {/* Artifacts count indicator */}
      {artifacts.length > 0 && (
        <div className="flex items-center gap-2 mt-2 text-xs text-muted-foreground">
          <FileText className="h-3 w-3" />
          <span>{artifacts.length} artifact{artifacts.length !== 1 ? 's' : ''}</span>
        </div>
      )}

      {/* Agent working indicator - simple status, not a full terminal */}
      {(isAgentActive || workflowEvents.length > 0) && !selectionMode && (
        <div className="flex items-center gap-2 mt-2 text-xs">
          {isAgentActive ? (
            <>
              <div className="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
              <span className="text-muted-foreground">
                {agentName || 'Agent'} working...
              </span>
            </>
          ) : (
            <>
              <Bot className="h-3 w-3 text-muted-foreground" />
              <span className="text-muted-foreground">
                {workflowEvents.length} workflow event{workflowEvents.length !== 1 ? 's' : ''}
              </span>
            </>
          )}
        </div>
      )}
    </KanbanCard>
  );
}
