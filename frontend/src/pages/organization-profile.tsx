import { useMemo, useState } from 'react';
import { useParams, useSearchParams, Link } from 'react-router-dom';
import { useQuery, useQueries, useMutation, useQueryClient } from '@tanstack/react-query';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Input } from '@/components/ui/input';
import { Progress } from '@/components/ui/progress';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import {
  Building2,
  Users,
  FolderOpen,
  Briefcase,
  Target,
  TrendingUp,
  ArrowLeft,
  Globe,
  ExternalLink,
  LayoutGrid,
  Contact2,
  DollarSign,
  Activity,
  X,
  Search,
  BookOpen,
  Brain,
  Share2,
  MessageSquare,
  FileText,
  Radio,
  Pencil,
  Boxes,
  Network,
  AlertTriangle,
  Linkedin,
  Instagram,
  Twitter,
  Facebook,
  Youtube,
  Inbox,
  Database,
  Plus,
  MoreHorizontal,
  Trash2,
  Upload,
  ArrowUpDown,
  ArrowUp,
  ArrowDown,
  GitBranch,
  Clock,
  ChevronDown,
  ChevronRight,
} from 'lucide-react';
import {
  organizationsApi,
  crmDealsApi,
  crmActivitiesApi,
  knowledgeApi,
  socialApi,
  tasksApi,
  type OrganizationData,
  type ClientData,
  type CrmActivityRecord,
  type ProjectKnowledgeResponse,
  type ProjectKnowledgeSource,
  type SocialAccountRecord,
  type SocialMentionRecord,
  type PersonOrgContact,
  dataSourcesApi,
  workflowsApi,
  type DataSourceRecord,
  type UpdateDataSourceRequest,
  type ExecutionArtifact,
  DATA_TYPE_OPTIONS,
} from '@/lib/api';
import { CrmPipelineBoard } from '@/components/crm/CrmPipelineBoard';

import { useOrgContacts, type OrgContact } from '@/hooks/useOrgContacts';
import { LIFECYCLE_STAGE_INFO, type LifecycleStage } from '@/types/crm';
import type { PipelineType } from '@/types/crm';

// ── Types ─────────────────────────────────────────────────────────────────────

interface OrgMember {
  id: string;
  user_id: string;
  role: string;
  joined_at: string;
  user?: { username: string; full_name: string; email: string; avatar_url?: string };
}

interface OrganizationProfilePageProps {
  defaultTab?: string;
  defaultPipeline?: string;
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function formatDate(iso: string) {
  return new Date(iso).toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
}

function formatCurrency(amount: number) {
  if (amount >= 1_000_000) return `$${(amount / 1_000_000).toFixed(1)}M`;
  if (amount >= 1_000) return `$${(amount / 1_000).toFixed(0)}k`;
  return `$${amount.toFixed(0)}`;
}

// ── Overview Tab ──────────────────────────────────────────────────────────────

function OverviewTab({
  orgId: _orgId,
  projectEntries,
  projectCount,
  clientCount: _clientCount,
  memberCount: _memberCount,
  totalDealValue,
  totalDeals,
  contactCount,
  onSwitchTab,
}: {
  orgId: string;
  projectEntries: { id: string; name: string }[];
  projectCount: number;
  clientCount: number;
  memberCount: number;
  totalDealValue: number;
  totalDeals: number;
  contactCount: number;
  onSwitchTab: (tab: string) => void;
}) {
  // Aggregate tasks across all projects
  const taskQueries = useQueries({
    queries: projectEntries.map((entry) => ({
      queryKey: ['tasks', entry.id],
      queryFn: () => tasksApi.getAll(entry.id),
      staleTime: 60_000,
      enabled: projectEntries.length > 0,
    })),
  });

  const totalTasks = useMemo(
    () => taskQueries.reduce((sum, q) => sum + (q.data?.length || 0), 0),
    [taskQueries]
  );

  // Aggregate activities across all projects
  const activityQueries = useQueries({
    queries: projectEntries.map((entry) => ({
      queryKey: ['crm-activities-org', entry.id],
      queryFn: () => crmActivitiesApi.listActivities({ project_id: entry.id, limit: 10 }),
      staleTime: 60_000,
      enabled: projectEntries.length > 0,
    })),
  });

  const recentActivities = useMemo(() => {
    const all: (CrmActivityRecord & { _projectName: string })[] = [];
    activityQueries.forEach((q, i) => {
      if (!q.data) return;
      const entry = projectEntries[i];
      q.data.forEach(a => all.push({ ...a, _projectName: entry.name }));
    });
    all.sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime());
    return all.slice(0, 20);
  }, [activityQueries, projectEntries]);

