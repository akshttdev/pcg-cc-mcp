import { useNavigate } from 'react-router-dom';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { EmptyState } from '@/components/ui/empty-state';
import { Skeleton } from '@/components/ui/skeleton';
import { Building2, ExternalLink, FolderKanban } from 'lucide-react';
import { useDealClient } from '@/hooks/useCrmPipeline';
import type { CrmDealWithContact } from '@/types/crm';

interface ProjectsTabProps {
  deal: CrmDealWithContact;
  orgId?: string;
}

export function ProjectsTab({ deal, orgId }: ProjectsTabProps) {
  const navigate = useNavigate();
  const { client, projects, isLoading } = useDealClient(orgId, deal);

  if (!deal.contact_company) {
    return (
      <div className="p-5 text-center py-12">
        <FolderKanban className="h-8 w-8 mx-auto mb-2 text-muted-foreground/40" />
        <p className="text-sm text-muted-foreground">No company linked to this deal.</p>
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="p-5 space-y-3">
        <Skeleton className="h-4 w-32" />
        <Skeleton className="h-20 rounded-lg" />
        <Skeleton className="h-20 rounded-lg" />
      </div>
    );
  }

  if (!client) {
    return (
      <EmptyState
        icon={FolderKanban}
        title={`No client found for "${deal.contact_company}"`}
        className="p-5"
      />
    );
  }

  if (projects.length === 0) {
    return (
      <EmptyState
        icon={FolderKanban}
        title={`No projects for ${client.name} yet`}
        className="p-5"
      />
    );
  }

  return (
    <div className="p-5 space-y-3">
      <div className="flex items-center gap-2 text-xs text-muted-foreground">
        <Building2 className="h-3.5 w-3.5" />
        <span className="font-medium">{client.name}</span>
        <Badge variant="secondary" className="text-[10px]">
          {projects.length} project{projects.length !== 1 ? 's' : ''}
        </Badge>
      </div>
      <div className="space-y-2">
        {projects.map((project) => (
          <Card key={project.id} className="hover:shadow-sm transition-shadow border-border/60">
            <CardContent className="p-3">
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2 min-w-0">
                  <FolderKanban className="h-4 w-4 text-primary shrink-0" />
                  <p className="text-sm font-medium truncate">{project.name}</p>
                </div>
                <Button
                  variant="ghost"
                  size="sm"
                  className="shrink-0 h-7 text-xs gap-1"
                  onClick={() => navigate(`/projects/${project.id}/tasks`)}
                >
                  Open <ExternalLink className="h-3 w-3" />
                </Button>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}
