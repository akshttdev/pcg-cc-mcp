import { useMemo, useState } from 'react';
import { useParams, useSearchParams, Link } from 'react-router-dom';
import { useQuery, useQueries, useMutation, useQueryClient } from '@tanstack/react-query';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Input } from '@/components/ui/input';
import { Progress } from '@/components/ui/progress';
import { ScrollArea } from '@/components/ui/scroll-area';
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
  Plug,
  CheckCircle2,
  AlertCircle,
  Clock,
  Settings2,
  Loader2,
  Unplug,
  Receipt,
  Wallet,
  Timer,
  CreditCard,
  UserCircle,
  Building,
  Copy,
  Check,
  Brain,
  RefreshCw,
} from 'lucide-react';
import {
  organizationsApi,
  crmDealsApi,
  crmActivitiesApi,
  knowledgeApi,
  socialApi,
  tasksApi,
  quickbooksApi,
  personsApi,
  pulseApi,
  type OrganizationData,
  type ClientData,
  type CrmActivityRecord,
  type PersonRecord,
  type ProjectKnowledgeResponse,
  type SocialAccountRecord,
  type SocialMentionRecord,
} from '@/lib/api';
import { CrmPipelineBoard } from '@/components/crm/CrmPipelineBoard';

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
          { label: 'Intelligence', icon: Brain, tab: 'intelligence', color: 'text-violet-500' },
          { label: 'Integrations', icon: Plug, tab: 'integrations', color: 'text-indigo-500' },
          { label: 'Leads', icon: UserCircle, tab: 'leads', color: 'text-orange-500' },
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
  const [searchQuery, setSearchQuery] = useState('');
  const [stageFilter, setStageFilter] = useState<string>('all');

  const { data: persons = [], isLoading } = useQuery<PersonRecord[]>({
    queryKey: ['org-persons-full', orgId],
    queryFn: () => organizationsApi.getPersons(orgId),
    enabled: !!orgId,
  });

  const filtered = useMemo(() => {
    let result = persons;
    if (searchQuery) {
      const q = searchQuery.toLowerCase();
      result = result.filter(
        p =>
          p.full_name.toLowerCase().includes(q) ||
          (p.email && p.email.toLowerCase().includes(q)) ||
          (p.company_name && p.company_name.toLowerCase().includes(q))
      );
    }
    if (stageFilter !== 'all') {
      result = result.filter(p => p.lifecycle_stage === stageFilter);
    }
    return result;
  }, [persons, searchQuery, stageFilter]);

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
      </div>

      {isLoading ? (
        <div className="text-center py-12 text-muted-foreground">
          <Contact2 className="h-8 w-8 mx-auto mb-2 opacity-40" />
          <p>Loading contacts…</p>
        </div>
      ) : filtered.length === 0 ? (
        <div className="text-center py-12 text-muted-foreground">
          <Contact2 className="h-8 w-8 mx-auto mb-2 opacity-40" />
          <p>No contacts found</p>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
          {filtered.map((person) => (
            <PersonContactCard key={person.id} person={person} />
          ))}
        </div>
      )}
    </div>
  );
}

