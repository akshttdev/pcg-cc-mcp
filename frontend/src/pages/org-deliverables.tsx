import { useParams, useNavigate } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import {
  Package,
  FileText,
  Code,
  Music,
  Video,
  Image,
  BookOpen,
  ArrowRight,
} from 'lucide-react';
import { deliverablesApi, projectsApi, type DeliverableRecord } from '@/lib/api';

const TYPE_ICONS: Record<string, React.ElementType> = {
  video: Video,
  audio: Music,
  graphic: Image,
  copy: FileText,
  code: Code,
  document: BookOpen,
  other: Package,
};

const STATUS_COLORS: Record<string, string> = {
  working: 'bg-gray-100 text-gray-600',
  internal_review: 'bg-blue-100 text-blue-700',
  client_review: 'bg-purple-100 text-purple-700',
  revision: 'bg-orange-100 text-orange-700',
  client_revision: 'bg-amber-100 text-amber-700',
  done: 'bg-green-100 text-green-700',
};

const STATUS_LABELS: Record<string, string> = {
  working: 'Working',
  internal_review: 'Internal Review',
  client_review: 'Client Review',
  revision: 'Revision',
  client_revision: 'Client Revision',
  done: 'Done',
};

export function OrgDeliverablesPage() {
  const { orgId } = useParams<{ orgId: string }>();
  const navigate = useNavigate();

  const { data: projects = [] } = useQuery({
    queryKey: ['orgProjects', orgId],
    queryFn: () => projectsApi.getAll(),
    enabled: !!orgId,
  });

  const orgProjects = projects.filter(
    (p) => (p as any).organization_id === orgId,
  );

  const { data: allDeliverables = [], isLoading } = useQuery({
    queryKey: ['orgDeliverables', orgId, orgProjects.map((p) => p.id)],
    queryFn: async () => {
      const results = await Promise.all(
        orgProjects.map((p) => deliverablesApi.listForProject(p.id)),
      );
      return results
        .flat()
        .sort(
          (a: DeliverableRecord, b: DeliverableRecord) =>
            new Date(b.updated_at || b.created_at).getTime() -
            new Date(a.updated_at || a.created_at).getTime(),
        );
    },
    enabled: orgProjects.length > 0,
  });

  const projectMap = Object.fromEntries(orgProjects.map((p) => [p.id, p.name]));

  const byStatus = allDeliverables.reduce(
    (acc: Record<string, DeliverableRecord[]>, d: DeliverableRecord) => {
      const s = d.status || 'working';
      if (!acc[s]) acc[s] = [];
      acc[s].push(d);
      return acc;
    },
    {} as Record<string, DeliverableRecord[]>,
  );

  if (isLoading) {
    return (
      <div className="p-6 space-y-4 max-w-5xl mx-auto">
        <Skeleton className="h-10 w-64" />
        <div className="space-y-3">
          {[...Array(4)].map((_, i) => (
            <Skeleton key={i} className="h-16 w-full rounded-lg" />
          ))}
        </div>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6 max-w-5xl mx-auto">
      <div>
        <h1 className="text-2xl font-semibold flex items-center gap-2">
          <Package className="h-6 w-6 text-muted-foreground" />
          Deliverables
        </h1>
        <p className="text-sm text-muted-foreground mt-1">
          {allDeliverables.length} deliverable
          {allDeliverables.length !== 1 ? 's' : ''} across{' '}
          {orgProjects.length} project
          {orgProjects.length !== 1 ? 's' : ''}
        </p>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-3">
        {Object.entries(STATUS_LABELS).map(([key, label]) => (
          <div
            key={key}
            className="border rounded-lg px-3 py-2 text-center"
          >
            <p className="text-xs text-muted-foreground">{label}</p>
            <p className="text-lg font-semibold">
              {(byStatus[key] || []).length}
            </p>
          </div>
        ))}
      </div>

      {/* List */}
      {allDeliverables.length === 0 ? (
        <div className="text-center py-12 text-muted-foreground">
          <Package className="h-10 w-10 mx-auto mb-3 opacity-40" />
          <p>No deliverables yet</p>
          <p className="text-xs mt-1">
            Create deliverables from individual project pages
          </p>
        </div>
      ) : (
        <div className="border rounded-xl overflow-hidden divide-y">
          {allDeliverables.map((d: DeliverableRecord) => {
            const Icon =
              TYPE_ICONS[(d as any).deliverable_type || 'other'] || Package;
            const statusCls =
              STATUS_COLORS[d.status || 'working'] || STATUS_COLORS.working;
            return (
              <div
                key={d.id}
                className="flex items-center gap-3 px-4 py-3 hover:bg-muted/40 cursor-pointer transition-colors"
                onClick={() =>
                  navigate(`/projects/${d.project_id}/deliverables`)
                }
              >
                <Icon className="h-4 w-4 text-muted-foreground shrink-0" />
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium truncate">{d.title}</p>
                  <p className="text-xs text-muted-foreground truncate">
                    {projectMap[d.project_id] || 'Unknown project'}
                    {d.due_date &&
                      ` · Due ${new Date(d.due_date).toLocaleDateString('en-US', { month: 'short', day: 'numeric' })}`}
                  </p>
                </div>
                <Badge className={`text-xs border-0 shrink-0 ${statusCls}`}>
                  {STATUS_LABELS[d.status || 'working'] || d.status}
                </Badge>
                <ArrowRight className="h-4 w-4 text-muted-foreground shrink-0" />
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
