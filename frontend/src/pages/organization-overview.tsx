import { useParams, Link, useNavigate } from 'react-router-dom';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { organizationsApi } from '@/lib/api';
import { Building2, Users, FolderKanban, ArrowRight, Trash2 } from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Loader } from '@/components/ui/loader';
import { Button } from '@/components/ui/button';
import NiceModal from '@ebay/nice-modal-react';
import '@/components/dialogs/shared/ConvertEntityDialog';

export function OrganizationOverview() {
  const { orgId } = useParams<{ orgId: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const { data: org, isLoading } = useQuery({
    queryKey: ['organization', orgId],
    queryFn: () => organizationsApi.getById(orgId!),
    enabled: !!orgId,
  });

  const { data: clients = [] } = useQuery({
    queryKey: ['orgClients', orgId],
    queryFn: () => organizationsApi.getClients(orgId!),
    enabled: !!orgId,
  });

  const { data: sidebarTree } = useQuery({
    queryKey: ['sidebarTree'],
    queryFn: () => organizationsApi.getSidebarTree(),
    staleTime: 5 * 60 * 1000,
  });

  if (isLoading) {
    return (
      <div className="flex items-center justify-center min-h-[400px]">
        <Loader message="Loading organization..." size={28} />
      </div>
    );
  }

  if (!org) {
    return (
      <div className="flex items-center justify-center min-h-[400px] text-muted-foreground">
        Organization not found
      </div>
    );
  }

  // Collect projects from sidebar tree
  const sidebarOrg = sidebarTree
    ? [...(sidebarTree.owned_orgs || []), ...(sidebarTree.member_orgs || [])].find(
        (o) => o.id === orgId
      )
    : undefined;

  const flattenProjects = (projects: any[], clientName?: string): { id: string; name: string; clientName?: string }[] =>
    projects.flatMap((p: any) => [
      { id: p.id, name: p.name, clientName },
      ...flattenProjects(p.children || [], clientName),
    ]);

  const allProjects = sidebarOrg
    ? [
        ...flattenProjects(sidebarOrg.internal_projects),
        ...sidebarOrg.clients.flatMap((c: any) => flattenProjects(c.projects, c.name)),
      ]
    : [];
  const totalProjects = allProjects.length;

  return (
    <div className="p-6 max-w-4xl mx-auto space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Building2 className="h-8 w-8 text-primary" />
          <div>
            <h1 className="text-2xl font-bold">{org.name}</h1>
            {org.description && (
              <p className="text-muted-foreground mt-1">{org.description}</p>
            )}
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() =>
              NiceModal.show('convert-entity', {
                sourceType: 'organization',
                sourceId: org.id,
                sourceName: org.name,
              })
            }
          >
            Convert to...
          </Button>
          <Button
            variant="outline"
            size="sm"
            className="text-destructive hover:text-destructive"
            onClick={async () => {
              if (!window.confirm(`Delete "${org.name}"? This will deactivate the organization and hide it from the sidebar.`)) return;
              try {
                await organizationsApi.delete(org.id);
                queryClient.invalidateQueries({ queryKey: ['sidebarTree'] });
                navigate('/');
              } catch (err) {
                console.error('Failed to delete organization:', err);
              }
            }}
          >
            <Trash2 className="h-4 w-4 mr-1" />
            Delete
          </Button>
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Card
          className="cursor-pointer hover:shadow-md transition-shadow"
          onClick={() => document.getElementById('clients-section')?.scrollIntoView({ behavior: 'smooth' })}
        >
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Clients</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{clients.length}</div>
          </CardContent>
        </Card>
        <Card
          className="cursor-pointer hover:shadow-md transition-shadow"
          onClick={() => document.getElementById('projects-section')?.scrollIntoView({ behavior: 'smooth' })}
        >
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Projects</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{totalProjects}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Status</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold capitalize">
              {sidebarOrg?.health_status || 'healthy'}
            </div>
          </CardContent>
        </Card>
      </div>

      {clients.length > 0 && (
        <div id="clients-section">
          <h2 className="text-lg font-semibold mb-3 flex items-center gap-2">
            <Users className="h-5 w-5" />
            Clients
          </h2>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            {clients.map((client: any) => (
              <Card
                key={client.id}
                className="hover:shadow-md transition-shadow cursor-pointer"
                onClick={() => navigate(`/organizations/${orgId}/clients/${client.id}`)}
              >
                <CardHeader className="pb-2">
                  <div className="flex items-center justify-between">
                    <CardTitle className="text-base">{client.name}</CardTitle>
                    <ArrowRight className="h-4 w-4 text-muted-foreground" />
                  </div>
                  {client.description && (
                    <CardDescription>{client.description}</CardDescription>
                  )}
                </CardHeader>
              </Card>
            ))}
          </div>
        </div>
      )}

      {allProjects.length > 0 && (
        <div id="projects-section">
          <h2 className="text-lg font-semibold mb-3 flex items-center gap-2">
            <FolderKanban className="h-5 w-5" />
            Projects
          </h2>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            {allProjects.map((project) => (
              <Card
                key={project.id}
                className="hover:shadow-md transition-shadow cursor-pointer"
                onClick={() => navigate(`/projects/${project.id}/tasks`)}
              >
                <CardHeader className="pb-2">
                  <div className="flex items-center justify-between">
                    <div>
                      <CardTitle className="text-base">{project.name}</CardTitle>
                      {project.clientName && (
                        <CardDescription>{project.clientName}</CardDescription>
                      )}
                    </div>
                    <ArrowRight className="h-4 w-4 text-muted-foreground" />
                  </div>
                </CardHeader>
              </Card>
            ))}
          </div>
        </div>
      )}

      {/* Quick links */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
        <Link
          to={`/organizations/${orgId}/crm/acquisition`}
          className="flex items-center justify-between p-4 rounded-lg border hover:bg-accent transition-colors"
        >
          <span className="font-medium">Acquisition Pipeline</span>
          <ArrowRight className="h-4 w-4" />
        </Link>
        <Link
          to={`/organizations/${orgId}/crm/lifecycle`}
          className="flex items-center justify-between p-4 rounded-lg border hover:bg-accent transition-colors"
        >
          <span className="font-medium">Lifecycle Pipeline</span>
          <ArrowRight className="h-4 w-4" />
        </Link>
      </div>
    </div>
  );
}

export default OrganizationOverview;
