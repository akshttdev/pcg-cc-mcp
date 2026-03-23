import { useCallback, useEffect, useMemo, useState } from 'react';
import { useSearchParams, useNavigate } from 'react-router-dom';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useMutationWithToast } from '@/hooks/useMutationWithToast';
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
import { IconButton } from '@/components/ui/icon-button';
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
import { crmKeys, projectKeys, commsKeys } from '@/lib/query-keys';
import { LIFECYCLE_STAGE_INFO, CONTACT_SOURCE_INFO } from '@/types/crm';
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
                  <div className="text-center py-12 text-muted-foreground">
                    <Users className="h-12 w-12 mx-auto mb-4 opacity-30" />
                    <p className="text-lg font-semibold mb-1">No contacts yet</p>
                    <p className="text-sm max-w-sm mx-auto">Add your first contact to start building your CRM pipeline, or connect an email account to import contacts automatically.</p>
                    <div className="flex gap-2 justify-center mt-4">
                      <Button size="sm" onClick={() => setIsCreateDialogOpen(true)}>
                        <UserPlus className="h-4 w-4 mr-1" />
                        Add Contact
                      </Button>
                      <Button size="sm" variant="outline" onClick={() => setActiveTab('email')}>
                        <Mail className="h-4 w-4 mr-1" />
                        Connect Email
                      </Button>
                    </div>
                  </div>
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

