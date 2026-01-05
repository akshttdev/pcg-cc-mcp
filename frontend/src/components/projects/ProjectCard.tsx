import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card.tsx';
import { Badge } from '@/components/ui/badge.tsx';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu.tsx';
import { Button } from '@/components/ui/button.tsx';
import {
  Calendar,
  Edit,
  ExternalLink,
  FolderOpen,
  MoreHorizontal,
  Trash2,
} from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { Project, ProjectBoard } from 'shared/types';
import { useEffect, useRef } from 'react';
import { useOpenProjectInEditor } from '@/hooks/useOpenProjectInEditor';
import { projectsApi } from '@/lib/api';
import { formatDistanceToNow } from 'date-fns';

export type ProjectBoardSummary = {
  totalBoards: number;
  corePresence: Record<ProjectBoard['board_type'], boolean>;
  customCount: number;
  totalTasks: number;
  tasksByType: Record<ProjectBoard['board_type'], number>;
  unassignedTasks: number;
  latestActivity?: string;
  error?: string;
};

const CORE_BOARD_LABELS: Record<ProjectBoard['board_type'], string> = {
  executive_assets: 'Exec',
  brand_assets: 'Brand',
  dev_assets: 'Dev',
  social_assets: 'Social',
  agent_flows: 'Flows',
  artifact_gallery: 'Artifacts',
  approval_queue: 'Approvals',
  research_hub: 'Research',
  custom: 'Custom',
};

const CORE_BOARD_TYPES: ProjectBoard['board_type'][] = [
  'executive_assets',
  'brand_assets',
  'dev_assets',
  'social_assets',
  'agent_flows',
  'artifact_gallery',
  'approval_queue',
  'research_hub',
];

type Props = {
  project: Project;
  isFocused: boolean;
  fetchProjects: () => void;
  setError: (error: string) => void;
  onEdit: (project: Project) => void;
  boardSummary?: ProjectBoardSummary;
};

function ProjectCard({
  project,
  isFocused,
  fetchProjects,
  setError,
  onEdit,
  boardSummary,
}: Props) {
  const navigate = useNavigate();
  const ref = useRef<HTMLDivElement>(null);
  const handleOpenInEditor = useOpenProjectInEditor(project);

  useEffect(() => {
    if (isFocused && ref.current) {
      ref.current.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
      ref.current.focus();
    }
  }, [isFocused]);

  const handleDelete = async (id: string, name: string) => {
    if (
      !confirm(
        `Are you sure you want to delete "${name}"? This action cannot be undone.`
      )
    )
      return;

    try {
      await projectsApi.delete(id);
      fetchProjects();
    } catch (error) {
      console.error('Failed to delete project:', error);
      setError('Failed to delete project');
    }
  };

  const handleEdit = (project: Project) => {
    onEdit(project);
  };

  const handleOpenInIDE = () => {
    handleOpenInEditor();
  };

  const latestActivityLabel = boardSummary?.latestActivity
    ? formatDistanceToNow(new Date(boardSummary.latestActivity), {
        addSuffix: true,
      })
    : null;

  return (
    <Card
      className={`hover:shadow-md transition-shadow cursor-pointer focus:ring-2 focus:ring-primary outline-none border`}
      onClick={() => navigate(`/projects/${project.id}`)}
      tabIndex={isFocused ? 0 : -1}
      ref={ref}
    >
      <CardHeader>
        <div className="flex items-start justify-between">
          <CardTitle className="text-lg">{project.name}</CardTitle>
          <div className="flex items-center gap-2">
            <Badge variant="secondary">Active</Badge>
            <DropdownMenu>
              <DropdownMenuTrigger asChild onClick={(e) => e.stopPropagation()}>
                <Button variant="ghost" size="sm" className="h-8 w-8 p-0">
                  <MoreHorizontal className="h-4 w-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem
                  onClick={(e) => {
                    e.stopPropagation();
                    navigate(`/projects/${project.id}`);
                  }}
                >
                  <ExternalLink className="mr-2 h-4 w-4" />
                  View Project
                </DropdownMenuItem>
                <DropdownMenuItem
                  onClick={(e) => {
                    e.stopPropagation();
                    handleOpenInIDE();
                  }}
                >
                  <FolderOpen className="mr-2 h-4 w-4" />
                  Open in IDE
                </DropdownMenuItem>
                <DropdownMenuItem
                  onClick={(e) => {
                    e.stopPropagation();
                    handleEdit(project);
                  }}
                >
                  <Edit className="mr-2 h-4 w-4" />
                  Edit
                </DropdownMenuItem>
                <DropdownMenuItem
                  onClick={(e) => {
                    e.stopPropagation();
                    handleDelete(project.id, project.name);
                  }}
                  className="text-destructive"
                >
                  <Trash2 className="mr-2 h-4 w-4" />
                  Delete
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>
        <CardDescription className="flex items-center">
          <Calendar className="mr-1 h-3 w-3" />
          Created {new Date(project.created_at).toLocaleDateString()}
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-2 pt-0">
        {!boardSummary ? (
          <div className="text-xs text-muted-foreground">
            Loading board data…
          </div>
        ) : boardSummary.error ? (
          <div className="text-xs text-destructive">
            {boardSummary.error}
          </div>
        ) : (
          <>
            <div className="flex flex-wrap items-center gap-1.5">
              {CORE_BOARD_TYPES.map((type) => {
                const present = boardSummary.corePresence[type];
                const tasks = boardSummary.tasksByType[type] ?? 0;
                return (
                  <Badge
                    key={type}
                    variant={present ? 'secondary' : 'outline'}
                    className="text-[10px] uppercase tracking-wide"
                  >
                    {CORE_BOARD_LABELS[type]}
                    {present && tasks > 0 ? ` · ${tasks}` : ''}
                  </Badge>
                );
              })}
              {boardSummary.customCount > 0 && (
                <Badge
                  variant="outline"
                  className="text-[10px] uppercase tracking-wide"
                >
                  {CORE_BOARD_LABELS.custom} ×{boardSummary.customCount}
                  {boardSummary.tasksByType.custom > 0
                    ? ` · ${boardSummary.tasksByType.custom}`
                    : ''}
                </Badge>
              )}
              {boardSummary.unassignedTasks > 0 && (
                <Badge
                  variant="destructive"
                  className="text-[10px] uppercase tracking-wide"
                >
                  Unassigned · {boardSummary.unassignedTasks}
                </Badge>
              )}
            </div>
            <div className="text-xs text-muted-foreground">
              {boardSummary.totalBoards} board
              {boardSummary.totalBoards === 1 ? '' : 's'} ·{' '}
              {boardSummary.totalTasks} task
              {boardSummary.totalTasks === 1 ? '' : 's'}
              {latestActivityLabel ? ` · Updated ${latestActivityLabel}` : ''}
            </div>
          </>
        )}
      </CardContent>
    </Card>
  );
}

export default ProjectCard;