  return (
    <div className="space-y-6">
      {/* Stat cards */}
      <div className="grid grid-cols-2 lg:grid-cols-5 gap-4 animate-stagger">
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardContent className="pt-4 pb-3">
            <div className="flex items-center gap-2 text-muted-foreground">
              <Activity className="h-4 w-4" />
              <span className="text-xs font-medium">Total Tasks</span>
            </div>
            <p className="text-2xl font-bold mt-1">{totalTasks}</p>
          </CardContent>
        </Card>
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardContent className="pt-4 pb-3">
            <div className="flex items-center gap-2 text-muted-foreground">
              <Target className="h-4 w-4" />
              <span className="text-xs font-medium">Total Deals</span>
            </div>
            <p className="text-2xl font-bold mt-1">{totalDeals}</p>
          </CardContent>
        </Card>
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardContent className="pt-4 pb-3">
            <div className="flex items-center gap-2 text-muted-foreground">
              <DollarSign className="h-4 w-4" />
              <span className="text-xs font-medium">Pipeline Value</span>
            </div>
            <p className="text-2xl font-bold mt-1">{formatCurrency(totalDealValue)}</p>
          </CardContent>
        </Card>
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardContent className="pt-4 pb-3">
            <div className="flex items-center gap-2 text-muted-foreground">
              <Contact2 className="h-4 w-4" />
              <span className="text-xs font-medium">Contacts</span>
            </div>
            <p className="text-2xl font-bold mt-1">{contactCount}</p>
          </CardContent>
        </Card>
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardContent className="pt-4 pb-3">
            <div className="flex items-center gap-2 text-muted-foreground">
              <FolderOpen className="h-4 w-4" />
              <span className="text-xs font-medium">Active Projects</span>
            </div>
            <p className="text-2xl font-bold mt-1">{projectCount}</p>
          </CardContent>
        </Card>
      </div>

      {/* Quick links */}
      <div className="grid grid-cols-2 lg:grid-cols-3 gap-3">
        {[
          { label: 'Pipelines', icon: Target, tab: 'pipelines', color: 'text-amber-500' },
          { label: 'Contacts', icon: Contact2, tab: 'contacts', color: 'text-blue-500' },
          { label: 'Projects', icon: FolderOpen, tab: 'projects', color: 'text-emerald-500' },
          { label: 'Social', icon: Share2, tab: 'social', color: 'text-pink-500' },
          { label: 'Intelligence', icon: Brain, tab: 'knowledge', color: 'text-orange-500' },
          { label: 'Members', icon: Users, tab: 'members', color: 'text-purple-500' },
        ].map(({ label, icon: Icon, tab, color }) => (
          <button
            key={tab}
            onClick={() => onSwitchTab(tab)}
            className="flex items-center gap-3 p-4 rounded-xl border border-border/50 bg-card/50 hover:bg-accent/50 hover:border-accent transition-all text-left group"
          >
            <Icon className={`h-5 w-5 ${color} group-hover:scale-110 transition-transform`} />
            <span className="text-sm font-medium">{label}</span>
            <ExternalLink className="h-3 w-3 text-muted-foreground ml-auto opacity-0 group-hover:opacity-100 transition-opacity" />
          </button>
        ))}
      </div>

      {/* Recent activity - aggregated across all projects */}
      <Card className="bg-card/80 backdrop-blur-sm border-border/50">
        <CardHeader>
          <CardTitle className="text-base flex items-center gap-2">
            <Activity className="h-4 w-4" />
            Recent Activity
          </CardTitle>
          <CardDescription>Latest CRM activity across all {projectCount} projects</CardDescription>
        </CardHeader>
        <CardContent>
          {recentActivities.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <Activity className="h-8 w-8 mx-auto mb-2 opacity-40" />
              <p>No recent activity</p>
            </div>
          ) : (
            <div className="space-y-3">
              {recentActivities.map((activity) => (
                <div key={activity.id} className="flex items-start gap-3 p-3 rounded-lg border border-border/30">
                  <div className="h-8 w-8 rounded-full bg-muted flex items-center justify-center shrink-0">
                    <Activity className="h-4 w-4 text-muted-foreground" />
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2 flex-wrap">
                      <Badge variant="outline" className="text-[10px]">
                        {activity.activity_type.replace(/_/g, ' ')}
                      </Badge>
                      <Badge variant="secondary" className="text-[10px]">
                        {activity._projectName}
                      </Badge>
                    </div>
                    {activity.subject && (
                      <p className="text-sm mt-1">{activity.subject}</p>
                    )}
                    {activity.description && (
                      <p className="text-xs text-muted-foreground mt-0.5 line-clamp-2">{activity.description}</p>
                    )}
                    <p className="text-[10px] text-muted-foreground mt-1">
                      {formatDate(activity.created_at)}
                    </p>
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

// ── Pipelines Tab ─────────────────────────────────────────────────────────────

function PipelinesTab({ orgId, defaultPipeline }: { orgId: string; defaultPipeline?: string }) {
  const [pipelineType, setPipelineType] = useState<'sales' | 'delivery'>(
    defaultPipeline === 'lifecycle' ? 'delivery' : 'sales'
  );

  return (
    <div className="space-y-4">
      <div className="flex gap-1 p-1 bg-muted/50 rounded-lg w-fit">
        <button
          onClick={() => setPipelineType('sales')}
          className={`px-4 py-1.5 text-sm font-medium rounded-md transition-all ${
            pipelineType === 'sales'
              ? 'bg-background text-foreground shadow-sm'
              : 'text-muted-foreground hover:text-foreground'
          }`}
        >
          <Target className="h-3.5 w-3.5 inline mr-1.5" />
          Acquisition
        </button>
        <button
          onClick={() => setPipelineType('delivery')}
          className={`px-4 py-1.5 text-sm font-medium rounded-md transition-all ${
            pipelineType === 'delivery'
              ? 'bg-background text-foreground shadow-sm'
              : 'text-muted-foreground hover:text-foreground'
          }`}
        >
          <TrendingUp className="h-3.5 w-3.5 inline mr-1.5" />
          Lifecycle
        </button>
      </div>
      <CrmPipelineBoard
        orgId={orgId}
        pipelineType={pipelineType as PipelineType}
        title={pipelineType === 'sales' ? 'Acquisition Pipeline' : 'Client Lifecycle'}
      />
    </div>
  );
}

// ── Contacts Tab ──────────────────────────────────────────────────────────────

function ContactsTab({ orgId }: { orgId: string }) {
  const { contacts, isLoading } = useOrgContacts(orgId);
  const [searchQuery, setSearchQuery] = useState('');
  const [stageFilter, setStageFilter] = useState<string>('all');

  const { data: personContacts = [] } = useQuery<PersonOrgContact[]>({
    queryKey: ['org-person-contacts', orgId],
    queryFn: () => organizationsApi.listPersonContacts(orgId),
    enabled: !!orgId,
    staleTime: 60_000,
  });

  // Build a lookup map: person_id → context
  const contextMap = useMemo(
    () => Object.fromEntries(personContacts.map(pc => [pc.person_id, pc.context])),
    [personContacts]
  );

  const filtered = useMemo(() => {
    let result = contacts;
    if (searchQuery) {
      const q = searchQuery.toLowerCase();
      result = result.filter(
        c =>
          (c.full_name && c.full_name.toLowerCase().includes(q)) ||
          (c.email && c.email.toLowerCase().includes(q)) ||
          (c.company_name && c.company_name.toLowerCase().includes(q))
      );
    }
    if (stageFilter !== 'all') {
      result = result.filter(c => c.lifecycle_stage === stageFilter);
    }
    return result;
  }, [contacts, searchQuery, stageFilter]);

  const stageInfo = LIFECYCLE_STAGE_INFO;

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-3">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search contacts..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9"
          />
        </div>
        <select
          value={stageFilter}
          onChange={(e) => setStageFilter(e.target.value)}
          className="h-9 rounded-md border border-input bg-background px-3 text-sm"
        >
          <option value="all">All Stages</option>
          {Object.entries(stageInfo).map(([key, info]) => (
            <option key={key} value={key}>{info.label}</option>
          ))}
        </select>
        {isLoading && (
          <span className="text-xs text-muted-foreground">Loading contacts…</span>
        )}
      </div>

      {filtered.length === 0 ? (
        <div className="text-center py-12 text-muted-foreground">
          <Contact2 className="h-8 w-8 mx-auto mb-2 opacity-40" />
          <p>{isLoading ? 'Loading contacts...' : 'No contacts found'}</p>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
          {filtered.map((contact) => (
            <ContactCard key={contact.id} contact={contact} context={contextMap[contact.id]} />
          ))}
        </div>
      )}
    </div>
  );
}

const CONTEXT_COLORS: Record<string, string> = {
  client:  'bg-green-100 text-green-700',
  vendor:  'bg-orange-100 text-orange-700',
  partner: 'bg-purple-100 text-purple-700',
  prospect:'bg-blue-100 text-blue-700',
  contact: 'bg-gray-100 text-gray-600',
};

function ContactCard({ contact, context }: { contact: OrgContact; context?: string }) {
  const stageInfo = LIFECYCLE_STAGE_INFO[contact.lifecycle_stage as LifecycleStage];

  return (
    <Link
      to={`/people/${contact.id}`}
      className="block p-4 rounded-xl border border-border/50 bg-card/50 hover:bg-accent/30 hover:border-accent/50 transition-all group"
    >
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <p className="text-sm font-medium truncate group-hover:text-foreground">
            {contact.full_name || 'Unnamed'}
          </p>
          {contact.job_title && (
            <p className="text-xs text-muted-foreground truncate mt-0.5">{contact.job_title}</p>
          )}
          {contact.email && (
            <p className="text-xs text-muted-foreground truncate">{contact.email}</p>
          )}
          {contact.company_name && (
            <p className="text-xs text-muted-foreground truncate">{contact.company_name}</p>
          )}
        </div>
        {contact.lead_score > 0 && (
          <span className="text-xs font-medium px-1.5 py-0.5 rounded bg-blue-100 text-blue-700 dark:bg-blue-950 dark:text-blue-300 shrink-0">
            {contact.lead_score}
          </span>
        )}
      </div>
      <div className="flex items-center gap-2 mt-3">
        {stageInfo && (
          <Badge
            variant="secondary"
            className="text-[10px]"
            style={{ backgroundColor: stageInfo.color + '20', color: stageInfo.color }}
          >
            {stageInfo.label}
          </Badge>
        )}
        {contact.person_type && (
          <Badge variant="outline" className="text-[10px] capitalize">{contact.person_type}</Badge>
        )}
        {context && context !== 'contact' && (
          <span className={`text-[10px] px-1.5 py-0.5 rounded capitalize font-medium ${CONTEXT_COLORS[context] ?? CONTEXT_COLORS.contact}`}>
            {context}
          </span>
        )}
      </div>
    </Link>
  );
}

// ── Projects Tab ──────────────────────────────────────────────────────────────

function ProjectsTab({
  orgId: _orgId,
  sidebarOrg,
  clientFilter,
  onClearClientFilter,
}: {
  orgId: string;
  sidebarOrg: any;
  clientFilter: string | null;
  onClearClientFilter: () => void;
}) {
  if (!sidebarOrg) {
    return (
      <div className="text-center py-12 text-muted-foreground">
        <FolderOpen className="h-8 w-8 mx-auto mb-2 opacity-40" />
        <p>No projects found</p>
      </div>
    );
  }

  const filteredClient = clientFilter
    ? sidebarOrg.clients?.find((c: any) => c.id === clientFilter)
    : null;

  return (
    <div className="space-y-4">
      {filteredClient && (
        <div className="flex items-center gap-2 p-3 rounded-lg bg-accent/30 border border-accent/50">
          <Briefcase className="h-4 w-4 text-purple-500" />
          <span className="text-sm font-medium">Viewing: {filteredClient.name}</span>
          <button
            onClick={onClearClientFilter}
            className="ml-auto p-1 rounded hover:bg-accent"
          >
            <X className="h-3.5 w-3.5" />
          </button>
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        {/* Internal projects (hidden if client filter active) */}
        {!clientFilter && (
          <Card className="bg-card/80 backdrop-blur-sm border-border/50">
            <CardHeader>
              <CardTitle className="text-base">Internal Projects</CardTitle>
              <CardDescription>Projects not assigned to a specific client</CardDescription>
            </CardHeader>
            <CardContent>
              {(sidebarOrg.internal_projects?.length === 0 &&
                (sidebarOrg.internal_folders || []).length === 0) ? (
                <p className="text-sm text-muted-foreground text-center py-4">No internal projects</p>
              ) : (
                <div className="space-y-2">
                  {sidebarOrg.internal_projects?.map((p: any) => (
                    <ProjectRow key={p.id} project={p} />
                  ))}
                  {(sidebarOrg.internal_folders || []).map((folder: any) =>
                    folder.projects.map((p: any) => (
                      <ProjectRow key={p.id} project={p} folderName={folder.name} />
                    ))
                  )}
                </div>
              )}
            </CardContent>
          </Card>
        )}

        {/* Client projects */}
        {(clientFilter ? [filteredClient].filter(Boolean) : sidebarOrg.clients || []).map((client: any) => (
          <Card key={client.id} className="bg-card/80 backdrop-blur-sm border-border/50">
            <CardHeader>
              <CardTitle className="text-base flex items-center gap-2">
                <Briefcase className="h-4 w-4 text-purple-600" />
                {client.name}
              </CardTitle>
              <CardDescription>Client projects</CardDescription>
            </CardHeader>
            <CardContent>
              {(client.projects?.length === 0 && (client.folders || []).length === 0) ? (
                <p className="text-sm text-muted-foreground text-center py-4">No projects</p>
              ) : (
                <div className="space-y-2">
                  {client.projects?.map((p: any) => (
                    <ProjectRow key={p.id} project={p} />
                  ))}
                  {(client.folders || []).map((folder: any) =>
                    folder.projects.map((p: any) => (
                      <ProjectRow key={p.id} project={p} folderName={folder.name} />
                    ))
                  )}
                </div>
              )}
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}

function ProjectRow({ project, folderName }: { project: any; folderName?: string }) {
  return (
    <Link
      to={`/projects/${project.id}`}
      className="flex items-center justify-between p-2 rounded-md hover:bg-muted transition-colors"
    >
      <div className="flex items-center gap-2 min-w-0">
        <FolderOpen className="h-4 w-4 text-muted-foreground shrink-0" />
        <span className="text-sm font-medium truncate">{project.name}</span>
        {folderName && <Badge variant="secondary" className="text-xs shrink-0">{folderName}</Badge>}
      </div>
      <ExternalLink className="h-3 w-3 text-muted-foreground shrink-0" />
    </Link>
  );
}

// ── Knowledge Tab ─────────────────────────────────────────────────────────────

const SOURCE_TYPE_META: Record<string, { label: string; icon: typeof BookOpen }> = {
  conversation: { label: 'Conversations', icon: MessageSquare },
  artifact: { label: 'Artifacts', icon: FileText },
  pulse_content: { label: 'Pulse Content', icon: Radio },
  context_injection: { label: 'Context Injections', icon: Pencil },
  entity: { label: 'Entities', icon: Boxes },
  topology_snapshot: { label: 'Topology Snapshots', icon: Network },
};

// ── Add Data Source Dialog ────────────────────────────────────────────────────

function AddDataSourceDialog({
  orgId,
  projectEntries,
  open,
  onOpenChange,
  editingSource,
}: {
  orgId: string;
  projectEntries: { id: string; name: string }[];
  open: boolean;
  onOpenChange: (open: boolean) => void;
  editingSource?: DataSourceRecord | null;
}) {
  const queryClient = useQueryClient();
  const isEdit = !!editingSource;

  const [selectedProject, setSelectedProject] = useState<string>(editingSource?.project_id ?? '__none__');
  const [sourceType, setSourceType] = useState<string>(editingSource?.source_type ?? 'text');
  const [dataType, setDataType] = useState<string>(editingSource?.data_type ?? 'conversation');
  const [title, setTitle] = useState(editingSource?.title ?? '');
  const [description, setDescription] = useState(editingSource?.description ?? '');
  const [content, setContent] = useState(editingSource?.content ?? '');
  const [file, setFile] = useState<File | null>(null);

  // Reset form when dialog opens/closes or editingSource changes
  const resetForm = () => {
    setSelectedProject(editingSource?.project_id ?? '__none__');
    setSourceType(editingSource?.source_type ?? 'text');
    setDataType(editingSource?.data_type ?? 'conversation');
    setTitle(editingSource?.title ?? '');
    setDescription(editingSource?.description ?? '');
    setContent(editingSource?.content ?? '');
    setFile(null);
  };

  const createMutation = useMutation({
    mutationFn: async () => {
      const projId = selectedProject !== '__none__' ? selectedProject : undefined;
      if (sourceType === 'file' && file) {
        const formData = new FormData();
        formData.append('file', file);
        formData.append('title', title.trim());
        formData.append('data_type', dataType);
        if (description.trim()) formData.append('description', description.trim());
        if (projId) formData.append('project_id', projId);
        formData.append('organization_id', orgId);
        return dataSourcesApi.upload(formData);
      }
      return dataSourcesApi.create({
        organization_id: orgId,
        project_id: projId,
        title: title.trim(),
        description: description.trim() || undefined,
        data_type: dataType,
        source_type: sourceType,
        content: sourceType === 'text' && content.trim() ? content.trim() : undefined,
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['dataSources', orgId] });
      onOpenChange(false);
      resetForm();
    },
  });

  const updateMutation = useMutation({
    mutationFn: (data: UpdateDataSourceRequest) =>
      dataSourcesApi.update(editingSource!.id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['dataSources', orgId] });
      onOpenChange(false);
    },
  });

  const handleSubmit = () => {
    if (!title.trim()) return;
    if (isEdit) {
      updateMutation.mutate({
        title: title.trim(),
        description: description.trim() || undefined,
        data_type: dataType,
        source_type: sourceType,
        content: sourceType === 'text' && content.trim() ? content.trim() : undefined,
      });
    } else {
      createMutation.mutate();
    }
  };

  const isPending = createMutation.isPending || updateMutation.isPending;

  return (
    <Dialog open={open} onOpenChange={(v) => { onOpenChange(v); if (!v) resetForm(); }}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Database className="h-4 w-4" />
            {isEdit ? 'Edit Data Source' : 'Add Data Source'}
          </DialogTitle>
        </DialogHeader>
        <div className="space-y-4 py-2">
          {/* Source type radio */}
          <div className="space-y-2">
            <Label>Source</Label>
            <RadioGroup
              value={sourceType}
              onValueChange={setSourceType}
              className="flex gap-4"
            >
              <div className="flex items-center gap-1.5">
                <RadioGroupItem value="text" id="st-text" />
                <Label htmlFor="st-text" className="text-sm font-normal cursor-pointer">Text</Label>
              </div>
              <div className="flex items-center gap-1.5">
                <RadioGroupItem value="file" id="st-file" />
                <Label htmlFor="st-file" className="text-sm font-normal cursor-pointer">File Upload</Label>
              </div>
              <div className="flex items-center gap-1.5">
                <RadioGroupItem value="integration" id="st-integration" />
                <Label htmlFor="st-integration" className="text-sm font-normal cursor-pointer">Integration</Label>
              </div>
            </RadioGroup>
          </div>

          <div className="space-y-2">
            <Label>Title</Label>
            <Input
              placeholder="e.g. Client kickoff call notes"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              autoFocus
            />
          </div>

          <div className="space-y-2">
            <Label>Data Type</Label>
            <Select value={dataType} onValueChange={setDataType}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {DATA_TYPE_OPTIONS.map((opt) => (
                  <SelectItem key={opt.value} value={opt.value}>{opt.label}</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-2">
            <Label>Description (optional)</Label>
            <Textarea
              placeholder="Brief description of this data source..."
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              rows={2}
            />
          </div>

          {/* Conditional input based on source type */}
          {sourceType === 'text' && (
            <div className="space-y-2">
              <Label>Content</Label>
              <Textarea
                placeholder="Paste or type your content here..."
                value={content}
                onChange={(e) => setContent(e.target.value)}
                rows={6}
                className="font-mono text-xs"
              />
            </div>
          )}
          {sourceType === 'file' && !isEdit && (
            <div className="space-y-2">
              <Label>File</Label>
              <div className="flex items-center gap-2">
                <Input
                  type="file"
                  onChange={(e) => setFile(e.target.files?.[0] ?? null)}
                  className="text-xs"
                />
                {file && (
                  <span className="text-xs text-muted-foreground shrink-0">
                    {(file.size / 1024).toFixed(0)} KB
                  </span>
                )}
              </div>
            </div>
          )}
          {sourceType === 'integration' && (
            <div className="rounded-md bg-muted/50 p-3 text-xs text-muted-foreground">
              Integration sources are populated automatically from connected services.
            </div>
          )}

          {projectEntries.length > 0 && (
            <div className="space-y-2">
              <Label>Project (optional)</Label>
              <Select value={selectedProject} onValueChange={setSelectedProject}>
                <SelectTrigger>
                  <SelectValue placeholder="No project (org-level)" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="__none__">No project (org-level)</SelectItem>
                  {projectEntries.map((p) => (
                    <SelectItem key={p.id} value={p.id}>{p.name}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          )}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>Cancel</Button>
          <Button onClick={handleSubmit} disabled={!title.trim() || isPending}>
            {isPending ? (isEdit ? 'Saving...' : 'Adding...') : (isEdit ? 'Save' : 'Add Source')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// ── Delete Confirmation Dialog ───────────────────────────────────────────────

function DeleteConfirmDialog({
  open,
  onOpenChange,
  sourceName,
  onConfirm,
  isPending,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  sourceName: string;
  onConfirm: () => void;
  isPending: boolean;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-sm">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2 text-destructive">
            <Trash2 className="h-4 w-4" />
            Delete Data Source
          </DialogTitle>
        </DialogHeader>
        <p className="text-sm text-muted-foreground">
          Are you sure you want to delete <strong>{sourceName}</strong>? This action cannot be undone.
        </p>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>Cancel</Button>
          <Button variant="destructive" onClick={onConfirm} disabled={isPending}>
            {isPending ? 'Deleting...' : 'Delete'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// ── Data Sources View ────────────────────────────────────────────────────────

type SortField = 'title' | 'data_type' | 'status' | 'created_at';
type SortDir = 'asc' | 'desc';

function DataSourcesView({
  orgId,
  projectEntries,
}: {
  orgId: string;
  projectEntries: { id: string; name: string }[];
}) {
  const queryClient = useQueryClient();
  const [addOpen, setAddOpen] = useState(false);
  const [editingSource, setEditingSource] = useState<DataSourceRecord | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<DataSourceRecord | null>(null);
  const [sortField, setSortField] = useState<SortField>('created_at');
  const [sortDir, setSortDir] = useState<SortDir>('desc');

  const { data: sources = [], isLoading } = useQuery({
    queryKey: ['dataSources', orgId],
    queryFn: () => dataSourcesApi.listByOrganization(orgId),
    staleTime: 30_000,
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => dataSourcesApi.delete(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['dataSources', orgId] });
      setDeleteTarget(null);
    },
  });

  const toggleSort = (field: SortField) => {
    if (sortField === field) {
      setSortDir(d => d === 'asc' ? 'desc' : 'asc');
    } else {
      setSortField(field);
      setSortDir('asc');
    }
  };

  const SortIcon = ({ field }: { field: SortField }) => {
    if (sortField !== field) return <ArrowUpDown className="h-3 w-3 opacity-40" />;
    return sortDir === 'asc' ? <ArrowUp className="h-3 w-3" /> : <ArrowDown className="h-3 w-3" />;
  };

  const sorted = useMemo(() => {
    const arr = [...sources];
    arr.sort((a, b) => {
      let cmp = 0;
      switch (sortField) {
        case 'title': cmp = a.title.localeCompare(b.title); break;
        case 'data_type': cmp = a.data_type.localeCompare(b.data_type); break;
        case 'status': cmp = a.status.localeCompare(b.status); break;
        case 'created_at': cmp = a.created_at.localeCompare(b.created_at); break;
      }
      return sortDir === 'asc' ? cmp : -cmp;
    });
    return arr;
  }, [sources, sortField, sortDir]);

  const dataTypeLabel = (dt: string) =>
    DATA_TYPE_OPTIONS.find(o => o.value === dt)?.label ?? dt;

  const formatFileSize = (bytes?: number) => {
    if (!bytes) return '';
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const formatDate = (iso: string) => {
    const d = new Date(iso);
    return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
  };

  const statusBadge = (status: string) => {
    const variants: Record<string, string> = {
      ready: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400',
      pending: 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400',
      processing: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400',
      error: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
    };
    return (
      <span className={`inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium ${variants[status] ?? 'bg-muted text-muted-foreground'}`}>
        {status}
      </span>
    );
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Database className="h-5 w-5 text-muted-foreground" />
          <h2 className="text-lg font-semibold">Data Sources</h2>
          <Badge variant="secondary">{sources.length}</Badge>
        </div>
        <Button size="sm" onClick={() => { setEditingSource(null); setAddOpen(true); }} className="gap-1">
          <Plus className="h-3.5 w-3.5" />
          Add Data Source
        </Button>
      </div>

      {isLoading ? (
        <div className="text-center py-12 text-muted-foreground text-sm">Loading data sources...</div>
      ) : sorted.length === 0 ? (
        <div className="text-center py-12 text-muted-foreground">
          <Database className="h-8 w-8 mx-auto mb-2 opacity-40" />
          <p>No data sources yet</p>
          <Button variant="outline" size="sm" className="mt-4 gap-1" onClick={() => setAddOpen(true)}>
            <Plus className="h-3.5 w-3.5" />
            Add your first data source
          </Button>
        </div>
      ) : (
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="cursor-pointer select-none" onClick={() => toggleSort('title')}>
                  <span className="flex items-center gap-1">Title <SortIcon field="title" /></span>
                </TableHead>
                <TableHead className="cursor-pointer select-none w-[120px]" onClick={() => toggleSort('data_type')}>
                  <span className="flex items-center gap-1">Type <SortIcon field="data_type" /></span>
                </TableHead>
                <TableHead className="w-[100px]">Source</TableHead>
                <TableHead className="cursor-pointer select-none w-[80px]" onClick={() => toggleSort('status')}>
                  <span className="flex items-center gap-1">Status <SortIcon field="status" /></span>
                </TableHead>
                <TableHead className="cursor-pointer select-none w-[110px]" onClick={() => toggleSort('created_at')}>
                  <span className="flex items-center gap-1">Created <SortIcon field="created_at" /></span>
                </TableHead>
                <TableHead className="w-[40px]" />
              </TableRow>
            </TableHeader>
            <TableBody>
              {sorted.map((source) => (
                <TableRow key={source.id}>
                  <TableCell>
                    <div className="min-w-0">
                      <Link to={`/organizations/${orgId}/data-sources/${source.id}`} className="text-sm font-medium truncate hover:underline block">{source.title}</Link>
                      {source.description && (
                        <p className="text-xs text-muted-foreground truncate mt-0.5">{source.description}</p>
                      )}
                    </div>
                  </TableCell>
                  <TableCell>
                    <Badge variant="outline" className="text-[10px]">{dataTypeLabel(source.data_type)}</Badge>
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-1 text-xs text-muted-foreground">
                      {source.source_type === 'file' && source.file_name ? (
                        <>
                          <Upload className="h-3 w-3 shrink-0" />
                          <span className="truncate max-w-[60px]">{source.file_name}</span>
                          {source.file_size_bytes != null && (
                            <span className="shrink-0">({formatFileSize(source.file_size_bytes)})</span>
                          )}
                        </>
                      ) : source.source_type === 'file' ? (
                        <>
                          <Upload className="h-3 w-3 shrink-0" />
                          <span>File</span>
                        </>
                      ) : source.source_type === 'text' ? (
                        <>
                          <FileText className="h-3 w-3 shrink-0" />
                          <span>Text</span>
                        </>
                      ) : (
                        <>
                          <Database className="h-3 w-3 shrink-0" />
                          <span>Integration</span>
                        </>
                      )}
                    </div>
                  </TableCell>
                  <TableCell>{statusBadge(source.status)}</TableCell>
                  <TableCell className="text-xs text-muted-foreground">{formatDate(source.created_at)}</TableCell>
                  <TableCell>
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <Button variant="ghost" size="sm" className="h-7 w-7 p-0">
                          <MoreHorizontal className="h-3.5 w-3.5" />
                        </Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuItem onClick={() => { setEditingSource(source); setAddOpen(true); }}>
                          <Pencil className="h-3.5 w-3.5 mr-2" />
                          Edit
                        </DropdownMenuItem>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem
                          className="text-destructive focus:text-destructive"
                          onClick={() => setDeleteTarget(source)}
                        >
                          <Trash2 className="h-3.5 w-3.5 mr-2" />
                          Delete
                        </DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </Card>
      )}

      <AddDataSourceDialog
        orgId={orgId}
        projectEntries={projectEntries}
        open={addOpen}
        onOpenChange={setAddOpen}
        editingSource={editingSource}
      />

      <DeleteConfirmDialog
        open={!!deleteTarget}
        onOpenChange={(v) => { if (!v) setDeleteTarget(null); }}
        sourceName={deleteTarget?.title ?? ''}
        onConfirm={() => deleteTarget && deleteMutation.mutate(deleteTarget.id)}
        isPending={deleteMutation.isPending}
      />
    </div>
  );
}

// ── Artifacts View ──────────────────────────────────────────────────────────

function ArtifactsView({ orgId }: { orgId: string }) {
  const [expandedIds, setExpandedIds] = useState<Set<string>>(new Set());

  const { data: artifacts = [], isLoading } = useQuery({
    queryKey: ['recentArtifacts'],
    queryFn: () => workflowsApi.listRecentArtifacts(),
  });

  const toggleExpand = (id: string) => {
    setExpandedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const parseContent = (content?: string) => {
    if (!content) return null;
    try { return JSON.parse(content); } catch { return content; }
  };

  const renderValue = (val: any): React.ReactNode => {
    if (val === null || val === undefined) return <span className="text-muted-foreground italic">null</span>;
    if (typeof val === 'string') return <span className="text-sm">{val}</span>;
    if (typeof val === 'number' || typeof val === 'boolean') return <span className="text-sm font-mono">{String(val)}</span>;
    if (Array.isArray(val)) {
      return (
        <div className="ml-3 space-y-1">
          {val.map((item, i) => (
            <div key={i} className="text-sm border-l-2 border-border/50 pl-2">
              {typeof item === 'object' ? renderValue(item) : String(item)}
            </div>
          ))}
        </div>
      );
    }
    if (typeof val === 'object') {
      return (
        <div className="ml-3 space-y-1">
          {Object.entries(val).map(([k, v]) => (
            <div key={k}>
              <span className="text-xs font-medium text-muted-foreground uppercase tracking-wide">{k.replace(/_/g, ' ')}: </span>
              {typeof v === 'object' && v !== null ? renderValue(v) : <span className="text-sm">{String(v ?? '')}</span>}
            </div>
          ))}
        </div>
      );
    }
    return <span>{String(val)}</span>;
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-48">
        <div className="text-sm text-muted-foreground">Loading artifacts...</div>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2">
        <FileText className="h-5 w-5 text-muted-foreground" />
        <h2 className="text-lg font-semibold">Artifacts</h2>
        <Badge variant="secondary">{artifacts.length}</Badge>
      </div>

      {artifacts.length === 0 ? (
        <div className="text-center py-12 text-muted-foreground">
          <FileText className="h-8 w-8 mx-auto mb-2 opacity-40" />
          <p className="text-sm">No artifacts generated yet.</p>
          <p className="text-xs mt-1">Run a workflow on a data source to generate artifacts.</p>
        </div>
      ) : (
        <div className="space-y-2">
          {artifacts.map((artifact: ExecutionArtifact) => {
            const isExpanded = expandedIds.has(artifact.id);
            const meta = artifact.metadata ? (() => { try { return JSON.parse(artifact.metadata); } catch { return {}; } })() : {};
            const content = parseContent(artifact.content ?? undefined);

            return (
              <Card key={artifact.id} className="bg-card/80 border-border/50">
                <button
                  className="flex items-center justify-between w-full p-4 text-left hover:bg-muted/30 transition-colors"
                  onClick={() => toggleExpand(artifact.id)}
                >
                  <div className="flex items-center gap-3 min-w-0">
                    {isExpanded ? <ChevronDown className="h-4 w-4 text-muted-foreground shrink-0" /> : <ChevronRight className="h-4 w-4 text-muted-foreground shrink-0" />}
                    <div className="min-w-0">
                      <p className="text-sm font-medium truncate">{artifact.title || 'Untitled Artifact'}</p>
                      <div className="flex items-center gap-2 mt-0.5">
                        <Badge variant="outline" className="text-[10px]">{artifact.artifact_type}</Badge>
                        {meta.step_id && <span className="text-xs text-muted-foreground">{meta.step_id.replace(/_/g, ' ')}</span>}
                      </div>
                    </div>
                  </div>
                  <span className="text-xs text-muted-foreground shrink-0 ml-2 flex items-center gap-1">
                    <Clock className="h-3 w-3" />
                    {new Date(artifact.created_at).toLocaleDateString('en-US', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })}
                  </span>
                </button>
                {isExpanded && (
                  <div className="px-4 pb-4 border-t">
                    <div className="mt-3">
                      {content ? renderValue(content) : <p className="text-sm text-muted-foreground italic">No content.</p>}
                    </div>
                    {meta.data_source_id && (
                      <div className="mt-3 pt-2 border-t border-border/30">
                        <Link
                          to={`/organizations/${orgId}/data-sources/${meta.data_source_id}`}
                          className="text-xs text-blue-600 hover:underline"
                        >
                          View source data source
                        </Link>
                      </div>
                    )}
                  </div>
                )}
              </Card>
            );
          })}
        </div>
      )}
    </div>
  );
}

// ── Workflows Management View ───────────────────────────────────────────────

function WorkflowsView({ orgId: _orgId }: { orgId: string }) {
  const { data: workflows = [], isLoading } = useQuery({
    queryKey: ['workflowDefinitions'],
    queryFn: () => workflowsApi.listDefinitions(),
  });

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-48">
        <div className="text-sm text-muted-foreground">Loading workflows...</div>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <GitBranch className="h-5 w-5 text-muted-foreground" />
          <h2 className="text-lg font-semibold">Workflows</h2>
          <Badge variant="secondary">{workflows.length}</Badge>
        </div>
        <Button size="sm" variant="outline" disabled className="gap-1.5">
          <Plus className="h-3.5 w-3.5" />
          New Workflow
        </Button>
      </div>

      <p className="text-sm text-muted-foreground">
        Workflows are data processing pipelines that extract structured information from data sources.
        Run them from any data source detail page.
      </p>

      {workflows.length === 0 ? (
        <div className="text-center py-12 text-muted-foreground">
          <GitBranch className="h-8 w-8 mx-auto mb-2 opacity-40" />
          <p className="text-sm">No workflows defined yet.</p>
        </div>
      ) : (
        <div className="space-y-3">
          {workflows.map((wf: any) => (
            <Card key={wf.id} className="bg-card/80 border-border/50">
              <CardHeader className="pb-2">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <GitBranch className="h-4 w-4 text-purple-500" />
                    <CardTitle className="text-base">{wf.name}</CardTitle>
                  </div>
                  <Badge variant="outline">{wf.steps?.length ?? 0} steps</Badge>
                </div>
                <CardDescription className="text-xs">ID: {wf.id}</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-1">
                  {(wf.steps ?? []).map((step: any, idx: number) => (
                    <div key={step.id} className="flex items-center gap-2">
                      <div className="flex items-center gap-1.5">
                        <div className="w-5 h-5 rounded-full bg-muted flex items-center justify-center text-[10px] font-medium text-muted-foreground">
                          {idx + 1}
                        </div>
                        {idx < (wf.steps?.length ?? 0) - 1 && (
                          <div className="absolute ml-2.5 mt-6 w-px h-3 bg-border" />
                        )}
                      </div>
                      <div className="flex-1 min-w-0">
                        <span className="text-sm">{step.name}</span>
                        {step.depends_on?.length > 0 && (
                          <span className="text-xs text-muted-foreground ml-2">
                            (depends on: {step.depends_on.join(', ')})
                          </span>
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}

function KnowledgeTab({
  orgId,
  projectEntries,
  view,
}: {
  orgId: string;
  projectEntries: { id: string; name: string }[];
  view?: string | null;
}) {
  const knowledgeQueries = useQueries({
    queries: projectEntries.map((entry) => ({
      queryKey: ['projectKnowledge', entry.id],
      queryFn: () => knowledgeApi.getProjectKnowledge(entry.id),
      staleTime: 60_000,
    })),
  });

  const isLoading = knowledgeQueries.some(q => q.isLoading);
  const loadedCount = knowledgeQueries.filter(q => q.isSuccess).length;

  // Aggregate across all projects
  const aggregated = useMemo(() => {
    let totalSources = 0;
    let staleSources = 0;
    let totalCoverage = 0;
    let coverageCount = 0;
    const sourcesByProject: { projectName: string; projectId: string; data: ProjectKnowledgeResponse }[] = [];
    const byType: Record<string, { source: ProjectKnowledgeSource; projectName: string; projectId: string }[]> = {};

    knowledgeQueries.forEach((q, i) => {
      if (!q.data) return;
      const entry = projectEntries[i];
      totalSources += q.data.total_sources;
      staleSources += q.data.stale_count;
      if (q.data.completeness) {
        totalCoverage += q.data.completeness.knowledge_completeness;
        coverageCount++;
      }
      if (q.data.total_sources > 0) {
        sourcesByProject.push({ projectName: entry.name, projectId: entry.id, data: q.data });
      }
      Object.entries(q.data.sources_by_type).forEach(([type, sources]) => {
        if (!byType[type]) byType[type] = [];
        sources.forEach(s => byType[type].push({ source: s, projectName: entry.name, projectId: entry.id }));
      });
    });

    const avgCompleteness = coverageCount > 0 ? Math.round((totalCoverage / coverageCount) * 100) : 0;
    return { totalSources, staleSources, avgCompleteness, sourcesByProject, byType };
  }, [knowledgeQueries, projectEntries]);

  // Deep view: "datasources" shows the new data sources table; others filter knowledge by type
  if (view && view !== 'overview') {
    if (view === 'datasources') {
      return (
        <DataSourcesView
          orgId={orgId}
          projectEntries={projectEntries}
        />
      );
    }

    if (view === 'artifacts') {
      return <ArtifactsView orgId={orgId} />;
    }

    if (view === 'workflows') {
      return <WorkflowsView orgId={orgId} />;
    }

    const typeKey =
      view === 'conversations' ? 'conversation'
      : view === 'pulse'      ? 'pulse_content'
      : view === 'topology'   ? 'topology_snapshot'
      : null;
    const items = typeKey ? (aggregated.byType[typeKey] || []) : [];
    const meta = typeKey ? SOURCE_TYPE_META[typeKey] : null;
    const Icon = meta?.icon ?? BookOpen;

    return (
      <div className="space-y-4">
        <div className="flex items-center gap-2">
          <Icon className="h-5 w-5 text-muted-foreground" />
          <h2 className="text-lg font-semibold">{meta?.label ?? view}</h2>
          <Badge variant="secondary">{items.length}</Badge>
          {isLoading && (
            <span className="text-xs text-muted-foreground ml-2">
              Loading {loadedCount}/{projectEntries.length} projects…
            </span>
          )}
        </div>
        {items.length === 0 && !isLoading ? (
          <div className="text-center py-12 text-muted-foreground">
            <Icon className="h-8 w-8 mx-auto mb-2 opacity-40" />
            <p>No {meta?.label.toLowerCase() ?? view} indexed yet</p>
          </div>
        ) : (
          <div className="space-y-2">
            {items.map(({ source, projectName, projectId }) => (
              <Card key={source.id} className="bg-card/80 backdrop-blur-sm border-border/50">
                <CardContent className="p-4">
                  <div className="flex items-start justify-between gap-3">
                    <div className="min-w-0">
                      <p className="font-medium text-sm truncate">{source.source_title}</p>
                      {source.source_summary && (
                        <p className="text-xs text-muted-foreground mt-1 line-clamp-2">{source.source_summary}</p>
                      )}
                      <div className="flex items-center gap-2 mt-2">
                        <Link
                          to={`/projects/${projectId}/knowledge`}
                          className="text-xs text-blue-600 hover:underline flex items-center gap-1"
                        >
                          <FolderOpen className="h-3 w-3" />
                          {projectName}
                        </Link>
                        {source.is_stale && (
                          <Badge variant="outline" className="text-xs text-yellow-600 border-yellow-300">stale</Badge>
                        )}
                      </div>
                    </div>
                    <div className="shrink-0 text-right">
                      <Progress value={Math.round(source.coverage_score * 100)} className="w-16 h-1.5 mb-1" />
                      <span className="text-xs text-muted-foreground">{Math.round(source.coverage_score * 100)}%</span>
                    </div>
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        )}
      </div>
    );
  }

  // Overview (default)
  return (
    <div className="space-y-6">
      {/* Summary cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Avg Completeness</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-3">
              <Progress value={aggregated.avgCompleteness} className="flex-1 h-2" />
              <span className="text-2xl font-bold">{aggregated.avgCompleteness}%</span>
            </div>
          </CardContent>
        </Card>
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Total Sources</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-baseline gap-2">
              <span className="text-2xl font-bold">{aggregated.totalSources}</span>
              <span className="text-sm text-muted-foreground">
                across {projectEntries.length} project{projectEntries.length !== 1 ? 's' : ''}
              </span>
            </div>
          </CardContent>
        </Card>
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Stale Sources</CardTitle>
          </CardHeader>
          <CardContent>
            <span className={`text-2xl font-bold ${aggregated.staleSources > 0 ? 'text-yellow-600' : ''}`}>
              {aggregated.staleSources}
            </span>
          </CardContent>
        </Card>
      </div>

      {/* Source type breakdown pills */}
      {aggregated.totalSources > 0 && (
        <div className="grid grid-cols-2 md:grid-cols-3 gap-3">
          {Object.entries(SOURCE_TYPE_META).map(([type, meta]) => {
            const count = (aggregated.byType[type] || []).length;
            if (count === 0) return null;
            const Icon = meta.icon;
            return (
              <Card key={type} className="bg-card/80 backdrop-blur-sm border-border/50">
                <CardContent className="p-3 flex items-center gap-3">
                  <Icon className="h-5 w-5 text-muted-foreground shrink-0" />
                  <div>
                    <p className="font-semibold text-sm">{count}</p>
                    <p className="text-xs text-muted-foreground">{meta.label}</p>
                  </div>
                </CardContent>
              </Card>
            );
          })}
        </div>
      )}

      {isLoading && (
        <p className="text-xs text-muted-foreground">Loading {loadedCount}/{projectEntries.length} projects...</p>
      )}

      {/* Per-project knowledge */}
      {aggregated.sourcesByProject.length === 0 && !isLoading ? (
        <div className="text-center py-12 text-muted-foreground">
          <BookOpen className="h-8 w-8 mx-auto mb-2 opacity-40" />
          <p>
            {projectEntries.length === 0
              ? 'No projects loaded — navigate to a project to index knowledge'
              : 'No knowledge sources indexed yet'}
          </p>
          <p className="text-xs mt-1 opacity-70">
            Use the sidebar Intelligence links to browse by category
          </p>
        </div>
      ) : (
        <div className="space-y-4">
          {aggregated.sourcesByProject.map(({ projectName, projectId, data }) => {
            const completeness = data.completeness
              ? Math.round(data.completeness.knowledge_completeness * 100)
              : 0;
            return (
              <Card key={projectId} className="bg-card/80 backdrop-blur-sm border-border/50">
                <CardHeader className="pb-3">
                  <div className="flex items-center justify-between">
                    <CardTitle className="text-base flex items-center gap-2">
                      <FolderOpen className="h-4 w-4 text-muted-foreground" />
                      {projectName}
                    </CardTitle>
                    <div className="flex items-center gap-2">
                      <Progress value={completeness} className="w-20 h-1.5" />
                      <span className="text-xs text-muted-foreground">{completeness}%</span>
                      <Link
                        to={`/projects/${projectId}/knowledge`}
                        className="text-xs text-blue-600 hover:underline ml-2"
                      >
                        View
                      </Link>
                    </div>
                  </div>
                </CardHeader>
                <CardContent>
                  <div className="flex flex-wrap gap-2">
                    {Object.entries(data.sources_by_type).map(([type, sources]) => {
                      if (sources.length === 0) return null;
                      const meta = SOURCE_TYPE_META[type];
                      const Icon = meta?.icon || BookOpen;
                      return (
                        <Badge key={type} variant="secondary" className="text-xs gap-1">
                          <Icon className="h-3 w-3" />
                          {meta?.label || type} ({sources.length})
                        </Badge>
                      );
                    })}
                    {data.stale_count > 0 && (
                      <Badge variant="outline" className="text-xs text-yellow-600 border-yellow-300 gap-1">
                        <AlertTriangle className="h-3 w-3" />
                        {data.stale_count} stale
                      </Badge>
                    )}
                  </div>
                </CardContent>
              </Card>
            );
          })}
        </div>
      )}
    </div>
  );
}

// ── Social Tab ────────────────────────────────────────────────────────────────

const PLATFORM_ICONS: Record<string, typeof Linkedin> = {
  linkedin: Linkedin,
  instagram: Instagram,
  twitter: Twitter,
  facebook: Facebook,
  youtube: Youtube,
};

const PLATFORM_COLORS: Record<string, string> = {
  linkedin: 'text-[#0A66C2]',
  instagram: 'text-[#E4405F]',
  twitter: 'text-foreground',
  facebook: 'text-[#1877F2]',
  youtube: 'text-[#FF0000]',
};

function SocialTab({ projectEntries }: { projectEntries: { id: string; name: string }[] }) {
  const accountQueries = useQueries({
    queries: projectEntries.map((entry) => ({
      queryKey: ['social-accounts', entry.id],
      queryFn: () => socialApi.listAccounts(entry.id),
      staleTime: 60_000,
      enabled: projectEntries.length > 0,
    })),
  });

  const mentionQueries = useQueries({
    queries: projectEntries.map((entry) => ({
      queryKey: ['social-mentions', entry.id],
      queryFn: () => socialApi.listMentions(entry.id, { limit: 20 }),
      staleTime: 60_000,
      enabled: projectEntries.length > 0,
    })),
  });

  const isLoading = accountQueries.some(q => q.isLoading);

  const aggregated = useMemo(() => {
    const allAccounts: (SocialAccountRecord & { _projectName: string; _projectId: string })[] = [];
    const allMentions: (SocialMentionRecord & { _projectName: string })[] = [];

    accountQueries.forEach((q, i) => {
      if (!q.data) return;
      const entry = projectEntries[i];
      q.data.forEach(a => allAccounts.push({ ...a, _projectName: entry.name, _projectId: entry.id }));
    });

    mentionQueries.forEach((q, i) => {
      if (!q.data) return;
      const entry = projectEntries[i];
      q.data.forEach(m => allMentions.push({ ...m, _projectName: entry.name }));
    });

    // Dedupe accounts by platform + username
    const uniqueAccounts = new Map<string, typeof allAccounts[0]>();
    for (const a of allAccounts) {
      const key = `${a.platform}:${a.username || a.display_name || a.id}`;
      if (!uniqueAccounts.has(key)) uniqueAccounts.set(key, a);
    }

    // Sort mentions by date
    allMentions.sort((a, b) => new Date(b.received_at).getTime() - new Date(a.received_at).getTime());

    const unreadCount = allMentions.filter(m => m.status === 'unread').length;
    const totalFollowers = [...uniqueAccounts.values()].reduce((sum, a) => sum + (a.follower_count || 0), 0);

    return {
      accounts: [...uniqueAccounts.values()],
      mentions: allMentions.slice(0, 30),
      unreadCount,
      totalFollowers,
    };
  }, [accountQueries, mentionQueries, projectEntries]);

  return (
    <div className="space-y-6">
      {/* Summary */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Connected Accounts</CardTitle>
          </CardHeader>
          <CardContent>
            <span className="text-2xl font-bold">{aggregated.accounts.length}</span>
          </CardContent>
        </Card>
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Total Followers</CardTitle>
          </CardHeader>
          <CardContent>
            <span className="text-2xl font-bold">
              {aggregated.totalFollowers >= 1000
                ? `${(aggregated.totalFollowers / 1000).toFixed(1)}k`
                : aggregated.totalFollowers}
            </span>
          </CardContent>
        </Card>
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Unread Mentions</CardTitle>
          </CardHeader>
          <CardContent>
            <span className={`text-2xl font-bold ${aggregated.unreadCount > 0 ? 'text-blue-600' : ''}`}>
              {aggregated.unreadCount}
            </span>
          </CardContent>
        </Card>
      </div>

      {isLoading && (
        <p className="text-xs text-muted-foreground">Loading social data...</p>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Accounts */}
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader>
            <CardTitle className="text-base flex items-center gap-2">
              <Share2 className="h-4 w-4" />
              Connected Accounts
            </CardTitle>
          </CardHeader>
          <CardContent>
            {aggregated.accounts.length === 0 ? (
              <div className="text-center py-8 text-muted-foreground">
                <Share2 className="h-8 w-8 mx-auto mb-2 opacity-40" />
                <p>No social accounts connected</p>
              </div>
            ) : (
              <div className="space-y-3">
                {aggregated.accounts.map((account) => {
                  const PlatformIcon = PLATFORM_ICONS[account.platform] || Globe;
                  const platformColor = PLATFORM_COLORS[account.platform] || 'text-muted-foreground';
                  return (
                    <div key={account.id} className="flex items-center gap-3 p-3 rounded-lg border border-border/50">
                      <PlatformIcon className={`h-5 w-5 shrink-0 ${platformColor}`} />
                      <div className="min-w-0 flex-1">
                        <p className="text-sm font-medium truncate">
                          {account.display_name || account.username || account.platform}
                        </p>
                        <div className="flex items-center gap-2 text-xs text-muted-foreground">
                          <span className="capitalize">{account.platform}</span>
                          {account.follower_count != null && (
                            <>
                              <span>-</span>
                              <span>{account.follower_count.toLocaleString()} followers</span>
                            </>
                          )}
                        </div>
                      </div>
                      <div className="flex items-center gap-2">
                        <Badge variant="outline" className="text-[10px]">{account._projectName}</Badge>
                        <Badge
                          variant={account.status === 'active' ? 'default' : 'secondary'}
                          className="text-[10px]"
                        >
                          {account.status}
                        </Badge>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </CardContent>
        </Card>

        {/* Recent Mentions */}
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader>
            <CardTitle className="text-base flex items-center gap-2">
              <Inbox className="h-4 w-4" />
              Recent Mentions
              {aggregated.unreadCount > 0 && (
                <Badge variant="default" className="text-[10px] ml-1">{aggregated.unreadCount} new</Badge>
              )}
            </CardTitle>
          </CardHeader>
          <CardContent>
            {aggregated.mentions.length === 0 ? (
              <div className="text-center py-8 text-muted-foreground">
                <Inbox className="h-8 w-8 mx-auto mb-2 opacity-40" />
                <p>No mentions yet</p>
              </div>
            ) : (
              <ScrollArea className="h-[400px]">
                <div className="space-y-2 pr-3">
                  {aggregated.mentions.map((mention) => {
                    const PlatformIcon = PLATFORM_ICONS[mention.platform] || Globe;
                    const isUnread = mention.status === 'unread';
                    return (
                      <div
                        key={mention.id}
                        className={`p-3 rounded-lg border border-border/50 ${isUnread ? 'bg-accent/20' : ''}`}
                      >
                        <div className="flex items-start gap-2">
                          <PlatformIcon className="h-4 w-4 shrink-0 mt-0.5 text-muted-foreground" />
                          <div className="min-w-0 flex-1">
                            <div className="flex items-center gap-2">
                              <span className="text-xs font-medium">
                                {mention.author_display_name || mention.author_username || 'Unknown'}
                              </span>
                              <Badge variant="outline" className="text-[9px]">{mention.mention_type}</Badge>
                              {isUnread && <span className="h-1.5 w-1.5 rounded-full bg-blue-500 shrink-0" />}
                            </div>
                            {mention.content && (
                              <p className="text-xs text-muted-foreground line-clamp-2 mt-1">{mention.content}</p>
                            )}
                            <div className="flex items-center gap-2 mt-1">
                              <span className="text-[10px] text-muted-foreground">
                                {formatDate(mention.received_at)}
                              </span>
                              <Badge variant="outline" className="text-[9px]">{mention._projectName}</Badge>
                            </div>
                          </div>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </ScrollArea>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

// ── Members Tab ───────────────────────────────────────────────────────────────

function MembersTab({ orgId, orgName }: { orgId: string; orgName: string }) {
  const { data: members = [] } = useQuery<OrgMember[]>({
    queryKey: ['org-members', orgId],
    queryFn: () => organizationsApi.getMembers(orgId),
    enabled: !!orgId,
  });

  return (
    <Card className="bg-card/80 backdrop-blur-sm border-border/50">
      <CardHeader>
        <CardTitle>Team Members</CardTitle>
        <CardDescription>People with access to {orgName}</CardDescription>
      </CardHeader>
      <CardContent>
        {members.length === 0 ? (
          <div className="text-center py-8 text-muted-foreground">
            <Users className="h-8 w-8 mx-auto mb-2 opacity-40" />
            <p>No members found</p>
          </div>
        ) : (
          <div className="space-y-2">
            {members.map((m: OrgMember) => (
              <div key={m.id} className="flex items-center justify-between p-3 border rounded-lg">
                <div className="flex items-center gap-3">
                  <div className="h-8 w-8 rounded-full bg-muted flex items-center justify-center text-sm font-medium">
                    {(m.user?.full_name || m.user?.username || '?')[0].toUpperCase()}
                  </div>
                  <div>
                    <p className="text-sm font-medium">
                      {m.user?.full_name || m.user?.username || m.user_id}
                    </p>
                    {m.user?.email && (
                      <p className="text-xs text-muted-foreground">{m.user.email}</p>
                    )}
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <Badge variant="outline" className="capitalize">{m.role}</Badge>
                  <span className="text-xs text-muted-foreground">
                    Since {formatDate(m.joined_at)}
                  </span>
                </div>
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

// ── Main Page ─────────────────────────────────────────────────────────────────

export function OrganizationProfilePage({ defaultTab, defaultPipeline }: OrganizationProfilePageProps = {}) {
  const { orgId } = useParams<{ orgId: string }>();
  const [searchParams, setSearchParams] = useSearchParams();

  const tabFromUrl = searchParams.get('tab') || defaultTab || 'overview';
  const pipelineFromUrl = searchParams.get('pipeline') || defaultPipeline;
  const clientFilter = searchParams.get('client');
  const viewFromUrl = searchParams.get('view');

  const setTab = (tab: string) => {
    const params = new URLSearchParams(searchParams);
    params.set('tab', tab);
    if (tab !== 'pipelines') params.delete('pipeline');
    if (tab !== 'projects') params.delete('client');
    if (tab !== 'knowledge') params.delete('view');
    setSearchParams(params, { replace: true });
  };

  const clearClientFilter = () => {
    const params = new URLSearchParams(searchParams);
    params.delete('client');
    setSearchParams(params, { replace: true });
  };

  const { data: org, isLoading: orgLoading } = useQuery<OrganizationData>({
    queryKey: ['organization', orgId],
    queryFn: () => organizationsApi.getById(orgId!),
    enabled: !!orgId,
  });

  const { data: members = [] } = useQuery<OrgMember[]>({
    queryKey: ['org-members', orgId],
    queryFn: () => organizationsApi.getMembers(orgId!),
    enabled: !!orgId,
  });

  const { data: clients = [] } = useQuery<ClientData[]>({
    queryKey: ['org-clients', orgId],
    queryFn: () => organizationsApi.getClients(orgId!),
    enabled: !!orgId,
  });

  const { data: sidebarTree } = useQuery({
    queryKey: ['sidebarTree'],
    queryFn: () => organizationsApi.getSidebarTree(),
    staleTime: 60_000,
  });

  const { data: orgDeals = [] } = useQuery({
    queryKey: ['org-deals', orgId],
    queryFn: () => crmDealsApi.listOrgDeals(orgId!),
    enabled: !!orgId,
    staleTime: 60_000,
  });

  const { contacts } = useOrgContacts(orgId);

  const sidebarOrg = sidebarTree
    ? [...(sidebarTree.owned_orgs || []), ...(sidebarTree.member_orgs || [])].find(o => o.id === orgId)
    : undefined;

  const allProjects = useMemo(() => {
    if (!sidebarOrg) return [];
    const collectProjects = (projects: any[]): any[] =>
      projects.flatMap((p: any) => [p, ...collectProjects(p.children || [])]);
    return [
      ...collectProjects(sidebarOrg.internal_projects || []),
      ...(sidebarOrg.clients || []).flatMap((c: any) => collectProjects(c.projects || [])),
    ];
  }, [sidebarOrg]);

  const totalDealValue = useMemo(
    () => orgDeals.reduce((sum, d) => sum + (d.amount || 0), 0),
    [orgDeals]
  );

  if (orgLoading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-muted-foreground">Loading organisation...</div>
      </div>
    );
  }

  if (!org || !orgId) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-muted-foreground">Organisation not found.</div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full bg-background">
      {/* Header */}
      <div className="border-b bg-card/50 backdrop-blur-sm shadow-sm">
        <div className="max-w-[1600px] mx-auto px-6 py-5">
          <div className="flex items-center gap-4">
            <Link to="/projects" className="text-muted-foreground hover:text-foreground transition-colors">
              <ArrowLeft className="h-5 w-5" />
            </Link>
            <div className="p-2.5 bg-blue-100 dark:bg-blue-950 rounded-xl">
              <Building2 className="w-6 h-6 text-blue-600" />
            </div>
            <div className="min-w-0">
              <h1 className="text-2xl font-bold text-foreground">{org.name}</h1>
              {org.description && (
                <p className="text-sm text-muted-foreground mt-0.5 truncate">{org.description}</p>
              )}
            </div>
            <div className="ml-auto flex items-center gap-3">
              <Badge variant={org.is_active ? 'default' : 'secondary'} className="text-xs">
                {org.is_active ? 'Active' : 'Inactive'}
              </Badge>
            </div>
          </div>

          {/* Stat pills */}
          <div className="flex items-center gap-3 mt-4 flex-wrap">
            <StatPill icon={FolderOpen} label="Projects" value={allProjects.length} />
            <StatPill icon={Briefcase} label="Clients" value={clients.length} />
            <StatPill icon={Users} label="Members" value={members.length} />
            <StatPill icon={DollarSign} label="Pipeline" value={formatCurrency(totalDealValue)} />
          </div>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex-1 overflow-auto">
        <div className="max-w-[1600px] mx-auto px-6 py-5">
          <Tabs value={tabFromUrl} onValueChange={setTab}>
            <TabsList className="mb-6">
              <TabsTrigger value="overview">
                <LayoutGrid className="h-4 w-4 mr-2" />
                Overview
              </TabsTrigger>
              <TabsTrigger value="pipelines">
                <Target className="h-4 w-4 mr-2" />
                Pipelines
              </TabsTrigger>
              <TabsTrigger value="contacts">
                <Contact2 className="h-4 w-4 mr-2" />
                Contacts
              </TabsTrigger>
              <TabsTrigger value="projects">
                <FolderOpen className="h-4 w-4 mr-2" />
                Projects
              </TabsTrigger>
              <TabsTrigger value="social">
                <Share2 className="h-4 w-4 mr-2" />
                Social
              </TabsTrigger>
              <TabsTrigger value="knowledge">
                <Brain className="h-4 w-4 mr-2" />
                Intelligence
              </TabsTrigger>
              <TabsTrigger value="members">
                <Users className="h-4 w-4 mr-2" />
                Members
              </TabsTrigger>
            </TabsList>

            <TabsContent value="overview">
              <OverviewTab
                orgId={orgId}
                projectEntries={allProjects.map(p => ({ id: p.id, name: p.name }))}
                projectCount={allProjects.length}
                clientCount={clients.length}
                memberCount={members.length}
                totalDealValue={totalDealValue}
                totalDeals={orgDeals.length}
                contactCount={contacts.length}
                onSwitchTab={setTab}
              />
            </TabsContent>

            <TabsContent value="pipelines">
              <PipelinesTab orgId={orgId} defaultPipeline={pipelineFromUrl || undefined} />
            </TabsContent>

            <TabsContent value="contacts">
              <ContactsTab orgId={orgId} />
            </TabsContent>

            <TabsContent value="projects">
              <ProjectsTab
                orgId={orgId}
                sidebarOrg={sidebarOrg}
                clientFilter={clientFilter}
                onClearClientFilter={clearClientFilter}
              />
            </TabsContent>

            <TabsContent value="social">
              <SocialTab projectEntries={allProjects.map(p => ({ id: p.id, name: p.name }))} />
            </TabsContent>

            <TabsContent value="knowledge">
              <KnowledgeTab
                orgId={orgId!}
                projectEntries={allProjects.map(p => ({ id: p.id, name: p.name }))}
                view={viewFromUrl}
              />
            </TabsContent>

            <TabsContent value="members">
              <MembersTab orgId={orgId} orgName={org.name} />
            </TabsContent>
          </Tabs>
        </div>
      </div>
    </div>
  );
}

function StatPill({ icon: Icon, label, value }: { icon: any; label: string; value: string | number }) {
  return (
    <div className="flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-muted/50 text-sm">
      <Icon className="h-3.5 w-3.5 text-muted-foreground" />
      <span className="text-muted-foreground">{label}</span>
      <span className="font-semibold">{value}</span>
    </div>
  );
}