function ContactCard({
  contact,
  projectId,
  onEdit,
  onDelete,
}: {
  contact: CrmContactRecord;
  projectId: string;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const navigate = useNavigate();
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
    <div className="group flex items-center gap-3 sm:gap-4 p-3 sm:p-4 border border-border/40 rounded-lg hover:bg-muted/30 hover:border-border/70 transition-all duration-200">
      {/* Avatar */}
      <div className="w-10 h-10 sm:w-12 sm:h-12 rounded-full bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center text-white font-semibold shrink-0 text-sm sm:text-base">
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
        <div className="flex items-center gap-2 mb-0.5">
          <h4
            className="font-medium truncate cursor-pointer hover:text-info transition-colors"
            onClick={() => navigate(`/projects/${projectId}/crm/contacts/${contact.id}`)}
          >
            {contact.full_name || contact.email || 'Unnamed Contact'}
          </h4>
          <Badge
            variant="outline"
            style={{
              borderColor: stageInfo.color,
              color: stageInfo.color,
            }}
            className="text-xs shrink-0"
          >
            {stageInfo.label}
          </Badge>
          {contact.lead_score > 50 && (
            <Badge className="text-xs bg-yellow-100 text-yellow-700 dark:bg-yellow-950/40 dark:text-yellow-300 shrink-0">
              <Star className="h-3 w-3 mr-1" />
              {contact.lead_score}
            </Badge>
          )}
        </div>
        <div className="flex items-center gap-3 sm:gap-4 text-sm text-muted-foreground flex-wrap">
          {contact.email && (
            <span className="flex items-center gap-1 truncate">
              <Mail className="h-3 w-3 shrink-0" />
              <span className="truncate">{contact.email}</span>
            </span>
          )}
          {contact.company_name && (
            <span className="hidden sm:flex items-center gap-1">
              <Building2 className="h-3 w-3 shrink-0" />
              {contact.company_name}
            </span>
          )}
          {contact.phone && (
            <span className="hidden md:flex items-center gap-1">
              <Phone className="h-3 w-3 shrink-0" />
              {contact.phone}
            </span>
          )}
        </div>
        <div className="flex items-center gap-4 mt-1 text-xs text-muted-foreground">
          <span>Last activity: {formatRelativeTime(contact.last_activity_at)}</span>
          <span className="hidden sm:inline">{contact.email_count} emails</span>
        </div>
      </div>

      {/* Social Links — hidden on mobile */}
      <div className="hidden md:flex items-center gap-1">
        {contact.linkedin_url && (
          <Button variant="ghost" size="icon" className="h-8 w-8" asChild title="LinkedIn profile">
            <a href={contact.linkedin_url} target="_blank" rel="noopener noreferrer">
              <Linkedin className="h-3.5 w-3.5" />
            </a>
          </Button>
        )}
        {contact.twitter_handle && (
          <Button variant="ghost" size="icon" className="h-8 w-8" asChild title="Twitter profile">
            <a
              href={`https://twitter.com/${contact.twitter_handle}`}
              target="_blank"
              rel="noopener noreferrer"
            >
              <Twitter className="h-3.5 w-3.5" />
            </a>
          </Button>
        )}
        {contact.website && (
          <Button variant="ghost" size="icon" className="h-8 w-8" asChild title="Website">
            <a href={contact.website} target="_blank" rel="noopener noreferrer">
              <Globe className="h-3.5 w-3.5" />
            </a>
          </Button>
        )}
      </div>

      {/* Actions */}
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <IconButton
            variant="ghost" className="h-8 w-8 opacity-0 group-hover:opacity-100 transition-opacity"
            icon={MoreVertical}
            label="More options"
          />
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
  // eslint-disable-next-line @typescript-eslint/no-unused-vars, unused-imports/no-unused-vars
  projectId: _projectId,
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
  const emptyForm = {
    first_name: '',
    last_name: '',
    email: '',
    phone: '',
    mobile: '',
    company_name: '',
    job_title: '',
    department: '',
    linkedin_url: '',
    twitter_handle: '',
    website: '',
    lifecycle_stage: 'lead',
    source: '' as string,
    tags: '',
  };

  const contactToForm = (c: CrmContactRecord) => ({
    first_name: c.first_name || '',
    last_name: c.last_name || '',
    email: c.email || '',
    phone: c.phone || '',
    mobile: c.mobile || '',
    company_name: c.company_name || '',
    job_title: c.job_title || '',
    department: c.department || '',
    linkedin_url: c.linkedin_url || '',
    twitter_handle: c.twitter_handle || '',
    website: c.website || '',
    lifecycle_stage: c.lifecycle_stage || 'lead',
    source: c.source || '',
    tags: Array.isArray(c.tags) ? c.tags.join(', ') : (c.tags || ''),
  });

  const [formData, setFormData] = useState(contact ? contactToForm(contact) : emptyForm);

  useEffect(() => {
    setFormData(contact ? contactToForm(contact) : emptyForm);
  }, [contact, open]);

  const handleFieldChange = useCallback(
    (field: string) => (e: React.ChangeEvent<HTMLInputElement>) => {
      setFormData((prev) => ({ ...prev, [field]: e.target.value }));
    },
    [],
  );

  const handleSelectChange = useCallback(
    (field: string) => (value: string) => {
      setFormData((prev) => ({ ...prev, [field]: value === '__none__' ? '' : value }));
    },
    [],
  );

  const handleCancel = useCallback(() => {
    onOpenChange(false);
  }, [onOpenChange]);

  const handleSubmit = useCallback((e: React.FormEvent) => {
    e.preventDefault();
    const parsedTags = formData.tags
      ? formData.tags.split(',').map((t) => t.trim()).filter(Boolean)
      : undefined;
    onSubmit({
      ...formData,
      first_name: formData.first_name || undefined,
      last_name: formData.last_name || undefined,
      email: formData.email || undefined,
      phone: formData.phone || undefined,
      mobile: formData.mobile || undefined,
      company_name: formData.company_name || undefined,
      job_title: formData.job_title || undefined,
      department: formData.department || undefined,
      linkedin_url: formData.linkedin_url || undefined,
      twitter_handle: formData.twitter_handle || undefined,
      website: formData.website || undefined,
      source: formData.source || undefined,
      tags: parsedTags,
    });
  }, [formData, onSubmit]);

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
        <form onSubmit={handleSubmit} className="space-y-4 max-h-[60vh] overflow-y-auto pr-1">
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="first_name">First Name</Label>
              <Input
                id="first_name"
                value={formData.first_name}
                onChange={handleFieldChange('first_name')}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="last_name">Last Name</Label>
              <Input
                id="last_name"
                value={formData.last_name}
                onChange={handleFieldChange('last_name')}
              />
            </div>
          </div>
          <div className="space-y-2">
            <Label htmlFor="email">Email</Label>
            <Input
              id="email"
              type="email"
              value={formData.email}
              onChange={handleFieldChange('email')}
            />
          </div>
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="phone">Phone</Label>
              <Input
                id="phone"
                value={formData.phone}
                onChange={handleFieldChange('phone')}
                placeholder="+1 555-0100"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="mobile">Mobile</Label>
              <Input
                id="mobile"
                value={formData.mobile}
                onChange={handleFieldChange('mobile')}
                placeholder="+1 555-0101"
              />
            </div>
          </div>
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="company_name">Company</Label>
              <Input
                id="company_name"
                value={formData.company_name}
                onChange={handleFieldChange('company_name')}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="job_title">Job Title</Label>
              <Input
                id="job_title"
                value={formData.job_title}
                onChange={handleFieldChange('job_title')}
              />
            </div>
          </div>
          <div className="space-y-2">
            <Label htmlFor="department">Department</Label>
            <Input
              id="department"
              value={formData.department}
              onChange={handleFieldChange('department')}
              placeholder="Engineering, Sales, Marketing..."
            />
          </div>
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="lifecycle_stage">Lifecycle Stage</Label>
              <Select
                value={formData.lifecycle_stage}
                onValueChange={handleSelectChange('lifecycle_stage')}
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
              <Label htmlFor="source">Source</Label>
              <Select
                value={formData.source || '__none__'}
                onValueChange={handleSelectChange('source')}
              >
                <SelectTrigger>
                  <SelectValue placeholder="Select source" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="__none__">Not specified</SelectItem>
                  {Object.entries(CONTACT_SOURCE_INFO).map(([key, info]) => (
                    <SelectItem key={key} value={key}>
                      {info.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>
          <div className="space-y-2">
            <Label htmlFor="linkedin_url">LinkedIn URL</Label>
            <Input
              id="linkedin_url"
              value={formData.linkedin_url}
              onChange={handleFieldChange('linkedin_url')}
              placeholder="https://linkedin.com/in/..."
            />
          </div>
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="twitter_handle">Twitter Handle</Label>
              <Input
                id="twitter_handle"
                value={formData.twitter_handle}
                onChange={handleFieldChange('twitter_handle')}
                placeholder="@handle"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="website">Website</Label>
              <Input
                id="website"
                value={formData.website}
                onChange={handleFieldChange('website')}
                placeholder="https://..."
              />
            </div>
          </div>
          <div className="space-y-2">
            <Label htmlFor="tags">Tags</Label>
            <Input
              id="tags"
              value={formData.tags}
              onChange={handleFieldChange('tags')}
              placeholder="Comma-separated tags, e.g. vip, conference-2026"
            />
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={handleCancel}>
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
