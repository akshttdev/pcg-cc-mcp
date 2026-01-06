import { useCallback, useEffect, useMemo, useRef, useState, type ComponentType } from 'react';
import NiceModal from '@ebay/nice-modal-react';
import { useNavigate } from 'react-router-dom';
import { useUserSystem } from '@/components/config-provider';
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
  AirtableBase,
  BrandProfile,
  Project,
  ProjectAsset,
  ProjectBoard,
  TaskWithAttemptStatus,
} from 'shared/types';
import { showProjectForm } from '@/lib/modals';
import {
  projectsApi,
  tasksApi,
  emailApi,
  socialApi,
  type EmailAccountRecord,
  type SocialAccountRecord,
} from '@/lib/api';
import { AirtableBaseConnect } from './AirtableBaseConnect';
import { cn } from '@/lib/utils';
import {
  AlertCircle,
  ArrowLeft,
  BarChart3,
  Calendar,
  Clock,
  ClipboardCheck,
  Edit,
  Loader2,
  Mail,
  Folder,
  Trash2,
  Plus,
  LayoutGrid,
  CreditCard,
  Sparkles,
  Code2,
  Palette,
  Share2,
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

type ProviderConnectionState = 'connected' | 'manual' | 'missing';

type ProviderStatus = {
  key: string;
  label: string;
  status: ProviderConnectionState;
  detail: string;
  meta?: string;
  actionLabel?: string;
  onAction?: () => void;
};

type IntegrationCategory = {
  key: string;
  label: string;
  icon: ComponentType<{ className?: string }>;
  accent: string;
  connectors: ProviderStatus[];
};

const integrationStatusStyles: Record<ProviderConnectionState, string> = {
  connected: 'bg-emerald-100 text-emerald-700 border-emerald-200',
  manual: 'bg-amber-100 text-amber-700 border-amber-200',
  missing: 'bg-rose-100 text-rose-700 border-rose-200',
};

const BRAND_PALETTES = [
  {
    primary: '#2563EB',
    secondary: '#EC4899',
    accent: 'from-sky-50 via-white to-indigo-100',
  },
  {
    primary: '#0EA5E9',
    secondary: '#10B981',
    accent: 'from-cyan-50 via-white to-emerald-100',
  },
  {
    primary: '#F97316',
    secondary: '#8B5CF6',
    accent: 'from-orange-50 via-white to-violet-100',
  },
  {
    primary: '#F43F5E',
    secondary: '#6366F1',
    accent: 'from-rose-50 via-white to-indigo-100',
  },
] as const;

// Legacy localStorage key for migration
const BRAND_PROFILE_STORAGE_PREFIX = 'brand-profile:';

function hexToRgba(hex: string, alpha: number) {
  const sanitized = hex.replace('#', '');
  if (sanitized.length !== 6) {
    return `rgba(0, 0, 0, ${alpha})`;
  }
  const value = Number.parseInt(sanitized, 16);
  const r = (value >> 16) & 255;
  const g = (value >> 8) & 255;
  const b = value & 255;
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

interface ProjectDetailProps {
  projectId: string;
  onBack: () => void;
}

export function ProjectDetail({ projectId, onBack }: ProjectDetailProps) {
  const navigate = useNavigate();
  const { config } = useUserSystem();
  const [project, setProject] = useState<Project | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [boards, setBoards] = useState<ProjectBoard[]>([]);
  const [boardsLoading, setBoardsLoading] = useState(false);
  const [boardsError, setBoardsError] = useState('');
  const [assets, setAssets] = useState<ProjectAsset[]>([]);
  const [assetsLoading, setAssetsLoading] = useState(false);
  const [assetsError, setAssetsError] = useState('');
  const [tasks, setTasks] = useState<TaskWithAttemptStatus[]>([]);
  const [tasksLoading, setTasksLoading] = useState(false);
  const [tasksError, setTasksError] = useState('');
  const [emailAccounts, setEmailAccounts] = useState<EmailAccountRecord[]>([]);
  const [emailAccountsLoading, setEmailAccountsLoading] = useState(true);
  const [emailIntegrationError, setEmailIntegrationError] = useState('');
  const [socialAccounts, setSocialAccounts] = useState<SocialAccountRecord[]>([]);
  const [socialAccountsLoading, setSocialAccountsLoading] = useState(true);
  const [socialIntegrationError, setSocialIntegrationError] = useState('');
  const [airtableConnections, setAirtableConnections] = useState<AirtableBase[]>([]);
  const [integrationsRefreshing, setIntegrationsRefreshing] = useState(false);
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
    boardId: 'none',
    checksum: '',
    mimeType: '',
    metadata: '',
    uploadedBy: '',
    byteSize: '',
  });
  const [assetFormSubmitting, setAssetFormSubmitting] = useState(false);
  const [assetFormError, setAssetFormError] = useState('');
  const [brandProfile, setBrandProfile] = useState<BrandProfile | null>(null);
  const [isBrandProfileDialogOpen, setIsBrandProfileDialogOpen] = useState(false);
  const [brandProfileDraft, setBrandProfileDraft] = useState<{
    tagline: string;
    industry: string;
    primaryColor: string;
    secondaryColor: string;
  } | null>(null);
  const assetCategoryOptions = ['file', 'transcript', 'link', 'note'] as const;
  const assetScopeOptions = ['owner', 'client', 'team', 'public'] as const;
  // Simplified board types - just default (auto-created) and custom (user-created)
  const boardTypeOptions: ProjectBoard['board_type'][] = ['default', 'custom'];
  const airtableSectionRef = useRef<HTMLDivElement | null>(null);

  const boardTypeLabels = useMemo<Record<ProjectBoard['board_type'], string>>(
    () => ({
      default: 'Main Board',
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

  // Simplified board metadata - just default and custom
  const boardTemplateMeta = useMemo<Record<ProjectBoard['board_type'], BoardMeta>>(
    () => ({
      default: {
        icon: LayoutGrid,
        accentBorder: 'border-blue-300/70',
        accentBackground: 'bg-gradient-to-br from-blue-200/30 via-blue-100/10 to-transparent',
        iconBg: 'bg-blue-100',
        iconColor: 'text-blue-600',
        tagline: 'Central hub for all project tasks, agent flows, and artifacts.',
        prompts: [
          'Track all tasks in one unified view',
          'Monitor agent execution and approvals',
          'Review artifacts and deliverables',
        ],
      },
      custom: {
        icon: Shapes,
        accentBorder: 'border-slate-300/60',
        accentBackground: 'bg-gradient-to-br from-slate-200/30 via-slate-100/10 to-transparent',
        iconBg: 'bg-slate-100',
        iconColor: 'text-slate-600',
        tagline: 'Create a focused workspace for specific teams or initiatives.',
        prompts: [
          'Name the focus and success criteria',
          'Link supporting assets and rituals',
          'Assign owners and recurring cadences',
        ],
      },
    }),
    []
  );

  const resetBoardForm = useCallback(() => {
    setBoardForm({
      name: '',
      boardType: 'custom',
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
      boardId: 'none',
      checksum: '',
      mimeType: '',
      metadata: '',
      uploadedBy: '',
      byteSize: '',
    });
    setAssetFormError('');
  }, []);

  const loadEmailAccounts = useCallback(async () => {
    setEmailIntegrationError('');
    setEmailAccountsLoading(true);
    try {
      const result = await emailApi.listAccounts(projectId);
      setEmailAccounts(result);
    } catch (err) {
      console.error('Failed to load email accounts:', err);
      setEmailIntegrationError('Unable to load email integrations right now.');
    } finally {
      setEmailAccountsLoading(false);
    }
  }, [projectId]);

  const loadSocialAccounts = useCallback(async () => {
    setSocialIntegrationError('');
    setSocialAccountsLoading(true);
    try {
      const result = await socialApi.listAccounts(projectId);
      setSocialAccounts(result);
    } catch (err) {
      console.error('Failed to load social accounts:', err);
      setSocialIntegrationError('Unable to load social integrations right now.');
    } finally {
      setSocialAccountsLoading(false);
    }
  }, [projectId]);

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

  const refreshIntegrationStatuses = useCallback(async () => {
    setIntegrationsRefreshing(true);
    try {
      await Promise.all([loadEmailAccounts(), loadSocialAccounts()]);
    } finally {
      setIntegrationsRefreshing(false);
    }
  }, [loadEmailAccounts, loadSocialAccounts]);

  const openCrmTab = useCallback(
    (tab: 'contacts' | 'email' = 'contacts') => {
      const params = new URLSearchParams();
      params.set('projectId', projectId);
      params.set('tab', tab);
      navigate({ pathname: '/crm', search: params.toString() });
    },
    [navigate, projectId]
  );

  const openCrmEmailTab = useCallback(() => openCrmTab('email'), [openCrmTab]);

  const openSocialCommand = useCallback(() => {
    const params = new URLSearchParams();
    params.set('projectId', projectId);
    navigate({ pathname: '/social-command', search: params.toString() });
  }, [navigate, projectId]);

  const openStripeDashboard = useCallback(() => {
    if (typeof window === 'undefined') return;
    window.open('https://dashboard.stripe.com/', '_blank', 'noopener,noreferrer');
  }, []);

  const openAnalyticsDashboard = useCallback(() => {
    if (typeof window === 'undefined') return;
    window.open('https://analytics.google.com/', '_blank', 'noopener,noreferrer');
  }, []);

  const openVercelDashboard = useCallback(() => {
    if (typeof window === 'undefined') return;
    window.open('https://vercel.com/dashboard', '_blank', 'noopener,noreferrer');
  }, []);

  const openGithubAuth = useCallback(() => {
    void NiceModal.show('github-login').finally(() => {
      NiceModal.hide('github-login').catch(() => {});
    });
  }, []);

  const scrollToAirtable = useCallback(() => {
    airtableSectionRef.current?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }, []);

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
    fetchBoards();
    fetchAssets();
    fetchTasks();
  }, [fetchBoards, fetchAssets, fetchTasks]);

  useEffect(() => {
    loadEmailAccounts();
    loadSocialAccounts();
  }, [loadEmailAccounts, loadSocialAccounts]);

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

  // Prevent deletion of the default board
  const isDefaultBoard = useCallback(
    (boardType: ProjectBoard['board_type']) => boardType === 'default',
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

  const formatRelativeTime = useCallback((dateString?: string | null) => {
    if (!dateString) return '—';
    const timestamp = new Date(dateString).getTime();
    if (Number.isNaN(timestamp)) return '—';
    const diffMs = Date.now() - timestamp;
    const diffMinutes = Math.floor(diffMs / 60000);
    if (diffMinutes < 1) return 'just now';
    if (diffMinutes < 60) return `${diffMinutes}m ago`;
    const diffHours = Math.floor(diffMinutes / 60);
    if (diffHours < 24) return `${diffHours}h ago`;
    const diffDays = Math.floor(diffHours / 24);
    if (diffDays < 30) return `${diffDays}d ago`;
    const diffMonths = Math.floor(diffDays / 30);
    if (diffMonths < 12) return `${diffMonths}mo ago`;
    const diffYears = Math.floor(diffMonths / 12);
    return `${diffYears}y ago`;
  }, []);

  const brandPalette = useMemo(() => {
    if (!project) return BRAND_PALETTES[0];
    const source = (project.id || project.name || '').split('');
    const sum = source.reduce((acc, char) => acc + char.charCodeAt(0), 0);
    return BRAND_PALETTES[sum % BRAND_PALETTES.length];
  }, [project]);

  const brandInitials = useMemo(() => {
    if (!project?.name) return 'PR';
    const parts = project.name.split(/\s+/).filter(Boolean);
    if (parts.length === 0) return 'PR';
    if (parts.length === 1) {
      const [first] = parts;
      return first.slice(0, 2).toUpperCase();
    }
    return `${parts[0][0]}${parts[1][0]}`.toUpperCase();
  }, [project?.name]);

const brandTagline = useMemo(() => {
  if (!project) return 'Centralized brand + ops workspace.';
  if (project.git_repo_path) {
    return `Source of truth: ${project.git_repo_path}`;
  }
  if (project.dev_script) {
    return `Runs ${project.dev_script} with live previews.`;
  }
  return 'Orchestrate brand systems, deliverables, and agents in one view.';
}, [project]);

  const brandProfileKey = project?.id
    ? `${BRAND_PROFILE_STORAGE_PREFIX}${project.id}`
    : null;

  // Default values for brand profile display
  const defaultBrandProfileValues = useMemo(
    () => ({
      tagline: brandTagline,
      industry: 'Brand Studio',
      primaryColor: brandPalette.primary,
      secondaryColor: brandPalette.secondary,
    }),
    [brandPalette.primary, brandPalette.secondary, brandTagline]
  );

  // Load brand profile from database, with localStorage migration
  useEffect(() => {
    if (!project?.id) {
      setBrandProfile(null);
      return;
    }

    const loadBrandProfile = async () => {
      try {
        // First try to load from database
        const dbProfile = await projectsApi.getBrandProfile(project.id);

        if (dbProfile) {
          setBrandProfile(dbProfile);
          // Clear localStorage if we have DB data (migration complete)
          if (brandProfileKey && typeof window !== 'undefined') {
            window.localStorage.removeItem(brandProfileKey);
          }
          return;
        }

        // Check localStorage for legacy data to migrate
        if (brandProfileKey && typeof window !== 'undefined') {
          const stored = window.localStorage.getItem(brandProfileKey);
          if (stored) {
            try {
              const parsed = JSON.parse(stored);
              // Migrate to database
              const migrated = await projectsApi.upsertBrandProfile(project.id, {
                tagline: parsed.tagline ?? null,
                industry: parsed.industry ?? null,
                primaryColor: parsed.primaryColor ?? defaultBrandProfileValues.primaryColor,
                secondaryColor: parsed.secondaryColor ?? defaultBrandProfileValues.secondaryColor,
                brandVoice: null,
                targetAudience: null,
                logoAssetId: null,
                guidelinesAssetId: null,
              });
              setBrandProfile(migrated);
              // Clear localStorage after successful migration
              window.localStorage.removeItem(brandProfileKey);
              console.log('[BrandProfile] Migrated from localStorage to database');
              return;
            } catch (err) {
              console.error('Failed to migrate brand profile:', err);
            }
          }
        }

        // No existing data - profile will be null until user saves
        setBrandProfile(null);
      } catch (err) {
        console.error('Failed to load brand profile:', err);
        setBrandProfile(null);
      }
    };

    loadBrandProfile();
  }, [project?.id, brandProfileKey, defaultBrandProfileValues]);

  const persistBrandProfile = useCallback(
    async (profileData: {
      tagline: string;
      industry: string;
      primaryColor: string;
      secondaryColor: string;
    }) => {
      if (!project?.id) return;
      try {
        const saved = await projectsApi.upsertBrandProfile(project.id, {
          tagline: profileData.tagline || null,
          industry: profileData.industry || null,
          primaryColor: profileData.primaryColor,
          secondaryColor: profileData.secondaryColor,
          brandVoice: null,
          targetAudience: null,
          logoAssetId: null,
          guidelinesAssetId: null,
        });
        setBrandProfile(saved);
      } catch (err) {
        console.error('Failed to save brand profile:', err);
      }
    },
    [project?.id]
  );

  // Create effective brand profile for display, merging DB data with defaults
  const effectiveBrandProfile = useMemo(() => ({
    tagline: brandProfile?.tagline ?? defaultBrandProfileValues.tagline,
    industry: brandProfile?.industry ?? defaultBrandProfileValues.industry,
    primaryColor: brandProfile?.primaryColor ?? defaultBrandProfileValues.primaryColor,
    secondaryColor: brandProfile?.secondaryColor ?? defaultBrandProfileValues.secondaryColor,
  }), [brandProfile, defaultBrandProfileValues]);

  const brandHeroGradient = useMemo(
    () => ({
      backgroundImage: `linear-gradient(135deg, ${hexToRgba(
        effectiveBrandProfile.primaryColor,
        0.18
      )}, ${hexToRgba(effectiveBrandProfile.secondaryColor, 0.18)})`,
    }),
    [effectiveBrandProfile]
  );

  const handleBrandProfileSave = useCallback(() => {
    if (!brandProfileDraft) return;
    persistBrandProfile(brandProfileDraft);
    setIsBrandProfileDialogOpen(false);
    setBrandProfileDraft(null);
  }, [brandProfileDraft, persistBrandProfile]);

  const handleBrandProfileDialogOpen = useCallback(() => {
    setBrandProfileDraft(effectiveBrandProfile);
    setIsBrandProfileDialogOpen(true);
  }, [effectiveBrandProfile]);

  const repoLabel = useMemo(() => {
    if (!project?.git_repo_path) return 'Not linked';
    const normalized = project.git_repo_path.replace(/\.git$/, '');
    const segments = normalized.split('/');
    if (segments.length >= 2) {
      return segments.slice(-2).join('/');
    }
    return normalized;
  }, [project?.git_repo_path]);

  const totalBoards = boards.length;
  const totalAssets = assets.length;
  const totalTasks = tasks.length;
  const activeTasks = useMemo(
    () =>
      tasks.filter(
        (task) => task.status !== 'done' && task.status !== 'cancelled'
      ).length,
    [tasks]
  );

  const integrationCategories = useMemo<IntegrationCategory[]>(() => {
    const hasGithub = Boolean(config?.github?.pat || config?.github?.oauth_token);
    const githubDetail = hasGithub
      ? config?.github?.username
        ? `@${config.github.username}`
        : 'Authenticated via Settings → GitHub'
      : 'Connect via Settings → GitHub';
    const githubMeta = hasGithub
      ? config?.github?.primary_email || project?.git_repo_path || undefined
      : undefined;

    const buildEmailProvider = (
      provider: 'gmail' | 'zoho',
      label: string
    ): ProviderStatus => {
      if (emailAccountsLoading) {
        return {
          key: provider,
          label,
          status: 'manual',
          detail: 'Checking connection…',
          actionLabel: 'Open CRM',
          onAction: openCrmEmailTab,
        };
      }
      const account = emailAccounts.find(
        (entry) => entry.provider?.toLowerCase() === provider
      );
      if (!account) {
        return {
          key: provider,
          label,
          status: 'missing',
          detail: 'Connect via CRM → Email Accounts',
          actionLabel: 'Connect account',
          onAction: openCrmEmailTab,
        };
      }
      const normalizedStatus = (account.status || '').toLowerCase();
      const isHealthy = normalizedStatus === 'active';
      return {
        key: provider,
        label,
        status: isHealthy ? 'connected' : 'manual',
        detail: account.email_address,
        meta: isHealthy
          ? account.updated_at
            ? `Synced ${formatRelativeTime(account.updated_at)}`
            : undefined
          : `Status: ${formatStatusLabel(account.status || 'inactive')}`,
        actionLabel: isHealthy ? 'Manage mail' : 'Connect account',
        onAction: openCrmEmailTab,
      };
    };

    const buildSocialProvider = (platformKey: string, label: string): ProviderStatus => {
      if (socialAccountsLoading) {
        return {
          key: platformKey,
          label,
          status: 'manual',
          detail: 'Checking connection…',
          actionLabel: 'Open Social Command',
          onAction: openSocialCommand,
        };
      }
      const normalized =
        platformKey === 'x'
          ? 'twitter'
          : platformKey;
      const account = socialAccounts.find(
        (entry) => entry.platform?.toLowerCase() === normalized
      );
      if (!account) {
        return {
          key: platformKey,
          label,
          status: 'missing',
          detail: 'Connect via Social Command → Accounts',
          actionLabel: 'Connect account',
          onAction: openSocialCommand,
        };
      }
      const normalizedStatus = (account.status || '').toLowerCase();
      const isHealthy = normalizedStatus === 'active';
      const summary =
        account.username ||
        account.display_name ||
        account.profile_url ||
        'Connected';
      return {
        key: platformKey,
        label,
        status: isHealthy ? 'connected' : 'manual',
        detail: summary,
        meta: isHealthy
          ? account.last_sync_at
            ? `Synced ${formatRelativeTime(account.last_sync_at)}`
            : undefined
          : `Status: ${formatStatusLabel(account.status || 'pending')}`,
        actionLabel: isHealthy ? 'Manage social' : 'Connect account',
        onAction: openSocialCommand,
      };
    };

    const firstConnection = airtableConnections[0];
    const airtableStatus: ProviderStatus = {
      key: 'airtable',
      label: 'Airtable',
      status:
        airtableConnections.length > 0
          ? 'connected'
          : config?.airtable?.token
          ? 'manual'
          : 'missing',
      detail:
        airtableConnections.length === 0
          ? config?.airtable?.token
            ? 'Token saved. Connect a base for this project.'
            : 'Add an Airtable token in Settings → Integrations'
          : airtableConnections.length === 1
          ? firstConnection.airtable_base_name || 'Base connected'
          : `${airtableConnections.length} bases connected`,
      meta:
        firstConnection?.last_synced_at
          ? `Synced ${formatRelativeTime(firstConnection.last_synced_at)}`
          : undefined,
      actionLabel:
        airtableConnections.length === 0 ? 'Connect base' : 'Manage bases',
      onAction: scrollToAirtable,
    };

    const commerceStatus: ProviderStatus = {
      key: 'stripe',
      label: 'Stripe',
      status: 'manual',
      detail: 'Billing reconciled via Stripe exports today.',
      actionLabel: 'Open Stripe',
      onAction: openStripeDashboard,
    };

    const analyticsStatus: ProviderStatus = {
      key: 'ga',
      label: 'Google Analytics',
      status: 'manual',
      detail: 'Reporting handled via shared GA dashboards for now.',
      actionLabel: 'Open GA',
      onAction: openAnalyticsDashboard,
    };

    return [
      {
        key: 'development',
        label: 'Development',
        icon: Code2,
        accent: 'from-sky-50 via-white to-indigo-100',
        connectors: [
          {
            key: 'github',
            label: 'GitHub',
            status: hasGithub ? 'connected' : 'missing',
            detail: githubDetail,
            meta: githubMeta,
            actionLabel: hasGithub ? 'Manage auth' : 'Sign in',
            onAction: openGithubAuth,
          },
          {
            key: 'vercel',
            label: 'Vercel',
            status: 'manual',
            detail: 'Deploy previews run via GitHub workflows.',
            actionLabel: 'Open Vercel',
            onAction: openVercelDashboard,
          },
        ],
      },
      {
        key: 'communications',
        label: 'Communications',
        icon: Mail,
        accent: 'from-rose-50 via-white to-amber-100',
        connectors: [
          buildEmailProvider('gmail', 'Gmail'),
          buildEmailProvider('zoho', 'Zoho Mail'),
        ],
      },
      {
        key: 'social',
        label: 'Social',
        icon: Share2,
        accent: 'from-emerald-50 via-white to-blue-100',
        connectors: [
          buildSocialProvider('instagram', 'Instagram'),
          buildSocialProvider('facebook', 'Facebook'),
          buildSocialProvider('linkedin', 'LinkedIn'),
          buildSocialProvider('x', 'X (Twitter)'),
          buildSocialProvider('youtube', 'YouTube'),
          buildSocialProvider('tiktok', 'TikTok'),
        ],
      },
      {
        key: 'productivity',
        label: 'Productivity',
        icon: ClipboardCheck,
        accent: 'from-slate-50 via-white to-lime-100',
        connectors: [airtableStatus],
      },
      {
        key: 'commerce',
        label: 'Commerce',
        icon: CreditCard,
        accent: 'from-orange-50 via-white to-amber-100',
        connectors: [commerceStatus],
      },
      {
        key: 'analytics',
        label: 'Analytics',
        icon: BarChart3,
        accent: 'from-indigo-50 via-white to-slate-100',
        connectors: [analyticsStatus],
      },
    ];
  }, [
    airtableConnections,
    config,
    emailAccounts,
    emailAccountsLoading,
    formatRelativeTime,
    formatStatusLabel,
    openAnalyticsDashboard,
    openCrmEmailTab,
    openGithubAuth,
    openSocialCommand,
    openStripeDashboard,
    openVercelDashboard,
    project?.git_repo_path,
    scrollToAirtable,
    socialAccounts,
    socialAccountsLoading,
  ]);

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
        <div className="flex flex-col gap-4 border-b border-border/60 pb-6 sm:flex-row sm:items-center sm:justify-between">
          <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:gap-4">
            <Button variant="outline" onClick={onBack} className="w-fit">
              <ArrowLeft className="mr-2 h-4 w-4" />
              Back to Projects
            </Button>
            <div>
              <div className="flex flex-wrap items-center gap-3">
                <h1 className="text-2xl font-bold">{project.name}</h1>
                <Badge variant="secondary">Active</Badge>
              </div>
              <p className="text-sm text-muted-foreground">
                Brand identity, integrations, and delivery health
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

        <div className="grid gap-6 lg:grid-cols-3">
          <Card
            className="relative overflow-hidden border p-0 lg:col-span-2"
            style={brandHeroGradient}
          >
            <CardContent className="relative space-y-6 p-6">
              <div className="space-y-4">
                <div className="flex flex-wrap items-center justify-between gap-3">
                  <div className="flex flex-wrap items-center gap-2 text-xs uppercase text-muted-foreground">
                    <span>
                      Updated {new Date(project.updated_at).toLocaleDateString()}
                    </span>
                    <span className="text-muted-foreground/60">•</span>
                    <span>
                      {totalBoards} board{totalBoards === 1 ? '' : 's'} · {totalTasks}{' '}
                      task{totalTasks === 1 ? '' : 's'}
                    </span>
                  </div>
                  <Button variant="outline" size="sm" onClick={handleBrandProfileDialogOpen}>
                    Adjust brand profile
                  </Button>
                </div>
                <div className="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
                  <div className="flex items-center gap-4">
                    <div className="flex h-16 w-16 items-center justify-center rounded-2xl border border-white/70 bg-white/80 text-2xl font-semibold text-foreground shadow-sm">
                      {brandInitials}
                    </div>
                    <div>
                      <p className="text-xs uppercase tracking-wide text-muted-foreground">
                        Brand Identity
                      </p>
                      <p className="text-3xl font-bold leading-tight text-foreground">
                        {project.name}
                      </p>
                      <p className="text-base text-muted-foreground">
                        {effectiveBrandProfile.tagline}
                      </p>
                      <p className="text-xs uppercase tracking-wide text-muted-foreground/80">
                        Industry
                      </p>
                      <p className="text-sm font-semibold text-foreground">
                        {effectiveBrandProfile.industry || 'Brand Studio'}
                      </p>
                    </div>
                  </div>
                  <div className="grid w-full gap-3 sm:grid-cols-3 lg:w-auto">
                    <div className="rounded-2xl bg-white/80 p-4 text-center shadow-sm">
                      <p className="text-xs uppercase text-muted-foreground">Boards</p>
                      <p className="text-2xl font-semibold text-foreground">
                        {totalBoards}
                      </p>
                      <p className="text-xs text-muted-foreground">Active lanes</p>
                    </div>
                    <div className="rounded-2xl bg-white/80 p-4 text-center shadow-sm">
                      <p className="text-xs uppercase text-muted-foreground">Active Tasks</p>
                      <p className="text-2xl font-semibold text-foreground">
                        {activeTasks}
                      </p>
                      <p className="text-xs text-muted-foreground">In flight</p>
                    </div>
                    <div className="rounded-2xl bg-white/80 p-4 text-center shadow-sm">
                      <p className="text-xs uppercase text-muted-foreground">Assets</p>
                      <p className="text-2xl font-semibold text-foreground">
                        {totalAssets}
                      </p>
                      <p className="text-xs text-muted-foreground">Catalogued</p>
                    </div>
                  </div>
                </div>
              </div>
              <div className="space-y-3">
                <div className="flex items-center gap-2 text-xs uppercase tracking-wide text-muted-foreground">
                  <Palette className="h-3.5 w-3.5" />
                  Brand palette
                </div>
                <div className="flex flex-wrap gap-6">
                  <div className="flex items-center gap-3">
                    <span
                      className="h-10 w-10 rounded-xl shadow-inner"
                      style={{ background: effectiveBrandProfile.primaryColor }}
                    />
                    <div>
                      <p className="text-xs uppercase text-muted-foreground">Primary</p>
                      <p className="font-mono text-sm">{effectiveBrandProfile.primaryColor}</p>
                    </div>
                  </div>
                  <div className="flex items-center gap-3">
                    <span
                      className="h-10 w-10 rounded-xl shadow-inner"
                      style={{ background: effectiveBrandProfile.secondaryColor }}
                    />
                    <div>
                      <p className="text-xs uppercase text-muted-foreground">Secondary</p>
                      <p className="font-mono text-sm">{effectiveBrandProfile.secondaryColor}</p>
                    </div>
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>
          <Card>
            <CardHeader>
              <CardTitle>Operational Snapshot</CardTitle>
              <CardDescription>
                Metadata, repo link, and automation hooks for this workspace.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4 text-sm">
              <div>
                <p className="text-xs uppercase text-muted-foreground">Project ID</p>
                <code className="mt-1 block rounded bg-muted px-2 py-1 font-mono text-xs">
                  {project.id}
                </code>
              </div>
              <div className="grid gap-3">
                <div className="flex items-center text-sm">
                  <Calendar className="mr-2 h-4 w-4 text-muted-foreground" />
                  Created {new Date(project.created_at).toLocaleDateString()}
                </div>
                <div className="flex items-center text-sm">
                  <Clock className="mr-2 h-4 w-4 text-muted-foreground" />
                  Updated {new Date(project.updated_at).toLocaleDateString()}
                </div>
              </div>
              <div>
                <p className="text-xs uppercase text-muted-foreground">Repository</p>
                <p className="text-sm font-medium text-foreground">{repoLabel}</p>
                <p className="text-xs text-muted-foreground">
                  {project.git_repo_path
                    ? 'Synced automatically from GitHub.'
                    : 'Link a repository to enable automated PRs.'}
                </p>
              </div>
              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <span className="text-xs uppercase text-muted-foreground">
                    Dev Script
                  </span>
                  <span className="font-medium text-foreground">
                    {project.dev_script || 'pnpm run dev'}
                  </span>
                </div>
                {project.setup_script && (
                  <div className="flex items-center justify-between">
                    <span className="text-xs uppercase text-muted-foreground">
                      Setup
                    </span>
                    <span className="font-medium text-foreground">
                      {project.setup_script}
                    </span>
                  </div>
                )}
                {project.cleanup_script && (
                  <div className="flex items-center justify-between">
                    <span className="text-xs uppercase text-muted-foreground">
                      Cleanup
                    </span>
                    <span className="font-medium text-foreground">
                      {project.cleanup_script}
                    </span>
                  </div>
                )}
              </div>
            </CardContent>
          </Card>
        </div>

        <Card>
          <CardHeader className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <Sparkles className="h-5 w-5" />
                Integrations Hub
              </CardTitle>
              <CardDescription>
                Central view of the systems powering this project.
              </CardDescription>
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={refreshIntegrationStatuses}
              disabled={integrationsRefreshing}
            >
              {integrationsRefreshing && (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              )}
              Refresh status
            </Button>
          </CardHeader>
          <CardContent className="space-y-4">
            {(emailIntegrationError || socialIntegrationError) && (
              <Alert variant="destructive">
                <AlertCircle className="h-4 w-4" />
                <AlertDescription>
                  {emailIntegrationError && <p>{emailIntegrationError}</p>}
                  {socialIntegrationError && <p>{socialIntegrationError}</p>}
                </AlertDescription>
              </Alert>
            )}
            <div className="grid gap-4 lg:grid-cols-2">
              {integrationCategories.map((category) => {
                const Icon = category.icon;
                const connectedCount = category.connectors.filter(
                  (connector) => connector.status === 'connected'
                ).length;
                return (
                  <div
                    key={category.key}
                    className="rounded-2xl border bg-background/80 p-4 shadow-sm"
                  >
                    <div className="flex items-start justify-between gap-3">
                      <div className="flex items-center gap-3">
                        <span
                          className={cn(
                            'flex h-10 w-10 items-center justify-center rounded-full bg-gradient-to-br text-primary',
                            category.accent
                          )}
                        >
                          <Icon className="h-5 w-5" />
                        </span>
                        <div>
                          <p className="text-sm font-semibold text-foreground">
                            {category.label}
                          </p>
                          <p className="text-xs text-muted-foreground">
                            {connectedCount}/{category.connectors.length} connected
                          </p>
                        </div>
                      </div>
                    </div>
                    <div className="mt-4 space-y-3">
                      {category.connectors.map((connector) => (
                        <div
                          key={connector.key}
                          className="rounded-xl border bg-background/70 p-3 shadow-sm"
                        >
                          <div className="flex items-center justify-between gap-3">
                            <p className="text-sm font-semibold text-foreground">
                              {connector.label}
                            </p>
                            <span
                              className={cn(
                                'rounded-full border px-2.5 py-0.5 text-xs font-medium',
                                integrationStatusStyles[connector.status]
                              )}
                            >
                              {formatStatusLabel(connector.status)}
                            </span>
                          </div>
                          <p className="text-xs text-muted-foreground">
                            {connector.detail}
                          </p>
                          {connector.meta && (
                            <p className="text-[11px] text-muted-foreground/80">
                              {connector.meta}
                            </p>
                          )}
                          {connector.onAction && (
                            <div className="pt-3">
                              <Button
                                variant="outline"
                                size="sm"
                                onClick={(event) => {
                                  event.stopPropagation();
                                  connector.onAction?.();
                                }}
                              >
                                {connector.actionLabel ?? 'Manage'}
                              </Button>
                            </div>
                          )}
                        </div>
                      ))}
                    </div>
                  </div>
                );
              })}
            </div>
          </CardContent>
        </Card>

        <div ref={airtableSectionRef}>
          <AirtableBaseConnect
            projectId={project.id}
            projectName={project.name}
            onConnectionsChange={setAirtableConnections}
          />
        </div>

      <div className="grid gap-6 lg:grid-cols-2">
        <Card className="h-full">
          <CardHeader className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <LayoutGrid className="h-5 w-5" />
                Workstreams & Boards
              </CardTitle>
              <CardDescription>
                Activate and monitor each delivery lane across the brand.
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
                          {!isDefaultBoard(board.board_type) && (
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
                <Folder className="h-5 w-5" />
                Asset Vault
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
      open={isBrandProfileDialogOpen}
      onOpenChange={(open) => {
        setIsBrandProfileDialogOpen(open);
        if (!open) {
          setBrandProfileDraft(null);
        }
      }}
    >
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>Adjust Brand Profile</DialogTitle>
          <p className="text-sm text-muted-foreground">
            Tune the tagline, industry, and palette for this project overview.
          </p>
        </DialogHeader>
        <div className="space-y-4">
          <div className="space-y-1.5">
            <Label htmlFor="brand-tagline">Tagline</Label>
            <Input
              id="brand-tagline"
              value={brandProfileDraft?.tagline ?? ''}
              onChange={(event) =>
                setBrandProfileDraft((prev) => ({
                  ...(prev ?? defaultBrandProfileValues),
                  tagline: event.target.value,
                }))
              }
            />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="brand-industry">Industry</Label>
            <Input
              id="brand-industry"
              value={brandProfileDraft?.industry ?? ''}
              onChange={(event) =>
                setBrandProfileDraft((prev) => ({
                  ...(prev ?? defaultBrandProfileValues),
                  industry: event.target.value,
                }))
              }
              placeholder="e.g. SaaS, Agency, E-commerce"
            />
          </div>
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="space-y-1.5">
              <Label htmlFor="brand-primary">Primary Color</Label>
              <div className="flex items-center gap-3">
                <Input
                  id="brand-primary"
                  type="color"
                  value={brandProfileDraft?.primaryColor ?? defaultBrandProfileValues.primaryColor}
                  onChange={(event) =>
                    setBrandProfileDraft((prev) => ({
                      ...(prev ?? defaultBrandProfileValues),
                      primaryColor: event.target.value,
                    }))
                  }
                  className="h-10 w-16 cursor-pointer"
                />
                <Input
                  value={brandProfileDraft?.primaryColor ?? defaultBrandProfileValues.primaryColor}
                  onChange={(event) =>
                    setBrandProfileDraft((prev) => ({
                      ...(prev ?? defaultBrandProfileValues),
                      primaryColor: event.target.value,
                    }))
                  }
                />
              </div>
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="brand-secondary">Secondary Color</Label>
              <div className="flex items-center gap-3">
                <Input
                  id="brand-secondary"
                  type="color"
                  value={brandProfileDraft?.secondaryColor ?? defaultBrandProfileValues.secondaryColor}
                  onChange={(event) =>
                    setBrandProfileDraft((prev) => ({
                      ...(prev ?? defaultBrandProfileValues),
                      secondaryColor: event.target.value,
                    }))
                  }
                  className="h-10 w-16 cursor-pointer"
                />
                <Input
                  value={brandProfileDraft?.secondaryColor ?? defaultBrandProfileValues.secondaryColor}
                  onChange={(event) =>
                    setBrandProfileDraft((prev) => ({
                      ...(prev ?? defaultBrandProfileValues),
                      secondaryColor: event.target.value,
                    }))
                  }
                />
              </div>
            </div>
          </div>
        </div>
        <DialogFooter>
          <Button
            variant="ghost"
            onClick={() => {
              setIsBrandProfileDialogOpen(false);
              setBrandProfileDraft(null);
            }}
          >
            Cancel
          </Button>
          <Button onClick={handleBrandProfileSave} disabled={!brandProfileDraft}>
            Save profile
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
