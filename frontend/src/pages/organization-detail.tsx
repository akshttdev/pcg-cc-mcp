import { useState } from 'react';
import { useParams, Link, useNavigate } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Input } from '@/components/ui/input';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  Building2,
  Users,
  FolderOpen,
  Briefcase,
  TrendingUp,
  ArrowLeft,
  Globe,
  ExternalLink,
  BookOpen,
  Calendar,
  ChevronDown,
  UserCheck,
  Search,
  ArrowRight,
  Mail,
  UserCircle,
} from 'lucide-react';
import { organizationsApi, personsApi, type OrganizationData, type ClientData, type PersonRecord } from '@/lib/api';
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

const PERSON_TYPE_INFO: Record<string, { label: string; color: string }> = {
  team:       { label: 'Team',       color: 'bg-violet-100 text-violet-700' },
  client:     { label: 'Client',     color: 'bg-green-100 text-green-700' },
  contractor: { label: 'Contractor', color: 'bg-amber-100 text-amber-700' },
  lead:       { label: 'Lead',       color: 'bg-blue-100 text-blue-700' },
  partner:    { label: 'Partner',    color: 'bg-pink-100 text-pink-700' },
  contact:    { label: 'Contact',    color: 'bg-gray-100 text-gray-600' },
};

const CONFIDENCE_COLOR = (c: number) =>
  c >= 0.7 ? 'text-green-600' : c >= 0.4 ? 'text-amber-500' : 'text-red-500';

// ── Sub-components ────────────────────────────────────────────────────────────

function OrgHeader({ org, orgId }: { org: OrganizationData; orgId: string }) {
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

          {/* CRM dropdown — replaces the old Acquisition / Lifecycle buttons */}
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="outline" size="sm">
                <UserCheck className="h-4 w-4 mr-1.5" />
                CRM
                <ChevronDown className="h-3.5 w-3.5 ml-1" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-48">
              <DropdownMenuItem asChild>
                <Link to={`/organizations/${orgId}?tab=crm`} className="flex items-center gap-2">
                  <Users className="h-4 w-4" />
                  People &amp; Leads
                </Link>
              </DropdownMenuItem>
              <DropdownMenuItem asChild>
                <Link to={`/organizations/${orgId}/crm/acquisition`} className="flex items-center gap-2">
                  <TrendingUp className="h-4 w-4" />
                  Acquisition Pipeline
                </Link>
              </DropdownMenuItem>
              <DropdownMenuItem asChild>
                <Link to={`/organizations/${orgId}/crm/lifecycle`} className="flex items-center gap-2">
                  <BookOpen className="h-4 w-4" />
                  Client Lifecycle
                </Link>
              </DropdownMenuItem>
              <DropdownMenuItem asChild>
                <Link to="/proposals" className="flex items-center gap-2">
                  <Briefcase className="h-4 w-4" />
                  Proposals
                </Link>
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
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

function PersonRow({ person }: { person: PersonRecord }) {
  const navigate = useNavigate();
  const typeInfo = PERSON_TYPE_INFO[person.person_type] ?? PERSON_TYPE_INFO.contact;
  const initials = person.full_name.split(' ').map(n => n[0]).slice(0, 2).join('').toUpperCase();

  return (
    <div
      className="flex items-center gap-3 p-3 border rounded-lg hover:bg-muted/40 cursor-pointer transition-colors group"
      onClick={() => navigate(`/people/${person.id}`)}
    >
      <div className="w-8 h-8 rounded-full bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center text-white text-xs font-semibold shrink-0">
        {initials}
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 flex-wrap">
          <span className="text-sm font-medium truncate">{person.full_name}</span>
          <Badge className={`text-xs px-1.5 py-0 ${typeInfo.color} border-0`}>{typeInfo.label}</Badge>
          {person.intelligence_confidence > 0 && (
            <span className={`text-xs font-medium ${CONFIDENCE_COLOR(person.intelligence_confidence)}`}>
              {Math.round(person.intelligence_confidence * 100)}%
            </span>
          )}
        </div>
        <div className="flex items-center gap-3 mt-0.5 text-xs text-muted-foreground">
          {person.company_name && (
            <span className="flex items-center gap-1 truncate">
              <Building2 className="h-3 w-3" />
              {person.company_name}
            </span>
          )}
          {person.email && (
            <span className="flex items-center gap-1 truncate">
              <Mail className="h-3 w-3" />
              {person.email}
            </span>
          )}
        </div>
        {person.intelligence_summary && (
          <p className="text-xs text-muted-foreground mt-0.5 line-clamp-1 italic">
            {person.intelligence_summary}
          </p>
        )}
      </div>
      <ArrowRight className="h-4 w-4 text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity shrink-0" />
    </div>
  );
}

// ── CRM Tab ───────────────────────────────────────────────────────────────────

function OrgCrmTab({ orgId, orgName }: { orgId: string; orgName: string }) {
  const navigate = useNavigate();
  const [search, setSearch] = useState('');
  const [activeType, setActiveType] = useState<string | undefined>(undefined);

  const { data: persons = [], isLoading } = useQuery<PersonRecord[]>({
    queryKey: ['org-persons', orgId, activeType, search],
    queryFn: () => personsApi.list({ organization_id: orgId, person_type: activeType, q: search || undefined, limit: 200 }),
    enabled: !!orgId,
  });

  const counts = persons.reduce<Record<string, number>>((acc, p) => {
    acc[p.person_type] = (acc[p.person_type] ?? 0) + 1;
    return acc;
  }, {});

  const TYPE_FILTERS = [
    { key: undefined, label: 'All' },
    { key: 'lead',   label: 'Leads' },
    { key: 'client', label: 'Clients' },
    { key: 'partner', label: 'Partners' },
    { key: 'contractor', label: 'Contractors' },
  ];

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm text-muted-foreground">
            {persons.length} {activeType ?? 'people'} in {orgName}
          </p>
        </div>
        <Button size="sm" onClick={() => navigate('/people/new')}>
          + Add Person
        </Button>
      </div>

      {/* Filter pills */}
      <div className="flex items-center gap-2 flex-wrap">
        {TYPE_FILTERS.map(f => {
          const count = f.key ? (counts[f.key] ?? 0) : persons.length;
          const active = activeType === f.key;
          return (
            <button
              key={f.key ?? 'all'}
              onClick={() => setActiveType(f.key)}
              className={`flex items-center gap-1.5 px-3 py-1.5 rounded-full text-sm border transition-colors ${
                active ? 'bg-primary text-primary-foreground border-primary' : 'border-border hover:bg-muted/60'
              }`}
            >
              {f.label}
              <span className={`text-xs ${active ? 'opacity-80' : 'text-muted-foreground'}`}>{count}</span>
            </button>
          );
        })}
      </div>

      {/* Search */}
      <div className="relative">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
        <Input
          placeholder="Search by name, company..."
          className="pl-9"
          value={search}
          onChange={e => setSearch(e.target.value)}
        />
      </div>

      {/* List */}
      {isLoading ? (
        <div className="space-y-2">
          {[...Array(5)].map((_, i) => (
            <div key={i} className="h-14 bg-muted rounded-lg animate-pulse" />
          ))}
        </div>
      ) : persons.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-16 text-muted-foreground">
          <UserCircle className="h-12 w-12 mb-3 opacity-30" />
          <p className="text-sm">No people found for {orgName}</p>
          <Button variant="outline" size="sm" className="mt-4" onClick={() => navigate('/people/new')}>
            Add first contact
          </Button>
        </div>
      ) : (
        <div className="space-y-2">
          {persons.map(p => <PersonRow key={p.id} person={p} />)}
        </div>
      )}
    </div>
  );
}

