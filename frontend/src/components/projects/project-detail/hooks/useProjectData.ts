import NiceModal from '@ebay/nice-modal-react';
import {
  BarChart3,
  ClipboardCheck,
  Code2,
  CreditCard,
  Mail,
  Share2,
} from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import type {
  AirtableBase,
  BrandProfile,
  Project,
  ProjectAsset,
  ProjectBoard,
  TaskWithAttemptStatus,
} from 'shared/types';

import { useUserSystem } from '@/components/config-provider';
import {
  type EmailAccountRecord,
  emailApi,
  projectsApi,
  type SocialAccountRecord,
  socialApi,
  tasksApi,
} from '@/lib/api';
import { showProjectForm } from '@/lib/modals';

import {
  formatRelativeTime,
  formatStatusLabel,
  getBrandInitials,
  getBrandTagline,
  getRepoLabel,
  hexToRgba,
  slugify,
} from '../helpers';
import type { IntegrationCategory, ProviderStatus } from '../types';
import { BRAND_PALETTES, BRAND_PROFILE_STORAGE_PREFIX } from '../types';

export function useProjectData(projectId: string, onBack: () => void) {
  const navigate = useNavigate();
  const { config } = useUserSystem();

  // --- Core data state ---
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

  // --- Integration state ---
  const [emailAccounts, setEmailAccounts] = useState<EmailAccountRecord[]>([]);
  const [emailAccountsLoading, setEmailAccountsLoading] = useState(true);
  const [emailIntegrationError, setEmailIntegrationError] = useState('');
  const [socialAccounts, setSocialAccounts] = useState<SocialAccountRecord[]>(
    []
  );
  const [socialAccountsLoading, setSocialAccountsLoading] = useState(true);
  const [socialIntegrationError, setSocialIntegrationError] = useState('');
  const [airtableConnections, setAirtableConnections] = useState<
    AirtableBase[]
  >([]);
  const [integrationsRefreshing, setIntegrationsRefreshing] = useState(false);

  // --- Board form state ---
  const [isCreateBoardOpen, setIsCreateBoardOpen] = useState(false);
  const [boardForm, setBoardForm] = useState({
    name: '',
    boardType: 'brand_assets' as ProjectBoard['board_type'],
    description: '',
  });
  const [boardFormSubmitting, setBoardFormSubmitting] = useState(false);
  const [boardFormError, setBoardFormError] = useState('');

  // --- Asset form state ---
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

  // --- Brand profile state ---
  const [brandProfile, setBrandProfile] = useState<BrandProfile | null>(null);
  const [isBrandProfileDialogOpen, setIsBrandProfileDialogOpen] =
    useState(false);
  const [brandProfileDraft, setBrandProfileDraft] = useState<{
    tagline: string;
    industry: string;
    primaryColor: string;
    secondaryColor: string;
  } | null>(null);

  const airtableSectionRef = useRef<HTMLDivElement | null>(null);

  // --- Fetch callbacks ---
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
      const result = await socialApi.listAccounts({ projectId });
      setSocialAccounts(result);
    } catch (err) {
      console.error('Failed to load social accounts:', err);
      setSocialIntegrationError(
        'Unable to load social integrations right now.'
      );
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
    } catch (error: unknown) {
      console.error('Failed to fetch project assets:', error);
      const message =
        error instanceof Error ? error.message : 'Failed to load brand assets';
      if (
        error instanceof Object &&
        'response' in error &&
        (error as { response?: { status?: number } }).response?.status === 404
      ) {
        setAssetsError('No assets found for this project.');
      } else {
        setAssetsError(message);
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

  // --- Navigation callbacks ---
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
    window.open(
      'https://dashboard.stripe.com/',
      '_blank',
      'noopener,noreferrer'
    );
  }, []);

  const openAnalyticsDashboard = useCallback(() => {
    if (typeof window === 'undefined') return;
    window.open(
      'https://analytics.google.com/',
      '_blank',
      'noopener,noreferrer'
    );
  }, []);

  const openVercelDashboard = useCallback(() => {
    if (typeof window === 'undefined') return;
    window.open(
      'https://vercel.com/dashboard',
      '_blank',
      'noopener,noreferrer'
    );
  }, []);

  const openGithubAuth = useCallback(() => {
    void NiceModal.show('github-login').finally(() => {
      NiceModal.hide('github-login').catch(() => {});
    });
  }, []);

  const scrollToAirtable = useCallback(() => {
    airtableSectionRef.current?.scrollIntoView({
      behavior: 'smooth',
      block: 'start',
    });
  }, []);

  // --- Form reset callbacks ---
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

  // --- CRUD handlers ---
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
    const { toast } = await import('sonner');
    try {
      await projectsApi.createAsset(projectId, {
        name: assetForm.name.trim(),
        storage_path: assetForm.storagePath.trim(),
        category: assetForm.category as 'file' | 'transcript' | 'link' | 'note',
        scope: assetForm.scope as 'owner' | 'client' | 'team' | 'public',
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
      const { toast } = await import('sonner');
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

  const handleCreateBoard = useCallback(async () => {
    const trimmedName = boardForm.name.trim();
    if (!trimmedName) {
      setBoardFormError('Board name is required.');
      return;
    }

    setBoardFormSubmitting(true);
    setBoardFormError('');
    const { toast } = await import('sonner');
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
  }, [boardForm, projectId, fetchBoards, fetchTasks, resetBoardForm]);

  const handleDeleteBoard = useCallback(
    async (board: ProjectBoard) => {
      if (
        !confirm(
          `Delete the "${board.name}" board? Tasks and assets linked to it will keep their board reference.`
        )
      ) {
        return;
      }
      const { toast } = await import('sonner');
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
    } catch {
      // User cancelled - do nothing
    }
  };

  // --- Effects ---
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

  // --- Derived data ---
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

  const isDefaultBoard = useCallback(
    (boardType: ProjectBoard['board_type']) => boardType === 'default',
    []
  );

  // --- Brand profile ---
  const brandPalette = useMemo(() => {
    if (!project) return BRAND_PALETTES[0];
    const source = (project.id || project.name || '').split('');
    const sum = source.reduce((acc, char) => acc + char.charCodeAt(0), 0);
    return BRAND_PALETTES[sum % BRAND_PALETTES.length];
  }, [project]);

  const brandInitials = useMemo(
    () => getBrandInitials(project?.name),
    [project?.name]
  );
  const brandTagline = useMemo(() => getBrandTagline(project), [project]);
  const repoLabel = useMemo(
    () => getRepoLabel(project?.git_repo_path),
    [project?.git_repo_path]
  );

  const brandProfileKey = project?.id
    ? `${BRAND_PROFILE_STORAGE_PREFIX}${project.id}`
    : null;

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
        const dbProfile = await projectsApi.getBrandProfile(project.id);

        if (dbProfile) {
          setBrandProfile(dbProfile);
          if (brandProfileKey && typeof window !== 'undefined') {
            window.localStorage.removeItem(brandProfileKey);
          }
          return;
        }

        if (brandProfileKey && typeof window !== 'undefined') {
          const stored = window.localStorage.getItem(brandProfileKey);
          if (stored) {
            try {
              const parsed = JSON.parse(stored);
              const migrated = await projectsApi.upsertBrandProfile(
                project.id,
                {
                  tagline: parsed.tagline ?? null,
                  industry: parsed.industry ?? null,
                  primaryColor:
                    parsed.primaryColor ??
                    defaultBrandProfileValues.primaryColor,
                  secondaryColor:
                    parsed.secondaryColor ??
                    defaultBrandProfileValues.secondaryColor,
                  brandVoice: null,
                  targetAudience: null,
                  logoAssetId: null,
                  guidelinesAssetId: null,
                }
              );
              setBrandProfile(migrated);
              window.localStorage.removeItem(brandProfileKey);
              return;
            } catch (err) {
              console.error('Failed to migrate brand profile:', err);
            }
          }
        }

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

  const effectiveBrandProfile = useMemo(
    () => ({
      tagline: brandProfile?.tagline ?? defaultBrandProfileValues.tagline,
      industry: brandProfile?.industry ?? defaultBrandProfileValues.industry,
      primaryColor:
        brandProfile?.primaryColor ?? defaultBrandProfileValues.primaryColor,
      secondaryColor:
        brandProfile?.secondaryColor ??
        defaultBrandProfileValues.secondaryColor,
    }),
    [brandProfile, defaultBrandProfileValues]
  );

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

  // --- Stats ---
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

  // --- Integration categories ---
  const integrationCategories = useMemo<IntegrationCategory[]>(() => {
    const hasGithub = Boolean(
      config?.github?.pat || config?.github?.oauth_token
    );
    const githubDetail = hasGithub
      ? config?.github?.username
        ? `@${config.github.username}`
        : 'Authenticated via Settings \u2192 GitHub'
      : 'Connect via Settings \u2192 GitHub';
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
          detail: 'Checking connection\u2026',
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
          detail: 'Connect via CRM \u2192 Email Accounts',
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

    const buildSocialProvider = (
      platformKey: string,
      label: string
    ): ProviderStatus => {
      if (socialAccountsLoading) {
        return {
          key: platformKey,
          label,
          status: 'manual',
          detail: 'Checking connection\u2026',
          actionLabel: 'Open Social Command',
          onAction: openSocialCommand,
        };
      }
      const normalized = platformKey === 'x' ? 'twitter' : platformKey;
      const account = socialAccounts.find(
        (entry) => entry.platform?.toLowerCase() === normalized
      );
      if (!account) {
        return {
          key: platformKey,
          label,
          status: 'missing',
          detail: 'Connect via Social Command \u2192 Accounts',
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
            : 'Add an Airtable token in Settings \u2192 Integrations'
          : airtableConnections.length === 1
            ? firstConnection.airtable_base_name || 'Base connected'
            : `${airtableConnections.length} bases connected`,
      meta: firstConnection?.last_synced_at
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

  return {
    // Core data
    project,
    loading,
    error,
    boards,
    boardsLoading,
    boardsError,
    assets,
    assetsLoading,
    assetsError,
    tasks,
    tasksLoading,
    tasksError,

    // Integration
    emailIntegrationError,
    socialIntegrationError,
    integrationsRefreshing,
    integrationCategories,
    airtableSectionRef,
    setAirtableConnections,

    // Board form
    isCreateBoardOpen,
    setIsCreateBoardOpen,
    boardForm,
    setBoardForm,
    boardFormSubmitting,
    boardFormError,
    resetBoardForm,
    handleCreateBoard,
    handleDeleteBoard,

    // Asset form
    isCreateAssetOpen,
    setIsCreateAssetOpen,
    assetForm,
    setAssetForm,
    assetFormSubmitting,
    assetFormError,
    resetAssetForm,
    handleCreateAsset,
    handleDeleteAsset,

    // Brand profile
    isBrandProfileDialogOpen,
    setIsBrandProfileDialogOpen,
    brandProfileDraft,
    setBrandProfileDraft,
    effectiveBrandProfile,
    defaultBrandProfileValues,
    brandHeroGradient,
    brandInitials,
    handleBrandProfileSave,
    handleBrandProfileDialogOpen,

    // Derived
    boardById,
    tasksByBoard,
    unassignedTasks,
    totalBoards,
    totalAssets,
    totalTasks,
    activeTasks,
    repoLabel,
    isDefaultBoard,

    // Actions
    handleDelete,
    handleEditClick,
    refreshIntegrationStatuses,
    navigate,
  };
}