function PersonContactCard({ person }: { person: PersonRecord }) {
  const stageInfo = LIFECYCLE_STAGE_INFO[person.lifecycle_stage as LifecycleStage];

  return (
    <Link
      to={`/people/${person.id}`}
      className="block p-4 rounded-xl border border-border/50 bg-card/50 hover:bg-accent/30 hover:border-accent/50 transition-all group"
    >
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <p className="text-sm font-medium truncate group-hover:text-foreground">{person.full_name}</p>
          {person.email && (
            <p className="text-xs text-muted-foreground truncate mt-0.5">{person.email}</p>
          )}
          {person.company_name && (
            <p className="text-xs text-muted-foreground truncate">{person.company_name}</p>
          )}
          {person.job_title && (
            <p className="text-xs text-muted-foreground truncate">{person.job_title}</p>
          )}
        </div>
        {person.lead_score > 0 && (
          <span className="text-xs font-medium px-1.5 py-0.5 rounded bg-blue-100 text-blue-700 dark:bg-blue-950 dark:text-blue-300 shrink-0">
            {person.lead_score}
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
        <Badge variant="outline" className="text-[10px] capitalize">
          {person.person_type}
        </Badge>
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

// ── Intelligence Tab (Social + Knowledge + Pulse) ─────────────────────────────

const SOURCE_TYPE_META: Record<string, { label: string; icon: typeof BookOpen }> = {
  conversation: { label: 'Conversations', icon: MessageSquare },
  artifact: { label: 'Artifacts', icon: FileText },
  pulse_content: { label: 'Pulse Content', icon: Radio },
  context_injection: { label: 'Context Injections', icon: Pencil },
  entity: { label: 'Entities', icon: Boxes },
  topology_snapshot: { label: 'Topology Snapshots', icon: Network },
};

function KnowledgeSection({ projectEntries }: { projectEntries: { id: string; name: string }[] }) {
  const knowledgeQueries = useQueries({
    queries: projectEntries.map((entry) => ({
      queryKey: ['projectKnowledge', entry.id],
      queryFn: () => knowledgeApi.getProjectKnowledge(entry.id),
      staleTime: 60_000,
      enabled: projectEntries.length > 0,
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
    });

    const avgCompleteness = coverageCount > 0 ? Math.round((totalCoverage / coverageCount) * 100) : 0;
    return { totalSources, staleSources, avgCompleteness, sourcesByProject };
  }, [knowledgeQueries, projectEntries]);

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
              <span className="text-sm text-muted-foreground">across {projectEntries.length} projects</span>
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

      {isLoading && (
        <p className="text-xs text-muted-foreground">Loading {loadedCount}/{projectEntries.length} projects...</p>
      )}

      {/* Per-project knowledge */}
      {aggregated.sourcesByProject.length === 0 && !isLoading ? (
        <div className="text-center py-12 text-muted-foreground">
          <BookOpen className="h-8 w-8 mx-auto mb-2 opacity-40" />
          <p>No knowledge sources indexed yet</p>
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

function SocialSection({ projectEntries }: { projectEntries: { id: string; name: string }[] }) {
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

// ── Pulse Section (used inside IntelligenceTab) ───────────────────────────────

function PulseSection({ projectEntries }: { projectEntries: { id: string; name: string }[] }) {
  const alertQueries = useQueries({
    queries: projectEntries.map((entry) => ({
      queryKey: ['pulse-alerts-org', entry.id],
      queryFn: () => pulseApi.getAlerts(entry.id, 20),
      staleTime: 60_000,
      enabled: projectEntries.length > 0,
    })),
  });

  const contentQueries = useQueries({
    queries: projectEntries.map((entry) => ({
      queryKey: ['pulse-content-org', entry.id],
      queryFn: () => pulseApi.getLatestContent(entry.id, 10),
      staleTime: 60_000,
      enabled: projectEntries.length > 0,
    })),
  });

  const aggregated = useMemo(() => {
    const allAlerts: any[] = [];
    const allContent: any[] = [];

    alertQueries.forEach((q, i) => {
      if (!q.data) return;
      const entry = projectEntries[i];
      q.data.forEach((a: any) => allAlerts.push({ ...a, _projectName: entry.name }));
    });

    contentQueries.forEach((q, i) => {
      if (!q.data?.items) return;
      const entry = projectEntries[i];
      q.data.items.forEach((c: any) => allContent.push({ ...c, _projectName: entry.name }));
    });

    allAlerts.sort((a, b) => new Date(b.triggered_at || b.created_at).getTime() - new Date(a.triggered_at || a.created_at).getTime());
    allContent.sort((a, b) => new Date(b.collected_at || b.created_at).getTime() - new Date(a.collected_at || a.created_at).getTime());

    const unacknowledged = allAlerts.filter(a => !a.acknowledged_at).length;

    return { alerts: allAlerts.slice(0, 20), content: allContent.slice(0, 20), unacknowledged };
  }, [alertQueries, contentQueries, projectEntries]);

  return (
    <div className="space-y-6">
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Unacknowledged Alerts</CardTitle>
          </CardHeader>
          <CardContent>
            <span className={`text-2xl font-bold ${aggregated.unacknowledged > 0 ? 'text-amber-600' : ''}`}>
              {aggregated.unacknowledged}
            </span>
          </CardContent>
        </Card>
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Recent Signals</CardTitle>
          </CardHeader>
          <CardContent>
            <span className="text-2xl font-bold">{aggregated.content.length}</span>
          </CardContent>
        </Card>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader>
            <CardTitle className="text-base flex items-center gap-2">
              <AlertTriangle className="h-4 w-4 text-amber-500" />
              Recent Alerts
              {aggregated.unacknowledged > 0 && (
                <Badge variant="default" className="text-[10px] ml-1">{aggregated.unacknowledged} new</Badge>
              )}
            </CardTitle>
          </CardHeader>
          <CardContent>
            {aggregated.alerts.length === 0 ? (
              <div className="text-center py-8 text-muted-foreground">
                <AlertTriangle className="h-8 w-8 mx-auto mb-2 opacity-40" />
                <p>No alerts</p>
              </div>
            ) : (
              <ScrollArea className="h-[300px]">
                <div className="space-y-2 pr-3">
                  {aggregated.alerts.map((alert: any) => (
                    <div
                      key={alert.id}
                      className={`p-3 rounded-lg border border-border/50 ${!alert.acknowledged_at ? 'bg-amber-50/30 dark:bg-amber-950/20' : ''}`}
                    >
                      <div className="flex items-start justify-between gap-2">
                        <div className="min-w-0">
                          <p className="text-sm font-medium truncate">{alert.rule_name || 'Alert'}</p>
                          {alert.message && (
                            <p className="text-xs text-muted-foreground line-clamp-2 mt-0.5">{alert.message}</p>
                          )}
                          <div className="flex items-center gap-2 mt-1">
                            <Badge variant="outline" className="text-[9px]">{alert._projectName}</Badge>
                            <span className="text-[10px] text-muted-foreground">
                              {formatDate(alert.triggered_at || alert.created_at)}
                            </span>
                          </div>
                        </div>
                        {!alert.acknowledged_at && (
                          <span className="h-2 w-2 rounded-full bg-amber-500 shrink-0 mt-1" />
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              </ScrollArea>
            )}
          </CardContent>
        </Card>

        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader>
            <CardTitle className="text-base flex items-center gap-2">
              <Radio className="h-4 w-4 text-blue-500" />
              Latest Signals
            </CardTitle>
          </CardHeader>
          <CardContent>
            {aggregated.content.length === 0 ? (
              <div className="text-center py-8 text-muted-foreground">
                <Radio className="h-8 w-8 mx-auto mb-2 opacity-40" />
                <p>No signals collected yet</p>
              </div>
            ) : (
              <ScrollArea className="h-[300px]">
                <div className="space-y-2 pr-3">
                  {aggregated.content.map((item: any) => (
                    <div key={item.id} className="p-3 rounded-lg border border-border/50">
                      <p className="text-sm font-medium line-clamp-2">{item.title || item.content_preview || 'Signal'}</p>
                      <div className="flex items-center gap-2 mt-1">
                        <Badge variant="outline" className="text-[9px]">{item._projectName}</Badge>
                        {item.source_name && (
                          <span className="text-[10px] text-muted-foreground">{item.source_name}</span>
                        )}
                        <span className="text-[10px] text-muted-foreground ml-auto">
                          {formatDate(item.collected_at || item.created_at)}
                        </span>
                      </div>
                    </div>
                  ))}
                </div>
              </ScrollArea>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

// ── Intelligence Tab (Social + Knowledge + Pulse combined) ────────────────────

function IntelligenceTab({ projectEntries }: { projectEntries: { id: string; name: string }[] }) {
  const [view, setView] = useState<'social' | 'knowledge' | 'pulse'>('social');

  const viewOptions = [
    { key: 'social' as const, label: 'Social', icon: Share2 },
    { key: 'knowledge' as const, label: 'Knowledge Graph', icon: BookOpen },
    { key: 'pulse' as const, label: 'Pulse Signals', icon: Radio },
  ];

  return (
    <div className="space-y-6">
      <div className="flex gap-1 p-1 bg-muted/50 rounded-lg w-fit">
        {viewOptions.map(({ key, label, icon: Icon }) => (
          <button
            key={key}
            onClick={() => setView(key)}
            className={`flex items-center gap-1.5 px-4 py-1.5 text-sm font-medium rounded-md transition-all ${
              view === key
                ? 'bg-background text-foreground shadow-sm'
                : 'text-muted-foreground hover:text-foreground'
            }`}
          >
            <Icon className="h-3.5 w-3.5" />
            {label}
          </button>
        ))}
      </div>

      {view === 'social' && <SocialSection projectEntries={projectEntries} />}
      {view === 'knowledge' && <KnowledgeSection projectEntries={projectEntries} />}
      {view === 'pulse' && <PulseSection projectEntries={projectEntries} />}
    </div>
  );
}

// ── Leads Tab ─────────────────────────────────────────────────────────────────

function LeadsTab({ orgId }: { orgId: string }) {
  const queryClient = useQueryClient();
  const [inviteUrls, setInviteUrls] = useState<Record<string, string>>({});
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [inviteEmail, setInviteEmail] = useState<Record<string, string>>({});

  const { data: leads = [], isLoading } = useQuery<PersonRecord[]>({
    queryKey: ['org-leads', orgId],
    queryFn: () => personsApi.list({ organization_id: orgId, limit: 200 }),
    enabled: !!orgId,
  });

  const provisionMutation = useMutation({
    mutationFn: (personId: string) => personsApi.provisionOrg(personId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['org-leads', orgId] });
    },
  });

  const generateInviteMutation = useMutation({
    mutationFn: ({ companyOrgId, email }: { companyOrgId: string; email?: string }) =>
      organizationsApi.generateInvite(companyOrgId, email || undefined),
    onSuccess: (data, variables) => {
      setInviteUrls((prev) => ({ ...prev, [variables.companyOrgId]: data.invite_url }));
    },
  });

  function copyUrl(orgId: string, url: string) {
    navigator.clipboard.writeText(url);
    setCopiedId(orgId);
    setTimeout(() => setCopiedId(null), 2000);
  }

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12 text-muted-foreground">
        Loading leads…
      </div>
    );
  }

  if (leads.length === 0) {
    return (
      <Card className="bg-card/80 backdrop-blur-sm border-border/50">
        <CardContent className="py-12 text-center text-muted-foreground">
          <UserCircle className="h-10 w-10 mx-auto mb-3 opacity-40" />
          <p>No leads found for this organisation.</p>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-3">
      {leads.map((person) => (
        <Card key={person.id} className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardContent className="py-4">
            <div className="flex items-start justify-between gap-4">
              <div className="flex items-center gap-3 min-w-0">
                <div className="h-9 w-9 rounded-full bg-muted flex items-center justify-center text-sm font-semibold shrink-0">
                  {person.full_name[0].toUpperCase()}
                </div>
                <div className="min-w-0">
                  <Link
                    to={`/people/${person.id}`}
                    className="text-sm font-medium hover:underline truncate block"
                  >
                    {person.full_name}
                  </Link>
                  {person.company_name && (
                    <p className="text-xs text-muted-foreground">{person.company_name}</p>
                  )}
                  {person.job_title && (
                    <p className="text-xs text-muted-foreground">{person.job_title}</p>
                  )}
                </div>
              </div>
              <div className="flex items-center gap-2 shrink-0 flex-wrap justify-end">
                <Badge variant="outline" className="text-xs capitalize">
                  {person.lifecycle_stage}
                </Badge>
                {person.lead_score > 0 && (
                  <Badge variant="secondary" className="text-xs">
                    Score {person.lead_score}
                  </Badge>
                )}
              </div>
            </div>

            {/* Provision / Invite actions */}
            <div className="mt-3 flex flex-wrap items-center gap-2">
              {!person.company_org_id ? (
                person.company_name ? (
                  <Button
                    size="sm"
                    variant="outline"
                    className="text-xs"
                    disabled={provisionMutation.isPending}
                    onClick={() => provisionMutation.mutate(person.id)}
                  >
                    <Building className="h-3 w-3 mr-1" />
                    Provision Company Profile
                  </Button>
                ) : (
                  <span className="text-xs text-muted-foreground italic">
                    No company name — set one to provision.
                  </span>
                )
              ) : (
                <>
                  <Link to={`/organizations/${person.company_org_id}`} className="text-xs text-blue-600 hover:underline flex items-center gap-1">
                    <Building className="h-3 w-3" />
                    View company org
                  </Link>
                  {!inviteUrls[person.company_org_id] ? (
                    <div className="flex items-center gap-1">
                      <Input
                        type="email"
                        placeholder="Email (optional)"
                        className="h-7 text-xs w-44"
                        value={inviteEmail[person.company_org_id] ?? ''}
                        onChange={(e) =>
                          setInviteEmail((prev) => ({ ...prev, [person.company_org_id!]: e.target.value }))
                        }
                      />
                      <Button
                        size="sm"
                        variant="secondary"
                        className="text-xs h-7"
                        disabled={generateInviteMutation.isPending}
                        onClick={() =>
                          generateInviteMutation.mutate({
                            companyOrgId: person.company_org_id!,
                            email: inviteEmail[person.company_org_id!] || undefined,
                          })
                        }
                      >
                        Generate Invite
                      </Button>
                    </div>
                  ) : (
                    <div className="flex items-center gap-1 max-w-full">
                      <Input
                        readOnly
                        value={inviteUrls[person.company_org_id]}
                        className="h-7 text-xs flex-1 min-w-0"
                      />
                      <Button
                        size="sm"
                        variant="ghost"
                        className="h-7 px-2"
                        onClick={() => copyUrl(person.company_org_id!, inviteUrls[person.company_org_id!])}
                      >
                        {copiedId === person.company_org_id ? (
                          <Check className="h-3 w-3 text-green-500" />
                        ) : (
                          <Copy className="h-3 w-3" />
                        )}
                      </Button>
                    </div>
                  )}
                </>
              )}
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}

// ── Members Tab ───────────────────────────────────────────────────────────────

// ── Integrations Tab ────────────────────────────────────────────────────────

function IntegrationsTab({ orgId }: { orgId: string }) {
  const [disconnecting, setDisconnecting] = useState(false);
  const [syncing, setSyncing] = useState(false);

  const { data: qbStatus, isLoading: qbLoading, refetch: refetchQB } = useQuery({
    queryKey: ['quickbooks-status', orgId],
    queryFn: () => quickbooksApi.getStatus(orgId),
    staleTime: 30_000,
  });

  const handleConnect = () => {
    const url = quickbooksApi.getConnectUrl(orgId);
    window.location.href = url;
  };

  const handleDisconnect = async () => {
    if (!qbStatus?.account?.id) return;
    if (!confirm('Disconnect QuickBooks? Entity mappings will be removed.')) return;
    setDisconnecting(true);
    try {
      await quickbooksApi.disconnect(qbStatus.account.id);
      refetchQB();
    } catch (e) {
      console.error('Failed to disconnect QuickBooks:', e);
    } finally {
      setDisconnecting(false);
    }
  };

  const handleSync = async () => {
    if (!qbStatus?.account?.id) return;
    setSyncing(true);
    try {
      await quickbooksApi.triggerSync(qbStatus.account.id);
      refetchQB();
    } catch (e) {
      console.error('Sync failed:', e);
    } finally {
      setSyncing(false);
    }
  };

  const handleRefreshToken = async () => {
    if (!qbStatus?.account?.id) return;
    try {
      await quickbooksApi.refreshToken(qbStatus.account.id);
      refetchQB();
    } catch (e) {
      console.error('Token refresh failed:', e);
    }
  };

  const qbAccount = qbStatus?.account;

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h2 className="text-lg font-semibold">Organization Integrations</h2>
        <p className="text-sm text-muted-foreground mt-1">
          Connect external services at the organization level. These integrations are shared across all projects.
        </p>
      </div>

      {/* Accounting & Finance Section */}
      <div className="space-y-4">
        <div className="flex items-center gap-2">
          <span className="flex h-8 w-8 items-center justify-center rounded-full bg-gradient-to-br from-emerald-50 to-blue-100">
            <Wallet className="h-4 w-4 text-emerald-600" />
          </span>
          <div>
            <h3 className="text-sm font-semibold">Accounting & Finance</h3>
            <p className="text-xs text-muted-foreground">Invoicing, expenses, and cost tracking</p>
          </div>
        </div>

        {/* QuickBooks Card */}
        <Card className="border-border/60 bg-card/80 backdrop-blur-sm overflow-hidden">
          <div className="flex items-stretch">
            {/* Left accent bar */}
            <div className={`w-1 shrink-0 ${
              qbLoading ? 'bg-muted' :
              qbStatus?.connected ? 'bg-emerald-500' :
              qbStatus?.needs_reauth ? 'bg-amber-500' :
              'bg-muted-foreground/20'
            }`} />

            <div className="flex-1 p-5">
              <div className="flex items-start justify-between gap-4">
                <div className="flex items-center gap-3">
                  {/* QB Logo placeholder */}
                  <div className="flex h-11 w-11 items-center justify-center rounded-xl bg-[#2CA01C]/10 border border-[#2CA01C]/20">
                    <Receipt className="h-5 w-5 text-[#2CA01C]" />
                  </div>
                  <div>
                    <div className="flex items-center gap-2">
                      <h4 className="text-sm font-semibold">QuickBooks Online</h4>
                      {qbLoading ? (
                        <Badge variant="secondary" className="text-[10px] px-1.5 py-0">
                          <Loader2 className="h-3 w-3 animate-spin mr-1" />
                          Checking
                        </Badge>
                      ) : qbStatus?.connected ? (
                        <Badge className="text-[10px] px-1.5 py-0 bg-emerald-100 text-emerald-700 border-emerald-200">
                          <CheckCircle2 className="h-3 w-3 mr-1" />
                          Connected
                        </Badge>
                      ) : qbStatus?.needs_reauth ? (
                        <Badge className="text-[10px] px-1.5 py-0 bg-amber-100 text-amber-700 border-amber-200">
                          <AlertCircle className="h-3 w-3 mr-1" />
                          Reauthorize
                        </Badge>
                      ) : (
                        <Badge variant="secondary" className="text-[10px] px-1.5 py-0">
                          Not connected
                        </Badge>
                      )}
                    </div>
                    {qbAccount?.company_name ? (
                      <p className="text-xs text-muted-foreground mt-0.5">
                        {qbAccount.company_name}
                        <span className="text-muted-foreground/60 ml-1.5">
                          Realm {qbAccount.realm_id}
                        </span>
                      </p>
                    ) : (
                      <p className="text-xs text-muted-foreground mt-0.5">
                        Sync invoices, customers, expenses & orchestrator costs with QuickBooks
                      </p>
                    )}
                  </div>
                </div>

                {/* Actions */}
                <div className="flex items-center gap-2">
                  {!qbLoading && qbStatus?.connected && (
                    <>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={handleSync}
                        disabled={syncing}
                        className="h-8 text-xs"
                      >
                        {syncing ? (
                          <Loader2 className="h-3.5 w-3.5 animate-spin mr-1.5" />
                        ) : (
                          <RefreshCw className="h-3.5 w-3.5 mr-1.5" />
                        )}
                        Sync
                      </Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={handleDisconnect}
                        disabled={disconnecting}
                        className="h-8 text-xs text-muted-foreground hover:text-destructive"
                      >
                        <Unplug className="h-3.5 w-3.5 mr-1.5" />
                        Disconnect
                      </Button>
                    </>
                  )}
                  {!qbLoading && qbStatus?.needs_reauth && (
                    <Button size="sm" onClick={handleRefreshToken} className="h-8 text-xs">
                      <RefreshCw className="h-3.5 w-3.5 mr-1.5" />
                      Reauthorize
                    </Button>
                  )}
                  {!qbLoading && !qbStatus?.connected && !qbStatus?.needs_reauth && (
                    <Button size="sm" onClick={handleConnect} className="h-8 text-xs">
                      <Plug className="h-3.5 w-3.5 mr-1.5" />
                      Connect QuickBooks
                    </Button>
                  )}
                </div>
              </div>

              {/* Connected: Show sync scope & stats */}
              {qbAccount && qbStatus?.connected && (
                <div className="mt-4 pt-4 border-t border-border/40">
                  <div className="grid grid-cols-2 md:grid-cols-5 gap-3">
                    {[
                      { key: 'invoices', label: 'Invoices', icon: Receipt, enabled: qbAccount.sync_invoices === 1 },
                      { key: 'customers', label: 'Customers', icon: Contact2, enabled: qbAccount.sync_customers === 1 },
                      { key: 'payments', label: 'Payments', icon: CreditCard, enabled: qbAccount.sync_payments === 1 },
                      { key: 'expenses', label: 'Expenses', icon: Wallet, enabled: qbAccount.sync_expenses === 1 },
                      { key: 'time', label: 'Time Tracking', icon: Timer, enabled: qbAccount.sync_time_tracking === 1 },
                    ].map(({ key, label, icon: ScopeIcon, enabled }) => (
                      <div
                        key={key}
                        className={`flex items-center gap-2 px-3 py-2 rounded-lg border text-xs ${
                          enabled
                            ? 'border-emerald-200/60 bg-emerald-50/50 text-emerald-700'
                            : 'border-border/40 bg-muted/30 text-muted-foreground'
                        }`}
                      >
                        <ScopeIcon className="h-3.5 w-3.5 shrink-0" />
                        <span className="font-medium">{label}</span>
                        {enabled ? (
                          <CheckCircle2 className="h-3 w-3 ml-auto shrink-0" />
                        ) : (
                          <span className="ml-auto text-[10px] opacity-60">Off</span>
                        )}
                      </div>
                    ))}
                  </div>

                  <div className="flex items-center gap-4 mt-3 text-xs text-muted-foreground">
                    <span className="flex items-center gap-1">
                      <Clock className="h-3 w-3" />
                      {qbAccount.last_sync_at
                        ? `Last synced ${new Date(qbAccount.last_sync_at).toLocaleString()}`
                        : 'Never synced'}
                    </span>
                    <span className="text-muted-foreground/40">|</span>
                    <span>
                      Environment: <span className="font-medium capitalize">{qbAccount.environment}</span>
                    </span>
                    <span className="text-muted-foreground/40">|</span>
                    <span>
                      Sync every <span className="font-medium">{qbAccount.sync_frequency_minutes}m</span>
                    </span>
                  </div>

                  {qbAccount.last_error && (
                    <div className="mt-3 flex items-start gap-2 text-xs text-amber-700 bg-amber-50 rounded-lg p-2.5 border border-amber-200/60">
                      <AlertTriangle className="h-3.5 w-3.5 shrink-0 mt-0.5" />
                      <span>{qbAccount.last_error}</span>
                    </div>
                  )}
                </div>
              )}
            </div>
          </div>
        </Card>
      </div>

      {/* Future: More org-level integrations */}
      <div className="space-y-4">
        <div className="flex items-center gap-2">
          <span className="flex h-8 w-8 items-center justify-center rounded-full bg-gradient-to-br from-slate-50 to-purple-100">
            <Settings2 className="h-4 w-4 text-slate-600" />
          </span>
          <div>
            <h3 className="text-sm font-semibold">More Integrations</h3>
            <p className="text-xs text-muted-foreground">Additional org-level services coming soon</p>
          </div>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
          {[
            { name: 'Stripe', desc: 'Payment processing & billing', icon: CreditCard, color: 'text-purple-500' },
            { name: 'Xero', desc: 'Alternative accounting platform', icon: Receipt, color: 'text-blue-500' },
            { name: 'HubSpot', desc: 'CRM & marketing automation', icon: Target, color: 'text-orange-500' },
          ].map(({ name, desc, icon: PlaceholderIcon, color }) => (
            <Card key={name} className="border-dashed border-border/40 bg-muted/20">
              <CardContent className="flex items-center gap-3 py-4 px-4">
                <PlaceholderIcon className={`h-5 w-5 ${color} opacity-40`} />
                <div className="flex-1">
                  <p className="text-sm font-medium text-muted-foreground/60">{name}</p>
                  <p className="text-xs text-muted-foreground/40">{desc}</p>
                </div>
                <Badge variant="outline" className="text-[10px] text-muted-foreground/40 border-border/30">
                  Soon
                </Badge>
              </CardContent>
            </Card>
          ))}
        </div>
      </div>
    </div>
  );
}

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

  const setTab = (tab: string) => {
    const params = new URLSearchParams(searchParams);
    params.set('tab', tab);
    if (tab !== 'pipelines') params.delete('pipeline');
    if (tab !== 'projects') params.delete('client');
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

  const { data: orgPersons = [] } = useQuery<PersonRecord[]>({
    queryKey: ['org-persons', orgId],
    queryFn: () => personsApi.list({ organization_id: orgId!, limit: 500 }),
    enabled: !!orgId,
    staleTime: 60_000,
  });

  const sidebarOrg = sidebarTree
    ? [...(sidebarTree.owned_orgs || []), ...(sidebarTree.member_orgs || [])].find(o => o.id === orgId)
    : undefined;

  const allProjects = useMemo(() => {
    if (!sidebarOrg) return [];
    return [
      ...(sidebarOrg.internal_projects || []),
      ...(sidebarOrg.internal_folders || []).flatMap((f: any) => f.projects),
      ...(sidebarOrg.clients || []).flatMap((c: any) => [
        ...(c.projects || []),
        ...(c.folders || []).flatMap((f: any) => f.projects),
      ]),
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
              <TabsTrigger value="intelligence">
                <Brain className="h-4 w-4 mr-2" />
                Intelligence
              </TabsTrigger>
              <TabsTrigger value="integrations">
                <Plug className="h-4 w-4 mr-2" />
                Integrations
              </TabsTrigger>
              <TabsTrigger value="members">
                <Users className="h-4 w-4 mr-2" />
                Members
              </TabsTrigger>
              <TabsTrigger value="leads">
                <UserCircle className="h-4 w-4 mr-2" />
                Leads
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
                contactCount={orgPersons.length}
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

            <TabsContent value="intelligence">
              <IntelligenceTab projectEntries={allProjects.map(p => ({ id: p.id, name: p.name }))} />
            </TabsContent>

            <TabsContent value="integrations">
              <IntegrationsTab orgId={orgId} />
            </TabsContent>

            <TabsContent value="members">
              <MembersTab orgId={orgId} orgName={org.name} />
            </TabsContent>

            <TabsContent value="leads">
              <LeadsTab orgId={orgId} />
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
