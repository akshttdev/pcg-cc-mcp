import { useMemo, useState } from 'react';
import { useParams, useSearchParams, Link } from 'react-router-dom';
import { useQuery, useQueries } from '@tanstack/react-query';
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
  RefreshCw,
  AlertTriangle,
  Linkedin,
  Instagram,
  Twitter,
  Facebook,
  Youtube,
  Inbox,
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
  type CrmContactRecord,
  type CrmActivityRecord,
  type ProjectKnowledgeResponse,
  type ProjectKnowledgeSource,
  type SocialAccountRecord,
  type SocialMentionRecord,
  type TaskWithAttemptStatus,
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
  orgId,
  projectEntries,
  projectCount,
  clientCount,
  memberCount,
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
          { label: 'Knowledge', icon: BookOpen, tab: 'knowledge', color: 'text-orange-500' },
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
  const { contacts, isLoading, projectCount, loadedCount } = useOrgContacts(orgId);
  const [searchQuery, setSearchQuery] = useState('');
  const [stageFilter, setStageFilter] = useState<string>('all');

  const filtered = useMemo(() => {
    let result = contacts;
    if (searchQuery) {
      const q = searchQuery.toLowerCase();
      result = result.filter(
        c =>
          (c.first_name && c.first_name.toLowerCase().includes(q)) ||
          (c.last_name && c.last_name.toLowerCase().includes(q)) ||
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
          <span className="text-xs text-muted-foreground">
            Loading {loadedCount}/{projectCount} projects...
          </span>
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
            <ContactCard key={contact.id} contact={contact} />
          ))}
        </div>
      )}
    </div>
  );
}

function ContactCard({ contact }: { contact: OrgContact }) {
  const stageInfo = LIFECYCLE_STAGE_INFO[contact.lifecycle_stage as LifecycleStage];
  const name = [contact.first_name, contact.last_name].filter(Boolean).join(' ') || 'Unnamed';

  return (
    <Link
      to={`/projects/${contact._sourceProjectId}/crm/contacts/${contact.id}`}
      className="block p-4 rounded-xl border border-border/50 bg-card/50 hover:bg-accent/30 hover:border-accent/50 transition-all group"
    >
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <p className="text-sm font-medium truncate group-hover:text-foreground">{name}</p>
          {contact.email && (
            <p className="text-xs text-muted-foreground truncate mt-0.5">{contact.email}</p>
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
        <Badge variant="outline" className="text-[10px]">
          {contact._sourceProjectName}
        </Badge>
      </div>
    </Link>
  );
}

// ── Projects Tab ──────────────────────────────────────────────────────────────

function ProjectsTab({
  orgId,
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

function KnowledgeTab({ projectEntries }: { projectEntries: { id: string; name: string }[] }) {
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
            const allSources = Object.values(data.sources_by_type).flat();
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

  const { contacts } = useOrgContacts(orgId);

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
              <TabsTrigger value="social">
                <Share2 className="h-4 w-4 mr-2" />
                Social
              </TabsTrigger>
              <TabsTrigger value="knowledge">
                <BookOpen className="h-4 w-4 mr-2" />
                Knowledge
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
              <KnowledgeTab projectEntries={allProjects.map(p => ({ id: p.id, name: p.name }))} />
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
