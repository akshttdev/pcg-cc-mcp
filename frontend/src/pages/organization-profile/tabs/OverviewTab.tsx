import { useMemo } from 'react';
import { useQueries, useQuery } from '@tanstack/react-query';
import { Link, useNavigate } from 'react-router-dom';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import {
  Activity,
  Target,
  DollarSign,
  Contact2,
  FolderOpen,
  Brain,
  Users,
  Plug,
  ChevronRight,
  Play,
  TrendingUp,
  CheckCircle2,
  Rocket,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  tasksApi,
  crmActivitiesApi,
  workflowsApi,
  type CrmActivityRecord,
  type WorkflowRun,
} from '@/lib/api';
import { taskKeys, organizationKeys } from '@/lib/query-keys';
import { useOrgOnboarding } from '@/hooks/useOrgOnboarding';
import { OnboardingCarousel, type OnboardingSegment } from '@/components/onboarding/OnboardingCarousel';
import { formatDate, formatCurrency } from '../helpers';

function TrendIndicator({ value }: { value: number }) {
  if (value === 0) return <span className="text-[10px] text-muted-foreground">No data</span>;
  return (
    <span className="text-[10px] text-green-600 dark:text-green-400 flex items-center gap-0.5">
      <TrendingUp className="h-3 w-3" /> Active
    </span>
  );
}

