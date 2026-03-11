import { useState } from 'react';
import { useParams, Link } from 'react-router-dom';
import { useQuery, useQueries } from '@tanstack/react-query';
import { organizationsApi, projectsApi } from '@/lib/api';
import { UserCircle, Users, Layers, ChevronDown, ChevronRight, LayoutGrid } from 'lucide-react';
import { Card } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Loader } from '@/components/ui/loader';
import { Button } from '@/components/ui/button';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible';
import { cn } from '@/lib/utils';
import NiceModal from '@ebay/nice-modal-react';
import '@/components/dialogs/shared/ConvertEntityDialog';
import { ClientMembersDialog } from '@/components/dialogs/client-members-dialog';

export function ClientOverview() {
  const { orgId, clientId } = useParams<{ orgId: string; clientId: string }>();
  const [membersOpen, setMembersOpen] = useState(false);
  const [expandedProjects, setExpandedProjects] = useState<Set<string>>(new Set());

  const { data: clients = [], isLoading } = useQuery({
    queryKey: ['orgClients', orgId],
    queryFn: () => organizationsApi.getClients(orgId!),
    enabled: !!orgId,
  });

  const { data: projects = [], isLoading: isProjectsLoading } = useQuery({
    queryKey: ['clientProjects', clientId],
    queryFn: () => projectsApi.getByClientId(clientId!),
    enabled: !!clientId,
  });

  // Fetch boards for all non-container projects
  const nonContainerProjects = projects.filter(
    (p: any) => p.git_repo_path && p.git_repo_path !== '' && !p.git_repo_path.startsWith('container:')
  );

  const boardQueries = useQueries({
    queries: nonContainerProjects.map((p: any) => ({
      queryKey: ['projectBoardsClientView', p.id],
      queryFn: () => projectsApi.listBoards(p.id),
      staleTime: 5 * 60 * 1000,
    })),
  });

  const client = clients.find((c: any) => c.id === clientId);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center min-h-[400px]">
        <Loader message="Loading client..." size={28} />
      </div>
    );
  }

  if (!client) {
    return (
      <div className="flex items-center justify-center min-h-[400px] text-muted-foreground">
        Client not found
      </div>
    );
  }

  const toggleProject = (projectId: string) => {
    setExpandedProjects(prev => {
      const key = `closed:${projectId}`;
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  return (
    <div className="p-6 max-w-5xl mx-auto space-y-6">
      {/* Header */}
      <div className="flex items-start justify-between">
        <div className="flex items-center gap-3">
          <UserCircle className="h-9 w-9 text-blue-500 shrink-0" />
          <div>
            <h1 className="text-2xl font-bold">{client.name}</h1>
            {client.description && (
              <p className="text-muted-foreground mt-0.5 text-sm">{client.description}</p>
            )}
            <div className="flex items-center gap-2 mt-1">
              <Badge variant={client.is_active ? 'default' : 'secondary'}>
                {client.is_active ? 'Active' : 'Inactive'}
              </Badge>
              <span className="text-xs text-muted-foreground font-mono">{client.slug}</span>
            </div>
          </div>
        </div>
        <div className="flex gap-2 shrink-0">
          <Button variant="outline" size="sm" onClick={() => setMembersOpen(true)}>
            <Users className="h-4 w-4 mr-2" />
            Members
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() =>
              NiceModal.show('convert-entity', {
                sourceType: 'client',
                sourceId: client.id,
                sourceName: client.name,
              })
            }
          >
            Convert to...
          </Button>
        </div>
      </div>

      {/* Projects */}
      <div>
        <div className="flex items-center gap-2 mb-3">
          <Layers className="h-4 w-4 text-muted-foreground" />
          <h2 className="text-sm font-semibold uppercase tracking-wider text-muted-foreground">
            Projects
          </h2>
          <span className="text-xs text-muted-foreground">({projects.length})</span>
        </div>

        {isProjectsLoading ? (
          <div className="flex items-center gap-2 text-sm text-muted-foreground py-4">
            <Loader size={16} />
            Loading projects...
          </div>
        ) : projects.length === 0 ? (
          <p className="text-sm text-muted-foreground py-4">No projects yet.</p>
        ) : (
          <div className="space-y-2">
            {projects.map((project: any, _idx: number) => {
              const isContainer = !project.git_repo_path || project.git_repo_path === '' || project.git_repo_path.startsWith('container:');
              const nonContainerIdx = nonContainerProjects.findIndex((p: any) => p.id === project.id);
              const boardQuery = !isContainer && nonContainerIdx >= 0 ? boardQueries[nonContainerIdx] : null;
              const boards = boardQuery?.data ?? [];
              // Default expanded — collapsed only if user explicitly closed it
              const isExpanded = !expandedProjects.has(`closed:${project.id}`);

              return (
                <Collapsible key={project.id} open={isExpanded} onOpenChange={() => toggleProject(project.id)}>
                  <Card className="overflow-hidden">
                    <div className="flex items-center justify-between px-4 py-3">
                      <Link
                        to={`/projects/${project.id}`}
                        className="flex items-center gap-2 flex-1 min-w-0 hover:text-primary transition-colors"
                        onClick={e => e.stopPropagation()}
                      >
                        <Layers className="h-4 w-4 text-muted-foreground shrink-0" />
                        <span className="font-medium text-sm truncate">{project.name}</span>
                      </Link>
                      <div className="flex items-center gap-2 shrink-0">
                        {!isContainer && (
                          <span className="text-xs text-muted-foreground">
                            {boardQuery?.isLoading ? '...' : `${boards.length} board${boards.length !== 1 ? 's' : ''}`}
                          </span>
                        )}
                        <CollapsibleTrigger asChild>
                          <button className="p-1 hover:bg-accent rounded-sm">
                            {isExpanded ? (
                              <ChevronDown className="h-3.5 w-3.5" />
                            ) : (
                              <ChevronRight className="h-3.5 w-3.5" />
                            )}
                          </button>
                        </CollapsibleTrigger>
                      </div>
                    </div>
                    <CollapsibleContent>
                      <div className={cn('px-4 pb-3 border-t border-border/50 pt-3')}>
                        {isContainer ? (
                          <p className="text-xs text-muted-foreground">Container project</p>
                        ) : boardQuery?.isLoading ? (
                          <div className="flex items-center gap-2 text-xs text-muted-foreground">
                            <Loader size={12} />
                            Loading boards...
                          </div>
                        ) : boards.length === 0 ? (
                          <p className="text-xs text-muted-foreground">No boards yet.</p>
                        ) : (
                          <div className="grid grid-cols-2 sm:grid-cols-3 gap-2">
                            {boards.map((board: any) => (
                              <Link
                                key={board.id}
                                to={`/projects/${project.id}/tasks?board=${board.id}`}
                                className="flex items-center gap-2 p-2 rounded-md border border-border/60 hover:bg-accent hover:border-border transition-colors text-xs"
                              >
                                <LayoutGrid className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                                <span className="truncate">{board.name}</span>
                              </Link>
                            ))}
                          </div>
                        )}
                        <div className="mt-2 flex gap-2">
                          <Link
                            to={`/projects/${project.id}/tasks`}
                            className="text-xs text-muted-foreground hover:text-foreground transition-colors"
                          >
                            Tasks →
                          </Link>
                          <Link
                            to={`/projects/${project.id}/pulse`}
                            className="text-xs text-muted-foreground hover:text-foreground transition-colors"
                          >
                            Pulse →
                          </Link>
                        </div>
                      </div>
                    </CollapsibleContent>
                  </Card>
                </Collapsible>
              );
            })}
          </div>
        )}
      </div>

      {clientId && (
        <ClientMembersDialog
          open={membersOpen}
          onOpenChange={setMembersOpen}
          clientId={clientId}
          clientName={client.name}
        />
      )}
    </div>
  );
}

export default ClientOverview;
