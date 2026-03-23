import { useQuery, useQueryClient } from '@tanstack/react-query';
import {
  Clock,
  Mail,
  Search,
  TrendingUp,
  UserPlus,
  Users,
} from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import type { Project } from 'shared/types';

import { EmailAccountConnect } from '@/components/email/EmailAccountConnect';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { EmptyState } from '@/components/ui/empty-state';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Skeleton } from '@/components/ui/skeleton';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useMutationWithToast } from '@/hooks/useMutationWithToast';
import {
  CreateCrmContactRequest,
  crmApi,
  CrmContactRecord,
  CrmContactStats,
  EmailAccountRecord,
  emailApi,
  projectsApi,
  UpdateCrmContactRequest,
} from '@/lib/api';
import { commsKeys, crmKeys, projectKeys } from '@/lib/query-keys';
import { LIFECYCLE_STAGE_INFO } from '@/types/crm';
import type { EmailProvider } from '@/types/email';

import { ContactCard } from './ContactCard';
import { ContactFormDialog } from './ContactFormDialog';
import { mapEmailAccounts } from './mapEmailAccounts';

export function CrmPage() {
  const queryClient = useQueryClient();
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedStage, setSelectedStage] = useState<string>('all');
  const [isCreateDialogOpen, setIsCreateDialogOpen] = useState(false);
  const [editingContact, setEditingContact] = useState<CrmContactRecord | null>(null);
  const [activeTab, setActiveTab] = useState('contacts');
  const [searchParams, setSearchParams] = useSearchParams();
  const [connectingProvider, setConnectingProvider] = useState<EmailProvider | null>(null);

  const {
    data: projects = [],
    isLoading: projectsLoading,
    error: projectsError,
  } = useQuery<Project[], Error>({
    queryKey: projectKeys.crm(),
    queryFn: projectsApi.getAll,
  });

  const projectParam = searchParams.get('projectId');
  const tabParam = searchParams.get('tab');

  useEffect(() => {
    if (!selectedProjectId && projects.length > 0) {
      setSelectedProjectId(projects[0].id);
    }
  }, [projects, selectedProjectId]);

  useEffect(() => {
    if (projectParam && projectParam !== selectedProjectId) {
      setSelectedProjectId(projectParam);
    }
  }, [projectParam, selectedProjectId]);

  useEffect(() => {
    if (tabParam && tabParam !== activeTab) {
      setActiveTab(tabParam);
    }
  }, [tabParam, activeTab]);

  useEffect(() => {
    if (!selectedProjectId) return;
    if (projectParam === selectedProjectId && (tabParam ?? 'contacts') === activeTab) {
      return;
    }
    const next = new URLSearchParams();
    next.set('projectId', selectedProjectId);
    next.set('tab', activeTab);
    setSearchParams(next, { replace: true });
  }, [selectedProjectId, activeTab, projectParam, tabParam, setSearchParams]);

  const contactsQuery = useQuery<CrmContactRecord[], Error>({
    queryKey: crmKeys.contactsFiltered(selectedProjectId, selectedStage),
    queryFn: () =>
      crmApi.listContacts(selectedProjectId!, {
        lifecycleStage: selectedStage === 'all' ? undefined : selectedStage,
        limit: 100,
      }),
    enabled: !!selectedProjectId,
  });

  const searchQuery_result = useQuery<CrmContactRecord[], Error>({
    queryKey: crmKeys.contactsSearch(selectedProjectId, searchQuery),
    queryFn: () => crmApi.searchContacts(selectedProjectId!, searchQuery, { limit: 50 }),
    enabled: !!selectedProjectId && searchQuery.length > 2,
  });

  const statsQuery = useQuery<CrmContactStats, Error>({
    queryKey: crmKeys.stats(selectedProjectId),
    queryFn: () => crmApi.getContactStats(selectedProjectId!),
    enabled: !!selectedProjectId,
  });

  const emailAccountsQuery = useQuery<EmailAccountRecord[], Error>({
    queryKey: commsKeys.emailAccounts('project', selectedProjectId ?? undefined),
    queryFn: () => emailApi.listAccounts(selectedProjectId ?? undefined),
    enabled: !!selectedProjectId,
  });

  const createContactMutation = useMutationWithToast({
    mutationFn: (data: CreateCrmContactRequest) => crmApi.createContact(data),
    successMessage: 'Contact created',
    errorMessage: 'Failed to create contact',
    invalidateKeys: [crmKeys.contactsAll(), crmKeys.statsAll()],
    onSuccess: () => setIsCreateDialogOpen(false),
  });

  const updateContactMutation = useMutationWithToast({
    mutationFn: ({ id, data }: { id: string; data: UpdateCrmContactRequest }) =>
      crmApi.updateContact(id, data),
    successMessage: 'Contact updated',
    errorMessage: 'Failed to update contact',
    invalidateKeys: [crmKeys.contactsAll(), crmKeys.statsAll()],
    onSuccess: () => setEditingContact(null),
  });

  const deleteContactMutation = useMutationWithToast({
    mutationFn: (id: string) => crmApi.deleteContact(id),
    successMessage: 'Contact deleted',
    errorMessage: 'Failed to delete contact',
    invalidateKeys: [crmKeys.contactsAll(), crmKeys.statsAll()],
  });

  const contacts = useMemo(() => {
    if (searchQuery.length > 2 && searchQuery_result.data) {
      return searchQuery_result.data;
    }
    return contactsQuery.data ?? [];
  }, [searchQuery, searchQuery_result.data, contactsQuery.data]);

  const handleConnectEmail = async (provider: EmailProvider) => {
    if (!selectedProjectId) return;
    try {
      setConnectingProvider(provider);
      const result = await emailApi.initiateOAuth(
        selectedProjectId,
        provider,
        `${window.location.origin}/oauth/${provider}/callback`
      );
      window.location.href = result.auth_url;
    } catch (error) {
      console.error('Failed to initiate OAuth:', error);
    } finally {
      setConnectingProvider(null);
    }
  };

  const handleSyncEmail = async (accountId: string) => {
    try {
      await emailApi.triggerSync(accountId);
      queryClient.invalidateQueries({ queryKey: commsKeys.emailAccountsAll() });
    } catch (error) {
      console.error('Failed to sync email:', error);
    }
  };

  const handleDisconnectEmail = async (accountId: string) => {
    try {
      await emailApi.deleteAccount(accountId);
      queryClient.invalidateQueries({ queryKey: commsKeys.emailAccountsAll() });
    } catch (error) {
      console.error('Failed to disconnect email:', error);
    }
  };

  return (
    <div className="page-container">
      <div className="page-header">
        <div className="flex items-center gap-3">
          <div className="section-header-icon">
            <Users className="h-5 w-5" />
          </div>
          <div>
            <h1 className="page-title">CRM Administration</h1>
            <p className="page-description">
              Platform-level contact management and email account configuration
            </p>
          </div>
        </div>
        <div className="w-full max-w-xs space-y-1">
          <Label htmlFor="project-select" className="text-xs text-muted-foreground">Project</Label>
          <Select
            value={selectedProjectId ?? ''}
            onValueChange={setSelectedProjectId}
            disabled={!projects.length}
          >
            <SelectTrigger id="project-select">
              <SelectValue placeholder="Select a project" />
            </SelectTrigger>
            <SelectContent>
              {projects.map((project) => (
                <SelectItem key={project.id} value={project.id}>
                  {project.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>

      {projectsLoading && <Skeleton className="h-8 w-1/3" />}

      {projectsError && (
        <Alert variant="destructive">
          <AlertDescription>
            {projectsError.message || 'Unable to load projects.'}
          </AlertDescription>
        </Alert>
      )}

      {!projects.length && !projectsLoading ? (
        <Alert>
          <AlertDescription>
            No projects detected. Create a project to enable CRM.
          </AlertDescription>
        </Alert>
      ) : (
        <Tabs value={activeTab} onValueChange={setActiveTab}>
          <TabsList>
            <TabsTrigger value="contacts" className="gap-2">
              <Users className="h-4 w-4" />
              Contacts
            </TabsTrigger>
            <TabsTrigger value="email" className="gap-2">
              <Mail className="h-4 w-4" />
              Email Accounts
            </TabsTrigger>
          </TabsList>

          <TabsContent value="contacts" className="space-y-4 sm:space-y-6">
            {/* Stats Cards */}
            {statsQuery.data && (
              <div className="stat-grid animate-stagger">
                <div className="stat-card">
                  <Users className="stat-card-icon" />
                  <div className="stat-card-value">{statsQuery.data.total}</div>
                  <div className="stat-card-label">Total Contacts</div>
                </div>
                <div className="stat-card">
                  <TrendingUp className="stat-card-icon" />
                  <div className="stat-card-value text-success">
                    {Math.round(statsQuery.data.avg_lead_score)}
                  </div>
                  <div className="stat-card-label">Avg Lead Score</div>
                </div>
                <div className="stat-card">
                  <Clock className="stat-card-icon" />
                  <div className="stat-card-value text-warning">{statsQuery.data.needs_follow_up}</div>
                  <div className="stat-card-label">Need Follow-up</div>
                </div>
                <div className="stat-card">
                  <div className="flex flex-wrap gap-1 mt-1">
                    {statsQuery.data.by_stage.slice(0, 4).map((item) => (
                      <Badge
                        key={item.stage}
                        variant="secondary"
                        className="text-xs"
                      >
                        {item.stage}: {item.count}
                      </Badge>
                    ))}
                  </div>
                  <div className="stat-card-label mt-2">By Stage</div>
                </div>
              </div>
            )}

            {/* Contacts List */}
            <Card className="card-elevated">
              <CardHeader>
                <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
                  <div>
                    <CardTitle>Contacts</CardTitle>
                    <CardDescription>
                      Manage your leads and customers
                    </CardDescription>
                  </div>
                  <div className="action-bar !flex-row !gap-2">
                    <div className="relative flex-1 sm:flex-none">
                      <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                      <Input
                        placeholder="Search contacts..."
                        value={searchQuery}
                        onChange={(e) => setSearchQuery(e.target.value)}
                        className="pl-9 w-full sm:w-56"
                      />
                    </div>
                    <Select value={selectedStage} onValueChange={setSelectedStage}>
                      <SelectTrigger className="w-32 sm:w-40">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="all">All Stages</SelectItem>
                        {Object.entries(LIFECYCLE_STAGE_INFO).map(([key, info]) => (
                          <SelectItem key={key} value={key}>
                            {info.label}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    <Button onClick={() => setIsCreateDialogOpen(true)} className="gap-2 shrink-0">
                      <UserPlus className="h-4 w-4" />
                      <span className="hidden sm:inline">Add Contact</span>
                    </Button>
                  </div>
                </div>
              </CardHeader>
              <CardContent>
                {contactsQuery.isLoading ? (
                  <div className="space-y-3">
                    {[1, 2, 3].map((i) => (
                      <Skeleton key={i} className="h-20 w-full" />
                    ))}
                  </div>
                ) : contacts.length === 0 ? (
                  <EmptyState
                    icon={Users}
                    title="No contacts yet"
                    description="Add your first contact to start building your CRM pipeline, or connect an email account to import contacts automatically."
                    action={{ label: 'Add Contact', onClick: () => setIsCreateDialogOpen(true) }}
                    secondaryAction={{ label: 'Connect Email', onClick: () => setActiveTab('email') }}
                  />
                ) : (
                  <div className="space-y-3">
                    {contacts.map((contact) => (
                      <ContactCard
                        key={contact.id}
                        contact={contact}
                        projectId={selectedProjectId!}
                        onEdit={() => setEditingContact(contact)}
                        onDelete={() => deleteContactMutation.mutate(contact.id)}
                      />
                    ))}
                  </div>
                )}
              </CardContent>
            </Card>
          </TabsContent>

          <TabsContent value="email" className="space-y-6">
            <EmailAccountConnect
              accounts={mapEmailAccounts(emailAccountsQuery.data ?? [])}
              onConnect={handleConnectEmail}
              onDisconnect={handleDisconnectEmail}
              onSync={handleSyncEmail}
              isConnecting={connectingProvider}
            />
          </TabsContent>
        </Tabs>
      )}

      {/* Create Contact Dialog */}
      <ContactFormDialog
        open={isCreateDialogOpen}
        onOpenChange={setIsCreateDialogOpen}
        projectId={selectedProjectId ?? ''}
        onSubmit={(data) => createContactMutation.mutate(data as CreateCrmContactRequest)}
        isLoading={createContactMutation.isPending}
      />

      {/* Edit Contact Dialog */}
      {editingContact && (
        <ContactFormDialog
          open={!!editingContact}
          onOpenChange={(open) => !open && setEditingContact(null)}
          projectId={selectedProjectId ?? ''}
          contact={editingContact}
          onSubmit={(data) =>
            updateContactMutation.mutate({
              id: editingContact.id,
              data: data as UpdateCrmContactRequest,
            })
          }
          isLoading={updateContactMutation.isPending}
        />
      )}
    </div>
  );
}

export default CrmPage;
