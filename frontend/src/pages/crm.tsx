import { useEffect, useMemo, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import type { Project } from 'shared/types';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Skeleton } from '@/components/ui/skeleton';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  Users,
  UserPlus,
  Search,
  Mail,
  Phone,
  Building2,
  MoreVertical,
  Edit,
  Trash2,
  TrendingUp,
  Clock,
  Star,
  Linkedin,
  Twitter,
  Globe,
} from 'lucide-react';
import {
  projectsApi,
  crmApi,
  emailApi,
  CrmContactRecord,
  CrmContactStats,
  EmailAccountRecord,
  CreateCrmContactRequest,
  UpdateCrmContactRequest,
} from '@/lib/api';
import { EmailAccountConnect } from '@/components/email/EmailAccountConnect';
import { LIFECYCLE_STAGE_INFO } from '@/types/crm';
import type { LifecycleStage } from '@/types/crm';
import type { EmailProvider } from '@/types/email';

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
    queryKey: ['projects', 'crm'],
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
    queryKey: ['crm-contacts', selectedProjectId, selectedStage],
    queryFn: () =>
      crmApi.listContacts(selectedProjectId!, {
        lifecycleStage: selectedStage === 'all' ? undefined : selectedStage,
        limit: 100,
      }),
    enabled: !!selectedProjectId,
  });

  const searchQuery_result = useQuery<CrmContactRecord[], Error>({
    queryKey: ['crm-contacts-search', selectedProjectId, searchQuery],
    queryFn: () => crmApi.searchContacts(selectedProjectId!, searchQuery, { limit: 50 }),
    enabled: !!selectedProjectId && searchQuery.length > 2,
  });

  const statsQuery = useQuery<CrmContactStats, Error>({
    queryKey: ['crm-stats', selectedProjectId],
    queryFn: () => crmApi.getContactStats(selectedProjectId!),
    enabled: !!selectedProjectId,
  });

  const emailAccountsQuery = useQuery<EmailAccountRecord[], Error>({
    queryKey: ['email-accounts', selectedProjectId],
    queryFn: () => emailApi.listAccounts(selectedProjectId ?? undefined),
    enabled: !!selectedProjectId,
  });

  const createContactMutation = useMutation({
    mutationFn: (data: CreateCrmContactRequest) => crmApi.createContact(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['crm-contacts'] });
      queryClient.invalidateQueries({ queryKey: ['crm-stats'] });
      setIsCreateDialogOpen(false);
    },
  });

  const updateContactMutation = useMutation({
    mutationFn: ({ id, data }: { id: string; data: UpdateCrmContactRequest }) =>
      crmApi.updateContact(id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['crm-contacts'] });
      queryClient.invalidateQueries({ queryKey: ['crm-stats'] });
      setEditingContact(null);
    },
  });

  const deleteContactMutation = useMutation({
    mutationFn: (id: string) => crmApi.deleteContact(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['crm-contacts'] });
      queryClient.invalidateQueries({ queryKey: ['crm-stats'] });
    },
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
      queryClient.invalidateQueries({ queryKey: ['email-accounts'] });
    } catch (error) {
      console.error('Failed to sync email:', error);
    }
  };

  const handleDisconnectEmail = async (accountId: string) => {
    try {
      await emailApi.deleteAccount(accountId);
      queryClient.invalidateQueries({ queryKey: ['email-accounts'] });
    } catch (error) {
      console.error('Failed to disconnect email:', error);
    }
  };

  return (
    <div className="p-6 space-y-6">
      <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
        <div>
          <CardTitle className="text-2xl">CRM & Email</CardTitle>
          <CardDescription>
            Manage contacts, track leads, and connect your email accounts for unified communication.
          </CardDescription>
        </div>
        <div className="w-full max-w-xs space-y-1">
          <Label htmlFor="project-select">Project</Label>
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

          <TabsContent value="contacts" className="space-y-6">
            {/* Stats Cards */}
            {statsQuery.data && (
              <div className="grid gap-4 md:grid-cols-4">
                <Card>
                  <CardContent className="pt-6">
                    <div className="flex items-center gap-3">
                      <div className="p-2 bg-blue-100 rounded-lg">
                        <Users className="h-5 w-5 text-blue-600" />
                      </div>
                      <div>
                        <p className="text-sm text-muted-foreground">Total Contacts</p>
                        <p className="text-2xl font-bold">{statsQuery.data.total}</p>
                      </div>
                    </div>
                  </CardContent>
                </Card>
                <Card>
                  <CardContent className="pt-6">
                    <div className="flex items-center gap-3">
                      <div className="p-2 bg-green-100 rounded-lg">
                        <TrendingUp className="h-5 w-5 text-green-600" />
                      </div>
                      <div>
                        <p className="text-sm text-muted-foreground">Avg Lead Score</p>
                        <p className="text-2xl font-bold">
                          {Math.round(statsQuery.data.avg_lead_score)}
                        </p>
                      </div>
                    </div>
                  </CardContent>
                </Card>
                <Card>
                  <CardContent className="pt-6">
                    <div className="flex items-center gap-3">
                      <div className="p-2 bg-yellow-100 rounded-lg">
                        <Clock className="h-5 w-5 text-yellow-600" />
                      </div>
                      <div>
                        <p className="text-sm text-muted-foreground">Need Follow-up</p>
                        <p className="text-2xl font-bold">{statsQuery.data.needs_follow_up}</p>
                      </div>
                    </div>
                  </CardContent>
                </Card>
                <Card>
                  <CardContent className="pt-6">
                    <div className="flex flex-wrap gap-1">
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
                  </CardContent>
                </Card>
              </div>
            )}

            {/* Contacts List */}
            <Card>
              <CardHeader>
                <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
                  <div>
                    <CardTitle>Contacts</CardTitle>
                    <CardDescription>
                      Manage your leads and customers
                    </CardDescription>
                  </div>
                  <div className="flex gap-2">
                    <div className="relative">
                      <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                      <Input
                        placeholder="Search contacts..."
                        value={searchQuery}
                        onChange={(e) => setSearchQuery(e.target.value)}
                        className="pl-9 w-64"
                      />
                    </div>
                    <Select value={selectedStage} onValueChange={setSelectedStage}>
                      <SelectTrigger className="w-40">
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
                    <Button onClick={() => setIsCreateDialogOpen(true)} className="gap-2">
                      <UserPlus className="h-4 w-4" />
                      Add Contact
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
                  <div className="text-center py-12 text-muted-foreground">
                    <Users className="h-12 w-12 mx-auto mb-4 opacity-50" />
                    <p className="text-lg font-medium">No contacts yet</p>
                    <p className="text-sm">Add your first contact to get started</p>
                  </div>
                ) : (
                  <div className="space-y-3">
                    {contacts.map((contact) => (
                      <ContactCard
                        key={contact.id}
                        contact={contact}
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

function ContactCard({
  contact,
  onEdit,
  onDelete,
}: {
  contact: CrmContactRecord;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const stageInfo = LIFECYCLE_STAGE_INFO[contact.lifecycle_stage as LifecycleStage] ?? {
    label: contact.lifecycle_stage,
    color: '#6B7280',
  };

  const formatRelativeTime = (dateString: string | null) => {
    if (!dateString) return 'Never';
    const date = new Date(dateString);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffDays === 0) return 'Today';
    if (diffDays === 1) return 'Yesterday';
    if (diffDays < 7) return `${diffDays} days ago`;
    if (diffDays < 30) return `${Math.floor(diffDays / 7)} weeks ago`;
    return `${Math.floor(diffDays / 30)} months ago`;
  };

  return (
    <div className="flex items-center gap-4 p-4 border rounded-lg hover:bg-muted/50 transition-colors">
      {/* Avatar */}
      <div className="w-12 h-12 rounded-full bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center text-white font-semibold">
        {contact.avatar_url ? (
          <img
            src={contact.avatar_url}
            alt={contact.full_name || 'Contact'}
            className="w-full h-full rounded-full object-cover"
          />
        ) : (
          (contact.first_name?.[0] || contact.email?.[0] || 'C').toUpperCase()
        )}
      </div>

      {/* Info */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-1">
          <h4 className="font-medium truncate">
            {contact.full_name || contact.email || 'Unnamed Contact'}
          </h4>
          <Badge
            variant="outline"
            style={{
              borderColor: stageInfo.color,
              color: stageInfo.color,
            }}
            className="text-xs"
          >
            {stageInfo.label}
          </Badge>
          {contact.lead_score > 50 && (
            <Badge className="text-xs bg-yellow-100 text-yellow-700">
              <Star className="h-3 w-3 mr-1" />
              {contact.lead_score}
            </Badge>
          )}
        </div>
        <div className="flex items-center gap-4 text-sm text-muted-foreground">
          {contact.email && (
            <span className="flex items-center gap-1 truncate">
              <Mail className="h-3 w-3" />
              {contact.email}
            </span>
          )}
          {contact.company_name && (
            <span className="flex items-center gap-1">
              <Building2 className="h-3 w-3" />
              {contact.company_name}
            </span>
          )}
          {contact.phone && (
            <span className="flex items-center gap-1">
              <Phone className="h-3 w-3" />
              {contact.phone}
            </span>
          )}
        </div>
        <div className="flex items-center gap-4 mt-1 text-xs text-muted-foreground">
          <span>Last activity: {formatRelativeTime(contact.last_activity_at)}</span>
          <span>{contact.email_count} emails</span>
        </div>
      </div>

      {/* Social Links */}
      <div className="flex items-center gap-1">
        {contact.linkedin_url && (
          <Button variant="ghost" size="icon" asChild>
            <a href={contact.linkedin_url} target="_blank" rel="noopener noreferrer">
              <Linkedin className="h-4 w-4" />
            </a>
          </Button>
        )}
        {contact.twitter_handle && (
          <Button variant="ghost" size="icon" asChild>
            <a
              href={`https://twitter.com/${contact.twitter_handle}`}
              target="_blank"
              rel="noopener noreferrer"
            >
              <Twitter className="h-4 w-4" />
            </a>
          </Button>
        )}
        {contact.website && (
          <Button variant="ghost" size="icon" asChild>
            <a href={contact.website} target="_blank" rel="noopener noreferrer">
              <Globe className="h-4 w-4" />
            </a>
          </Button>
        )}
      </div>

      {/* Actions */}
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="ghost" size="icon">
            <MoreVertical className="h-4 w-4" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem onClick={onEdit}>
            <Edit className="h-4 w-4 mr-2" />
            Edit
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem onClick={onDelete} className="text-red-600">
            <Trash2 className="h-4 w-4 mr-2" />
            Delete
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}

function ContactFormDialog({
  open,
  onOpenChange,
  projectId,
  contact,
  onSubmit,
  isLoading,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  projectId: string;
  contact?: CrmContactRecord;
  onSubmit: (data: CreateCrmContactRequest | UpdateCrmContactRequest) => void;
  isLoading: boolean;
}) {
  const [formData, setFormData] = useState({
    first_name: contact?.first_name || '',
    last_name: contact?.last_name || '',
    email: contact?.email || '',
    phone: contact?.phone || '',
    company_name: contact?.company_name || '',
    job_title: contact?.job_title || '',
    linkedin_url: contact?.linkedin_url || '',
    twitter_handle: contact?.twitter_handle || '',
    website: contact?.website || '',
    lifecycle_stage: contact?.lifecycle_stage || 'lead',
  });

  useEffect(() => {
    if (contact) {
      setFormData({
        first_name: contact.first_name || '',
        last_name: contact.last_name || '',
        email: contact.email || '',
        phone: contact.phone || '',
        company_name: contact.company_name || '',
        job_title: contact.job_title || '',
        linkedin_url: contact.linkedin_url || '',
        twitter_handle: contact.twitter_handle || '',
        website: contact.website || '',
        lifecycle_stage: contact.lifecycle_stage || 'lead',
      });
    } else {
      setFormData({
        first_name: '',
        last_name: '',
        email: '',
        phone: '',
        company_name: '',
        job_title: '',
        linkedin_url: '',
        twitter_handle: '',
        website: '',
        lifecycle_stage: 'lead',
      });
    }
  }, [contact, open]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit({
      project_id: projectId,
      ...formData,
      first_name: formData.first_name || undefined,
      last_name: formData.last_name || undefined,
      email: formData.email || undefined,
      phone: formData.phone || undefined,
      company_name: formData.company_name || undefined,
      job_title: formData.job_title || undefined,
      linkedin_url: formData.linkedin_url || undefined,
      twitter_handle: formData.twitter_handle || undefined,
      website: formData.website || undefined,
    });
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>{contact ? 'Edit Contact' : 'Add Contact'}</DialogTitle>
          <DialogDescription>
            {contact
              ? 'Update contact information'
              : 'Add a new contact to your CRM'}
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="first_name">First Name</Label>
              <Input
                id="first_name"
                value={formData.first_name}
                onChange={(e) =>
                  setFormData({ ...formData, first_name: e.target.value })
                }
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="last_name">Last Name</Label>
              <Input
                id="last_name"
                value={formData.last_name}
                onChange={(e) =>
                  setFormData({ ...formData, last_name: e.target.value })
                }
              />
            </div>
          </div>
          <div className="space-y-2">
            <Label htmlFor="email">Email</Label>
            <Input
              id="email"
              type="email"
              value={formData.email}
              onChange={(e) => setFormData({ ...formData, email: e.target.value })}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="phone">Phone</Label>
            <Input
              id="phone"
              value={formData.phone}
              onChange={(e) => setFormData({ ...formData, phone: e.target.value })}
            />
          </div>
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="company_name">Company</Label>
              <Input
                id="company_name"
                value={formData.company_name}
                onChange={(e) =>
                  setFormData({ ...formData, company_name: e.target.value })
                }
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="job_title">Job Title</Label>
              <Input
                id="job_title"
                value={formData.job_title}
                onChange={(e) =>
                  setFormData({ ...formData, job_title: e.target.value })
                }
              />
            </div>
          </div>
          <div className="space-y-2">
            <Label htmlFor="lifecycle_stage">Lifecycle Stage</Label>
            <Select
              value={formData.lifecycle_stage}
              onValueChange={(value) =>
                setFormData({ ...formData, lifecycle_stage: value })
              }
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {Object.entries(LIFECYCLE_STAGE_INFO).map(([key, info]) => (
                  <SelectItem key={key} value={key}>
                    {info.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className="space-y-2">
            <Label htmlFor="linkedin_url">LinkedIn URL</Label>
            <Input
              id="linkedin_url"
              value={formData.linkedin_url}
              onChange={(e) =>
                setFormData({ ...formData, linkedin_url: e.target.value })
              }
              placeholder="https://linkedin.com/in/..."
            />
          </div>
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="twitter_handle">Twitter Handle</Label>
              <Input
                id="twitter_handle"
                value={formData.twitter_handle}
                onChange={(e) =>
                  setFormData({ ...formData, twitter_handle: e.target.value })
                }
                placeholder="@handle"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="website">Website</Label>
              <Input
                id="website"
                value={formData.website}
                onChange={(e) =>
                  setFormData({ ...formData, website: e.target.value })
                }
                placeholder="https://..."
              />
            </div>
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button type="submit" disabled={isLoading}>
              {isLoading ? 'Saving...' : contact ? 'Save Changes' : 'Add Contact'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function mapEmailAccounts(records: EmailAccountRecord[]) {
  return records.map((record) => ({
    id: record.id,
    project_id: record.project_id,
    provider: record.provider as 'gmail' | 'zoho' | 'imap_custom',
    account_type: (record.account_type || 'primary') as
      | 'primary'
      | 'team'
      | 'notifications'
      | 'marketing'
      | 'support',
    email_address: record.email_address,
    display_name: record.display_name ?? undefined,
    avatar_url: record.avatar_url ?? undefined,
    unread_count: record.unread_count ?? undefined,
    status: (record.status || 'active') as
      | 'active'
      | 'inactive'
      | 'expired'
      | 'error'
      | 'pending_auth'
      | 'revoked',
    last_sync_at: record.last_sync_at ?? undefined,
    last_error: record.last_error ?? undefined,
    sync_enabled: record.sync_enabled === 1,
    created_at: record.created_at,
    updated_at: record.updated_at,
  }));
}

export default CrmPage;
