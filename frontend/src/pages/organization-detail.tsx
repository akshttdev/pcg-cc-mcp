import { useParams, Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
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
  BookOpen,
  Calendar,
} from 'lucide-react';
import { organizationsApi, type OrganizationData, type ClientData } from '@/lib/api';
import { organizationsApi as orgApi } from '@/lib/api';

// ── Types ─────────────────────────────────────────────────────────────────────

interface OrgMember {
  id: string;
  user_id: string;
  role: string;
  joined_at: string;
  user?: { username: string; full_name: string; email: string; avatar_url?: string };
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function formatDate(iso: string) {
  return new Date(iso).toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
}

// ── Sub-components ────────────────────────────────────────────────────────────

function OrgHeader({ org }: { org: OrganizationData }) {
  return (
    <div className="border-b bg-white dark:bg-gray-900 shadow-sm">
      <div className="flex items-center gap-4 p-6">
        <Link to="/projects" className="text-muted-foreground hover:text-foreground">
          <ArrowLeft className="h-5 w-5" />
        </Link>
        <div className="flex items-center gap-3">
          <div className="p-2 bg-blue-100 dark:bg-blue-950 rounded-lg">
            <Building2 className="w-6 h-6 text-blue-600" />
          </div>
          <div>
            <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100">{org.name}</h1>
            {org.description && (
              <p className="text-sm text-gray-500 dark:text-gray-400 mt-0.5">{org.description}</p>
            )}
          </div>
        </div>
        <div className="ml-auto flex items-center gap-2">
          <Badge variant={org.is_active ? 'default' : 'secondary'}>
            {org.is_active ? 'Active' : 'Inactive'}
          </Badge>
          <Button variant="outline" size="sm" asChild>
            <Link to={`/organizations/${org.id}/crm/acquisition`}>
              <Target className="h-4 w-4 mr-1" />
              Acquisition
            </Link>
          </Button>
          <Button variant="outline" size="sm" asChild>
            <Link to={`/organizations/${org.id}/crm/lifecycle`}>
              <TrendingUp className="h-4 w-4 mr-1" />
              Lifecycle
            </Link>
          </Button>
        </div>
      </div>
    </div>
  );
}

function ClientCard({ client }: { client: ClientData }) {
  return (
    <div className="flex items-center justify-between p-3 border rounded-lg hover:bg-muted/50 transition-colors">
      <div className="flex items-center gap-3">
        <div className="p-1.5 bg-purple-100 dark:bg-purple-950 rounded">
          <Briefcase className="h-4 w-4 text-purple-600" />
        </div>
        <div>
          <div className="font-medium text-sm">{client.name}</div>
          {client.website && (
            <a href={client.website} target="_blank" rel="noopener noreferrer"
              className="text-xs text-muted-foreground hover:text-foreground flex items-center gap-1 mt-0.5">
              <Globe className="h-3 w-3" />
              {client.website.replace(/^https?:\/\//, '')}
            </a>
          )}
        </div>
      </div>
      <Badge variant="outline" className="text-xs">
        {client.is_active ? 'Active' : 'Inactive'}
      </Badge>
    </div>
  );
}

// ── Main Page ─────────────────────────────────────────────────────────────────

export function OrganizationDetailPage() {
  const { orgId } = useParams<{ orgId: string }>();

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
    queryFn: () => orgApi.getClients(orgId!),
    enabled: !!orgId,
  });

  // Fetch sidebar tree to get projects grouped under this org
  const { data: sidebarTree } = useQuery({
    queryKey: ['sidebarTree'],
    queryFn: () => organizationsApi.getSidebarTree(),
  });

  const sidebarOrg = sidebarTree
    ? [...(sidebarTree.owned_orgs || []), ...(sidebarTree.member_orgs || [])].find(o => o.id === orgId)
    : undefined;

  if (orgLoading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-muted-foreground">Loading organisation...</div>
      </div>
    );
  }