export function OverviewTab({
  orgId,
  projectEntries,
  projectCount,
  totalDealValue,
  totalDeals,
  contactCount,
}: {
  orgId: string;
  projectEntries: { id: string; name: string }[];
  projectCount: number;
  totalDealValue: number;
  totalDeals: number;
  contactCount: number;
}) {
  const navigate = useNavigate();

  // Org onboarding
  const { data: onboardingData, isLoading: onboardingLoading, startOnboarding, startSegment } = useOrgOnboarding(orgId);
  const onboardingStatus = onboardingData?.onboarding?.status;
  const onboardingSegments: OnboardingSegment[] = useMemo(
    () =>
      (onboardingData?.segments ?? []).map((s) => ({
        id: s.id,
        segment_type: s.segment_type as OnboardingSegment['segment_type'],
        name: s.name,
        assigned_agent_name: s.assigned_agent_name ?? undefined,
        status: s.status as OnboardingSegment['status'],
        recommendations: s.recommendations ?? undefined,
        user_decisions: s.user_decisions ?? undefined,
        order_index: s.order_index,
      })),
    [onboardingData?.segments]
  );

  // Aggregate tasks across all projects
  const taskQueries = useQueries({
    queries: projectEntries.map((entry) => ({
      queryKey: taskKeys.list(entry.id),
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
      queryKey: organizationKeys.activitiesOrg(entry.id),
      queryFn: () => crmActivitiesApi.listActivities({ organization_id: entry.id, limit: 10 }),
      staleTime: 60_000,
      enabled: projectEntries.length > 0,
    })),
  });

  // Fetch recent workflow runs for the organization
  const { data: recentWorkflowRuns = [] } = useQuery({
    queryKey: organizationKeys.workflowRuns(orgId),
    queryFn: () => workflowsApi.listRecentRuns({ organization_id: orgId, limit: 10 }),
    staleTime: 60_000,
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
            <TrendIndicator value={totalTasks} />
          </CardContent>
        </Card>
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardContent className="pt-4 pb-3">
            <div className="flex items-center gap-2 text-muted-foreground">
              <Target className="h-4 w-4" />
              <span className="text-xs font-medium">Total Deals</span>
            </div>
            <p className="text-2xl font-bold mt-1">{totalDeals}</p>
            <TrendIndicator value={totalDeals} />
          </CardContent>
        </Card>
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardContent className="pt-4 pb-3">
            <div className="flex items-center gap-2 text-muted-foreground">
              <DollarSign className="h-4 w-4" />
              <span className="text-xs font-medium">Pipeline Value</span>
            </div>
            <p className="text-2xl font-bold mt-1">{formatCurrency(totalDealValue)}</p>
            <TrendIndicator value={totalDealValue} />
          </CardContent>
        </Card>
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardContent className="pt-4 pb-3">
            <div className="flex items-center gap-2 text-muted-foreground">
              <Contact2 className="h-4 w-4" />
              <span className="text-xs font-medium">Contacts</span>
            </div>
            <p className="text-2xl font-bold mt-1">{contactCount}</p>
            <TrendIndicator value={contactCount} />
          </CardContent>
        </Card>
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardContent className="pt-4 pb-3">
            <div className="flex items-center gap-2 text-muted-foreground">
              <FolderOpen className="h-4 w-4" />
              <span className="text-xs font-medium">Active Projects</span>
            </div>
            <p className="text-2xl font-bold mt-1">{projectCount}</p>
            <TrendIndicator value={projectCount} />
          </CardContent>
        </Card>
      </div>

      {/* Org Onboarding Carousel */}
      {onboardingStatus === 'active' && onboardingSegments.length > 0 && (
        <OnboardingCarousel
          segments={onboardingSegments}
          currentPhase={onboardingData?.onboarding?.current_phase ?? 'context_gathering'}
          onSegmentClick={(segment) => {
            const segmentRoutes: Record<string, string> = {
              crm: `crm/pipeline`,
              intelligence: `intelligence`,
              integrations: `integrations`,
              social: `social`,
              research: `projects`,
              brand: `projects`,
              website: `projects`,
              email: `projects`,
              legal: `projects`,
            };
            const route = segmentRoutes[segment.segment_type] ?? 'projects';
            navigate(`/organizations/${orgId}/${route}`);
          }}
          onStartSegment={(segment) => {
            startSegment.mutate(segment.id);
          }}
        />
      )}
      {onboardingStatus === 'completed' && (
        <div className="flex items-center gap-2 p-3 rounded-lg border border-green-500/20 bg-green-500/5">
          <CheckCircle2 className="h-4 w-4 text-green-600 dark:text-green-400" />
          <span className="text-sm font-medium text-green-700 dark:text-green-300">Organization setup complete</span>
        </div>
      )}
      {!onboardingLoading && !onboardingData && (
        <Card className="bg-card/80 backdrop-blur-sm border-border/50 border-dashed">
          <CardContent className="py-6 text-center space-y-3">
            <Rocket className="h-8 w-8 mx-auto text-primary/40" />
            <div>
              <p className="text-sm font-medium">Set up your organization</p>
              <p className="text-xs text-muted-foreground mt-1">
                Walk through CRM, integrations, intelligence, and more with guided setup.
              </p>
            </div>
            <Button
              size="sm"
              onClick={() => startOnboarding.mutate(undefined)}
              disabled={startOnboarding.isPending}
            >
              {startOnboarding.isPending ? 'Starting...' : 'Start Setup'}
            </Button>
          </CardContent>
        </Card>
      )}

      {/* Quick links */}
      <div className="grid grid-cols-2 lg:grid-cols-3 gap-3">
        {[
          { label: 'Pipelines', icon: Target, path: 'crm/pipeline', color: 'text-amber-500', summary: `${totalDeals} deals · ${formatCurrency(totalDealValue)}` },
          { label: 'Contacts', icon: Contact2, path: 'crm/contacts', color: 'text-blue-500', summary: `${contactCount} contacts` },
          { label: 'Projects', icon: FolderOpen, path: 'projects', color: 'text-emerald-500', summary: `${projectCount} active` },
          { label: 'Intelligence', icon: Brain, path: 'intelligence', color: 'text-orange-500', summary: 'Data sources & workflows' },
          { label: 'Members', icon: Users, path: 'members', color: 'text-purple-500', summary: 'Team & roles' },
          { label: 'Integrations', icon: Plug, path: 'integrations', color: 'text-indigo-500', summary: 'Connected services' },
        ].map(({ label, icon: Icon, path, color, summary }) => (
          <Link
            key={path}
            to={`/organizations/${orgId}/${path}`}
            className="flex items-center gap-3 p-4 rounded-xl border border-border/50 bg-card/50 hover:bg-accent/50 hover:border-accent transition-all text-left group cursor-pointer"
          >
            <Icon className={`h-5 w-5 ${color} group-hover:scale-110 transition-transform`} />
            <div className="min-w-0">
              <span className="text-sm font-medium block">{label}</span>
              <span className="text-xs text-muted-foreground">{summary}</span>
            </div>
            <ChevronRight className="h-4 w-4 text-muted-foreground ml-auto opacity-0 group-hover:opacity-100 transition-opacity" />
          </Link>
        ))}
      </div>

      {/* Recent activity - workflow runs + CRM activities */}
      <Card className="bg-card/80 backdrop-blur-sm border-border/50">
        <CardHeader>
          <CardTitle className="text-base flex items-center gap-2">
            <Activity className="h-4 w-4" />
            Recent Activity
          </CardTitle>
          <CardDescription>Latest workflow runs and CRM activity</CardDescription>
        </CardHeader>
        <CardContent>
          {recentWorkflowRuns.length === 0 && recentActivities.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <Activity className="h-8 w-8 mx-auto mb-2 opacity-40" />
              <p>No recent activity</p>
              <p className="text-xs mt-1">Activity from tasks, deals, and contacts will appear here.</p>
            </div>
          ) : (
            <div className="space-y-3">
              {/* Workflow runs */}
              {recentWorkflowRuns.map((run: WorkflowRun) => {
                const statusColor = run.status === 'completed' ? 'text-green-600' : run.status === 'failed' ? 'text-red-600' : 'text-blue-600';
                const statusBg = run.status === 'completed' ? 'bg-green-100 dark:bg-green-900/30' : run.status === 'failed' ? 'bg-red-100 dark:bg-red-900/30' : 'bg-blue-100 dark:bg-blue-900/30';
                return (
                  <div key={run.id} className="flex items-start gap-3 p-3 rounded-lg border border-border/30">
                    <div className={`h-8 w-8 rounded-full ${statusBg} flex items-center justify-center shrink-0`}>
                      <Play className={`h-4 w-4 ${statusColor}`} />
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-2 flex-wrap">
                        <Badge variant="outline" className="text-[10px]">workflow run</Badge>
                        <Badge variant="secondary" className={`text-[10px] ${statusColor}`}>
                          {run.status}
                        </Badge>
                      </div>
                      <p className="text-sm mt-1 font-medium">{run.workflow_name}</p>
                      <div className="flex items-center gap-3 mt-1 text-xs text-muted-foreground">
                        {(run.total_records_staged ?? 0) > 0 && (
                          <span>{run.total_records_staged} staged</span>
                        )}
                        {(run.total_records_committed ?? 0) > 0 && (
                          <span className="text-green-600">{run.total_records_committed} committed</span>
                        )}
                        {(run.total_duplicates_found ?? 0) > 0 && (
                          <span className="text-amber-600">{run.total_duplicates_found} duplicates</span>
                        )}
                        {run.model_used && (
                          <span>{run.model_used}</span>
                        )}
                      </div>
                      <p className="text-[10px] text-muted-foreground mt-1">
                        {formatDate(run.created_at)}
                      </p>
                    </div>
                  </div>
                );
              })}
              {/* CRM activities */}
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
