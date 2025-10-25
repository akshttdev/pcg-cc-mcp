import { useCallback, useEffect, useMemo, useState, type ComponentType } from 'react';
import { useNavigate } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Alert, AlertDescription } from '@/components/ui/alert';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Label } from '@/components/ui/label';
import {
  Project,
  ProjectPod,
  ProjectAsset,
  ProjectBoard,
  TaskWithAttemptStatus,
} from 'shared/types';
import { showProjectForm } from '@/lib/modals';
import { projectsApi, tasksApi } from '@/lib/api';
import { cn } from '@/lib/utils';
import {
  AlertCircle,
  ArrowLeft,
  Calendar,
  Clock,
  Edit,
  Loader2,
  Folder,
  Users,
  Trash2,
  Plus,
  LayoutGrid,
  Briefcase,
  Sparkles,
  Code2,
  Megaphone,
  Shapes,
} from 'lucide-react';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { toast } from 'sonner';

interface ProjectDetailProps {
  projectId: string;
  onBack: () => void;
}

export function ProjectDetail({ projectId, onBack }: ProjectDetailProps) {
  const navigate = useNavigate();
  const [project, setProject] = useState<Project | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [pods, setPods] = useState<ProjectPod[]>([]);
  const [podsLoading, setPodsLoading] = useState(false);
  const [podsError, setPodsError] = useState('');
  const [boards, setBoards] = useState<ProjectBoard[]>([]);
  const [boardsLoading, setBoardsLoading] = useState(false);
  const [boardsError, setBoardsError] = useState('');
  const [assets, setAssets] = useState<ProjectAsset[]>([]);
  const [assetsLoading, setAssetsLoading] = useState(false);
  const [assetsError, setAssetsError] = useState('');
  const [tasks, setTasks] = useState<TaskWithAttemptStatus[]>([]);
  const [tasksLoading, setTasksLoading] = useState(false);
  const [tasksError, setTasksError] = useState('');
  const [isCreatePodOpen, setIsCreatePodOpen] = useState(false);
  const [podForm, setPodForm] = useState({
    title: '',
    description: '',
    status: 'active',
    lead: '',
  });
  const [podFormSubmitting, setPodFormSubmitting] = useState(false);
  const [podFormError, setPodFormError] = useState('');
  const [isCreateBoardOpen, setIsCreateBoardOpen] = useState(false);
  const [boardForm, setBoardForm] = useState({
    name: '',
    boardType: 'brand_assets' as ProjectBoard['board_type'],
    description: '',
  });
  const [boardFormSubmitting, setBoardFormSubmitting] = useState(false);
  const [boardFormError, setBoardFormError] = useState('');
  const [isCreateAssetOpen, setIsCreateAssetOpen] = useState(false);
  const [assetForm, setAssetForm] = useState({
    name: '',
    storagePath: '',
    category: 'file',
    scope: 'team',
    podId: 'none',
    boardId: 'none',
    checksum: '',
    mimeType: '',
    metadata: '',
    uploadedBy: '',
    byteSize: '',
  });
  const [assetFormSubmitting, setAssetFormSubmitting] = useState(false);
  const [assetFormError, setAssetFormError] = useState('');
  const podStatusOptions = ['active', 'paused', 'completed', 'archived'] as const;
  const assetCategoryOptions = ['file', 'transcript', 'link', 'note'] as const;
  const assetScopeOptions = ['owner', 'client', 'team', 'public'] as const;
  const boardTypeOptions: ProjectBoard['board_type'][] = [
    'executive_assets',
    'brand_assets',
    'dev_assets',
    'social_assets',
    'custom',
  ];

  const boardTypeLabels = useMemo<Record<ProjectBoard['board_type'], string>>(
    () => ({
      executive_assets: 'Executive Assets',
      brand_assets: 'Brand Assets',
      dev_assets: 'Dev Assets',
      social_assets: 'Social Assets',
      custom: 'Custom',
    }),
    []
  );

  type BoardMeta = {
    icon: ComponentType<{ className?: string }>;
    accentBorder: string;
    accentBackground: string;
    iconBg: string;
    iconColor: string;
    tagline: string;
    prompts: string[];
  };

  const boardTemplateMeta = useMemo<Record<ProjectBoard['board_type'], BoardMeta>>(
    () => ({
      executive_assets: {
        icon: Briefcase,
        accentBorder: 'border-amber-300/70',
        accentBackground: 'bg-gradient-to-br from-amber-200/30 via-amber-100/10 to-transparent',
        iconBg: 'bg-amber-100',
        iconColor: 'text-amber-600',
        tagline:
          'Align leadership rituals, investor updates, and financial guardrails in one lane.',
        prompts: [
          'Weekly leadership brief & run-rate snapshots',
          'Investor update drafts and milestone narratives',
          'Budget approvals, risk register, and decision logs',
        ],
      },
      brand_assets: {
        icon: Sparkles,
        accentBorder: 'border-rose-300/70',
        accentBackground: 'bg-gradient-to-br from-rose-200/30 via-rose-100/10 to-transparent',
        iconBg: 'bg-rose-100',
        iconColor: 'text-rose-600',
        tagline: 'Keep brand voice, research, and storytelling artefacts stitched together.',
        prompts: [
          'Messaging pillars, voice & tone guardrails',
          'Customer research, transcripts, and highlights',
          'Campaign briefs, creative direction, and approvals',
        ],
      },
      dev_assets: {
        icon: Code2,
        accentBorder: 'border-sky-300/70',
        accentBackground: 'bg-gradient-to-br from-sky-200/30 via-sky-100/10 to-transparent',
        iconBg: 'bg-sky-100',
        iconColor: 'text-sky-600',
        tagline: 'Map repositories, architecture decisions, and release pipelines.',
        prompts: [
          'Repository map, CI/CD links, and runbooks',
          'Architecture decisions, ADRs, and RFC drafts',
          'Sprint goals, QA checklists, and launch readiness',
        ],
      },
      social_assets: {
        icon: Megaphone,
        accentBorder: 'border-emerald-300/70',
        accentBackground: 'bg-gradient-to-br from-emerald-200/30 via-emerald-100/10 to-transparent',
        iconBg: 'bg-emerald-100',
        iconColor: 'text-emerald-600',
        tagline: 'Run campaign calendars, automation flows, and performance loops.',
        prompts: [
          'Editorial calendar and content pipeline',
          'Automation recipes, tooling configs, and workflows',
          'Engagement learnings and performance retrospectives',
        ],
      },
      custom: {
        icon: Shapes,
        accentBorder: 'border-slate-300/60',
        accentBackground: 'bg-gradient-to-br from-slate-200/30 via-slate-100/10 to-transparent',
        iconBg: 'bg-slate-100',
        iconColor: 'text-slate-600',
        tagline: 'Spin up a bespoke lane for experiments, partners, or local initiatives.',
        prompts: [
          'Name the focus and success criteria',
          'Link supporting pods, assets, and rituals',
          'Assign owners and recurring cadences',
        ],
      },
    }),
    []
  );

  const resetPodForm = useCallback(() => {
    setPodForm({
      title: '',
      description: '',
      status: 'active',
      lead: '',
    });
    setPodFormError('');
  }, []);

  const resetBoardForm = useCallback(() => {
    setBoardForm({
      name: '',
      boardType: 'brand_assets',
      description: '',
    });
    setBoardFormError('');
  }, []);

  const resetAssetForm = useCallback(() => {
    setAssetForm({
      name: '',
      storagePath: '',
      category: 'file',
      scope: 'team',
      podId: 'none',
      boardId: 'none',
      checksum: '',
      mimeType: '',
      metadata: '',
      uploadedBy: '',
      byteSize: '',
    });
    setAssetFormError('');
  }, []);

  const fetchProject = useCallback(async () => {
    setLoading(true);
    setError('');

    try {
      const result = await projectsApi.getById(projectId);
      setProject(result);
    } catch (error) {
      console.error('Failed to fetch project:', error);
      // @ts-expect-error it is type ApiError
      setError(error.message || 'Failed to load project');
    }

    setLoading(false);
  }, [projectId]);

  const fetchPods = useCallback(async () => {
    setPodsLoading(true);
    setPodsError('');

    try {
      const result = await projectsApi.listPods(projectId);
      setPods(result);
    } catch (error) {
      console.error('Failed to fetch project pods:', error);
      // @ts-expect-error it is type ApiError
      setPodsError(error.message || 'Failed to load team pods');
    }

    setPodsLoading(false);
  }, [projectId]);

  const fetchBoards = useCallback(async () => {
    setBoardsLoading(true);
    setBoardsError('');

    try {
      const result = await projectsApi.listBoards(projectId);
      setBoards(result);
    } catch (error) {
      console.error('Failed to fetch project boards:', error);
      // @ts-expect-error it is type ApiError
      setBoardsError(error.message || 'Failed to load boards');
    }

    setBoardsLoading(false);
  }, [projectId]);

  const fetchAssets = useCallback(async () => {
    setAssetsLoading(true);
    setAssetsError('');

    try {
      const result = await projectsApi.listAssets(projectId);
      setAssets(result);
    } catch (error: any) {
      console.error('Failed to fetch project assets:', error);
      if (error.response?.status === 404) {
        setAssetsError('No assets found for this project.');
      } else {
        setAssetsError(error.message || 'Failed to load brand assets');
      }
    }

    setAssetsLoading(false);
  }, [projectId]);

  const fetchTasks = useCallback(async () => {
    setTasksLoading(true);
    setTasksError('');

    try {
      const result = await tasksApi.getAll(projectId);
      setTasks(result);
    } catch (error) {
      console.error('Failed to fetch project tasks:', error);
      // @ts-expect-error ApiError shape
      setTasksError(error.message || 'Failed to load tasks');
    }

    setTasksLoading(false);
  }, [projectId]);

  const handleCreatePod = useCallback(async () => {
    if (!podForm.title.trim()) {
      setPodFormError('Title is required.');
      return;
    }

    setPodFormSubmitting(true);
    setPodFormError('');
    try {
      await projectsApi.createPod(projectId, {
        title: podForm.title.trim(),
        description: podForm.description.trim() || undefined,
        status: podForm.status as (typeof podStatusOptions)[number],
        lead: podForm.lead.trim() || undefined,
      });
      toast.success('Pod created');
      setIsCreatePodOpen(false);
      resetPodForm();
      fetchPods();
      fetchAssets();
      fetchTasks();
    } catch (error) {
      console.error('Failed to create pod:', error);
      setPodFormError(
        error instanceof Error ? error.message : 'Failed to create pod'
      );
    } finally {
      setPodFormSubmitting(false);
    }
  }, [podForm, projectId, fetchPods, fetchAssets, fetchTasks, resetPodForm]);

  const handleDeletePod = useCallback(
    async (podId: string) => {
      if (
        !confirm(
          'Delete this pod? Tasks linked to it will become unassigned.'
        )
      ) {
        return;
      }

      try {
        await projectsApi.deletePod(projectId, podId);
        toast.success('Pod deleted');
        fetchPods();
        fetchAssets();
        fetchTasks();
      } catch (error) {
        console.error('Failed to delete pod:', error);
        toast.error(
          error instanceof Error ? error.message : 'Failed to delete pod'
        );
      }
    },
    [projectId, fetchPods, fetchAssets, fetchTasks]
  );

  const handleCreateAsset = useCallback(async () => {
    if (!assetForm.name.trim() || !assetForm.storagePath.trim()) {
      setAssetFormError('Name and storage path are required.');
      return;
    }

    let byteSizeValue: bigint | undefined;
    if (assetForm.byteSize.trim()) {
      try {
        byteSizeValue = BigInt(assetForm.byteSize.trim());
      } catch (error) {
        console.error('Invalid byte size', error);
        setAssetFormError('Byte size must be a whole number.');
        return;
      }
    }

    setAssetFormSubmitting(true);
    setAssetFormError('');
    try {
      await projectsApi.createAsset(projectId, {
        name: assetForm.name.trim(),
        storage_path: assetForm.storagePath.trim(),
        category: assetForm.category as (typeof assetCategoryOptions)[number],
        scope: assetForm.scope as (typeof assetScopeOptions)[number],
        pod_id: assetForm.podId !== 'none' ? assetForm.podId : undefined,
        board_id: assetForm.boardId !== 'none' ? assetForm.boardId : undefined,
        checksum: assetForm.checksum.trim() || undefined,
        mime_type: assetForm.mimeType.trim() || undefined,
        metadata: assetForm.metadata.trim() || undefined,
        uploaded_by: assetForm.uploadedBy.trim() || undefined,
        byte_size: byteSizeValue,
      });
      toast.success('Asset added');
      setIsCreateAssetOpen(false);
      resetAssetForm();
      fetchAssets();
    } catch (error) {
      console.error('Failed to create asset:', error);
      setAssetFormError(
        error instanceof Error ? error.message : 'Failed to add asset'
      );
    } finally {
      setAssetFormSubmitting(false);
    }
  }, [assetForm, projectId, fetchAssets, resetAssetForm]);

  const handleDeleteAsset = useCallback(
    async (assetId: string) => {
      if (!confirm('Delete this asset?')) {
        return;
      }

      try {
        await projectsApi.deleteAsset(projectId, assetId);
        toast.success('Asset deleted');
        fetchAssets();
      } catch (error) {
        console.error('Failed to delete asset:', error);
        toast.error(
          error instanceof Error ? error.message : 'Failed to delete asset'
        );
      }
    },
    [projectId, fetchAssets]
  );

  const slugify = useCallback((value: string) => {
    return value
      .trim()
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '');
  }, []);

  const handleCreateBoard = useCallback(async () => {
    const trimmedName = boardForm.name.trim();
    if (!trimmedName) {
      setBoardFormError('Board name is required.');
      return;
    }

    setBoardFormSubmitting(true);
    setBoardFormError('');
    try {
      await projectsApi.createBoard(projectId, {
        name: trimmedName,
        slug: slugify(trimmedName) || trimmedName,
        board_type: boardForm.boardType,
        description: boardForm.description.trim() || null,
        metadata: null,
      });
      toast.success('Board created');
      setIsCreateBoardOpen(false);
      resetBoardForm();
      fetchBoards();
      fetchTasks();
    } catch (error) {
      console.error('Failed to create board:', error);
      setBoardFormError(
        error instanceof Error ? error.message : 'Failed to create board'
      );
    } finally {
      setBoardFormSubmitting(false);
    }
  }, [boardForm, projectId, slugify, fetchBoards, fetchTasks, resetBoardForm]);

  const handleDeleteBoard = useCallback(
    async (board: ProjectBoard) => {
      if (
        !confirm(
          `Delete the "${board.name}" board? Tasks and assets linked to it will keep their board reference.`
        )
      ) {
        return;
      }

      try {
        await projectsApi.deleteBoard(board.project_id, board.id);
        toast.success('Board deleted');
        fetchBoards();
        fetchAssets();
        fetchTasks();
      } catch (error) {
        console.error('Failed to delete board:', error);
        toast.error(
          error instanceof Error ? error.message : 'Failed to delete board'
        );
      }
    },
    [fetchBoards, fetchAssets, fetchTasks]
  );

  const handleDelete = async () => {
    if (!project) return;
    if (
      !confirm(
        `Are you sure you want to delete "${project.name}"? This action cannot be undone.`
      )
    )
      return;

    try {
      await projectsApi.delete(projectId);
      onBack();
    } catch (error) {
      console.error('Failed to delete project:', error);
      // @ts-expect-error it is type ApiError
      setError(error.message || 'Failed to delete project');
    }
  };

  const handleEditClick = async () => {
    try {
      const result = await showProjectForm({ project });
      if (result === 'saved') {
        fetchProject();
      }
    } catch (error) {
      // User cancelled - do nothing
    }
  };

  useEffect(() => {
    fetchProject();
  }, [fetchProject]);

  useEffect(() => {
    fetchPods();
    fetchBoards();
    fetchAssets();
    fetchTasks();
  }, [fetchPods, fetchBoards, fetchAssets, fetchTasks]);

  const podTitleById = useMemo(() => {
    const map = new Map<string, string>();
    pods.forEach((pod) => {
      map.set(pod.id, pod.title);
    });
    return map;
  }, [pods]);

  const boardById = useMemo(() => {
    const map = new Map<string, ProjectBoard>();
    boards.forEach((board) => {
      map.set(board.id, board);
    });
    return map;
  }, [boards]);

  const tasksByBoard = useMemo(() => {
    const map = new Map<string, TaskWithAttemptStatus[]>();
    tasks.forEach((task) => {
      const key = task.board_id ?? 'unassigned';
      if (!map.has(key)) {
        map.set(key, []);
      }
      map.get(key)!.push(task);
    });
    return map;
  }, [tasks]);

  const unassignedTasks = tasksByBoard.get('unassigned') ?? [];

  useEffect(() => {
    if (!boards.length) return;
    setAssetForm((prev) => {
      if (prev.boardId && boardById.has(prev.boardId)) {
        return prev;
      }
      return { ...prev, boardId: boards[0]?.id ?? '' };
    });
  }, [boards, boardById]);

  const coreBoardTypeSet = useMemo(
    () =>
      new Set<ProjectBoard['board_type']>([
        'executive_assets',
        'brand_assets',
        'dev_assets',
        'social_assets',
      ]),
    []
  );

  const formatStatusLabel = useCallback((value: string) => {
    return value
      .split('_')
      .map((chunk) => chunk.charAt(0).toUpperCase() + chunk.slice(1))
      .join(' ');
  }, []);

  const formatBoardLabel = useCallback(
    (value: ProjectBoard['board_type']) => boardTypeLabels[value] ?? formatStatusLabel(value),
    [boardTypeLabels, formatStatusLabel]
  );

  const formatByteSize = useCallback((value: ProjectAsset['byte_size']) => {
    if (!value) return '—';
    const size = Number(value);
    if (!Number.isFinite(size) || size <= 0) return '—';
    const units = ['B', 'KB', 'MB', 'GB'];
    const index = Math.min(
      Math.floor(Math.log10(size) / Math.log10(1024)),
      units.length - 1
    );
    const formatted = size / 1024 ** index;
    return `${formatted.toFixed(index === 0 ? 0 : 1)} ${units[index]}`;
  }, []);

  if (loading) {
    return (
      <div className="flex items-center justify-center py-12">
        <Loader2 className="mr-2 h-4 w-4 animate-spin" />
        Loading project...
      </div>
    );
  }

  if (error || !project) {
    return (
      <div className="space-y-4 py-12 px-4">
        <Button variant="outline" onClick={onBack}>
          <ArrowLeft className="mr-2 h-4 w-4" />
          Back to Projects
        </Button>
        <Card>
          <CardContent className="py-12 text-center">
            <div className="mx-auto flex h-12 w-12 items-center justify-center rounded-lg bg-muted">
              <AlertCircle className="h-6 w-6 text-muted-foreground" />
            </div>
            <h3 className="mt-4 text-lg font-semibold">Project not found</h3>
            <p className="mt-2 text-sm text-muted-foreground">
              {error ||
                "The project you're looking for doesn't exist or has been deleted."}
            </p>
            <Button className="mt-4" onClick={onBack}>
              Back to Projects
            </Button>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <>
      <div className="space-y-6 py-12 px-4">
      <div className="flex justify-between items-start">
        <div className="flex items-center space-x-4">
          <Button variant="outline" onClick={onBack}>
            <ArrowLeft className="mr-2 h-4 w-4" />
            Back to Projects
          </Button>
          <div>
            <div className="flex items-center gap-3">
              <h1 className="text-2xl font-bold">{project.name}</h1>
            </div>
            <p className="text-sm text-muted-foreground">
              Project details and settings
            </p>
          </div>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" onClick={handleEditClick}>
            <Edit className="mr-2 h-4 w-4" />
            Edit
          </Button>
          <Button
            variant="outline"
            onClick={handleDelete}
            className="text-destructive hover:text-destructive-foreground hover:bg-destructive/10"
          >
            <Trash2 className="mr-2 h-4 w-4" />
            Delete
          </Button>
        </div>
      </div>

      {error && (
        <Alert variant="destructive">
          <AlertCircle className="h-4 w-4" />
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      <div className="grid gap-6 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center">
              <Calendar className="mr-2 h-5 w-5" />
              Project Information
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium text-muted-foreground">
                Status
              </span>
              <Badge variant="secondary">Active</Badge>
            </div>
            <div className="space-y-2">
              <div className="flex items-center text-sm">
                <Calendar className="mr-2 h-4 w-4 text-muted-foreground" />
                <span className="text-muted-foreground">Created:</span>
                <span className="ml-2">
                  {new Date(project.created_at).toLocaleDateString()}
                </span>
              </div>
              <div className="flex items-center text-sm">
                <Clock className="mr-2 h-4 w-4 text-muted-foreground" />
                <span className="text-muted-foreground">Last Updated:</span>
                <span className="ml-2">
                  {new Date(project.updated_at).toLocaleDateString()}
                </span>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Project Details</CardTitle>
            <CardDescription>
              Technical information about this project
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <div>
              <h4 className="text-sm font-medium text-muted-foreground">
                Project ID
              </h4>
              <code className="mt-1 block text-xs bg-muted p-2 rounded font-mono">
                {project.id}
              </code>
            </div>
            <div>
              <h4 className="text-sm font-medium text-muted-foreground">
                Created At
              </h4>
              <p className="mt-1 text-sm">
                {new Date(project.created_at).toLocaleString()}
              </p>
            </div>
            <div>
              <h4 className="text-sm font-medium text-muted-foreground">
                Last Modified
              </h4>
              <p className="mt-1 text-sm">
                {new Date(project.updated_at).toLocaleString()}
              </p>
            </div>
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-6 lg:grid-cols-2">
        <Card className="h-full">
          <CardHeader className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <LayoutGrid className="h-5 w-5" />
                Project Boards
              </CardTitle>
              <CardDescription>
                Buckets for executive, brand, dev, and social workstreams.
              </CardDescription>
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                resetBoardForm();
                setIsCreateBoardOpen(true);
              }}
            >
              <Plus className="mr-1 h-4 w-4" /> Create Board
            </Button>
          </CardHeader>
          <CardContent className="space-y-4">
            {boardsLoading ? (
              <div className="flex items-center gap-2 text-muted-foreground">
                <Loader2 className="h-4 w-4 animate-spin" />
                Loading boards…
              </div>
            ) : boardsError ? (
              <Alert variant="destructive">
                <AlertCircle className="h-4 w-4" />
                <AlertDescription>{boardsError}</AlertDescription>
              </Alert>
            ) : boards.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                Boards will appear here after the database migration completes.
              </p>
            ) : (
              <div className="space-y-4">
                {tasksError && (
                  <Alert variant="destructive">
                    <AlertCircle className="h-4 w-4" />
                    <AlertDescription>{tasksError}</AlertDescription>
                  </Alert>
                )}
                <div className="grid gap-4 lg:grid-cols-2">
                  {boards.map((board) => {
                    const meta = boardTemplateMeta[board.board_type] || boardTemplateMeta.custom;
                    const Icon = meta.icon;
                    const boardTasks = tasksByBoard.get(board.id) ?? [];
                    const openBoardWorkspace = () => {
                      const params = new URLSearchParams();
                      params.set('board', board.id);
                      navigate({
                        pathname: `/projects/${projectId}/tasks`,
                        search: params.toString(),
                      });
                    };
                    return (
                      <div
                        key={board.id}
                        role="button"
                        tabIndex={0}
                        onClick={openBoardWorkspace}
                        onKeyDown={(event) => {
                          if (event.key === 'Enter' || event.key === ' ') {
                            event.preventDefault();
                            openBoardWorkspace();
                          }
                        }}
                        className={cn(
                          'relative overflow-hidden rounded-2xl border bg-background p-5 text-left shadow-sm transition focus:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 hover:shadow-md',
                          meta.accentBorder
                        )}
                      >
                        <div
                          className={cn(
                            'pointer-events-none absolute inset-0 rounded-2xl',
                            meta.accentBackground
                          )}
                        />
                        <div className="relative flex items-start justify-between gap-4">
                          <div className="flex items-start gap-3">
                            <span
                              className={cn(
                                'flex h-11 w-11 items-center justify-center rounded-full',
                                meta.iconBg
                              )}
                            >
                              <Icon className={cn('h-5 w-5', meta.iconColor)} />
                            </span>
                            <div>
                              <div className="flex flex-wrap items-center gap-2">
                                <p className="text-base font-semibold leading-tight">
                                  {board.name}
                                </p>
                                <Badge variant="outline" className="uppercase">
                                  {formatBoardLabel(board.board_type)}
                                </Badge>
                              </div>
                              <p className="mt-1 text-sm text-muted-foreground">
                                {board.description || meta.tagline}
                              </p>
                            </div>
                          </div>
                          {!coreBoardTypeSet.has(board.board_type) && (
                            <Button
                              type="button"
                              variant="ghost"
                              size="icon"
                              className="relative text-muted-foreground hover:text-destructive"
                              onClick={(event) => {
                                event.stopPropagation();
                                event.preventDefault();
                                handleDeleteBoard(board);
                              }}
                              aria-label={`Delete board ${board.name}`}
                            >
                              <Trash2 className="h-4 w-4" />
                            </Button>
                          )}
                        </div>
                        <div className="relative mt-4 space-y-2">
                          <div className="flex items-center justify-between text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                            <span>Tasks</span>
                            <span>{boardTasks.length}</span>
                          </div>
                          {tasksLoading ? (
                            <div className="flex items-center gap-2 text-xs text-muted-foreground">
                              <Loader2 className="h-3.5 w-3.5 animate-spin" />
                              Loading tasks…
                            </div>
                          ) : boardTasks.length === 0 ? (
                            <p className="text-xs text-muted-foreground">
                              No tasks assigned to this board yet.
                            </p>
                          ) : (
                            <ul className="space-y-1">
                              {boardTasks.slice(0, 5).map((task) => (
                                <li
                                  key={task.id}
                                  className="flex items-center justify-between gap-2 rounded-lg border bg-muted/40 px-2 py-1"
                                >
                                  <span className="truncate text-sm font-medium text-foreground">
                                    {task.title}
                                  </span>
                                  <Badge
                                    variant={task.status === 'done' ? 'secondary' : 'outline'}
                                    className="uppercase text-[10px]"
                                  >
                                    {formatStatusLabel(task.status)}
                                  </Badge>
                                </li>
                              ))}
                              {boardTasks.length > 5 && (
                                <li className="text-xs text-muted-foreground">
                                  +{boardTasks.length - 5} more task
                                  {boardTasks.length - 5 === 1 ? '' : 's'}
                                </li>
                              )}
                            </ul>
                          )}
                          <div className="flex flex-wrap items-center justify-between gap-3 pt-3 text-[11px] uppercase tracking-wide text-muted-foreground">
                            <span>
                              Updated {new Date(board.updated_at).toLocaleDateString()}
                            </span>
                            <Button
                              type="button"
                              variant="outline"
                              size="sm"
                              className="h-7"
                              onClick={(event) => {
                                event.stopPropagation();
                                event.preventDefault();
                                openBoardWorkspace();
                              }}
                            >
                              Open board
                            </Button>
                          </div>
                        </div>
                      </div>
                    );
                  })}
                </div>
                {unassignedTasks.length > 0 && (
                  <div
                    role="button"
                    tabIndex={0}
                    onClick={() => {
                      const params = new URLSearchParams();
                      params.set('board', 'unassigned');
                      navigate({
                        pathname: `/projects/${projectId}/tasks`,
                        search: params.toString(),
                      });
                    }}
                    onKeyDown={(event) => {
                      if (event.key === 'Enter' || event.key === ' ') {
                        event.preventDefault();
                        const params = new URLSearchParams();
                        params.set('board', 'unassigned');
                        navigate({
                          pathname: `/projects/${projectId}/tasks`,
                          search: params.toString(),
                        });
                      }
                    }}
                    className={cn(
                      'rounded-2xl border border-dashed bg-muted/30 p-4 text-left transition hover:border-muted-foreground/50 hover:bg-muted/50 focus:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2'
                    )}
                  >
                    <div className="flex items-start justify-between gap-4">
                      <div>
                        <p className="text-sm font-semibold">Unassigned Tasks</p>
                        <p className="text-xs text-muted-foreground">
                          Tasks without a board. Assign them to keep workstreams organized.
                        </p>
                      </div>
                      <Badge variant="outline" className="uppercase text-[10px]">
                        {unassignedTasks.length}
                      </Badge>
                    </div>
                    <ul className="mt-3 space-y-1">
                      {unassignedTasks.slice(0, 6).map((task) => (
                        <li
                          key={task.id}
                          className="flex items-center justify-between gap-2 rounded-lg border bg-background px-2 py-1"
                        >
                          <span className="truncate text-sm font-medium text-foreground">
                            {task.title}
                          </span>
                          <Badge
                            variant={task.status === 'done' ? 'secondary' : 'outline'}
                            className="uppercase text-[10px]"
                          >
                            {formatStatusLabel(task.status)}
                          </Badge>
                        </li>
                      ))}
                      {unassignedTasks.length > 6 && (
                        <li className="text-xs text-muted-foreground">
                          +{unassignedTasks.length - 6} more task
                          {unassignedTasks.length - 6 === 1 ? '' : 's'}
                        </li>
                      )}
                    </ul>
                  </div>
                )}
              </div>
            )}
          </CardContent>
        </Card>

        <Card className="h-full">
          <CardHeader className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <Users className="h-5 w-5" />
                Team Pods
              </CardTitle>
              <CardDescription>
                Sub-projects grouped by goals or delivery teams.
              </CardDescription>
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                resetPodForm();
                setIsCreatePodOpen(true);
              }}
            >
              <Plus className="mr-1 h-4 w-4" /> Create Pod
            </Button>
          </CardHeader>
          <CardContent className="space-y-4">
            {podsLoading ? (
              <div className="flex items-center gap-2 text-muted-foreground">
                <Loader2 className="h-4 w-4 animate-spin" />
                Loading pods…
              </div>
            ) : podsError ? (
              <Alert variant="destructive">
                <AlertCircle className="h-4 w-4" />
                <AlertDescription>{podsError}</AlertDescription>
              </Alert>
            ) : pods.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                No pods yet — create mission threads to track goals and
                owners within this brand.
              </p>
            ) : (
              <div className="space-y-3">
                {pods.map((pod) => (
                  <div
                    key={pod.id}
                    className="rounded-lg border bg-background p-3 shadow-sm"
                  >
                    <div className="flex items-start justify-between gap-4">
                      <div>
                        <p className="text-sm font-medium">{pod.title}</p>
                        {pod.description && (
                          <p className="mt-1 text-xs text-muted-foreground">
                            {pod.description}
                          </p>
                        )}
                      </div>
                      <div className="flex items-center gap-2">
                        <Badge variant="secondary">
                          {formatStatusLabel(pod.status)}
                        </Badge>
                        <Button
                          variant="ghost"
                          size="icon"
                          className="text-muted-foreground hover:text-destructive"
                          onClick={() => handleDeletePod(pod.id)}
                          aria-label={`Delete pod ${pod.title}`}
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      </div>
                    </div>
                    <div className="mt-3 flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground">
                      <span>
                        Lead:{' '}
                        <span className="font-medium text-foreground">
                          {pod.lead || 'Unassigned'}
                        </span>
                      </span>
                      <span>
                        Updated{' '}
                        {new Date(pod.updated_at).toLocaleDateString()}
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>

        <Card className="h-full">
          <CardHeader className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <Folder className="h-5 w-5" />
                Brand Assets
              </CardTitle>
              <CardDescription>
                Logos, guides, transcripts, and collateral linked to this brand.
              </CardDescription>
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                resetAssetForm();
                setIsCreateAssetOpen(true);
              }}
            >
              <Plus className="mr-1 h-4 w-4" /> Add Asset
            </Button>
          </CardHeader>
          <CardContent className="space-y-4">
            {assetsLoading ? (
              <div className="flex items-center gap-2 text-muted-foreground">
                <Loader2 className="h-4 w-4 animate-spin" />
                Loading assets…
              </div>
            ) : assetsError ? (
              <Alert variant="destructive">
                <AlertCircle className="h-4 w-4" />
                <AlertDescription>{assetsError}</AlertDescription>
              </Alert>
            ) : assets.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                No assets have been catalogued yet. Upload brand kits,
                strategy docs, call transcripts, and web repos here as you
                onboard the client.
              </p>
            ) : (
              <div className="overflow-x-auto">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Name</TableHead>
                      <TableHead>Category</TableHead>
                      <TableHead>Scope</TableHead>
                      <TableHead className="hidden xl:table-cell">
                        Board
                      </TableHead>
                      <TableHead className="hidden xl:table-cell">
                        Linked Pod
                      </TableHead>
                      <TableHead className="hidden xl:table-cell">
                        Size
                      </TableHead>
                      <TableHead className="text-right">
                        Updated
                      </TableHead>
                      <TableHead className="text-right">Actions</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {assets.map((asset) => (
                      <TableRow key={asset.id}>
                        <TableCell className="max-w-[160px] truncate font-medium">
                          {asset.name}
                        </TableCell>
                        <TableCell>
                          <Badge variant="outline" className="uppercase">
                            {asset.category}
                          </Badge>
                        </TableCell>
                        <TableCell>
                          <Badge variant="secondary" className="uppercase">
                            {asset.scope}
                          </Badge>
                        </TableCell>
                        <TableCell className="hidden xl:table-cell text-sm text-muted-foreground">
                          {asset.board_id
                            ? boardById.get(asset.board_id)?.name ||
                              formatBoardLabel(
                                boardById.get(asset.board_id)?.board_type ??
                                  'custom'
                              )
                            : '—'}
                        </TableCell>
                        <TableCell className="hidden xl:table-cell text-sm text-muted-foreground">
                          {asset.pod_id ? podTitleById.get(asset.pod_id) || '—' : '—'}
                        </TableCell>
                        <TableCell className="hidden xl:table-cell text-sm text-muted-foreground">
                          {formatByteSize(asset.byte_size)}
                        </TableCell>
                        <TableCell className="text-right text-sm text-muted-foreground">
                          {new Date(asset.updated_at).toLocaleDateString()}
                        </TableCell>
                        <TableCell className="text-right">
                          <Button
                            variant="ghost"
                            size="icon"
                            className="text-muted-foreground hover:text-destructive"
                            onClick={() => handleDeleteAsset(asset.id)}
                            aria-label={`Delete asset ${asset.name}`}
                          >
                            <Trash2 className="h-4 w-4" />
                          </Button>
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    </div>

    <Dialog
      open={isCreatePodOpen}
      onOpenChange={(open) => {
        setIsCreatePodOpen(open);
        if (!open) {
          resetPodForm();
        }
      }}
    >
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>Create Pod</DialogTitle>
          <p className="text-sm text-muted-foreground">
            Define a pod to organise workstreams for this engagement.
          </p>
        </DialogHeader>
        <div className="space-y-4">
          <div className="space-y-1.5">
            <Label htmlFor="pod-title">Title</Label>
            <Input
              id="pod-title"
              value={podForm.title}
              onChange={(e) =>
                setPodForm((prev) => ({ ...prev, title: e.target.value }))
              }
              disabled={podFormSubmitting}
              placeholder="e.g. Website launch"
            />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="pod-description">Description</Label>
            <Textarea
              id="pod-description"
              value={podForm.description}
              onChange={(e) =>
                setPodForm((prev) => ({ ...prev, description: e.target.value }))
              }
              disabled={podFormSubmitting}
              rows={3}
              placeholder="What is this pod responsible for?"
            />
          </div>
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="space-y-1.5">
              <Label>Status</Label>
              <Select
                value={podForm.status}
                onValueChange={(value) =>
                  setPodForm((prev) => ({ ...prev, status: value }))
                }
                disabled={podFormSubmitting}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {podStatusOptions.map((status) => (
                    <SelectItem key={status} value={status}>
                      {formatStatusLabel(status)}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="pod-lead">Lead (optional)</Label>
              <Input
                id="pod-lead"
                value={podForm.lead}
                onChange={(e) =>
                  setPodForm((prev) => ({ ...prev, lead: e.target.value }))
                }
                disabled={podFormSubmitting}
                placeholder="Primary owner"
              />
            </div>
          </div>
          {podFormError && (
            <p className="text-sm text-destructive">{podFormError}</p>
          )}
        </div>
        <DialogFooter>
          <Button
            variant="ghost"
            onClick={() => {
              setIsCreatePodOpen(false);
              resetPodForm();
            }}
            disabled={podFormSubmitting}
          >
            Cancel
          </Button>
          <Button onClick={handleCreatePod} disabled={podFormSubmitting}>
            {podFormSubmitting ? 'Creating…' : 'Create Pod'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    <Dialog
      open={isCreateBoardOpen}
      onOpenChange={(open) => {
        setIsCreateBoardOpen(open);
        if (!open) {
          resetBoardForm();
        }
      }}
    >
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>Create Project Board</DialogTitle>
          <p className="text-sm text-muted-foreground">
            Add a dedicated board to hold tasks and assets for a new focus area.
          </p>
        </DialogHeader>
        <div className="space-y-4">
          <div className="space-y-1.5">
            <Label htmlFor="board-name">Name</Label>
            <Input
              id="board-name"
              value={boardForm.name}
              onChange={(e) =>
                setBoardForm((prev) => ({ ...prev, name: e.target.value }))
              }
              disabled={boardFormSubmitting}
              placeholder="e.g. Lifecycle Experiments"
            />
          </div>
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="space-y-1.5">
              <Label>Board type</Label>
              <Select
                value={boardForm.boardType}
                onValueChange={(value) =>
                  setBoardForm((prev) => ({
                    ...prev,
                    boardType: value as ProjectBoard['board_type'],
                  }))
                }
                disabled={boardFormSubmitting}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {boardTypeOptions.map((option) => (
                    <SelectItem key={option} value={option}>
                      {boardTypeLabels[option]}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="board-description">Description (optional)</Label>
            <Textarea
              id="board-description"
              value={boardForm.description}
              onChange={(e) =>
                setBoardForm((prev) => ({
                  ...prev,
                  description: e.target.value,
                }))
              }
              disabled={boardFormSubmitting}
              rows={3}
              placeholder="Clarify what lives inside this board."
            />
          </div>
          {boardFormError && (
            <p className="text-sm text-destructive">{boardFormError}</p>
          )}
        </div>
        <DialogFooter>
          <Button
            variant="ghost"
            onClick={() => {
              setIsCreateBoardOpen(false);
              resetBoardForm();
            }}
            disabled={boardFormSubmitting}
          >
            Cancel
          </Button>
          <Button onClick={handleCreateBoard} disabled={boardFormSubmitting}>
            {boardFormSubmitting ? 'Creating…' : 'Create Board'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    <Dialog
      open={isCreateAssetOpen}
      onOpenChange={(open) => {
        setIsCreateAssetOpen(open);
        if (!open) {
          resetAssetForm();
        }
      }}
    >
      <DialogContent className="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>Add Brand Asset</DialogTitle>
          <p className="text-sm text-muted-foreground">
            Catalogue files, transcripts, or collateral to keep the team aligned.
          </p>
        </DialogHeader>
        <div className="space-y-4">
          <div className="space-y-1.5">
            <Label htmlFor="asset-name">Name</Label>
            <Input
              id="asset-name"
              value={assetForm.name}
              onChange={(e) =>
                setAssetForm((prev) => ({ ...prev, name: e.target.value }))
              }
              disabled={assetFormSubmitting}
              placeholder="e.g. Primary logo"
            />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="asset-storage">Storage path / URL</Label>
            <Input
              id="asset-storage"
              value={assetForm.storagePath}
              onChange={(e) =>
                setAssetForm((prev) => ({ ...prev, storagePath: e.target.value }))
              }
              disabled={assetFormSubmitting}
              placeholder="s3://bucket/logo.svg or https://..."
            />
          </div>
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
            <div className="space-y-1.5">
              <Label>Category</Label>
              <Select
                value={assetForm.category}
                onValueChange={(value) =>
                  setAssetForm((prev) => ({ ...prev, category: value }))
                }
                disabled={assetFormSubmitting}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {assetCategoryOptions.map((category) => (
                    <SelectItem key={category} value={category}>
                      {formatStatusLabel(category)}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-1.5">
              <Label>Scope</Label>
              <Select
                value={assetForm.scope}
                onValueChange={(value) =>
                  setAssetForm((prev) => ({ ...prev, scope: value }))
                }
                disabled={assetFormSubmitting}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {assetScopeOptions.map((scope) => (
                    <SelectItem key={scope} value={scope}>
                      {formatStatusLabel(scope)}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-1.5">
              <Label>Board</Label>
              <Select
                value={assetForm.boardId}
                onValueChange={(value) =>
                  setAssetForm((prev) => ({ ...prev, boardId: value }))
                }
                disabled={assetFormSubmitting || boardsLoading}
              >
                <SelectTrigger>
                  <SelectValue
                    placeholder={
                      boardsLoading
                        ? 'Loading boards…'
                        : boards.length
                        ? 'Select board'
                        : 'No boards available'
                    }
                  />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="none">No board</SelectItem>
                  {boards.map((board) => (
                    <SelectItem key={board.id} value={board.id}>
                      {board.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-1.5">
              <Label>Linked pod</Label>
              <Select
                value={assetForm.podId}
                onValueChange={(value) =>
                  setAssetForm((prev) => ({ ...prev, podId: value }))
                }
                disabled={assetFormSubmitting || podsLoading}
              >
                <SelectTrigger>
                  <SelectValue placeholder="Unassigned" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="none">No pod</SelectItem>
                  {pods.map((pod) => (
                    <SelectItem key={pod.id} value={pod.id}>
                      {pod.title}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="space-y-1.5">
              <Label htmlFor="asset-byte-size">Byte size (optional)</Label>
              <Input
                id="asset-byte-size"
                type="number"
                value={assetForm.byteSize}
                onChange={(e) =>
                  setAssetForm((prev) => ({ ...prev, byteSize: e.target.value }))
                }
                disabled={assetFormSubmitting}
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="asset-mime">MIME type (optional)</Label>
              <Input
                id="asset-mime"
                value={assetForm.mimeType}
                onChange={(e) =>
                  setAssetForm((prev) => ({ ...prev, mimeType: e.target.value }))
                }
                disabled={assetFormSubmitting}
              />
            </div>
          </div>
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="space-y-1.5">
              <Label htmlFor="asset-checksum">Checksum (optional)</Label>
              <Input
                id="asset-checksum"
                value={assetForm.checksum}
                onChange={(e) =>
                  setAssetForm((prev) => ({ ...prev, checksum: e.target.value }))
                }
                disabled={assetFormSubmitting}
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="asset-uploaded-by">Uploaded by (optional)</Label>
              <Input
                id="asset-uploaded-by"
                value={assetForm.uploadedBy}
                onChange={(e) =>
                  setAssetForm((prev) => ({ ...prev, uploadedBy: e.target.value }))
                }
                disabled={assetFormSubmitting}
              />
            </div>
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="asset-metadata">Metadata (optional)</Label>
            <Textarea
              id="asset-metadata"
              value={assetForm.metadata}
              onChange={(e) =>
                setAssetForm((prev) => ({ ...prev, metadata: e.target.value }))
              }
              disabled={assetFormSubmitting}
              rows={3}
              placeholder="JSON or notes for this asset"
            />
          </div>
          {assetFormError && (
            <p className="text-sm text-destructive">{assetFormError}</p>
          )}
        </div>
        <DialogFooter>
          <Button
            variant="ghost"
            onClick={() => {
              setIsCreateAssetOpen(false);
              resetAssetForm();
            }}
            disabled={assetFormSubmitting}
          >
            Cancel
          </Button>
          <Button onClick={handleCreateAsset} disabled={assetFormSubmitting}>
            {assetFormSubmitting ? 'Adding…' : 'Add Asset'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  </>
  );
}
