import {
  Bot,
  CheckCircle,
  Clapperboard,
  Code,
  Copy,
  Edit,
  FileText,
  Image,
  Loader2,
  MoreHorizontal,
  Play,
  Terminal,
  Trash2,
  Video,
  XCircle,
} from 'lucide-react';
import { useCallback } from 'react';
import type {
  AgentFlowEvent,
  ArtifactType,
  ExecutionArtifact,
  TaskWithAttemptStatus,
} from 'shared/types';
import type { AgentWithParsedFields } from 'shared/types';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { KanbanCard } from '@/components/ui/shadcn-io/kanban';
import { TagChips } from '@/components/ui/tag-chips';
import type { AgentFlow, UserListItem } from '@/lib/api';

import { AgentFlowBadges } from './AgentFlowBadges';
import { ExecutionSummaryInline } from './ExecutionSummaryInline';
import {
  CollaboratorAvatars,
  PriorityBadge,
  useResolvedAgent,
  useResolvedAssignee,
  useScrollIntoView,
} from './task-card-parts';

const VIDEO_EDIT_TYPES: ArtifactType[] = [
  'video_edit_session',
  'render_deliverable',
];

type Task = TaskWithAttemptStatus;

// Task card display modes based on primary artifact/content type
export type TaskCardMode =
  | 'terminal' // Code execution, diffs, logs (default for coding tasks)
  | 'visual' // Images, screenshots, visual_brief
  | 'document' // content_draft, research_report, strategy_document
  | 'media' // video, walkthrough, browser_recording
  | 'compact'; // Minimal card for quick tasks

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
  usersMap?: Map<string, UserListItem>;
  agentsMap?: Map<string, AgentWithParsedFields>;
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
    const visualTypes: ArtifactType[] = [
      'screenshot',
      'visual_brief',
      'platform_screenshot',
    ];
    const documentTypes: ArtifactType[] = [
      'research_report',
      'strategy_document',
      'content_draft',
      'content_calendar',
      'competitor_analysis',
    ];
    const mediaTypes: ArtifactType[] = ['walkthrough', 'browser_recording'];

    if (visualTypes.includes(primaryArtifact.artifact_type)) return 'visual';
    if (documentTypes.includes(primaryArtifact.artifact_type))
      return 'document';
    if (mediaTypes.includes(primaryArtifact.artifact_type)) return 'media';
  }

  // Check task tags or custom properties for hints
  const tags = task.tags?.toLowerCase() || '';
  if (
    tags.includes('design') ||
    tags.includes('visual') ||
    tags.includes('image')
  )
    return 'visual';
  if (
    tags.includes('content') ||
    tags.includes('blog') ||
    tags.includes('document') ||
    tags.includes('research')
  )
    return 'document';
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

    case 'media': {
      const isVideoEdit = VIDEO_EDIT_TYPES.includes(artifact.artifact_type);
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
                {Math.floor(metadata.duration_seconds / 60)}:
                {String(metadata.duration_seconds % 60).padStart(2, '0')}
              </span>
            )}
          </div>
          {isVideoEdit && (
            <button
              title="Open Review"
              onClick={async (e) => {
                e.stopPropagation();
                try {
                  const res = await fetch(
                    `/api/artifacts/${artifact.id}/review-link`,
                    {
                      method: 'POST',
                      credentials: 'include',
                    }
                  );
                  const data = await res.json();
                  if (data?.data?.token) {
                    window.open(
                      `${window.location.origin}/review/${data.data.token}`,
                      '_blank'
                    );
                  }
                } catch {
                  /* ignore */
                }
              }}
              className="absolute top-1.5 right-1.5 flex items-center gap-1 bg-amber-500/90 hover:bg-amber-400 text-white rounded px-1.5 py-0.5 text-[10px] font-medium transition-colors"
            >
              <Clapperboard className="h-2.5 w-2.5" />
              Review
            </button>
          )}
        </div>
      );
    }

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
  usersMap,
  agentsMap,
}: EnhancedTaskCardProps) {
  const cardMode = deriveCardMode(task, primaryArtifact, defaultMode);
  const isAgentActive = task.has_in_progress_attempt;

  const localRef = useScrollIntoView(isOpen);
  const assignee = useResolvedAssignee(task.assignee_id, usersMap);
  const resolvedAgent = useResolvedAgent(
    task.assigned_agent,
    agentsMap,
    workflowEvents
  );

  const handleClick = useCallback(() => {
    if (selectionMode && onToggleSelection) {
      onToggleSelection(task.id);
    } else {
      onViewDetails(task);
    }
  }, [task, onViewDetails, selectionMode, onToggleSelection]);

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
      {/* Card content */}
      <div className="flex flex-col gap-1.5 min-w-0">
        {/* Title row */}
        <div className="flex items-start gap-2 min-w-0">
          {selectionMode && (
            <div
              className="pt-0.5"
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
          <Badge variant="outline" className="h-5 px-1.5 gap-1 shrink-0">
            {modeIcons[cardMode]}
          </Badge>

          <h4 className="flex-1 min-w-0 line-clamp-2 font-light text-sm">
            {task.title}
          </h4>

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
                  className="h-6 w-6 p-0 hover:bg-muted shrink-0"
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

        {/* Meta row: priority + assignee */}
        <div className="flex items-center justify-between gap-2">
          <div className="flex items-center flex-wrap gap-1">
            <PriorityBadge priority={task.priority} />
          </div>
          {assignee && (
            <div className="flex items-center gap-1.5 shrink-0">
              <div
                className="h-5 w-5 rounded-full flex items-center justify-center text-[10px] font-medium bg-violet-100 text-violet-700 dark:bg-violet-900 dark:text-violet-300 border border-violet-200 dark:border-violet-800"
                title={assignee.displayName}
              >
                {assignee.initials}
              </div>
              <span className="text-xs text-muted-foreground truncate max-w-[80px]">
                {assignee.displayName}
              </span>
            </div>
          )}
        </div>

        {/* Tag chips */}
        {task.tags && <TagChips tags={task.tags} maxVisible={2} size="xs" />}

        {/* Status indicators row */}
        <div className="flex items-center flex-wrap gap-1">
          {task.has_in_progress_attempt && (
            <Loader2 className="h-3 w-3 animate-spin text-blue-500" />
          )}
          {task.has_merged_attempt && (
            <CheckCircle className="h-3 w-3 text-green-500" />
          )}
          {task.last_attempt_failed && !task.has_merged_attempt && (
            <XCircle className="h-3 w-3 text-destructive" />
          )}
          {agentFlow && <AgentFlowBadges flow={agentFlow} compact />}
          {task.last_execution_summary && (
            <ExecutionSummaryInline
              summary={task.last_execution_summary}
              compact
            />
          )}
          {task.parsed_collaborators &&
            task.parsed_collaborators.length > 0 && (
              <CollaboratorAvatars
                collaborators={task.parsed_collaborators}
                usersMap={usersMap}
                agentsMap={agentsMap}
              />
            )}
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
          <span>
            {artifacts.length} artifact{artifacts.length !== 1 ? 's' : ''}
          </span>
        </div>
      )}

      {/* Agent working indicator */}
      {(isAgentActive || workflowEvents.length > 0) && !selectionMode && (
        <div className="flex items-center gap-2 mt-2 text-xs">
          {isAgentActive ? (
            <>
              <div className="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
              <span className="text-muted-foreground">
                {resolvedAgent?.displayName || 'Agent'} working...
              </span>
            </>
          ) : (
            <>
              <Bot className="h-3 w-3 text-muted-foreground" />
              <span className="text-muted-foreground">
                {workflowEvents.length} workflow event
                {workflowEvents.length !== 1 ? 's' : ''}
              </span>
            </>
          )}
        </div>
      )}
    </KanbanCard>
  );
}