// ── Main Page ─────────────────────────────────────────────────────────────────

export function OrganizationDetailPage() {
  const { orgId } = useParams<{ orgId: string }>();

  // Support ?tab=crm deep-link from the CRM dropdown
  const defaultTab = new URLSearchParams(window.location.search).get('tab') ?? 'projects';

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
      <OrgHeader org={org} orgId={orgId!} />

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

        <Tabs defaultValue={defaultTab} className="space-y-4">
          <TabsList>
            <TabsTrigger value="projects">
              <FolderOpen className="h-4 w-4 mr-2" />
              Projects
            </TabsTrigger>
            <TabsTrigger value="crm">
              <UserCheck className="h-4 w-4 mr-2" />
              CRM
            </TabsTrigger>
            <TabsTrigger value="clients">
              <Briefcase className="h-4 w-4 mr-2" />
              Clients
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

          {/* Projects Tab */}
          <TabsContent value="projects">
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
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
                          <Link key={p.id} to={`/projects/${p.id}`}
                            className="flex items-center justify-between p-2 rounded-md hover:bg-muted transition-colors">
                            <div className="flex items-center gap-2">
                              <FolderOpen className="h-4 w-4 text-muted-foreground" />
                              <span className="text-sm font-medium">{p.name}</span>
                            </div>
                            <ExternalLink className="h-3 w-3 text-muted-foreground" />
                          </Link>
                        ))}
                        {(sidebarOrg.internal_folders || []).map(folder =>
                          folder.projects.map(p => (
                            <Link key={p.id} to={`/projects/${p.id}`}
                              className="flex items-center justify-between p-2 rounded-md hover:bg-muted transition-colors">
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
                          <Link key={p.id} to={`/projects/${p.id}`}
                            className="flex items-center justify-between p-2 rounded-md hover:bg-muted transition-colors">
                            <div className="flex items-center gap-2">
                              <FolderOpen className="h-4 w-4 text-muted-foreground" />
                              <span className="text-sm font-medium">{p.name}</span>
                            </div>
                            <ExternalLink className="h-3 w-3 text-muted-foreground" />
                          </Link>
                        ))}
                        {(client.folders || []).map(folder =>
                          folder.projects.map(p => (
                            <Link key={p.id} to={`/projects/${p.id}`}
                              className="flex items-center justify-between p-2 rounded-md hover:bg-muted transition-colors">
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

          {/* CRM Tab */}
          <TabsContent value="crm">
            <OrgCrmTab orgId={orgId!} orgName={org.name} />
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
                            <Link to={`/projects/${p.id}/knowledge`}
                              className="text-xs text-blue-600 hover:underline shrink-0 ml-2">
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
                      <TrendingUp className="h-4 w-4 mr-2 text-blue-600" />
                      Acquisition Pipeline
                    </Link>
                  </Button>
                  <Button variant="outline" className="w-full justify-start" asChild>
                    <Link to={`/organizations/${orgId}/crm/lifecycle`}>
                      <BookOpen className="h-4 w-4 mr-2 text-green-600" />
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
