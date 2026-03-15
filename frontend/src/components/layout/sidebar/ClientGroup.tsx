import { useEffect, useState } from 'react';
import { Link, useLocation } from 'react-router-dom';
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@/components/ui/collapsible';
import {
  ChevronDown,
  ChevronRight,
  Plus,
  Users,
  UserCircle,
  Package,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { QueryClient } from '@tanstack/react-query';
import type { SidebarClient as SidebarClientType } from '@/lib/api';
import NiceModal from '@ebay/nice-modal-react';
import type { ProjectFormDialogResult } from '@/components/dialogs';
import { HealthDot } from './HealthDot';
import { countProjects, isProjectInTree } from './helpers';
import { SortableProjectList } from './ProjectList';

// ============================================================================
// ClientGroup — renders a client with expand/collapse and project list
// ============================================================================

export function ClientGroup({
  client,
  organizationId,
  projectId,
  expandedProjects,
  onToggleProject,
  queryClient,
}: {
  client: SidebarClientType;
  organizationId: string;
  projectId?: string;
  expandedProjects: Set<string>;
  onToggleProject: (id: string) => void;
  queryClient?: QueryClient;
}) {
  const location = useLocation();
  const hasActiveProject = isProjectInTree(client.projects, projectId || '');
  const storageKey = `sidebar:client:${client.id}:expanded`;
  const [expanded, setExpanded] = useState<boolean>(() => {
    if (hasActiveProject) return true;
    const stored = localStorage.getItem(storageKey);
    return stored !== null ? stored === 'true' : false;
  });
  const handleSetExpanded = (next: boolean) => {
    setExpanded(next);
    localStorage.setItem(storageKey, String(next));
  };

  useEffect(() => {
    if (hasActiveProject && !expanded) {
      handleSetExpanded(true);
    }
  }, [hasActiveProject]);

  return (
    <Collapsible open={expanded} onOpenChange={handleSetExpanded}>
      {/* Header row: chevron toggles expand/collapse, name navigates to client page */}
      <div
        className={cn(
          'flex items-center gap-1.5 px-2 py-1.5 rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors group/client text-xs',
          hasActiveProject && 'bg-primary/10 text-foreground font-medium'
        )}
      >
        <CollapsibleTrigger asChild>
          <button className="shrink-0 p-0 hover:text-foreground" onClick={(e) => e.stopPropagation()}>
            {expanded ? (
              <ChevronDown className="h-3.5 w-3.5 text-muted-foreground" />
            ) : (
              <ChevronRight className="h-3.5 w-3.5 text-muted-foreground" />
            )}
          </button>
        </CollapsibleTrigger>
        <HealthDot status={client.health_status} />
        <UserCircle className="h-3.5 w-3.5 text-primary shrink-0" />
        <Link
          to={`/organizations/${organizationId}/clients/${client.id}`}
          className="truncate flex-1 font-normal hover:underline"
        >
          {client.name}
        </Link>
        <div className="flex items-center gap-1 shrink-0">
          {client.active_issues_count != null && client.active_issues_count > 0 && (
            <span className="text-[9px] px-1 py-0.5 rounded bg-destructive/10 text-destructive">
              {client.active_issues_count}
            </span>
          )}
          <CollapsibleTrigger asChild>
            <button className="shrink-0 p-0">
              <span className="text-[10px] text-muted-foreground">
                {countProjects(client.projects)}
              </span>
            </button>
          </CollapsibleTrigger>
        </div>
      </div>
      <CollapsibleContent className="pl-4">
        <div className="space-y-0.5 py-0.5">
          <SortableProjectList
            scopeKey={`client:${client.id}`}
            projects={client.projects}
            projectId={projectId}
            expandedProjects={expandedProjects}
            onToggleProject={onToggleProject}
            queryClient={queryClient}
          />

          {/* Client context quick links */}
          <div className="pt-1 mt-1 border-t border-border/40 space-y-0.5">
            <Link
              to={`/organizations/${organizationId}/clients/${client.id}`}
              className={cn(
                'flex items-center gap-1.5 px-2 py-1 text-[10px] rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors text-muted-foreground',
                location.pathname === `/organizations/${organizationId}/clients/${client.id}` && 'bg-primary/10 text-foreground font-medium'
              )}
            >
              <UserCircle className="h-3 w-3 shrink-0" />
              <span>Client Overview</span>
            </Link>
            {client.crm_person_id && (
              <Link
                to={`/people/${client.crm_person_id}`}
                className={cn(
                  'flex items-center gap-1.5 px-2 py-1 text-[10px] rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors text-muted-foreground',
                  location.pathname === `/people/${client.crm_person_id}` && 'bg-primary/10 text-foreground font-medium'
                )}
              >
                <Users className="h-3 w-3 shrink-0" />
                <span>CRM Profile</span>
              </Link>
            )}
            <Link
              to={`/organizations/${organizationId}/crm/pipeline?client=${client.id}`}
              className={cn(
                'flex items-center gap-1.5 px-2 py-1 text-[10px] rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors text-muted-foreground',
              )}
            >
              <Package className="h-3 w-3 shrink-0" />
              <span>Pipeline</span>
            </Link>
            <button
              className="flex items-center gap-1.5 px-2 py-1 text-[10px] rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors text-muted-foreground w-full text-left opacity-0 group-hover/client:opacity-100"
              onClick={async (e) => {
                e.stopPropagation();
                try {
                  const result = await NiceModal.show('project-form', {
                    organization_id: organizationId,
                    client_id: client.id,
                  }) as ProjectFormDialogResult;
                  if (result === 'saved') {
                    queryClient?.invalidateQueries({ queryKey: ['sidebarTree'] });
                  }
                } catch {
                  // dialog dismissed
                }
              }}
            >
              <Plus className="h-3 w-3 shrink-0" />
              <span>Add Project</span>
            </button>
          </div>
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
}