  if (!org) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-muted-foreground">Organisation not found.</div>
      </div>
    );
  }

  // Flatten all projects from the sidebar tree entry
  const allProjects = sidebarOrg
    ? [
        ...(sidebarOrg.internal_projects || []),
        ...(sidebarOrg.internal_folders || []).flatMap(f => f.projects),
        ...(sidebarOrg.clients || []).flatMap(c => [
          ...(c.projects || []),
          ...(c.folders || []).flatMap(f => f.projects),
        ]),
      ]
    : [];

  return (
    <div className="flex flex-col h-full bg-background">
      <OrgHeader org={org} />

      <div className="flex-1 p-6 overflow-auto">
        {/* Stats row */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-6">
          <Card>
            <CardContent className="pt-4 pb-3">
              <div className="flex items-center gap-2">
                <FolderOpen className="h-4 w-4 text-muted-foreground" />
                <span className="text-sm text-muted-foreground">Projects</span>
              </div>
              <p className="text-2xl font-bold mt-1">{allProjects.length}</p>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="pt-4 pb-3">
              <div className="flex items-center gap-2">
                <Briefcase className="h-4 w-4 text-muted-foreground" />
                <span className="text-sm text-muted-foreground">Clients</span>
              </div>
              <p className="text-2xl font-bold mt-1">{clients.length}</p>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="pt-4 pb-3">
              <div className="flex items-center gap-2">
                <Users className="h-4 w-4 text-muted-foreground" />
                <span className="text-sm text-muted-foreground">Members</span>
              </div>
              <p className="text-2xl font-bold mt-1">{members.length}</p>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="pt-4 pb-3">
              <div className="flex items-center gap-2">
                <Calendar className="h-4 w-4 text-muted-foreground" />
                <span className="text-sm text-muted-foreground">Created</span>
              </div>
              <p className="text-sm font-medium mt-1">{formatDate(org.created_at)}</p>
            </CardContent>
          </Card>
        </div>

        <Tabs defaultValue="projects" className="space-y-4">
          <TabsList>
            <TabsTrigger value="projects">
              <FolderOpen className="h-4 w-4 mr-2" />
              Projects
            </TabsTrigger>
            <TabsTrigger value="clients">
              <Briefcase className="h-4 w-4 mr-2" />
              Clients
            </TabsTrigger>
            <TabsTrigger value="knowledge">
              <BookOpen className="h-4 w-4 mr-2" />
              Knowledge Graph
            </TabsTrigger>
            <TabsTrigger value="members">
              <Users className="h-4 w-4 mr-2" />
              Members
            </TabsTrigger>
          </TabsList>

          {/* Projects Tab */}
          <TabsContent value="projects">
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
              {/* Internal projects */}
              {sidebarOrg && (
                <Card>
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
                        {sidebarOrg.internal_projects?.map(p => (
                          <Link
                            key={p.id}
                            to={`/projects/${p.id}`}
                            className="flex items-center justify-between p-2 rounded-md hover:bg-muted transition-colors"
                          >
                            <div className="flex items-center gap-2">
                              <FolderOpen className="h-4 w-4 text-muted-foreground" />
                              <span className="text-sm font-medium">{p.name}</span>
                            </div>
                            <ExternalLink className="h-3 w-3 text-muted-foreground" />
                          </Link>
                        ))}
                        {(sidebarOrg.internal_folders || []).map(folder =>
                          folder.projects.map(p => (
                            <Link
                              key={p.id}
                              to={`/projects/${p.id}`}
                              className="flex items-center justify-between p-2 rounded-md hover:bg-muted transition-colors"
                            >
                              <div className="flex items-center gap-2 min-w-0">
                                <FolderOpen className="h-4 w-4 text-muted-foreground shrink-0" />
                                <span className="text-sm font-medium truncate">{p.name}</span>
                                <Badge variant="secondary" className="text-xs shrink-0">{folder.name}</Badge>
                              </div>
                              <ExternalLink className="h-3 w-3 text-muted-foreground shrink-0" />
                            </Link>
                          ))
                        )}
                      </div>
                    )}
                  </CardContent>
                </Card>
              )}

              {/* Client projects */}
              {sidebarOrg?.clients?.map(client => (
                <Card key={client.id}>
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
                        {client.projects?.map(p => (
                          <Link
                            key={p.id}
                            to={`/projects/${p.id}`}
                            className="flex items-center justify-between p-2 rounded-md hover:bg-muted transition-colors"
                          >
                            <div className="flex items-center gap-2">
                              <FolderOpen className="h-4 w-4 text-muted-foreground" />
                              <span className="text-sm font-medium">{p.name}</span>
                            </div>
                            <ExternalLink className="h-3 w-3 text-muted-foreground" />
                          </Link>
                        ))}
                        {(client.folders || []).map(folder =>
                          folder.projects.map(p => (
                            <Link
                              key={p.id}
                              to={`/projects/${p.id}`}
                              className="flex items-center justify-between p-2 rounded-md hover:bg-muted transition-colors"
                            >
                              <div className="flex items-center gap-2 min-w-0">
                                <FolderOpen className="h-4 w-4 text-muted-foreground shrink-0" />
                                <span className="text-sm font-medium truncate">{p.name}</span>
                                <Badge variant="secondary" className="text-xs shrink-0">{folder.name}</Badge>
                              </div>
                              <ExternalLink className="h-3 w-3 text-muted-foreground shrink-0" />
                            </Link>
                          ))
                        )}
                      </div>
                    )}
                  </CardContent>
                </Card>
              ))}

              {!sidebarOrg && (
                <div className="col-span-2 text-center text-muted-foreground py-8">
                  <FolderOpen className="h-8 w-8 mx-auto mb-2 opacity-40" />
                  <p>No projects found</p>
                </div>
              )}
            </div>
          </TabsContent>

          {/* Clients Tab */}
          <TabsContent value="clients">
            <Card>
              <CardHeader>
                <CardTitle>Clients</CardTitle>
                <CardDescription>All clients under {org.name}</CardDescription>
              </CardHeader>
              <CardContent>
                {clients.length === 0 ? (
                  <div className="text-center py-8 text-muted-foreground">
                    <Briefcase className="h-8 w-8 mx-auto mb-2 opacity-40" />
                    <p>No clients yet</p>
                  </div>
                ) : (
                  <div className="space-y-2">
                    {clients.map(c => <ClientCard key={c.id} client={c} />)}
                  </div>
                )}
              </CardContent>
            </Card>
          </TabsContent>

          {/* Knowledge Graph Tab */}
          <TabsContent value="knowledge">
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
              <Card>
                <CardHeader>
                  <CardTitle className="text-base">Project Knowledge Sources</CardTitle>
                  <CardDescription>
                    Knowledge sources indexed across all {org.name} projects
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  {allProjects.length === 0 ? (
                    <p className="text-sm text-muted-foreground text-center py-4">No projects</p>
                  ) : (
                    <ScrollArea className="h-64">
                      <div className="space-y-2">
                        {allProjects.map(p => (
                          <div key={p.id} className="flex items-center justify-between p-2 rounded-md border">
                            <div className="flex items-center gap-2 min-w-0">
                              <BookOpen className="h-4 w-4 text-muted-foreground shrink-0" />
                              <span className="text-sm truncate">{p.name}</span>
                            </div>
                            <Link
                              to={`/projects/${p.id}/knowledge`}
                              className="text-xs text-blue-600 hover:underline shrink-0 ml-2"
                            >
                              View
                            </Link>
                          </div>
                        ))}
                      </div>
                    </ScrollArea>
                  )}
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle className="text-base">CRM Intelligence</CardTitle>
                  <CardDescription>Pipeline health and deal activity</CardDescription>
                </CardHeader>
                <CardContent className="space-y-3">
                  <Button variant="outline" className="w-full justify-start" asChild>
                    <Link to={`/organizations/${orgId}/crm/acquisition`}>
                      <Target className="h-4 w-4 mr-2 text-blue-600" />
                      Acquisition Pipeline
                    </Link>
                  </Button>
                  <Button variant="outline" className="w-full justify-start" asChild>
                    <Link to={`/organizations/${orgId}/crm/lifecycle`}>
                      <TrendingUp className="h-4 w-4 mr-2 text-green-600" />
                      Client Lifecycle Pipeline
                    </Link>
                  </Button>
                </CardContent>
              </Card>
            </div>
          </TabsContent>

          {/* Members Tab */}
          <TabsContent value="members">
            <Card>
              <CardHeader>
                <CardTitle>Team Members</CardTitle>
                <CardDescription>People with access to {org.name}</CardDescription>
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
          </TabsContent>
        </Tabs>
      </div>
    </div>
  );
}
