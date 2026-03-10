import { useCallback, useMemo, useState } from 'react';
import { useParams, useSearchParams, useLocation, useNavigate, Link } from 'react-router-dom';
import { useQuery, useQueries, useMutation, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
// Tabs UI removed — navigation now driven entirely by sidebar + URL routing
import { Input } from '@/components/ui/input';
import { Progress } from '@/components/ui/progress';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
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
  Contact2,
  DollarSign,
  Activity,
  X,
  Search,
  BookOpen,
  Brain,
  Share2,
  MessageSquare,
  FileText,
  Radio,
  Pencil,
  Boxes,
  Network,
  AlertTriangle,
  Linkedin,
  Instagram,
  Twitter,
  Facebook,
  Youtube,
  Inbox,
  Database,
  Plus,
  MoreHorizontal,
  Trash2,
  Upload,
  ArrowUpDown,
  ArrowUp,
  ArrowDown,
  GitBranch,
  Clock,
  ChevronDown,
  ChevronRight,
  Plug,
  Mail,
  RefreshCw,
  CheckCircle2,
  AlertCircle,
  Loader2,
  UserPlus,
  Link2,
  Eye,
  Shield,
  Copy,
  Phone,
  MapPin,
  Play,
} from 'lucide-react';
import {
  organizationsApi,
  crmApi,
  crmDealsApi,
  crmActivitiesApi,
  knowledgeApi,
  pulseApi,
  socialApi,
  tasksApi,
  emailApi,
  quickbooksApi,
  airtableApi,
  githubAuthApi,
  discordApi,
  type DiscordSessionSummary,
  type OrganizationData,
  type ClientData,
  type CrmActivityRecord,
  type ProjectKnowledgeResponse,
  type ProjectKnowledgeSource,
  type SocialAccountRecord,
  type SocialMentionRecord,
  type PersonOrgContact,
  dataSourcesApi,
  companiesApi,
  workflowsApi,
  resolveApiUrl,
  type DataSourceRecord,
  type UpdateDataSourceRequest,
  type ExecutionArtifact,
  DATA_TYPE_OPTIONS,
  type EmailAccountRecord,
} from '@/lib/api';
import { useUserSystem } from '@/components/config-provider';
import { WorkflowEditor as WorkflowEditorComponent } from '@/components/workflows/WorkflowEditor';
import type { WorkflowDefinition } from '@/lib/api';
import { CrmPipelineBoard } from '@/components/crm/CrmPipelineBoard';

import { useOrgContacts, type OrgContact } from '@/hooks/useOrgContacts';
import { LIFECYCLE_STAGE_INFO, CONTACT_SOURCE_INFO, type LifecycleStage } from '@/types/crm';
import type { PipelineType } from '@/types/crm';

// ── Types ─────────────────────────────────────────────────────────────────────

interface OrgMember {
  id: string;
  user_id: string;
  role: string;
  joined_at: string;
  user?: { username: string; full_name: string; email: string; avatar_url?: string };
}

interface OrganizationProfilePageProps {
  defaultTab?: string;
  defaultPipeline?: string;
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function formatDate(iso: string) {
  return new Date(iso).toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
}

function formatCurrency(amount: number) {
  if (amount >= 1_000_000) return `$${(amount / 1_000_000).toFixed(1)}M`;
  if (amount >= 1_000) return `$${(amount / 1_000).toFixed(0)}k`;
  return `$${amount.toFixed(0)}`;
}

// ── Overview Tab ──────────────────────────────────────────────────────────────

function OverviewTab({
  orgId,
  projectEntries,
  projectCount,
  clientCount: _clientCount,
  memberCount: _memberCount,
  totalDealValue,
  totalDeals,
  contactCount,
}: {
  orgId: string;
  projectEntries: { id: string; name: string }[];
  projectCount: number;
  clientCount: number;
  memberCount: number;
  totalDealValue: number;
  totalDeals: number;
  contactCount: number;
}) {
  // Aggregate tasks across all projects
  const taskQueries = useQueries({
    queries: projectEntries.map((entry) => ({
      queryKey: ['tasks', entry.id],
      queryFn: () => tasksApi.getAll(entry.id),
      staleTime: 60_000,
      enabled: projectEntries.length > 0,
    })),
  });

  const totalTasks = useMemo(
    () => taskQueries.reduce((sum, q) => sum + (q.data?.length || 0), 0),
    [taskQueries]
  );

  // Aggregate activities across all projects
  const activityQueries = useQueries({
    queries: projectEntries.map((entry) => ({
      queryKey: ['crm-activities-org', entry.id],
      queryFn: () => crmActivitiesApi.listActivities({ organization_id: entry.id, limit: 10 }),
      staleTime: 60_000,
      enabled: projectEntries.length > 0,
    })),
  });

  // Fetch recent workflow runs for the organization
  const { data: recentWorkflowRuns = [] } = useQuery({
    queryKey: ['workflow-runs', orgId],
    queryFn: () => workflowsApi.listRecentRuns({ organization_id: orgId, limit: 10 }),
    staleTime: 60_000,
  });

  const recentActivities = useMemo(() => {
    const all: (CrmActivityRecord & { _projectName: string })[] = [];
    activityQueries.forEach((q, i) => {
      if (!q.data) return;
      const entry = projectEntries[i];
      q.data.forEach(a => all.push({ ...a, _projectName: entry.name }));
    });
    all.sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime());
    return all.slice(0, 20);
  }, [activityQueries, projectEntries]);

  return (
    <div className="space-y-6">
      {/* Stat cards */}
      <div className="grid grid-cols-2 lg:grid-cols-5 gap-4 animate-stagger">
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardContent className="pt-4 pb-3">
            <div className="flex items-center gap-2 text-muted-foreground">
              <Activity className="h-4 w-4" />
              <span className="text-xs font-medium">Total Tasks</span>
            </div>
            <p className="text-2xl font-bold mt-1">{totalTasks}</p>
          </CardContent>
        </Card>
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardContent className="pt-4 pb-3">
            <div className="flex items-center gap-2 text-muted-foreground">
              <Target className="h-4 w-4" />
              <span className="text-xs font-medium">Total Deals</span>
            </div>
            <p className="text-2xl font-bold mt-1">{totalDeals}</p>
          </CardContent>
        </Card>
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardContent className="pt-4 pb-3">
            <div className="flex items-center gap-2 text-muted-foreground">
              <DollarSign className="h-4 w-4" />
              <span className="text-xs font-medium">Pipeline Value</span>
            </div>
            <p className="text-2xl font-bold mt-1">{formatCurrency(totalDealValue)}</p>
          </CardContent>
        </Card>
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardContent className="pt-4 pb-3">
            <div className="flex items-center gap-2 text-muted-foreground">
              <Contact2 className="h-4 w-4" />
              <span className="text-xs font-medium">Contacts</span>
            </div>
            <p className="text-2xl font-bold mt-1">{contactCount}</p>
          </CardContent>
        </Card>
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardContent className="pt-4 pb-3">
            <div className="flex items-center gap-2 text-muted-foreground">
              <FolderOpen className="h-4 w-4" />
              <span className="text-xs font-medium">Active Projects</span>
            </div>
            <p className="text-2xl font-bold mt-1">{projectCount}</p>
          </CardContent>
        </Card>
      </div>

      {/* Quick links */}
      <div className="grid grid-cols-2 lg:grid-cols-3 gap-3">
        {[
          { label: 'Pipelines', icon: Target, path: 'crm/pipeline', color: 'text-amber-500', summary: `${totalDeals} deals · ${formatCurrency(totalDealValue)}` },
          { label: 'Contacts', icon: Contact2, path: 'crm/contacts', color: 'text-blue-500', summary: `${contactCount} contacts` },
          { label: 'Projects', icon: FolderOpen, path: 'projects', color: 'text-emerald-500', summary: `${projectCount} active` },
          { label: 'Intelligence', icon: Brain, path: 'intelligence', color: 'text-orange-500', summary: 'Data sources & workflows' },
          { label: 'Members', icon: Users, path: 'members', color: 'text-purple-500', summary: 'Team & roles' },
          { label: 'Integrations', icon: Plug, path: 'integrations', color: 'text-indigo-500', summary: 'Connected services' },
        ].map(({ label, icon: Icon, path, color, summary }) => (
          <Link
            key={path}
            to={`/organizations/${orgId}/${path}`}
            className="flex items-center gap-3 p-4 rounded-xl border border-border/50 bg-card/50 hover:bg-accent/50 hover:border-accent transition-all text-left group cursor-pointer"
          >
            <Icon className={`h-5 w-5 ${color} group-hover:scale-110 transition-transform`} />
            <div className="min-w-0">
              <span className="text-sm font-medium block">{label}</span>
              <span className="text-xs text-muted-foreground">{summary}</span>
            </div>
            <ChevronRight className="h-4 w-4 text-muted-foreground ml-auto opacity-0 group-hover:opacity-100 transition-opacity" />
          </Link>
        ))}
      </div>

      {/* Recent activity - workflow runs + CRM activities */}
      <Card className="bg-card/80 backdrop-blur-sm border-border/50">
        <CardHeader>
          <CardTitle className="text-base flex items-center gap-2">
            <Activity className="h-4 w-4" />
            Recent Activity
          </CardTitle>
          <CardDescription>Latest workflow runs and CRM activity</CardDescription>
        </CardHeader>
        <CardContent>
          {recentWorkflowRuns.length === 0 && recentActivities.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <Activity className="h-8 w-8 mx-auto mb-2 opacity-40" />
              <p>No recent activity</p>
            </div>
          ) : (
            <div className="space-y-3">
              {/* Workflow runs */}
              {recentWorkflowRuns.map((run: any) => {
                const statusColor = run.status === 'completed' ? 'text-green-600' : run.status === 'failed' ? 'text-red-600' : 'text-blue-600';
                const statusBg = run.status === 'completed' ? 'bg-green-100 dark:bg-green-900/30' : run.status === 'failed' ? 'bg-red-100 dark:bg-red-900/30' : 'bg-blue-100 dark:bg-blue-900/30';
                return (
                  <div key={run.id} className="flex items-start gap-3 p-3 rounded-lg border border-border/30">
                    <div className={`h-8 w-8 rounded-full ${statusBg} flex items-center justify-center shrink-0`}>
                      <Play className={`h-4 w-4 ${statusColor}`} />
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-2 flex-wrap">
                        <Badge variant="outline" className="text-[10px]">workflow run</Badge>
                        <Badge variant="secondary" className={`text-[10px] ${statusColor}`}>
                          {run.status}
                        </Badge>
                      </div>
                      <p className="text-sm mt-1 font-medium">{run.workflow_name}</p>
                      <div className="flex items-center gap-3 mt-1 text-xs text-muted-foreground">
                        {run.total_records_staged > 0 && (
                          <span>{run.total_records_staged} staged</span>
                        )}
                        {run.total_records_committed > 0 && (
                          <span className="text-green-600">{run.total_records_committed} committed</span>
                        )}
                        {run.total_duplicates_found > 0 && (
                          <span className="text-amber-600">{run.total_duplicates_found} duplicates</span>
                        )}
                        {run.model_used && (
                          <span>{run.model_used}</span>
                        )}
                      </div>
                      <p className="text-[10px] text-muted-foreground mt-1">
                        {formatDate(run.created_at)}
                      </p>
                    </div>
                  </div>
                );
              })}
              {/* CRM activities */}
              {recentActivities.map((activity) => (
                <div key={activity.id} className="flex items-start gap-3 p-3 rounded-lg border border-border/30">
                  <div className="h-8 w-8 rounded-full bg-muted flex items-center justify-center shrink-0">
                    <Activity className="h-4 w-4 text-muted-foreground" />
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2 flex-wrap">
                      <Badge variant="outline" className="text-[10px]">
                        {activity.activity_type.replace(/_/g, ' ')}
                      </Badge>
                      <Badge variant="secondary" className="text-[10px]">
                        {activity._projectName}
                      </Badge>
                    </div>
                    {activity.subject && (
                      <p className="text-sm mt-1">{activity.subject}</p>
                    )}
                    {activity.description && (
                      <p className="text-xs text-muted-foreground mt-0.5 line-clamp-2">{activity.description}</p>
                    )}
                    <p className="text-[10px] text-muted-foreground mt-1">
                      {formatDate(activity.created_at)}
                    </p>
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

// ── Pipelines Tab ─────────────────────────────────────────────────────────────

function PipelinesTab({ orgId, defaultPipeline }: { orgId: string; defaultPipeline?: string }) {
  const [pipelineType, setPipelineType] = useState<'sales' | 'delivery'>(
    defaultPipeline === 'lifecycle' ? 'delivery' : 'sales'
  );

  return (
    <div className="space-y-4">
      <div className="flex gap-1 p-1 bg-muted/50 rounded-lg w-fit">
        <button
          onClick={() => setPipelineType('sales')}
          className={`px-4 py-1.5 text-sm font-medium rounded-md transition-all ${
            pipelineType === 'sales'
              ? 'bg-background text-foreground shadow-sm'
              : 'text-muted-foreground hover:text-foreground'
          }`}
        >
          <Target className="h-3.5 w-3.5 inline mr-1.5" />
          Acquisition
        </button>
        <button
          onClick={() => setPipelineType('delivery')}
          className={`px-4 py-1.5 text-sm font-medium rounded-md transition-all ${
            pipelineType === 'delivery'
              ? 'bg-background text-foreground shadow-sm'
              : 'text-muted-foreground hover:text-foreground'
          }`}
        >
          <TrendingUp className="h-3.5 w-3.5 inline mr-1.5" />
          Lifecycle
        </button>
      </div>
      <CrmPipelineBoard
        orgId={orgId}
        pipelineType={pipelineType as PipelineType}
        title={pipelineType === 'sales' ? 'Acquisition Pipeline' : 'Client Lifecycle'}
      />
    </div>
  );
}

// ── Create Contact Dialog ─────────────────────────────────────────────────────

function CreateContactDialog({
  open,
  onClose,
  orgId,
}: {
  open: boolean;
  onClose: () => void;
  orgId: string;
}) {
  const queryClient = useQueryClient();
  const [formData, setFormData] = useState({
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
    source: '',
    tags: '',
  });

  const resetForm = useCallback(() => {
    setFormData({
      first_name: '', last_name: '', email: '', phone: '', mobile: '',
      company_name: '', job_title: '', department: '',
      linkedin_url: '', twitter_handle: '', website: '',
      lifecycle_stage: 'lead', source: '', tags: '',
    });
  }, []);

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

  const create = useMutation({
    mutationFn: async () => {
      const parsedTags = formData.tags
        ? formData.tags.split(',').map((t) => t.trim()).filter(Boolean)
        : undefined;
      const res = await fetch(resolveApiUrl('/api/crm/contacts'), {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        credentials: 'include',
        body: JSON.stringify({
          organization_id: orgId,
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
          lifecycle_stage: formData.lifecycle_stage || undefined,
          source: formData.source || undefined,
          tags: parsedTags,
        }),
      });
      if (!res.ok) throw new Error('Failed to create contact');
      return res.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['org-contacts', orgId] });
      toast.success('Contact created');
      resetForm();
      onClose();
    },
    onError: () => toast.error('Failed to create contact'),
  });

  const handleOpenChange = useCallback((v: boolean) => { if (!v) onClose(); }, [onClose]);
  const handleCreate = useCallback(() => { create.mutate(); }, [create]);

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>New Contact</DialogTitle>
        </DialogHeader>
        <div className="grid gap-3 py-2 max-h-[60vh] overflow-y-auto pr-1">
          <div className="grid grid-cols-2 gap-3">
            <div className="grid gap-1.5">
              <Label htmlFor="ct-fn">First Name</Label>
              <Input id="ct-fn" value={formData.first_name} onChange={handleFieldChange('first_name')} placeholder="Jane" />
            </div>
            <div className="grid gap-1.5">
              <Label htmlFor="ct-ln">Last Name</Label>
              <Input id="ct-ln" value={formData.last_name} onChange={handleFieldChange('last_name')} placeholder="Doe" />
            </div>
          </div>
          <div className="grid gap-1.5">
            <Label htmlFor="ct-em">Email</Label>
            <Input id="ct-em" type="email" value={formData.email} onChange={handleFieldChange('email')} placeholder="jane@example.com" />
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div className="grid gap-1.5">
              <Label htmlFor="ct-ph">Phone</Label>
              <Input id="ct-ph" value={formData.phone} onChange={handleFieldChange('phone')} placeholder="+1 555-0100" />
            </div>
            <div className="grid gap-1.5">
              <Label htmlFor="ct-mob">Mobile</Label>
              <Input id="ct-mob" value={formData.mobile} onChange={handleFieldChange('mobile')} placeholder="+1 555-0101" />
            </div>
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div className="grid gap-1.5">
              <Label htmlFor="ct-co">Company</Label>
              <Input id="ct-co" value={formData.company_name} onChange={handleFieldChange('company_name')} placeholder="Acme Corp" />
            </div>
            <div className="grid gap-1.5">
              <Label htmlFor="ct-jt">Job Title</Label>
              <Input id="ct-jt" value={formData.job_title} onChange={handleFieldChange('job_title')} placeholder="VP of Sales" />
            </div>
          </div>
          <div className="grid gap-1.5">
            <Label htmlFor="ct-dept">Department</Label>
            <Input id="ct-dept" value={formData.department} onChange={handleFieldChange('department')} placeholder="Engineering, Sales, Marketing..." />
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div className="grid gap-1.5">
              <Label htmlFor="ct-stage">Lifecycle Stage</Label>
              <Select value={formData.lifecycle_stage} onValueChange={handleSelectChange('lifecycle_stage')}>
                <SelectTrigger><SelectValue /></SelectTrigger>
                <SelectContent>
                  {Object.entries(LIFECYCLE_STAGE_INFO).map(([key, info]) => (
                    <SelectItem key={key} value={key}>{info.label}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="grid gap-1.5">
              <Label htmlFor="ct-src">Source</Label>
              <Select value={formData.source || '__none__'} onValueChange={handleSelectChange('source')}>
                <SelectTrigger><SelectValue placeholder="Select source" /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="__none__">Not specified</SelectItem>
                  {Object.entries(CONTACT_SOURCE_INFO).map(([key, info]) => (
                    <SelectItem key={key} value={key}>{info.label}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>
          <div className="grid gap-1.5">
            <Label htmlFor="ct-li">LinkedIn URL</Label>
            <Input id="ct-li" value={formData.linkedin_url} onChange={handleFieldChange('linkedin_url')} placeholder="https://linkedin.com/in/..." />
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div className="grid gap-1.5">
              <Label htmlFor="ct-tw">Twitter Handle</Label>
              <Input id="ct-tw" value={formData.twitter_handle} onChange={handleFieldChange('twitter_handle')} placeholder="@handle" />
            </div>
            <div className="grid gap-1.5">
              <Label htmlFor="ct-web">Website</Label>
              <Input id="ct-web" value={formData.website} onChange={handleFieldChange('website')} placeholder="https://..." />
            </div>
          </div>
          <div className="grid gap-1.5">
            <Label htmlFor="ct-tags">Tags</Label>
            <Input id="ct-tags" value={formData.tags} onChange={handleFieldChange('tags')} placeholder="Comma-separated tags, e.g. vip, conference-2026" />
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>Cancel</Button>
          <Button
            disabled={(!formData.first_name.trim() && !formData.last_name.trim() && !formData.email.trim()) || create.isPending}
            onClick={handleCreate}
          >
            Create
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// ── Contacts Tab ──────────────────────────────────────────────────────────────

function ContactsTab({ orgId }: { orgId: string }) {
  const [searchParams] = useSearchParams();
  const { contacts, isLoading } = useOrgContacts(orgId);
  const [searchQuery, setSearchQuery] = useState('');
  const [stageFilter, setStageFilter] = useState<string>('all');
  const [showCreateContact, setShowCreateContact] = useState(false);
  const contactFromUrl = searchParams.get('contact');
  const [selectedContactId, setSelectedContactId] = useState<string | null>(contactFromUrl);
  const selectedContact = contacts.find(c => c.id === selectedContactId);

  const { data: personContacts = [] } = useQuery<PersonOrgContact[]>({
    queryKey: ['org-person-contacts', orgId],
    queryFn: () => organizationsApi.listPersonContacts(orgId),
    enabled: !!orgId,
    staleTime: 60_000,
  });

  // Build a lookup map: person_id → context
  const contextMap = useMemo(
    () => Object.fromEntries(personContacts.map(pc => [pc.person_id, pc.context])),
    [personContacts]
  );

  const filtered = useMemo(() => {
    let result = contacts;
    if (searchQuery) {
      const q = searchQuery.toLowerCase();
      result = result.filter(
        c =>
          (c.full_name && c.full_name.toLowerCase().includes(q)) ||
          (c.email && c.email.toLowerCase().includes(q)) ||
          (c.company_name && c.company_name.toLowerCase().includes(q))
      );
    }
    if (stageFilter !== 'all') {
      result = result.filter(c => c.lifecycle_stage === stageFilter);
    }
    return result;
  }, [contacts, searchQuery, stageFilter]);

  const stageInfo = LIFECYCLE_STAGE_INFO;

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-3">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search contacts..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9"
          />
        </div>
        <select
          value={stageFilter}
          onChange={(e) => setStageFilter(e.target.value)}
          className="h-9 rounded-md border border-input bg-background px-3 text-sm"
        >
          <option value="all">All Stages</option>
          {Object.entries(stageInfo).map(([key, info]) => (
            <option key={key} value={key}>{info.label}</option>
          ))}
        </select>
        <Button size="sm" onClick={() => setShowCreateContact(true)}>
          <Plus className="h-4 w-4 mr-1" />
          Add Contact
        </Button>
        {isLoading && (
          <span className="text-xs text-muted-foreground">Loading contacts…</span>
        )}
      </div>

      <CreateContactDialog
        open={showCreateContact}
        onClose={() => setShowCreateContact(false)}
        orgId={orgId}
      />

      {filtered.length === 0 ? (
        <div className="text-center py-12 text-muted-foreground">
          <Contact2 className="h-8 w-8 mx-auto mb-2 opacity-40" />
          <p>{isLoading ? 'Loading contacts...' : 'No contacts found'}</p>
          <Button size="sm" variant="outline" className="mt-3" onClick={() => setShowCreateContact(true)}>
            <Plus className="h-4 w-4 mr-1" />
            Add Contact
          </Button>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
          {filtered.map((contact) => (
            <ContactCard key={contact.id} contact={contact} context={contextMap[contact.id]} onClick={() => setSelectedContactId(contact.id)} />
          ))}
        </div>
      )}

      {selectedContact && (
        <ContactDetailModal
          contact={selectedContact}
          orgId={orgId}
          open={!!selectedContactId}
          onClose={() => setSelectedContactId(null)}
        />
      )}
    </div>
  );
}

const CONTEXT_COLORS: Record<string, string> = {
  client:  'bg-green-100 text-green-700',
  vendor:  'bg-orange-100 text-orange-700',
  partner: 'bg-purple-100 text-purple-700',
  prospect:'bg-blue-100 text-blue-700',
  contact: 'bg-gray-100 text-gray-600',
};

function ContactCard({ contact, context, onClick }: { contact: OrgContact; context?: string; onClick?: () => void }) {
  const stageInfo = LIFECYCLE_STAGE_INFO[contact.lifecycle_stage as LifecycleStage];

  // Check if contact was imported via workflow
  let importedViaWorkflow = false;
  if (contact.custom_fields) {
    try {
      const cf = typeof contact.custom_fields === 'string' ? JSON.parse(contact.custom_fields) : contact.custom_fields;
      importedViaWorkflow = !!cf.source_workflow_run_id;
    } catch { /* ignore */ }
  }

  return (
    <button
      type="button"
      onClick={onClick}
      className="block w-full text-left p-4 rounded-xl border border-border/50 bg-card/50 hover:bg-accent/30 hover:border-accent/50 transition-all group"
    >
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <p className="text-sm font-medium truncate group-hover:text-foreground">
            {contact.full_name || 'Unnamed'}
          </p>
          {contact.job_title && (
            <p className="text-xs text-muted-foreground truncate mt-0.5">{contact.job_title}</p>
          )}
          {contact.email && (
            <p className="text-xs text-muted-foreground truncate">{contact.email}</p>
          )}
          {contact.company_name && (
            <p className="text-xs text-muted-foreground truncate">{contact.company_name}</p>
          )}
        </div>
        {contact.lead_score > 0 && (
          <span className="text-xs font-medium px-1.5 py-0.5 rounded bg-blue-100 text-blue-700 dark:bg-blue-950 dark:text-blue-300 shrink-0">
            {contact.lead_score}
          </span>
        )}
      </div>
      <div className="flex items-center gap-2 mt-3">
        {stageInfo && (
          <Badge
            variant="secondary"
            className="text-[10px]"
            style={{ backgroundColor: stageInfo.color + '20', color: stageInfo.color }}
          >
            {stageInfo.label}
          </Badge>
        )}
        {contact.person_type && (
          <Badge variant="outline" className="text-[10px] capitalize">{contact.person_type}</Badge>
        )}
        {context && context !== 'contact' && (
          <span className={`text-[10px] px-1.5 py-0.5 rounded capitalize font-medium ${CONTEXT_COLORS[context] ?? CONTEXT_COLORS.contact}`}>
            {context}
          </span>
        )}
        {importedViaWorkflow && (
          <Badge variant="outline" className="text-[10px] gap-0.5" title="Imported via workflow">
            <GitBranch className="h-2.5 w-2.5" />
            Workflow
          </Badge>
        )}
      </div>
    </button>
  );
}

// ── Contact Detail Modal ──────────────────────────────────────────────────────

function ContactDetailModal({
  contact,
  orgId,
  open,
  onClose,
}: {
  contact: OrgContact;
  orgId: string;
  open: boolean;
  onClose: () => void;
}) {
  const queryClient = useQueryClient();
  const [isEditing, setIsEditing] = useState(false);
  const [editData, setEditData] = useState({
    first_name: contact.first_name ?? '',
    last_name: contact.last_name ?? '',
    email: contact.email ?? '',
    phone: contact.phone ?? '',
    company_name: contact.company_name ?? '',
    job_title: contact.job_title ?? '',
    department: contact.department ?? '',
    linkedin_url: contact.linkedin_url ?? '',
    website: contact.website ?? '',
  });

  const stageInfo = LIFECYCLE_STAGE_INFO[contact.lifecycle_stage as LifecycleStage];

  const { data: deals = [] } = useQuery({
    queryKey: ['contact-deals', contact.id],
    queryFn: () => crmDealsApi.listDeals({ contact_id: contact.id }),
    enabled: open,
    staleTime: 30_000,
  });

  const { data: activities = [] } = useQuery<CrmActivityRecord[]>({
    queryKey: ['contact-activities', contact.id],
    queryFn: () => crmActivitiesApi.listActivities({ organization_id: orgId, contact_id: contact.id }),
    enabled: open,
    staleTime: 30_000,
  });

  const updateMutation = useMutation({
    mutationFn: () => crmApi.updateContact(contact.id, editData),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['org-crm-contacts'] });
      setIsEditing(false);
      toast.success('Contact updated');
    },
    onError: () => toast.error('Failed to update contact'),
  });

  const deleteMutation = useMutation({
    mutationFn: () => crmApi.deleteContact(contact.id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['org-crm-contacts'] });
      onClose();
      toast.success('Contact deleted');
    },
    onError: () => toast.error('Failed to delete contact'),
  });

  return (
    <Dialog open={open} onOpenChange={(v) => !v && onClose()}>
      <DialogContent className="max-w-2xl max-h-[85vh] overflow-y-auto">
        <DialogHeader>
          <div className="flex items-center justify-between">
            <DialogTitle className="text-lg">
              {contact.full_name || `${contact.first_name ?? ''} ${contact.last_name ?? ''}`.trim() || 'Unnamed Contact'}
            </DialogTitle>
            <div className="flex items-center gap-2">
              <Button size="sm" variant="ghost" onClick={() => setIsEditing(!isEditing)}>
                <Pencil className="h-3.5 w-3.5" />
              </Button>
              <Button size="sm" variant="ghost" className="text-destructive" onClick={() => {
                if (confirm('Delete this contact?')) deleteMutation.mutate();
              }}>
                <Trash2 className="h-3.5 w-3.5" />
              </Button>
            </div>
          </div>
        </DialogHeader>

        {/* Contact Info */}
        <div className="space-y-4">
          {isEditing ? (
            <div className="grid grid-cols-2 gap-3">
              <div>
                <Label className="text-xs">First Name</Label>
                <Input value={editData.first_name} onChange={(e) => setEditData(d => ({ ...d, first_name: e.target.value }))} />
              </div>
              <div>
                <Label className="text-xs">Last Name</Label>
                <Input value={editData.last_name} onChange={(e) => setEditData(d => ({ ...d, last_name: e.target.value }))} />
              </div>
              <div>
                <Label className="text-xs">Email</Label>
                <Input value={editData.email} onChange={(e) => setEditData(d => ({ ...d, email: e.target.value }))} />
              </div>
              <div>
                <Label className="text-xs">Phone</Label>
                <Input value={editData.phone} onChange={(e) => setEditData(d => ({ ...d, phone: e.target.value }))} />
              </div>
              <div>
                <Label className="text-xs">Company</Label>
                <Input value={editData.company_name} onChange={(e) => setEditData(d => ({ ...d, company_name: e.target.value }))} />
              </div>
              <div>
                <Label className="text-xs">Job Title</Label>
                <Input value={editData.job_title} onChange={(e) => setEditData(d => ({ ...d, job_title: e.target.value }))} />
              </div>
              <div>
                <Label className="text-xs">Department</Label>
                <Input value={editData.department} onChange={(e) => setEditData(d => ({ ...d, department: e.target.value }))} />
              </div>
              <div>
                <Label className="text-xs">LinkedIn URL</Label>
                <Input value={editData.linkedin_url} onChange={(e) => setEditData(d => ({ ...d, linkedin_url: e.target.value }))} />
              </div>
              <div className="col-span-2 flex gap-2 justify-end">
                <Button size="sm" variant="outline" onClick={() => setIsEditing(false)}>Cancel</Button>
                <Button size="sm" onClick={() => updateMutation.mutate()} disabled={updateMutation.isPending}>
                  {updateMutation.isPending ? 'Saving...' : 'Save'}
                </Button>
              </div>
            </div>
          ) : (
            <div className="space-y-3">
              <div className="flex items-center gap-3 flex-wrap">
                {stageInfo && (
                  <Badge variant="secondary" style={{ backgroundColor: stageInfo.color + '20', color: stageInfo.color }}>
                    {stageInfo.label}
                  </Badge>
                )}
                {contact.lead_score > 0 && (
                  <Badge variant="outline">Score: {contact.lead_score}</Badge>
                )}
                {contact.source && (
                  <Badge variant="outline" className="capitalize">{contact.source}</Badge>
                )}
              </div>

              <div className="grid grid-cols-2 gap-2 text-sm">
                {contact.email && (
                  <div className="flex items-center gap-2 text-muted-foreground">
                    <Mail className="h-3.5 w-3.5 shrink-0" />
                    <a href={`mailto:${contact.email}`} className="truncate hover:text-foreground">{contact.email}</a>
                  </div>
                )}
                {contact.phone && (
                  <div className="flex items-center gap-2 text-muted-foreground">
                    <Phone className="h-3.5 w-3.5 shrink-0" />
                    <span className="truncate">{contact.phone}</span>
                  </div>
                )}
                {contact.company_name && (
                  <div className="flex items-center gap-2 text-muted-foreground">
                    <Building2 className="h-3.5 w-3.5 shrink-0" />
                    <span className="truncate">{contact.company_name}</span>
                  </div>
                )}
                {contact.job_title && (
                  <div className="flex items-center gap-2 text-muted-foreground">
                    <Briefcase className="h-3.5 w-3.5 shrink-0" />
                    <span className="truncate">{contact.job_title}</span>
                  </div>
                )}
                {contact.department && (
                  <div className="flex items-center gap-2 text-muted-foreground">
                    <Users className="h-3.5 w-3.5 shrink-0" />
                    <span className="truncate">{contact.department}</span>
                  </div>
                )}
                {contact.linkedin_url && (
                  <div className="flex items-center gap-2 text-muted-foreground">
                    <Linkedin className="h-3.5 w-3.5 shrink-0" />
                    <a href={contact.linkedin_url} target="_blank" rel="noreferrer" className="truncate hover:text-foreground">LinkedIn</a>
                  </div>
                )}
                {(contact.city || contact.country) && (
                  <div className="flex items-center gap-2 text-muted-foreground">
                    <MapPin className="h-3.5 w-3.5 shrink-0" />
                    <span className="truncate">{[contact.city, contact.state, contact.country].filter(Boolean).join(', ')}</span>
                  </div>
                )}
                {contact.website && (
                  <div className="flex items-center gap-2 text-muted-foreground">
                    <Globe className="h-3.5 w-3.5 shrink-0" />
                    <a href={contact.website} target="_blank" rel="noreferrer" className="truncate hover:text-foreground">{contact.website}</a>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Deals */}
          {deals.length > 0 && (
            <div>
              <h4 className="text-xs font-medium text-muted-foreground mb-2 uppercase tracking-wider">Deals ({deals.length})</h4>
              <div className="space-y-1.5">
                {deals.map((deal: { id: string; name: string; amount?: number | null; currency?: string; stage?: string }) => (
                  <div key={deal.id} className="flex items-center justify-between p-2 rounded-lg bg-muted/30 text-sm">
                    <span className="truncate">{deal.name}</span>
                    <div className="flex items-center gap-2 shrink-0">
                      {deal.amount != null && (
                        <span className="text-xs font-medium">
                          {new Intl.NumberFormat('en-US', { style: 'currency', currency: deal.currency || 'USD' }).format(deal.amount)}
                        </span>
                      )}
                      {deal.stage && (
                        <Badge variant="outline" className="text-[10px]">{deal.stage}</Badge>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Recent Activity */}
          {activities.length > 0 && (
            <div>
              <h4 className="text-xs font-medium text-muted-foreground mb-2 uppercase tracking-wider">Recent Activity</h4>
              <div className="space-y-1.5 max-h-40 overflow-y-auto">
                {activities.slice(0, 10).map((activity) => (
                  <div key={activity.id} className="flex items-center justify-between p-2 rounded-lg bg-muted/30 text-sm">
                    <div className="flex items-center gap-2 min-w-0">
                      <Activity className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                      <span className="truncate">{activity.subject || activity.activity_type}</span>
                    </div>
                    <span className="text-[10px] text-muted-foreground shrink-0">
                      {new Date(activity.activity_at).toLocaleDateString()}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Metadata */}
          <div className="text-[10px] text-muted-foreground pt-2 border-t border-border/50 flex items-center justify-between">
            <span>Created {new Date(contact.created_at).toLocaleDateString()}</span>
            {contact.last_activity_at && (
              <span>Last active {new Date(contact.last_activity_at).toLocaleDateString()}</span>
            )}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}

// ── Companies Tab ─────────────────────────────────────────────────────────────

interface CompanyRecord {
  id: string;
  name: string;
  domain?: string;
  industry?: string;
  size?: string;
  description?: string;
  phone?: string;
  email?: string;
  city?: string;
  country?: string;
  linkedin_url?: string;
  tags?: string;
  created_at?: string;
}

function CreateCompanyInlineDialog({
  open,
  onClose,
  orgId,
  onCreated,
}: {
  open: boolean;
  onClose: () => void;
  orgId: string;
  onCreated: () => void;
}) {
  const [formData, setFormData] = useState({
    name: '',
    website: '',
    industry: '',
    description: '',
    headquarters: '',
    phone: '',
    email: '',
    linkedin_url: '',
    twitter_handle: '',
    instagram_handle: '',
  });

  const resetForm = useCallback(() => {
    setFormData({
      name: '',
      website: '',
      industry: '',
      description: '',
      headquarters: '',
      phone: '',
      email: '',
      linkedin_url: '',
      twitter_handle: '',
      instagram_handle: '',
    });
  }, []);

  const create = useMutation({
    mutationFn: () =>
      companiesApi.create({
        name: formData.name,
        website: formData.website || undefined,
        industry: formData.industry || undefined,
        description: formData.description || undefined,
        headquarters: formData.headquarters || undefined,
        created_by_org_id: orgId,
      }),
    onSuccess: (company) => {
      const extraFields: Record<string, string> = {};
      if (formData.phone) extraFields.phone = formData.phone;
      if (formData.email) extraFields.email = formData.email;
      if (formData.linkedin_url) extraFields.linkedin_url = formData.linkedin_url;
      if (formData.twitter_handle) extraFields.twitter_handle = formData.twitter_handle;
      if (formData.instagram_handle) extraFields.instagram_handle = formData.instagram_handle;

      if (Object.keys(extraFields).length > 0) {
        companiesApi.update(company.id, extraFields).catch(() => {});
      }

      onCreated();
      toast.success('Company created');
      resetForm();
      onClose();
    },
    onError: () => toast.error('Failed to create company'),
  });

  const handleFieldChange = useCallback(
    (field: string) => (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
      setFormData((prev) => ({ ...prev, [field]: e.target.value }));
    },
    [],
  );

  const handleOpenChange = useCallback(
    (v: boolean) => { if (!v) onClose(); },
    [onClose],
  );

  const handleSubmit = useCallback(() => {
    create.mutate();
  }, [create]);

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="sm:max-w-[550px]">
        <DialogHeader>
          <DialogTitle>New Company</DialogTitle>
        </DialogHeader>
        <div className="grid gap-3 py-2 max-h-[60vh] overflow-y-auto pr-1">
          <div className="grid gap-1.5">
            <Label htmlFor="co-name">Name *</Label>
            <Input
              id="co-name"
              value={formData.name}
              onChange={handleFieldChange('name')}
              placeholder="Acme Corp"
            />
          </div>
          <div className="grid gap-4 md:grid-cols-2">
            <div className="grid gap-1.5">
              <Label htmlFor="co-web">Website</Label>
              <Input
                id="co-web"
                value={formData.website}
                onChange={handleFieldChange('website')}
                placeholder="https://example.com"
              />
            </div>
            <div className="grid gap-1.5">
              <Label htmlFor="co-ind">Industry</Label>
              <Input
                id="co-ind"
                value={formData.industry}
                onChange={handleFieldChange('industry')}
                placeholder="Technology"
              />
            </div>
          </div>
          <div className="grid gap-1.5">
            <Label htmlFor="co-desc">Description</Label>
            <Textarea
              id="co-desc"
              value={formData.description}
              onChange={handleFieldChange('description')}
              placeholder="Brief description of the company..."
              rows={2}
            />
          </div>
          <div className="grid gap-1.5">
            <Label htmlFor="co-hq">Headquarters</Label>
            <Input
              id="co-hq"
              value={formData.headquarters}
              onChange={handleFieldChange('headquarters')}
              placeholder="Miami, FL"
            />
          </div>
          <div className="grid gap-4 md:grid-cols-2">
            <div className="grid gap-1.5">
              <Label htmlFor="co-phone">Phone</Label>
              <Input
                id="co-phone"
                value={formData.phone}
                onChange={handleFieldChange('phone')}
                placeholder="+1 555-0100"
              />
            </div>
            <div className="grid gap-1.5">
              <Label htmlFor="co-email">Email</Label>
              <Input
                id="co-email"
                type="email"
                value={formData.email}
                onChange={handleFieldChange('email')}
                placeholder="info@company.com"
              />
            </div>
          </div>
          <div className="grid gap-4 md:grid-cols-3">
            <div className="grid gap-1.5">
              <Label htmlFor="co-li">LinkedIn</Label>
              <Input
                id="co-li"
                value={formData.linkedin_url}
                onChange={handleFieldChange('linkedin_url')}
                placeholder="linkedin.com/company/..."
              />
            </div>
            <div className="grid gap-1.5">
              <Label htmlFor="co-tw">Twitter</Label>
              <Input
                id="co-tw"
                value={formData.twitter_handle}
                onChange={handleFieldChange('twitter_handle')}
                placeholder="@handle"
              />
            </div>
            <div className="grid gap-1.5">
              <Label htmlFor="co-ig">Instagram</Label>
              <Input
                id="co-ig"
                value={formData.instagram_handle}
                onChange={handleFieldChange('instagram_handle')}
                placeholder="@handle"
              />
            </div>
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>Cancel</Button>
          <Button
            disabled={!formData.name.trim() || create.isPending}
            onClick={handleSubmit}
          >
            Create
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function CompaniesTab({ orgId }: { orgId: string }) {
  const [searchQuery, setSearchQuery] = useState('');
  const [industryFilter, setIndustryFilter] = useState('all');
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const queryClient = useQueryClient();

  const { data: companies = [], isLoading } = useQuery<CompanyRecord[]>({
    queryKey: ['org-companies', orgId],
    queryFn: async () => {
      const res = await fetch(resolveApiUrl(`/api/organizations/${orgId}/companies`), { credentials: 'include' });
      if (!res.ok) return [];
      const data = await res.json();
      return Array.isArray(data) ? data : (data?.data ?? []);
    },
    staleTime: 30_000,
  });

  const industries = useMemo(() => {
    const set = new Set<string>();
    companies.forEach(c => { if (c.industry) set.add(c.industry); });
    return Array.from(set).sort();
  }, [companies]);

  const filtered = useMemo(() => {
    let result = companies;
    if (searchQuery) {
      const q = searchQuery.toLowerCase();
      result = result.filter(c =>
        c.name.toLowerCase().includes(q) ||
        (c.domain && c.domain.toLowerCase().includes(q)) ||
        (c.industry && c.industry.toLowerCase().includes(q))
      );
    }
    if (industryFilter !== 'all') {
      result = result.filter(c => c.industry === industryFilter);
    }
    return result;
  }, [companies, searchQuery, industryFilter]);

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-3">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search companies..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9"
          />
        </div>
        {industries.length > 0 && (
          <select
            value={industryFilter}
            onChange={(e) => setIndustryFilter(e.target.value)}
            className="h-9 rounded-md border border-input bg-background px-3 text-sm"
          >
            <option value="all">All Industries</option>
            {industries.map(ind => (
              <option key={ind} value={ind}>{ind}</option>
            ))}
          </select>
        )}
        {isLoading && (
          <span className="text-xs text-muted-foreground">Loading companies...</span>
        )}
        <Button size="sm" onClick={() => setShowCreateDialog(true)}>
          <Plus className="h-4 w-4 mr-1" />
          Add Company
        </Button>
      </div>

      <CreateCompanyInlineDialog
        open={showCreateDialog}
        onClose={() => setShowCreateDialog(false)}
        orgId={orgId}
        onCreated={() => queryClient.invalidateQueries({ queryKey: ['org-companies', orgId] })}
      />

      {filtered.length === 0 ? (
        <div className="text-center py-12 text-muted-foreground">
          <Building2 className="h-8 w-8 mx-auto mb-2 opacity-40" />
          <p>{isLoading ? 'Loading companies...' : 'No companies found'}</p>
          <p className="text-xs mt-1">Add a company manually or let workflow pipelines extract them automatically.</p>
          <Button size="sm" variant="outline" className="mt-3" onClick={() => setShowCreateDialog(true)}>
            <Plus className="h-4 w-4 mr-1" />
            Add Company
          </Button>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
          {filtered.map((company) => (
            <div
              key={company.id}
              className="p-4 rounded-xl border border-border/50 bg-card/50 hover:bg-accent/30 hover:border-accent/50 transition-all"
            >
              <div className="flex items-start justify-between gap-2">
                <div className="min-w-0">
                  <p className="text-sm font-medium truncate">{company.name}</p>
                  {company.domain && (
                    <p className="text-xs text-muted-foreground truncate mt-0.5">{company.domain}</p>
                  )}
                  {company.email && (
                    <p className="text-xs text-muted-foreground truncate">{company.email}</p>
                  )}
                </div>
                {company.size && (
                  <Badge variant="outline" className="text-[10px] shrink-0">{company.size}</Badge>
                )}
              </div>
              <div className="flex items-center gap-2 mt-3 flex-wrap">
                {company.industry && (
                  <Badge variant="secondary" className="text-[10px]">{company.industry}</Badge>
                )}
                {company.city && (
                  <span className="text-[10px] text-muted-foreground">{company.city}{company.country ? `, ${company.country}` : ''}</span>
                )}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// ── Deliverables Tab ─────────────────────────────────────────────────────────

function DeliverablesTab({
  projectEntries,
}: {
  orgId: string;
  projectEntries: { id: string; name: string }[];
}) {
  const deliverableQueries = useQueries({
    queries: projectEntries.map((entry) => ({
      queryKey: ['deliverables', entry.id],
      queryFn: async () => {
        const res = await fetch(resolveApiUrl(`/api/projects/${entry.id}/deliverables`), { credentials: 'include' });
        if (!res.ok) return [];
        const data = await res.json();
        const items = Array.isArray(data) ? data : (data?.data ?? []);
        return items.map((d: any) => ({ ...d, _projectName: entry.name }));
      },
      staleTime: 30_000,
    })),
  });

  const isLoading = deliverableQueries.some((q) => q.isLoading);
  const allDeliverables = deliverableQueries.flatMap((q) => q.data ?? []);

  const statusGroups = useMemo(() => {
    const groups: Record<string, typeof allDeliverables> = {
      draft: [],
      in_progress: [],
      review: [],
      delivered: [],
    };
    allDeliverables.forEach((d) => {
      const status = d.status || 'draft';
      if (!groups[status]) groups[status] = [];
      groups[status].push(d);
    });
    return groups;
  }, [allDeliverables]);

  const statusLabels: Record<string, { label: string; color: string }> = {
    draft: { label: 'Draft', color: 'text-muted-foreground' },
    in_progress: { label: 'In Progress', color: 'text-blue-500' },
    review: { label: 'In Review', color: 'text-amber-500' },
    delivered: { label: 'Delivered', color: 'text-green-500' },
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12 text-muted-foreground">
        <Loader2 className="h-5 w-5 animate-spin mr-2" />
        Loading deliverables...
      </div>
    );
  }

  if (allDeliverables.length === 0) {
    return (
      <div className="text-center py-16 text-muted-foreground">
        <Boxes className="h-10 w-10 mx-auto mb-3 opacity-30" />
        <p className="font-medium text-foreground mb-1">No deliverables yet</p>
        <p className="text-sm max-w-md mx-auto">
          Deliverables are tangible outputs produced for clients — reports, designs, assets, or completed work products.
          Create them from individual project pages.
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold">{allDeliverables.length} Deliverables</h2>
          <p className="text-sm text-muted-foreground">Across {projectEntries.length} projects</p>
        </div>
      </div>

      {Object.entries(statusGroups).map(([status, items]) => {
        if (items.length === 0) return null;
        const info = statusLabels[status] || { label: status, color: 'text-muted-foreground' };
        return (
          <div key={status}>
            <div className="flex items-center gap-2 mb-2">
              <span className={`text-sm font-medium ${info.color}`}>{info.label}</span>
              <Badge variant="secondary" className="text-[10px]">{items.length}</Badge>
            </div>
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
              {items.map((d: any) => (
                <Card key={d.id} className="p-4">
                  <div className="flex items-start justify-between gap-2">
                    <div className="min-w-0">
                      <p className="text-sm font-medium truncate">{d.title}</p>
                      {d.description && (
                        <p className="text-xs text-muted-foreground mt-0.5 line-clamp-2">{d.description}</p>
                      )}
                    </div>
                    {d.deliverable_type && (
                      <Badge variant="outline" className="text-[10px] shrink-0 capitalize">{d.deliverable_type}</Badge>
                    )}
                  </div>
                  <div className="flex items-center gap-2 mt-3 text-[10px] text-muted-foreground">
                    <FolderOpen className="h-3 w-3" />
                    <span>{d._projectName}</span>
                    {d.due_date && (
                      <>
                        <span className="mx-1">·</span>
                        <Clock className="h-3 w-3" />
                        <span>Due {formatDate(d.due_date)}</span>
                      </>
                    )}
                  </div>
                </Card>
              ))}
            </div>
          </div>
        );
      })}
    </div>
  );
}

// ── Projects Tab ──────────────────────────────────────────────────────────────

function ProjectsTab({
  orgId: _orgId,
  sidebarOrg,
  clientFilter,
  onClearClientFilter,
}: {
  orgId: string;
  sidebarOrg: any;
  clientFilter: string | null;
  onClearClientFilter: () => void;
}) {
  if (!sidebarOrg) {
    return (
      <div className="text-center py-12 text-muted-foreground">
        <FolderOpen className="h-8 w-8 mx-auto mb-2 opacity-40" />
        <p>No projects found</p>
      </div>
    );
  }

  const filteredClient = clientFilter
    ? sidebarOrg.clients?.find((c: any) => c.id === clientFilter)
    : null;

  return (
    <div className="space-y-4">
      {filteredClient && (
        <div className="flex items-center gap-2 p-3 rounded-lg bg-accent/30 border border-accent/50">
          <Briefcase className="h-4 w-4 text-purple-500" />
          <span className="text-sm font-medium">Viewing: {filteredClient.name}</span>
          <button
            onClick={onClearClientFilter}
            className="ml-auto p-1 rounded hover:bg-accent"
          >
            <X className="h-3.5 w-3.5" />
          </button>
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        {/* Internal projects (hidden if client filter active) */}
        {!clientFilter && (
          <Card className="bg-card/80 backdrop-blur-sm border-border/50">
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
                  {sidebarOrg.internal_projects?.map((p: any) => (
                    <ProjectRow key={p.id} project={p} />
                  ))}
                  {(sidebarOrg.internal_folders || []).map((folder: any) =>
                    folder.projects.map((p: any) => (
                      <ProjectRow key={p.id} project={p} folderName={folder.name} />
                    ))
                  )}
                </div>
              )}
            </CardContent>
          </Card>
        )}

        {/* Client projects */}
        {(clientFilter ? [filteredClient].filter(Boolean) : sidebarOrg.clients || []).map((client: any) => (
          <Card key={client.id} className="bg-card/80 backdrop-blur-sm border-border/50">
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
                  {client.projects?.map((p: any) => (
                    <ProjectRow key={p.id} project={p} />
                  ))}
                  {(client.folders || []).map((folder: any) =>
                    folder.projects.map((p: any) => (
                      <ProjectRow key={p.id} project={p} folderName={folder.name} />
                    ))
                  )}
                </div>
              )}
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}

function ProjectRow({ project, folderName }: { project: any; folderName?: string }) {
  return (
    <Link
      to={`/projects/${project.id}`}
      className="flex items-center justify-between p-2 rounded-md hover:bg-muted transition-colors"
    >
      <div className="flex items-center gap-2 min-w-0">
        <FolderOpen className="h-4 w-4 text-muted-foreground shrink-0" />
        <span className="text-sm font-medium truncate">{project.name}</span>
        {folderName && <Badge variant="secondary" className="text-xs shrink-0">{folderName}</Badge>}
      </div>
      <ExternalLink className="h-3 w-3 text-muted-foreground shrink-0" />
    </Link>
  );
}

// ── Knowledge Tab ─────────────────────────────────────────────────────────────

const SOURCE_TYPE_META: Record<string, { label: string; icon: typeof BookOpen }> = {
  conversation: { label: 'Conversations', icon: MessageSquare },
  artifact: { label: 'Artifacts', icon: FileText },
  pulse_content: { label: 'Pulse Content', icon: Radio },
  context_injection: { label: 'Context Injections', icon: Pencil },
  entity: { label: 'Entities', icon: Boxes },
  topology_snapshot: { label: 'Topology Snapshots', icon: Network },
};

// ── Add Data Source Dialog ────────────────────────────────────────────────────

function AddDataSourceDialog({
  orgId,
  projectEntries,
  open,
  onOpenChange,
  editingSource,
}: {
  orgId: string;
  projectEntries: { id: string; name: string }[];
  open: boolean;
  onOpenChange: (open: boolean) => void;
  editingSource?: DataSourceRecord | null;
}) {
  const queryClient = useQueryClient();
  const isEdit = !!editingSource;

  const [selectedProject, setSelectedProject] = useState<string>(editingSource?.project_id ?? '__none__');
  const [sourceType, setSourceType] = useState<string>(editingSource?.source_type ?? 'text');
  const [dataType, setDataType] = useState<string>(editingSource?.data_type ?? 'conversation');
  const [title, setTitle] = useState(editingSource?.title ?? '');
  const [description, setDescription] = useState(editingSource?.description ?? '');
  const [content, setContent] = useState(editingSource?.content ?? '');
  const [file, setFile] = useState<File | null>(null);

  // Reset form when dialog opens/closes or editingSource changes
  const resetForm = () => {
    setSelectedProject(editingSource?.project_id ?? '__none__');
    setSourceType(editingSource?.source_type ?? 'text');
    setDataType(editingSource?.data_type ?? 'conversation');
    setTitle(editingSource?.title ?? '');
    setDescription(editingSource?.description ?? '');
    setContent(editingSource?.content ?? '');
    setFile(null);
  };

  const createMutation = useMutation({
    mutationFn: async () => {
      const projId = selectedProject !== '__none__' ? selectedProject : undefined;
      if (sourceType === 'file' && file) {
        const formData = new FormData();
        formData.append('file', file);
        formData.append('title', title.trim());
        formData.append('data_type', dataType);
        if (description.trim()) formData.append('description', description.trim());
        if (projId) formData.append('project_id', projId);
        formData.append('organization_id', orgId);
        return dataSourcesApi.upload(formData);
      }
      return dataSourcesApi.create({
        organization_id: orgId,
        project_id: projId,
        title: title.trim(),
        description: description.trim() || undefined,
        data_type: dataType,
        source_type: sourceType,
        content: sourceType === 'text' && content.trim() ? content.trim() : undefined,
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['dataSources', orgId] });
      onOpenChange(false);
      resetForm();
    },
  });

  const updateMutation = useMutation({
    mutationFn: (data: UpdateDataSourceRequest) =>
      dataSourcesApi.update(editingSource!.id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['dataSources', orgId] });
      onOpenChange(false);
    },
  });

  const handleSubmit = () => {
    if (!title.trim()) return;
    if (isEdit) {
      updateMutation.mutate({
        title: title.trim(),
        description: description.trim() || undefined,
        data_type: dataType,
        source_type: sourceType,
        content: sourceType === 'text' && content.trim() ? content.trim() : undefined,
      });
    } else {
      createMutation.mutate();
    }
  };

  const isPending = createMutation.isPending || updateMutation.isPending;

  return (
    <Dialog open={open} onOpenChange={(v) => { onOpenChange(v); if (!v) resetForm(); }}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Database className="h-4 w-4" />
            {isEdit ? 'Edit Data Source' : 'Add Data Source'}
          </DialogTitle>
        </DialogHeader>
        <div className="space-y-4 py-2">
          {/* Source type radio */}
          <div className="space-y-2">
            <Label>Source</Label>
            <RadioGroup
              value={sourceType}
              onValueChange={setSourceType}
              className="flex gap-4"
            >
              <div className="flex items-center gap-1.5">
                <RadioGroupItem value="text" id="st-text" />
                <Label htmlFor="st-text" className="text-sm font-normal cursor-pointer">Text</Label>
              </div>
              <div className="flex items-center gap-1.5">
                <RadioGroupItem value="file" id="st-file" />
                <Label htmlFor="st-file" className="text-sm font-normal cursor-pointer">File Upload</Label>
              </div>
              <div className="flex items-center gap-1.5">
                <RadioGroupItem value="integration" id="st-integration" />
                <Label htmlFor="st-integration" className="text-sm font-normal cursor-pointer">Integration</Label>
              </div>
            </RadioGroup>
          </div>

          <div className="space-y-2">
            <Label>Title</Label>
            <Input
              placeholder="e.g. Client kickoff call notes"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              autoFocus
            />
          </div>

          <div className="space-y-2">
            <Label>Data Type</Label>
            <Select value={dataType} onValueChange={setDataType}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {DATA_TYPE_OPTIONS.map((opt) => (
                  <SelectItem key={opt.value} value={opt.value}>{opt.label}</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-2">
            <Label>Description (optional)</Label>
            <Textarea
              placeholder="Brief description of this data source..."
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              rows={2}
            />
          </div>

          {/* Conditional input based on source type */}
          {sourceType === 'text' && (
            <div className="space-y-2">
              <Label>Content</Label>
              <Textarea
                placeholder="Paste or type your content here..."
                value={content}
                onChange={(e) => setContent(e.target.value)}
                rows={6}
                className="font-mono text-xs"
              />
            </div>
          )}
          {sourceType === 'file' && !isEdit && (
            <div className="space-y-2">
              <Label>File</Label>
              <div className="flex items-center gap-2">
                <Input
                  type="file"
                  onChange={(e) => setFile(e.target.files?.[0] ?? null)}
                  className="text-xs"
                />
                {file && (
                  <span className="text-xs text-muted-foreground shrink-0">
                    {(file.size / 1024).toFixed(0)} KB
                  </span>
                )}
              </div>
            </div>
          )}
          {sourceType === 'integration' && (
            <div className="rounded-md bg-muted/50 p-3 text-xs text-muted-foreground">
              Integration sources are populated automatically from connected services.
            </div>
          )}

          {projectEntries.length > 0 && (
            <div className="space-y-2">
              <Label>Project (optional)</Label>
              <Select value={selectedProject} onValueChange={setSelectedProject}>
                <SelectTrigger>
                  <SelectValue placeholder="No project (org-level)" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="__none__">No project (org-level)</SelectItem>
                  {projectEntries.map((p) => (
                    <SelectItem key={p.id} value={p.id}>{p.name}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          )}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>Cancel</Button>
          <Button onClick={handleSubmit} disabled={!title.trim() || isPending}>
            {isPending ? (isEdit ? 'Saving...' : 'Adding...') : (isEdit ? 'Save' : 'Add Source')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// ── Delete Confirmation Dialog ───────────────────────────────────────────────

function DeleteConfirmDialog({
  open,
  onOpenChange,
  sourceName,
  onConfirm,
  isPending,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  sourceName: string;
  onConfirm: () => void;
  isPending: boolean;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-sm">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2 text-destructive">
            <Trash2 className="h-4 w-4" />
            Delete Data Source
          </DialogTitle>
        </DialogHeader>
        <p className="text-sm text-muted-foreground">
          Are you sure you want to delete <strong>{sourceName}</strong>? This action cannot be undone.
        </p>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>Cancel</Button>
          <Button variant="destructive" onClick={onConfirm} disabled={isPending}>
            {isPending ? 'Deleting...' : 'Delete'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// ── Data Sources View ────────────────────────────────────────────────────────

type SortField = 'title' | 'data_type' | 'status' | 'created_at';
type SortDir = 'asc' | 'desc';

function DataSourcesView({
  orgId,
  projectEntries,
}: {
  orgId: string;
  projectEntries: { id: string; name: string }[];
}) {
  const queryClient = useQueryClient();
  const [addOpen, setAddOpen] = useState(false);
  const [editingSource, setEditingSource] = useState<DataSourceRecord | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<DataSourceRecord | null>(null);
  const [sortField, setSortField] = useState<SortField>('created_at');
  const [sortDir, setSortDir] = useState<SortDir>('desc');

  const { data: sources = [], isLoading } = useQuery({
    queryKey: ['dataSources', orgId],
    queryFn: () => dataSourcesApi.listByOrganization(orgId),
    staleTime: 30_000,
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => dataSourcesApi.delete(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['dataSources', orgId] });
      setDeleteTarget(null);
    },
  });

  const toggleSort = (field: SortField) => {
    if (sortField === field) {
      setSortDir(d => d === 'asc' ? 'desc' : 'asc');
    } else {
      setSortField(field);
      setSortDir('asc');
    }
  };

  const SortIcon = ({ field }: { field: SortField }) => {
    if (sortField !== field) return <ArrowUpDown className="h-3 w-3 opacity-40" />;
    return sortDir === 'asc' ? <ArrowUp className="h-3 w-3" /> : <ArrowDown className="h-3 w-3" />;
  };

  const sorted = useMemo(() => {
    const arr = [...sources];
    arr.sort((a, b) => {
      let cmp = 0;
      switch (sortField) {
        case 'title': cmp = a.title.localeCompare(b.title); break;
        case 'data_type': cmp = a.data_type.localeCompare(b.data_type); break;
        case 'status': cmp = a.status.localeCompare(b.status); break;
        case 'created_at': cmp = a.created_at.localeCompare(b.created_at); break;
      }
      return sortDir === 'asc' ? cmp : -cmp;
    });
    return arr;
  }, [sources, sortField, sortDir]);

  const dataTypeLabel = (dt: string) =>
    DATA_TYPE_OPTIONS.find(o => o.value === dt)?.label ?? dt;

  const formatFileSize = (bytes?: number) => {
    if (!bytes) return '';
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const formatDate = (iso: string) => {
    const d = new Date(iso);
    return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
  };

  const statusBadge = (status: string) => {
    const variants: Record<string, string> = {
      ready: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400',
      pending: 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400',
      processing: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400',
      error: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
    };
    return (
      <span className={`inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium ${variants[status] ?? 'bg-muted text-muted-foreground'}`}>
        {status}
      </span>
    );
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Database className="h-5 w-5 text-muted-foreground" />
          <h2 className="text-lg font-semibold">Data Sources</h2>
          <Badge variant="secondary">{sources.length}</Badge>
        </div>
        <Button size="sm" onClick={() => { setEditingSource(null); setAddOpen(true); }} className="gap-1">
          <Plus className="h-3.5 w-3.5" />
          Add Data Source
        </Button>
      </div>

      {isLoading ? (
        <div className="text-center py-12 text-muted-foreground text-sm">Loading data sources...</div>
      ) : sorted.length === 0 ? (
        <div className="text-center py-12 text-muted-foreground">
          <Database className="h-8 w-8 mx-auto mb-2 opacity-40" />
          <p>No data sources yet</p>
          <Button variant="outline" size="sm" className="mt-4 gap-1" onClick={() => setAddOpen(true)}>
            <Plus className="h-3.5 w-3.5" />
            Add your first data source
          </Button>
        </div>
      ) : (
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="cursor-pointer select-none" onClick={() => toggleSort('title')}>
                  <span className="flex items-center gap-1">Title <SortIcon field="title" /></span>
                </TableHead>
                <TableHead className="cursor-pointer select-none w-[120px]" onClick={() => toggleSort('data_type')}>
                  <span className="flex items-center gap-1">Type <SortIcon field="data_type" /></span>
                </TableHead>
                <TableHead className="w-[100px]">Source</TableHead>
                <TableHead className="cursor-pointer select-none w-[80px]" onClick={() => toggleSort('status')}>
                  <span className="flex items-center gap-1">Status <SortIcon field="status" /></span>
                </TableHead>
                <TableHead className="cursor-pointer select-none w-[110px]" onClick={() => toggleSort('created_at')}>
                  <span className="flex items-center gap-1">Created <SortIcon field="created_at" /></span>
                </TableHead>
                <TableHead className="w-[40px]" />
              </TableRow>
            </TableHeader>
            <TableBody>
              {sorted.map((source) => (
                <TableRow key={source.id}>
                  <TableCell>
                    <div className="min-w-0">
                      <Link to={`/organizations/${orgId}/data-sources/${source.id}`} className="text-sm font-medium truncate hover:underline block">{source.title}</Link>
                      {source.description && (
                        <p className="text-xs text-muted-foreground truncate mt-0.5">{source.description}</p>
                      )}
                    </div>
                  </TableCell>
                  <TableCell>
                    <Badge variant="outline" className="text-[10px]">{dataTypeLabel(source.data_type)}</Badge>
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-1 text-xs text-muted-foreground">
                      {source.source_type === 'file' && source.file_name ? (
                        <>
                          <Upload className="h-3 w-3 shrink-0" />
                          <span className="truncate max-w-[60px]">{source.file_name}</span>
                          {source.file_size_bytes != null && (
                            <span className="shrink-0">({formatFileSize(source.file_size_bytes)})</span>
                          )}
                        </>
                      ) : source.source_type === 'file' ? (
                        <>
                          <Upload className="h-3 w-3 shrink-0" />
                          <span>File</span>
                        </>
                      ) : source.source_type === 'text' ? (
                        <>
                          <FileText className="h-3 w-3 shrink-0" />
                          <span>Text</span>
                        </>
                      ) : (
                        <>
                          <Database className="h-3 w-3 shrink-0" />
                          <span>Integration</span>
                        </>
                      )}
                    </div>
                  </TableCell>
                  <TableCell>{statusBadge(source.status)}</TableCell>
                  <TableCell className="text-xs text-muted-foreground">{formatDate(source.created_at)}</TableCell>
                  <TableCell>
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <Button variant="ghost" size="sm" className="h-7 w-7 p-0">
                          <MoreHorizontal className="h-3.5 w-3.5" />
                        </Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuItem onClick={() => { setEditingSource(source); setAddOpen(true); }}>
                          <Pencil className="h-3.5 w-3.5 mr-2" />
                          Edit
                        </DropdownMenuItem>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem
                          className="text-destructive focus:text-destructive"
                          onClick={() => setDeleteTarget(source)}
                        >
                          <Trash2 className="h-3.5 w-3.5 mr-2" />
                          Delete
                        </DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </Card>
      )}

      <AddDataSourceDialog
        orgId={orgId}
        projectEntries={projectEntries}
        open={addOpen}
        onOpenChange={setAddOpen}
        editingSource={editingSource}
      />

      <DeleteConfirmDialog
        open={!!deleteTarget}
        onOpenChange={(v) => { if (!v) setDeleteTarget(null); }}
        sourceName={deleteTarget?.title ?? ''}
        onConfirm={() => deleteTarget && deleteMutation.mutate(deleteTarget.id)}
        isPending={deleteMutation.isPending}
      />
    </div>
  );
}

// ── Artifacts View ──────────────────────────────────────────────────────────

function ArtifactsView({ orgId }: { orgId: string }) {
  const [expandedIds, setExpandedIds] = useState<Set<string>>(new Set());

  const { data: artifacts = [], isLoading } = useQuery({
    queryKey: ['recentArtifacts', orgId],
    queryFn: () => workflowsApi.listRecentArtifacts(orgId),
  });

  const toggleExpand = (id: string) => {
    setExpandedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const parseContent = (content?: string) => {
    if (!content) return null;
    try { return JSON.parse(content); } catch { return content; }
  };

  const renderValue = (val: any): React.ReactNode => {
    if (val === null || val === undefined) return <span className="text-muted-foreground italic">null</span>;
    if (typeof val === 'string') return <span className="text-sm">{val}</span>;
    if (typeof val === 'number' || typeof val === 'boolean') return <span className="text-sm font-mono">{String(val)}</span>;
    if (Array.isArray(val)) {
      return (
        <div className="ml-3 space-y-1">
          {val.map((item, i) => (
            <div key={i} className="text-sm border-l-2 border-border/50 pl-2">
              {typeof item === 'object' ? renderValue(item) : String(item)}
            </div>
          ))}
        </div>
      );
    }
    if (typeof val === 'object') {
      return (
        <div className="ml-3 space-y-1">
          {Object.entries(val).map(([k, v]) => (
            <div key={k}>
              <span className="text-xs font-medium text-muted-foreground uppercase tracking-wide">{k.replace(/_/g, ' ')}: </span>
              {typeof v === 'object' && v !== null ? renderValue(v) : <span className="text-sm">{String(v ?? '')}</span>}
            </div>
          ))}
        </div>
      );
    }
    return <span>{String(val)}</span>;
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-48">
        <div className="text-sm text-muted-foreground">Loading artifacts...</div>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2">
        <FileText className="h-5 w-5 text-muted-foreground" />
        <h2 className="text-lg font-semibold">Artifacts</h2>
        <Badge variant="secondary">{artifacts.length}</Badge>
      </div>

      {artifacts.length === 0 ? (
        <div className="text-center py-12 text-muted-foreground">
          <FileText className="h-8 w-8 mx-auto mb-2 opacity-40" />
          <p className="text-sm">No artifacts generated yet.</p>
          <p className="text-xs mt-1">Run a workflow on a data source to generate artifacts.</p>
        </div>
      ) : (
        <div className="space-y-2">
          {artifacts.map((artifact: ExecutionArtifact) => {
            const isExpanded = expandedIds.has(artifact.id);
            const meta = artifact.metadata ? (() => { try { return JSON.parse(artifact.metadata); } catch { return {}; } })() : {};
            const content = parseContent(artifact.content ?? undefined);

            return (
              <Card key={artifact.id} className="bg-card/80 border-border/50">
                <button
                  className="flex items-center justify-between w-full p-4 text-left hover:bg-muted/30 transition-colors"
                  onClick={() => toggleExpand(artifact.id)}
                >
                  <div className="flex items-center gap-3 min-w-0">
                    {isExpanded ? <ChevronDown className="h-4 w-4 text-muted-foreground shrink-0" /> : <ChevronRight className="h-4 w-4 text-muted-foreground shrink-0" />}
                    <div className="min-w-0">
                      <p className="text-sm font-medium truncate">{artifact.title || 'Untitled Artifact'}</p>
                      <div className="flex items-center gap-2 mt-0.5">
                        <Badge variant="outline" className="text-[10px]">{artifact.artifact_type}</Badge>
                        {meta.step_id && <span className="text-xs text-muted-foreground">{meta.step_id.replace(/_/g, ' ')}</span>}
                      </div>
                    </div>
                  </div>
                  <span className="text-xs text-muted-foreground shrink-0 ml-2 flex items-center gap-1">
                    <Clock className="h-3 w-3" />
                    {new Date(artifact.created_at).toLocaleDateString('en-US', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })}
                  </span>
                </button>
                {isExpanded && (
                  <div className="px-4 pb-4 border-t">
                    <div className="mt-3">
                      {content ? renderValue(content) : <p className="text-sm text-muted-foreground italic">No content.</p>}
                    </div>
                    {meta.data_source_id && (
                      <div className="mt-3 pt-2 border-t border-border/30">
                        <Link
                          to={`/organizations/${orgId}/data-sources/${meta.data_source_id}`}
                          className="text-xs text-blue-600 hover:underline"
                        >
                          View source data source
                        </Link>
                      </div>
                    )}
                  </div>
                )}
              </Card>
            );
          })}
        </div>
      )}
    </div>
  );
}

// ── Workflows Management View ───────────────────────────────────────────────

interface PipelineNode {
  id: string;
  label: string;
  agent?: string;
  type: 'agent' | 'human' | 'parallel' | 'tool';
  description: string;
  tools?: string[];
  parallel?: boolean;
}

interface PipelineBlueprint {
  id: string;
  name: string;
  description: string;
  category: string;
  color: string;
  nodes: PipelineNode[];
}

const CONFERENCE_PIPELINE: PipelineBlueprint = {
  id: 'conference_research',
  name: 'Conference Research',
  description: 'Full pipeline: conference intel → speaker/brand/side-event research (parallel) → article writing → QA → social publishing.',
  category: 'Research',
  color: 'blue',
  nodes: [
    { id: 'conf_intel', label: 'Conference Intel', agent: 'Scout', type: 'agent', description: 'Research event, venue, organizers, agenda, and key themes.' },
    { id: 'speaker_res', label: 'Speaker Research', agent: 'Scout', type: 'parallel', description: 'Profile each speaker in parallel — bio, publications, LinkedIn, social presence.', parallel: true },
    { id: 'brand_res', label: 'Brand Research', agent: 'Scout', type: 'parallel', description: 'Profile sponsors and brands in parallel — positioning, news, key contacts.', parallel: true },
    { id: 'prod_team', label: 'Production Team', agent: 'Scout', type: 'agent', description: 'Identify AV production companies, photographers, and crew.' },
    { id: 'comp_intel', label: 'Competitive Intel', agent: 'Scout', type: 'agent', description: 'Analyze competing events, positioning, and attendee overlap.' },
    { id: 'side_events', label: 'Side Events Discovery', agent: 'Scout', type: 'parallel', description: 'Discover Lu.ma, Eventbrite, and Partiful side events in parallel.', parallel: true },
    { id: 'articles', label: 'Article Writing', agent: 'Astra', type: 'agent', description: 'Write thought-leadership articles per speaker using research context.' },
    { id: 'qa', label: 'QA Review', agent: 'Astra', type: 'agent', description: 'Quality-check all content for accuracy, tone, and brand alignment.' },
    { id: 'social', label: 'Social Publishing', agent: 'Creative', type: 'agent', description: 'Schedule and publish posts across connected social accounts.' },
  ],
};

const EDITRON_PIPELINE: PipelineBlueprint = {
  id: 'editron',
  name: 'Editron Production',
  description: 'Video production pipeline: intake → scene detection (Maci) → audio/music (Sonix) → colour → assembly → review → export.',
  category: 'Production',
  color: 'amber',
  nodes: [
    { id: 'intake', label: 'Intake & Indexing', agent: 'Nora', type: 'agent', description: 'Receive footage from Nora task, generate proxy files for fast editing.', tools: ['FFmpeg', 'Proxy Manager'] },
    { id: 'scene', label: 'Scene Detection', agent: 'Maci', type: 'agent', description: 'Shot selection via visual QC — detect scenes, label content, rank clips by quality.', tools: ['Maci', 'Visual QC'] },
    { id: 'music', label: 'Music & Sound', agent: 'Sonix', type: 'tool', description: 'Audio engineering: music recommendations, loudness normalization, compression. Libraries: Artlist, Epidemic Sound, Soundstripe.', tools: ['Sonix', 'Artlist', 'Epidemic Sound', 'Soundstripe'] },
    { id: 'color', label: 'Colour Grading', agent: 'Editron', type: 'tool', description: 'Apply LUT and colour grade presets matched to project brand guide.', tools: ['Colour Engine', 'LUTs'] },
    { id: 'assembly', label: 'Edit Assembly', agent: 'Editron', type: 'agent', description: 'Assemble timeline — clips, transitions, music sync, markers. Output Premiere .prproj.', tools: ['Edit Assembly', 'Premiere Bridge'] },
    { id: 'review', label: 'Human Review', agent: undefined, type: 'human', description: 'Creative director reviews cut, provides revision notes.' },
    { id: 'export', label: 'Export & Deliver', agent: 'Editron', type: 'tool', description: 'Final encode via Media Encoder or FFmpeg. Deliver to client asset folder.', tools: ['Media Encoder', 'FFmpeg'] },
  ],
};

const STATIC_PIPELINES = [CONFERENCE_PIPELINE, EDITRON_PIPELINE];

const AGENT_COLORS: Record<string, string> = {
  Scout: 'bg-blue-500/15 border-blue-500/40 text-blue-400',
  Astra: 'bg-purple-500/15 border-purple-500/40 text-purple-400',
  Creative: 'bg-pink-500/15 border-pink-500/40 text-pink-400',
  Maci: 'bg-orange-500/15 border-orange-500/40 text-orange-400',
  Sonix: 'bg-green-500/15 border-green-500/40 text-green-400',
  Nora: 'bg-indigo-500/15 border-indigo-500/40 text-indigo-400',
  Editron: 'bg-amber-500/15 border-amber-500/40 text-amber-400',
  human: 'bg-emerald-500/15 border-emerald-500/40 text-emerald-400',
};

function PipelineNodeCard({ node, isLast }: { node: PipelineNode; isLast: boolean }) {
  const agentKey = node.type === 'human' ? 'human' : (node.agent ?? '');
  const colorClass = AGENT_COLORS[agentKey] ?? 'bg-muted/50 border-border text-muted-foreground';

  return (
    <div className="flex items-start gap-0">
      <div className={`relative border rounded-lg p-3 w-44 shrink-0 ${colorClass}`}>
        {node.parallel && (
          <div className="absolute -top-2 right-2">
            <Badge variant="outline" className="text-[10px] px-1 py-0">parallel</Badge>
          </div>
        )}
        <div className="font-medium text-sm leading-tight mb-1">{node.label}</div>
        {node.agent && (
          <div className="text-[10px] opacity-70 mb-1.5">{node.agent}</div>
        )}
        {node.type === 'human' && (
          <div className="text-[10px] opacity-70 mb-1.5">Human Gate</div>
        )}
        <div className="text-[10px] opacity-60 leading-snug line-clamp-3">{node.description}</div>
        {node.tools && node.tools.length > 0 && (
          <div className="flex flex-wrap gap-1 mt-2">
            {node.tools.map((t) => (
              <span key={t} className="text-[9px] bg-black/20 rounded px-1 py-0.5">{t}</span>
            ))}
          </div>
        )}
      </div>
      {!isLast && (
        <div className="flex items-center self-center shrink-0 px-1">
          <div className="w-6 h-px bg-border" />
          <svg width="8" height="8" viewBox="0 0 8 8" className="text-muted-foreground shrink-0">
            <path d="M0 4h6M3 1l3 3-3 3" stroke="currentColor" strokeWidth="1.5" fill="none" strokeLinecap="round" strokeLinejoin="round"/>
          </svg>
        </div>
      )}
    </div>
  );
}

function EditableWorkflowsView({ orgId }: { orgId: string }) {
  const queryClient = useQueryClient();
  const { data: workflows = [], isLoading } = useQuery({
    queryKey: ['workflowDefinitions'],
    queryFn: () => workflowsApi.listDefinitions(),
  });

  const [editorOpen, setEditorOpen] = useState(false);
  const [editingWorkflow, setEditingWorkflow] = useState<WorkflowDefinition | null>(null);

  const saveMutation = useMutation({
    mutationFn: async (data: { id: string; name: string; description?: string; nodes: any[]; connections: any[] }) => {
      if (editingWorkflow) {
        return workflowsApi.updateDefinition(data.id, {
          name: data.name,
          description: data.description,
          nodes: data.nodes,
          connections: data.connections,
        });
      } else {
        return workflowsApi.createDefinition({
          ...data,
          owner_type: 'organization',
          owner_id: orgId,
        });
      }
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['workflowDefinitions'] });
      setEditorOpen(false);
      setEditingWorkflow(null);
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => workflowsApi.deleteDefinition(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['workflowDefinitions'] });
    },
  });

  const openNew = () => {
    setEditingWorkflow(null);
    setEditorOpen(true);
  };

  const openEdit = (wf: WorkflowDefinition) => {
    setEditingWorkflow(wf);
    setEditorOpen(true);
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-48">
        <div className="text-sm text-muted-foreground">Loading workflows...</div>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <GitBranch className="h-5 w-5 text-muted-foreground" />
          <h2 className="text-lg font-semibold">Workflows</h2>
          <Badge variant="secondary">{workflows.length}</Badge>
        </div>
        <Button size="sm" variant="outline" className="gap-1.5" onClick={openNew}>
          <Plus className="h-3.5 w-3.5" />
          New Workflow
        </Button>
      </div>

      <p className="text-sm text-muted-foreground">
        Workflows are data processing pipelines that extract structured information from data sources.
        Run them from any data source detail page.
      </p>

      {workflows.length === 0 ? (
        <div className="text-center py-12 text-muted-foreground">
          <GitBranch className="h-8 w-8 mx-auto mb-2 opacity-40" />
          <p className="text-sm">No workflows defined yet.</p>
          <Button size="sm" variant="outline" className="mt-3 gap-1.5" onClick={openNew}>
            <Plus className="h-3.5 w-3.5" />
            Create your first workflow
          </Button>
        </div>
      ) : (
        <div className="space-y-3">
          {workflows.map((wf: WorkflowDefinition) => {
            const nodeCount = wf.nodes?.length ?? 0;
            return (
              <Card
                key={wf.id}
                className="bg-card/80 border-border/50 hover:border-primary/30 transition-colors cursor-pointer"
                onClick={() => openEdit(wf)}
              >
                <CardHeader className="pb-2">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <GitBranch className="h-4 w-4 text-purple-500" />
                      <CardTitle className="text-base">{wf.name}</CardTitle>
                      {wf.is_system && (
                        <Badge variant="secondary" className="text-[10px]">System</Badge>
                      )}
                    </div>
                    <div className="flex items-center gap-2">
                      <Badge variant="outline">{nodeCount} node{nodeCount !== 1 ? 's' : ''}</Badge>
                      {!wf.is_system && (
                        <Button
                          size="sm"
                          variant="ghost"
                          className="h-7 w-7 p-0 text-muted-foreground hover:text-destructive"
                          onClick={(e) => {
                            e.stopPropagation();
                            if (confirm(`Delete workflow "${wf.name}"?`)) {
                              deleteMutation.mutate(wf.id);
                            }
                          }}
                        >
                          <Trash2 className="h-3.5 w-3.5" />
                        </Button>
                      )}
                    </div>
                  </div>
                  {wf.description && (
                    <CardDescription className="text-xs">{wf.description}</CardDescription>
                  )}
                </CardHeader>
                <CardContent>
                  <div className="flex flex-wrap gap-2">
                    {(wf.nodes ?? []).map((node: any, idx: number) => {
                      const inputs = (wf.connections ?? []).filter((c: any) => c.target === node.id);
                      return (
                        <div
                          key={node.id}
                          className="flex items-center gap-1.5 rounded-md border bg-muted/50 px-2 py-1 text-xs"
                        >
                          <div className="w-4 h-4 rounded bg-primary/20 flex items-center justify-center text-[9px] font-bold">
                            {idx + 1}
                          </div>
                          <span>{node.name}</span>
                          {inputs.length > 0 && (
                            <span className="text-muted-foreground">
                              ({inputs.length} input{inputs.length !== 1 ? 's' : ''})
                            </span>
                          )}
                        </div>
                      );
                    })}
                  </div>
                </CardContent>
              </Card>
            );
          })}
        </div>
      )}

      <WorkflowEditorComponent
        open={editorOpen}
        onOpenChange={(v) => { setEditorOpen(v); if (!v) setEditingWorkflow(null); }}
        workflow={editingWorkflow}
        onSave={(data) => saveMutation.mutate(data)}
        isSaving={saveMutation.isPending}
      />
    </div>
  );
}

function PipelineView({ pipeline }: { pipeline: PipelineBlueprint }) {
  return (
    <div className="space-y-3">
      <div>
        <p className="text-sm text-muted-foreground">{pipeline.description}</p>
        <div className="flex gap-3 mt-2 flex-wrap text-xs text-muted-foreground">
          {['Scout', 'Astra', 'Maci', 'Sonix', 'Editron', 'Creative', 'Nora'].map((agent) => {
            if (!pipeline.nodes.some((n) => n.agent === agent)) return null;
            const c = AGENT_COLORS[agent] ?? '';
            return (
              <span key={agent} className={`inline-flex items-center gap-1 border rounded px-1.5 py-0.5 ${c}`}>
                <span className="w-1.5 h-1.5 rounded-full bg-current opacity-60" />
                {agent}
              </span>
            );
          })}
          {pipeline.nodes.some((n) => n.type === 'human') && (
            <span className={`inline-flex items-center gap-1 border rounded px-1.5 py-0.5 ${AGENT_COLORS.human}`}>
              <span className="w-1.5 h-1.5 rounded-full bg-current opacity-60" />
              Human Gate
            </span>
          )}
        </div>
      </div>
      <div className="overflow-x-auto pb-2">
        <div className="flex items-start gap-0 min-w-max">
          {pipeline.nodes.map((node, i) => (
            <PipelineNodeCard key={node.id} node={node} isLast={i === pipeline.nodes.length - 1} />
          ))}
        </div>
      </div>
    </div>
  );
}

function TemplatePipelineView({ template }: { template: any }) {
  return (
    <div className="space-y-4">
      <p className="text-sm text-muted-foreground">{template.description}</p>
      <div className="space-y-6">
        {(template.phases ?? []).map((phase: any, pi: number) => (
          <div key={phase.name}>
            <div className="flex items-center gap-2 mb-3">
              <div className="w-6 h-6 rounded-full bg-muted flex items-center justify-center text-xs font-medium shrink-0">{pi + 1}</div>
              <div>
                <div className="font-medium text-sm">{phase.name}</div>
                <div className="text-xs text-muted-foreground">{phase.description}</div>
              </div>
              {phase.is_recurring && <Badge variant="outline" className="text-xs ml-auto">Recurring</Badge>}
            </div>
            <div className="overflow-x-auto pb-1 pl-8">
              <div className="flex items-start gap-0 min-w-max">
                {(phase.tasks ?? []).map((task: any, ti: number) => {
                  const agentKey = task.task_type === 'human_review' ? 'human' : (task.agent_role ?? '');
                  const colorClass = AGENT_COLORS[agentKey] ?? (task.task_type === 'human_review' ? AGENT_COLORS.human : 'bg-muted/50 border-border text-muted-foreground');
                  const isLast = ti === phase.tasks.length - 1;
                  return (
                    <div key={task.title} className="flex items-start gap-0">
                      <div className={`border rounded-lg p-2.5 w-40 shrink-0 ${colorClass}`}>
                        {task.requires_approval && (
                          <div className="text-[9px] mb-1 opacity-60">⛔ Gate</div>
                        )}
                        <div className="font-medium text-xs leading-tight mb-1">{task.title}</div>
                        {task.agent_role && (
                          <div className="text-[10px] opacity-60 capitalize">{task.agent_role}</div>
                        )}
                        {task.task_type === 'human_review' && (
                          <div className="text-[10px] opacity-60">Human Review</div>
                        )}
                        {task.tags?.length > 0 && (
                          <div className="flex flex-wrap gap-0.5 mt-1.5">
                            {task.tags.slice(0, 2).map((t: string) => (
                              <span key={t} className="text-[9px] bg-black/20 rounded px-1">{t}</span>
                            ))}
                          </div>
                        )}
                      </div>
                      {!isLast && (
                        <div className="flex items-center self-center shrink-0 px-1">
                          <div className="w-5 h-px bg-border" />
                          <svg width="8" height="8" viewBox="0 0 8 8" className="text-muted-foreground shrink-0">
                            <path d="M0 4h6M3 1l3 3-3 3" stroke="currentColor" strokeWidth="1.5" fill="none" strokeLinecap="round" strokeLinejoin="round"/>
                          </svg>
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function LegacyPipelinesView({ orgId: _orgId }: { orgId: string }) {
  const [selected, setSelected] = useState<string | null>(null);

  const { data: templates = [] } = useQuery({
    queryKey: ['workflowTemplates'],
    queryFn: () =>
      fetch(resolveApiUrl('/api/workflow-templates'), { credentials: 'include' })
        .then((r) => r.json())
        .then((res) => res?.data ?? []),
  });

  const allPipelines: Array<{ id: string; name: string; category: string; source: 'template' | 'static'; data: any }> = [
    ...(templates as any[]).map((t: any) => ({
      id: t.id,
      name: t.name,
      category: t.client_type === 'foundation_build' ? 'Client Engagement' : 'Client Engagement',
      source: 'template' as const,
      data: t,
    })),
    ...STATIC_PIPELINES.map((p) => ({
      id: p.id,
      name: p.name,
      category: p.category,
      source: 'static' as const,
      data: p,
    })),
  ];

  const selectedPipeline = allPipelines.find((p) => p.id === selected) ?? allPipelines[0] ?? null;

  return (
    <div className="flex gap-4 h-full min-h-[500px]">
      {/* Sidebar */}
      <div className="w-52 shrink-0 space-y-1">
        <div className="text-xs font-medium text-muted-foreground uppercase tracking-wide mb-2 px-1">Workflows</div>
        {allPipelines.map((p) => (
          <button
            key={p.id}
            onClick={() => setSelected(p.id)}
            className={`w-full text-left px-3 py-2 rounded-lg text-sm transition-colors ${
              (selectedPipeline?.id === p.id)
                ? 'bg-accent text-accent-foreground'
                : 'text-muted-foreground hover:bg-muted/50 hover:text-foreground'
            }`}
          >
            <div className="font-medium leading-tight">{p.name}</div>
            <div className="text-[10px] opacity-60 mt-0.5">{p.category}</div>
          </button>
        ))}
      </div>

      {/* Pipeline canvas */}
      <div className="flex-1 min-w-0 border border-border/50 rounded-xl bg-card/40 p-4 overflow-auto">
        {selectedPipeline ? (
          <div className="space-y-4">
            <div className="flex items-center gap-3">
              <GitBranch className="h-5 w-5 text-muted-foreground shrink-0" />
              <div>
                <h3 className="font-semibold">{selectedPipeline.name}</h3>
                <div className="text-xs text-muted-foreground">{selectedPipeline.category}</div>
              </div>
            </div>
            {selectedPipeline.source === 'static'
              ? <PipelineView pipeline={selectedPipeline.data as PipelineBlueprint} />
              : <TemplatePipelineView template={selectedPipeline.data} />
            }
          </div>
        ) : (
          <div className="flex items-center justify-center h-full text-muted-foreground">
            <div className="text-center">
              <GitBranch className="h-8 w-8 mx-auto mb-2 opacity-40" />
              <p className="text-sm">Select a workflow to view its pipeline</p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}



// ── Data Sources Intel View (restored from main — summary cards) ──────────────

function DataSourcesIntelView({ orgId, projectEntries }: { orgId: string; projectEntries: { id: string; name: string }[] }) {
  const { data: orgSources = [], isLoading: orgLoading } = useQuery({
    queryKey: ['dataSources', orgId],
    queryFn: () => dataSourcesApi.listByOrganization(orgId),
    staleTime: 60_000,
  });

  const projSourceQueries = useQueries({
    queries: projectEntries.map((entry) => ({
      queryKey: ['dataSourcesProject', entry.id],
      queryFn: () => dataSourcesApi.listByProject(entry.id),
      staleTime: 60_000,
      enabled: projectEntries.length > 0,
    })),
  });

  const isLoading = orgLoading || projSourceQueries.some(q => q.isLoading);

  const sources = useMemo(() => {
    const projSources = projSourceQueries.flatMap(q => q.data || []);
    const all = [...orgSources, ...projSources];
    const seen = new Set<string>();
    return all.filter(s => {
      if (seen.has((s as any).id)) return false;
      seen.add((s as any).id);
      return true;
    });
  }, [orgSources, projSourceQueries]);

  const typeCount = useMemo(() => {
    const counts: Record<string, number> = {};
    sources.forEach((s: any) => { counts[s.data_type] = (counts[s.data_type] || 0) + 1; });
    return counts;
  }, [sources]);

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium text-muted-foreground">Data Library Summary</h3>
        <Link
          to={`/organizations/${orgId}/data-sources`}
          className="inline-flex items-center gap-1.5 text-sm text-primary hover:underline"
        >
          <Database className="h-3.5 w-3.5" />
          Open Full Library
          <ExternalLink className="h-3 w-3" />
        </Link>
      </div>
      {isLoading ? (
        <div className="flex items-center gap-2 text-muted-foreground text-sm py-4">
          <Loader2 className="h-4 w-4 animate-spin" />Loading data sources...
        </div>
      ) : sources.length === 0 ? (
        <Card className="bg-card/80 border-border/50">
          <CardContent className="py-8 text-center">
            <Database className="h-8 w-8 text-muted-foreground mx-auto mb-2" />
            <p className="text-sm text-muted-foreground">No data sources yet.</p>
            <Link to={`/organizations/${orgId}/intelligence/data-sources`} className="text-sm text-primary hover:underline mt-1 inline-block">
              Add your first data source →
            </Link>
          </CardContent>
        </Card>
      ) : (
        <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
          {Object.entries(typeCount).map(([type, count]) => (
            <Card key={type} className="bg-card/80 border-border/50">
              <CardContent className="pt-4 pb-4">
                <p className="text-xs text-muted-foreground capitalize">{type.replace(/_/g, ' ')}</p>
                <p className="text-2xl font-bold mt-1">{count as number}</p>
              </CardContent>
            </Card>
          ))}
          <Card className="bg-primary/5 border-primary/20">
            <CardContent className="pt-4 pb-4">
              <p className="text-xs text-muted-foreground">Total Files</p>
              <p className="text-2xl font-bold mt-1 text-primary">{sources.length}</p>
            </CardContent>
          </Card>
        </div>
      )}
    </div>
  );
}

// ── Artifacts Intel View (restored from main — knowledge graph) ───────────────

function ArtifactsIntelView({ projectEntries }: { projectEntries: { id: string; name: string }[] }) {
  const knowledgeQueries = useQueries({
    queries: projectEntries.map((entry) => ({
      queryKey: ['projectKnowledge', entry.id],
      queryFn: () => knowledgeApi.getProjectKnowledge(entry.id),
      staleTime: 60_000,
      enabled: projectEntries.length > 0,
    })),
  });

  const artifacts = useMemo(() => {
    const all: { title: string; summary?: string; projectName: string; projectId: string }[] = [];
    knowledgeQueries.forEach((q, i) => {
      if (!q.data) return;
      const entry = projectEntries[i];
      ((q.data as any).sources_by_type?.artifact || []).forEach((src: any) => {
        all.push({ title: src.source_title, summary: src.source_summary, projectName: entry.name, projectId: entry.id });
      });
    });
    return all;
  }, [knowledgeQueries, projectEntries]);

  const isLoading = knowledgeQueries.some(q => q.isLoading);

  if (isLoading) return (
    <div className="flex items-center gap-2 text-muted-foreground text-sm py-4">
      <Loader2 className="h-4 w-4 animate-spin" />Loading knowledge graph artifacts...
    </div>
  );

  if (artifacts.length === 0) return (
    <Card className="bg-card/80 border-border/50">
      <CardContent className="py-8 text-center">
        <FileText className="h-8 w-8 text-muted-foreground mx-auto mb-2" />
        <p className="text-sm text-muted-foreground">No artifacts in the knowledge graph yet.</p>
        <p className="text-xs text-muted-foreground mt-1">Artifacts are added automatically when deliverables are marked done.</p>
      </CardContent>
    </Card>
  );

  return (
    <div className="space-y-3">
      <h3 className="text-sm font-medium text-muted-foreground">Knowledge Graph Artifacts</h3>
      <p className="text-xs text-muted-foreground">{artifacts.length} artifact{artifacts.length !== 1 ? 's' : ''} across {new Set(artifacts.map(a => a.projectId)).size} project{new Set(artifacts.map(a => a.projectId)).size !== 1 ? 's' : ''}</p>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
        {artifacts.map((a, i) => (
          <Card key={i} className="bg-card/80 border-border/50">
            <CardContent className="pt-4 pb-4">
              <div className="flex items-start gap-2">
                <FileText className="h-4 w-4 text-[hsl(var(--brand))] shrink-0 mt-0.5" />
                <div className="min-w-0">
                  <p className="text-sm font-medium truncate">{a.title}</p>
                  {a.summary && <p className="text-xs text-muted-foreground mt-0.5 line-clamp-2">{a.summary}</p>}
                  <Link to={`/projects/${a.projectId}`} className="text-[10px] text-muted-foreground hover:text-foreground mt-1 block">{a.projectName}</Link>
                </div>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}

// ── System Automations Section (restored from main) ───────────────────────────

function SystemAutomationsSection() {
  const { data: automations = [] } = useQuery({
    queryKey: ['system-automations'],
    queryFn: async () => {
      const r = await fetch('/api/automations', { credentials: 'include' });
      const d = await r.json();
      return d.data || [];
    },
    staleTime: 5 * 60_000,
  });

  if (automations.length === 0) return null;

  return (
    <div className="border-t pt-6">
      <p className="text-xs font-medium text-muted-foreground uppercase tracking-wider mb-2">
        System Automations ({automations.length} active)
      </p>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
        {automations.map((a: any) => (
          <Card key={a.id} className="bg-card/80 border-border/50">
            <CardContent className="pt-4 pb-4">
              <div className="flex items-start gap-2">
                <Activity className="h-4 w-4 text-green-500 shrink-0 mt-0.5" />
                <div className="min-w-0">
                  <p className="text-sm font-medium truncate">{a.name}</p>
                  {a.description && <p className="text-xs text-muted-foreground mt-0.5 line-clamp-2">{a.description}</p>}
                  <div className="flex items-center gap-1.5 mt-1">
                    <Badge variant="default" className="text-[9px] bg-green-500/10 text-green-700 border-green-200">Active</Badge>
                    {a.schedule && <span className="text-[9px] text-muted-foreground">{a.schedule}</span>}
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}

// ── Pulse View — alerts + signals aggregated across projects ──────────────────

function PulseView({ projectEntries }: { projectEntries: { id: string; name: string }[] }) {
  const alertQueries = useQueries({
    queries: projectEntries.map((entry) => ({
      queryKey: ['pulse-alerts-org', entry.id],
      queryFn: () => pulseApi.getAlerts(entry.id, 20),
      staleTime: 60_000,
      enabled: projectEntries.length > 0,
    })),
  });

  const contentQueries = useQueries({
    queries: projectEntries.map((entry) => ({
      queryKey: ['pulse-content-org', entry.id],
      queryFn: () => pulseApi.getLatestContent(entry.id, 10),
      staleTime: 60_000,
      enabled: projectEntries.length > 0,
    })),
  });

  const aggregated = useMemo(() => {
    const allAlerts: any[] = [];
    const allContent: any[] = [];

    alertQueries.forEach((q, i) => {
      if (!q.data) return;
      const entry = projectEntries[i];
      (q.data as any[]).forEach((a: any) => allAlerts.push({ ...a, _projectName: entry.name }));
    });

    contentQueries.forEach((q, i) => {
      if (!q.data?.items) return;
      const entry = projectEntries[i];
      q.data.items.forEach((c: any) => allContent.push({ ...c, _projectName: entry.name }));
    });

    allAlerts.sort((a, b) => new Date(b.triggered_at || b.created_at).getTime() - new Date(a.triggered_at || a.created_at).getTime());
    allContent.sort((a, b) => new Date(b.collected_at || b.created_at).getTime() - new Date(a.collected_at || a.created_at).getTime());

    const unacknowledged = allAlerts.filter(a => !a.acknowledged_at).length;
    return { alerts: allAlerts.slice(0, 20), content: allContent.slice(0, 20), unacknowledged };
  }, [alertQueries, contentQueries, projectEntries]);

  const fmtDate = (iso: string) => {
    try { return new Date(iso).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' }); }
    catch { return '—'; }
  };

  return (
    <div className="space-y-6">
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Unacknowledged Alerts</CardTitle>
          </CardHeader>
          <CardContent>
            <span className={`text-2xl font-bold ${aggregated.unacknowledged > 0 ? 'text-amber-600' : ''}`}>
              {aggregated.unacknowledged}
              {aggregated.unacknowledged > 0 && (
                <Badge variant="default" className="text-[10px] ml-1">{aggregated.unacknowledged} new</Badge>
              )}
            </span>
          </CardContent>
        </Card>
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Recent Signals</CardTitle>
          </CardHeader>
          <CardContent>
            <span className="text-2xl font-bold">{aggregated.content.length}</span>
          </CardContent>
        </Card>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader>
            <CardTitle className="text-base flex items-center gap-2">
              <AlertTriangle className="h-4 w-4 text-amber-500" />
              Recent Alerts
              {aggregated.unacknowledged > 0 && (
                <Badge variant="default" className="text-[10px] ml-1">{aggregated.unacknowledged} new</Badge>
              )}
            </CardTitle>
          </CardHeader>
          <CardContent>
            {aggregated.alerts.length === 0 ? (
              <div className="text-center py-8 text-muted-foreground">
                <AlertTriangle className="h-8 w-8 mx-auto mb-2 opacity-40" />
                <p>No alerts</p>
              </div>
            ) : (
              <ScrollArea className="h-[300px]">
                <div className="space-y-2 pr-3">
                  {aggregated.alerts.map((alert: any) => (
                    <div
                      key={alert.id}
                      className={`p-3 rounded-lg border border-border/50 ${!alert.acknowledged_at ? 'bg-amber-50/30 dark:bg-amber-950/20' : ''}`}
                    >
                      <div className="flex items-start justify-between gap-2">
                        <div className="min-w-0">
                          <p className="text-sm font-medium truncate">{alert.rule_name || 'Alert'}</p>
                          {alert.message && (
                            <p className="text-xs text-muted-foreground line-clamp-2 mt-0.5">{alert.message}</p>
                          )}
                          <div className="flex items-center gap-2 mt-1">
                            <Badge variant="outline" className="text-[9px]">{alert._projectName}</Badge>
                            <span className="text-[10px] text-muted-foreground">
                              {fmtDate(alert.triggered_at || alert.created_at)}
                            </span>
                          </div>
                        </div>
                        {!alert.acknowledged_at && (
                          <span className="h-2 w-2 rounded-full bg-amber-500 shrink-0 mt-1" />
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              </ScrollArea>
            )}
          </CardContent>
        </Card>

        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader>
            <CardTitle className="text-base flex items-center gap-2">
              <Radio className="h-4 w-4 text-blue-500" />
              Latest Signals
            </CardTitle>
          </CardHeader>
          <CardContent>
            {aggregated.content.length === 0 ? (
              <div className="text-center py-8 text-muted-foreground">
                <Radio className="h-8 w-8 mx-auto mb-2 opacity-40" />
                <p>No signals collected yet</p>
              </div>
            ) : (
              <ScrollArea className="h-[300px]">
                <div className="space-y-2 pr-3">
                  {aggregated.content.map((item: any) => (
                    <div key={item.id} className="p-3 rounded-lg border border-border/50">
                      <p className="text-sm font-medium line-clamp-2">{item.title || item.content_preview || 'Signal'}</p>
                      <div className="flex items-center gap-2 mt-1">
                        <Badge variant="outline" className="text-[9px]">{item._projectName}</Badge>
                        {item.source_name && (
                          <span className="text-[10px] text-muted-foreground">{item.source_name}</span>
                        )}
                        <span className="text-[10px] text-muted-foreground ml-auto">
                          {fmtDate(item.collected_at || item.created_at)}
                        </span>
                      </div>
                    </div>
                  ))}
                </div>
              </ScrollArea>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

// ── Topology View — knowledge graph topology snapshots ─────────────────────────

function TopologyView({
  projectEntries,
  aggregated,
  isLoading,
  loadedCount,
}: {
  projectEntries: { id: string; name: string }[];
  aggregated: { byType: Record<string, { source: ProjectKnowledgeSource; projectName: string; projectId: string }[]> };
  isLoading: boolean;
  loadedCount: number;
}) {
  const topologyItems = aggregated.byType['topology_snapshot'] || [];

  if (isLoading && topologyItems.length === 0) return (
    <div className="flex items-center gap-2 text-muted-foreground text-sm py-4">
      <Loader2 className="h-4 w-4 animate-spin" />Loading topology data ({loadedCount}/{projectEntries.length} projects)...
    </div>
  );

  if (topologyItems.length === 0) return (
    <Card className="bg-card/80 border-border/50">
      <CardContent className="py-8 text-center">
        <Network className="h-8 w-8 text-muted-foreground mx-auto mb-2" />
        <p className="text-sm text-muted-foreground">No topology snapshots in the knowledge graph yet.</p>
      </CardContent>
    </Card>
  );

  return (
    <div className="space-y-3">
      <div className="flex items-center gap-2">
        <Network className="h-5 w-5 text-muted-foreground" />
        <h2 className="text-lg font-semibold">Topology</h2>
        <Badge variant="secondary">{topologyItems.length}</Badge>
      </div>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
        {topologyItems.map(({ source, projectName, projectId }) => (
          <Card key={source.id} className={`bg-card/80 ${source.is_stale ? 'border-yellow-500/30' : 'border-border/50'}`}>
            <CardContent className="pt-4 pb-4">
              <div className="flex items-start gap-2">
                <Network className="h-4 w-4 text-[hsl(var(--info))] shrink-0 mt-0.5" />
                <div className="min-w-0 flex-1">
                  <p className="text-sm font-medium truncate">{source.source_title}</p>
                  {source.source_summary && <p className="text-xs text-muted-foreground mt-0.5 line-clamp-2">{source.source_summary}</p>}
                  <div className="flex items-center justify-between mt-1.5">
                    <Link to={`/projects/${projectId}`} className="text-[10px] text-muted-foreground hover:text-foreground">{projectName}</Link>
                    <span className="text-[10px] text-muted-foreground">{Math.round(source.coverage_score * 100)}% coverage</span>
                  </div>
                  {source.is_stale && <Badge variant="outline" className="text-[9px] mt-1 text-yellow-600 border-yellow-600">Stale</Badge>}
                </div>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}

// ── Knowledge Tab (main intelligence container) ───────────────────────────────

function KnowledgeTab({
  orgId,
  projectEntries,
  view,
}: {
  orgId: string;
  projectEntries: { id: string; name: string }[];
  view?: string | null;
}) {
  const knowledgeQueries = useQueries({
    queries: projectEntries.map((entry) => ({
      queryKey: ['projectKnowledge', entry.id],
      queryFn: () => knowledgeApi.getProjectKnowledge(entry.id),
      staleTime: 60_000,
    })),
  });

  // Fetch org-level data sources for accurate stats
  const { data: orgDataSources = [] } = useQuery({
    queryKey: ['orgDataSources', orgId],
    queryFn: () => dataSourcesApi.listByOrganization(orgId),
    staleTime: 60_000,
  });

  const isLoading = knowledgeQueries.some(q => q.isLoading);
  const loadedCount = knowledgeQueries.filter(q => q.isSuccess).length;

  // Aggregate across all projects
  const aggregated = useMemo(() => {
    let totalSources = 0;
    let staleSources = 0;
    let totalCoverage = 0;
    let coverageCount = 0;
    const sourcesByProject: { projectName: string; projectId: string; data: ProjectKnowledgeResponse }[] = [];
    const byType: Record<string, { source: ProjectKnowledgeSource; projectName: string; projectId: string }[]> = {};

    knowledgeQueries.forEach((q, i) => {
      if (!q.data) return;
      const entry = projectEntries[i];
      totalSources += q.data.total_sources;
      staleSources += q.data.stale_count;
      if (q.data.completeness) {
        totalCoverage += q.data.completeness.knowledge_completeness;
        coverageCount++;
      }
      if (q.data.total_sources > 0) {
        sourcesByProject.push({ projectName: entry.name, projectId: entry.id, data: q.data });
      }
      Object.entries(q.data.sources_by_type).forEach(([type, sources]) => {
        if (!byType[type]) byType[type] = [];
        sources.forEach(s => byType[type].push({ source: s, projectName: entry.name, projectId: entry.id }));
      });
    });

    const avgCompleteness = coverageCount > 0 ? Math.round((totalCoverage / coverageCount) * 100) : 0;
    return { totalSources, staleSources, avgCompleteness, sourcesByProject, byType };
  }, [knowledgeQueries, projectEntries]);

  // Deep view: "datasources" shows the new data sources table; others filter knowledge by type
  if (view && view !== 'overview') {
    if (view === 'datasources') {
      return (
        <div className="space-y-6">
          <DataSourcesIntelView orgId={orgId} projectEntries={projectEntries} />
          <DataSourcesView orgId={orgId} projectEntries={projectEntries} />
        </div>
      );
    }

    if (view === 'artifacts') {
      return (
        <div className="space-y-6">
          <ArtifactsIntelView projectEntries={projectEntries} />
          <ArtifactsView orgId={orgId} />
        </div>
      );
    }

    if (view === 'workflows') {
      return (
        <div className="space-y-8">
          <EditableWorkflowsView orgId={orgId} />
          <SystemAutomationsSection />
          <div className="border-t pt-6">
            <div className="flex items-center gap-2 mb-4">
              <Network className="h-5 w-5 text-muted-foreground" />
              <h2 className="text-lg font-semibold">Pipeline Blueprints</h2>
              <Badge variant="secondary" className="text-[10px]">Legacy</Badge>
            </div>
            <p className="text-sm text-muted-foreground mb-4">
              Agent-based pipeline templates for client engagements and production workflows. These are read-only blueprints — use the workflow editor above to build custom pipelines.
            </p>
            <LegacyPipelinesView orgId={orgId} />
          </div>
        </div>
      );
    }

    if (view === 'pulse') {
      return <PulseView projectEntries={projectEntries} />;
    }

    if (view === 'topology') {
      return <TopologyView projectEntries={projectEntries} aggregated={aggregated} isLoading={isLoading} loadedCount={loadedCount} />;
    }

    // Generic fallback for other knowledge source types (e.g. conversations)
    const typeKey =
      view === 'conversations' ? 'conversation'
      : null;
    const items = typeKey ? (aggregated.byType[typeKey] || []) : [];
    const meta = typeKey ? SOURCE_TYPE_META[typeKey] : null;
    const Icon = meta?.icon ?? BookOpen;

    return (
      <div className="space-y-4">
        <div className="flex items-center gap-2">
          <Icon className="h-5 w-5 text-muted-foreground" />
          <h2 className="text-lg font-semibold">{meta?.label ?? view}</h2>
          <Badge variant="secondary">{items.length}</Badge>
          {isLoading && (
            <span className="text-xs text-muted-foreground ml-2">
              Loading {loadedCount}/{projectEntries.length} projects…
            </span>
          )}
        </div>
        {items.length === 0 && !isLoading ? (
          <div className="text-center py-12 text-muted-foreground">
            <Icon className="h-8 w-8 mx-auto mb-2 opacity-40" />
            <p>No {meta?.label.toLowerCase() ?? view} indexed yet</p>
          </div>
        ) : (
          <div className="space-y-2">
            {items.map(({ source, projectName, projectId }) => (
              <Card key={source.id} className="bg-card/80 backdrop-blur-sm border-border/50">
                <CardContent className="p-4">
                  <div className="flex items-start justify-between gap-3">
                    <div className="min-w-0">
                      <p className="font-medium text-sm truncate">{source.source_title}</p>
                      {source.source_summary && (
                        <p className="text-xs text-muted-foreground mt-1 line-clamp-2">{source.source_summary}</p>
                      )}
                      <div className="flex items-center gap-2 mt-2">
                        <Link
                          to={`/projects/${projectId}/knowledge`}
                          className="text-xs text-blue-600 hover:underline flex items-center gap-1"
                        >
                          <FolderOpen className="h-3 w-3" />
                          {projectName}
                        </Link>
                        {source.is_stale && (
                          <Badge variant="outline" className="text-xs text-yellow-600 border-yellow-300">stale</Badge>
                        )}
                      </div>
                    </div>
                    <div className="shrink-0 text-right">
                      <Progress value={Math.round(source.coverage_score * 100)} className="w-16 h-1.5 mb-1" />
                      <span className="text-xs text-muted-foreground">{Math.round(source.coverage_score * 100)}%</span>
                    </div>
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        )}
      </div>
    );
  }

  // Overview (default)
  return (
    <div className="space-y-6">
      {/* Summary cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Avg Completeness</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-3">
              <Progress value={aggregated.avgCompleteness} className="flex-1 h-2" />
              <span className="text-2xl font-bold">{aggregated.avgCompleteness}%</span>
            </div>
          </CardContent>
        </Card>
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Total Sources</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-baseline gap-2">
              <span className="text-2xl font-bold">{aggregated.totalSources + orgDataSources.length}</span>
              <span className="text-sm text-muted-foreground">
                {orgDataSources.length > 0 && `${orgDataSources.length} org · `}
                {aggregated.totalSources} across {projectEntries.length} project{projectEntries.length !== 1 ? 's' : ''}
              </span>
            </div>
          </CardContent>
        </Card>
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Stale Sources</CardTitle>
          </CardHeader>
          <CardContent>
            <span className={`text-2xl font-bold ${aggregated.staleSources > 0 ? 'text-yellow-600' : ''}`}>
              {aggregated.staleSources}
            </span>
          </CardContent>
        </Card>
      </div>

      {/* Source type breakdown pills */}
      {aggregated.totalSources > 0 && (
        <div className="grid grid-cols-2 md:grid-cols-3 gap-3">
          {Object.entries(SOURCE_TYPE_META).map(([type, meta]) => {
            const count = (aggregated.byType[type] || []).length;
            if (count === 0) return null;
            const Icon = meta.icon;
            return (
              <Card key={type} className="bg-card/80 backdrop-blur-sm border-border/50">
                <CardContent className="p-3 flex items-center gap-3">
                  <Icon className="h-5 w-5 text-muted-foreground shrink-0" />
                  <div>
                    <p className="font-semibold text-sm">{count}</p>
                    <p className="text-xs text-muted-foreground">{meta.label}</p>
                  </div>
                </CardContent>
              </Card>
            );
          })}
        </div>
      )}

      {isLoading && (
        <p className="text-xs text-muted-foreground">Loading {loadedCount}/{projectEntries.length} projects...</p>
      )}

      {/* Per-project knowledge */}
      {aggregated.sourcesByProject.length === 0 && !isLoading ? (
        <div className="text-center py-12 text-muted-foreground">
          <BookOpen className="h-8 w-8 mx-auto mb-2 opacity-40" />
          <p>
            {projectEntries.length === 0
              ? 'No projects loaded — navigate to a project to index knowledge'
              : 'No knowledge sources indexed yet'}
          </p>
          <p className="text-xs mt-1 opacity-70">
            Use the sidebar Intelligence links to browse by category
          </p>
        </div>
      ) : (
        <div className="space-y-4">
          {aggregated.sourcesByProject.map(({ projectName, projectId, data }) => {
            const completeness = data.completeness
              ? Math.round(data.completeness.knowledge_completeness * 100)
              : 0;
            return (
              <Card key={projectId} className="bg-card/80 backdrop-blur-sm border-border/50">
                <CardHeader className="pb-3">
                  <div className="flex items-center justify-between">
                    <CardTitle className="text-base flex items-center gap-2">
                      <FolderOpen className="h-4 w-4 text-muted-foreground" />
                      {projectName}
                    </CardTitle>
                    <div className="flex items-center gap-2">
                      <Progress value={completeness} className="w-20 h-1.5" />
                      <span className="text-xs text-muted-foreground">{completeness}%</span>
                      <Link
                        to={`/projects/${projectId}/knowledge`}
                        className="text-xs text-blue-600 hover:underline ml-2"
                      >
                        View
                      </Link>
                    </div>
                  </div>
                </CardHeader>
                <CardContent>
                  <div className="flex flex-wrap gap-2">
                    {Object.entries(data.sources_by_type).map(([type, sources]) => {
                      if (sources.length === 0) return null;
                      const meta = SOURCE_TYPE_META[type];
                      const Icon = meta?.icon || BookOpen;
                      return (
                        <Badge key={type} variant="secondary" className="text-xs gap-1">
                          <Icon className="h-3 w-3" />
                          {meta?.label || type} ({sources.length})
                        </Badge>
                      );
                    })}
                    {data.stale_count > 0 && (
                      <Badge variant="outline" className="text-xs text-yellow-600 border-yellow-300 gap-1">
                        <AlertTriangle className="h-3 w-3" />
                        {data.stale_count} stale
                      </Badge>
                    )}
                  </div>
                </CardContent>
              </Card>
            );
          })}
        </div>
      )}
    </div>
  );
}

// ── Social Tab ────────────────────────────────────────────────────────────────

const PLATFORM_ICONS: Record<string, typeof Linkedin> = {
  linkedin: Linkedin,
  instagram: Instagram,
  twitter: Twitter,
  facebook: Facebook,
  youtube: Youtube,
};

const PLATFORM_COLORS: Record<string, string> = {
  linkedin: 'text-[#0A66C2]',
  instagram: 'text-[#E4405F]',
  twitter: 'text-foreground',
  facebook: 'text-[#1877F2]',
  youtube: 'text-[#FF0000]',
};

function SocialTab({ projectEntries }: { projectEntries: { id: string; name: string }[] }) {
  const accountQueries = useQueries({
    queries: projectEntries.map((entry) => ({
      queryKey: ['social-accounts', entry.id],
      queryFn: () => socialApi.listAccounts(entry.id),
      staleTime: 60_000,
      enabled: projectEntries.length > 0,
    })),
  });

  const mentionQueries = useQueries({
    queries: projectEntries.map((entry) => ({
      queryKey: ['social-mentions', entry.id],
      queryFn: () => socialApi.listMentions(entry.id, { limit: 20 }),
      staleTime: 60_000,
      enabled: projectEntries.length > 0,
    })),
  });

  const isLoading = accountQueries.some(q => q.isLoading);

  const aggregated = useMemo(() => {
    const allAccounts: (SocialAccountRecord & { _projectName: string; _projectId: string })[] = [];
    const allMentions: (SocialMentionRecord & { _projectName: string })[] = [];

    accountQueries.forEach((q, i) => {
      if (!q.data) return;
      const entry = projectEntries[i];
      q.data.forEach(a => allAccounts.push({ ...a, _projectName: entry.name, _projectId: entry.id }));
    });

    mentionQueries.forEach((q, i) => {
      if (!q.data) return;
      const entry = projectEntries[i];
      q.data.forEach(m => allMentions.push({ ...m, _projectName: entry.name }));
    });

    // Dedupe accounts by platform + username
    const uniqueAccounts = new Map<string, typeof allAccounts[0]>();
    for (const a of allAccounts) {
      const key = `${a.platform}:${a.username || a.display_name || a.id}`;
      if (!uniqueAccounts.has(key)) uniqueAccounts.set(key, a);
    }

    // Sort mentions by date
    allMentions.sort((a, b) => new Date(b.received_at).getTime() - new Date(a.received_at).getTime());

    const unreadCount = allMentions.filter(m => m.status === 'unread').length;
    const totalFollowers = [...uniqueAccounts.values()].reduce((sum, a) => sum + (a.follower_count || 0), 0);

    return {
      accounts: [...uniqueAccounts.values()],
      mentions: allMentions.slice(0, 30),
      unreadCount,
      totalFollowers,
    };
  }, [accountQueries, mentionQueries, projectEntries]);

  return (
    <div className="space-y-6">
      {/* Summary */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Connected Accounts</CardTitle>
          </CardHeader>
          <CardContent>
            <span className="text-2xl font-bold">{aggregated.accounts.length}</span>
          </CardContent>
        </Card>
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Total Followers</CardTitle>
          </CardHeader>
          <CardContent>
            <span className="text-2xl font-bold">
              {aggregated.totalFollowers >= 1000
                ? `${(aggregated.totalFollowers / 1000).toFixed(1)}k`
                : aggregated.totalFollowers}
            </span>
          </CardContent>
        </Card>
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Unread Mentions</CardTitle>
          </CardHeader>
          <CardContent>
            <span className={`text-2xl font-bold ${aggregated.unreadCount > 0 ? 'text-blue-600' : ''}`}>
              {aggregated.unreadCount}
            </span>
          </CardContent>
        </Card>
      </div>

      {isLoading && (
        <p className="text-xs text-muted-foreground">Loading social data...</p>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Accounts */}
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader>
            <CardTitle className="text-base flex items-center gap-2">
              <Share2 className="h-4 w-4" />
              Connected Accounts
            </CardTitle>
          </CardHeader>
          <CardContent>
            {aggregated.accounts.length === 0 ? (
              <div className="text-center py-8 text-muted-foreground">
                <Share2 className="h-8 w-8 mx-auto mb-2 opacity-40" />
                <p>No social accounts connected</p>
              </div>
            ) : (
              <div className="space-y-3">
                {aggregated.accounts.map((account) => {
                  const PlatformIcon = PLATFORM_ICONS[account.platform] || Globe;
                  const platformColor = PLATFORM_COLORS[account.platform] || 'text-muted-foreground';
                  return (
                    <div key={account.id} className="flex items-center gap-3 p-3 rounded-lg border border-border/50">
                      <PlatformIcon className={`h-5 w-5 shrink-0 ${platformColor}`} />
                      <div className="min-w-0 flex-1">
                        <p className="text-sm font-medium truncate">
                          {account.display_name || account.username || account.platform}
                        </p>
                        <div className="flex items-center gap-2 text-xs text-muted-foreground">
                          <span className="capitalize">{account.platform}</span>
                          {account.follower_count != null && (
                            <>
                              <span>-</span>
                              <span>{account.follower_count.toLocaleString()} followers</span>
                            </>
                          )}
                        </div>
                      </div>
                      <div className="flex items-center gap-2">
                        <Badge variant="outline" className="text-[10px]">{account._projectName}</Badge>
                        <Badge
                          variant={account.status === 'active' ? 'default' : 'secondary'}
                          className="text-[10px]"
                        >
                          {account.status}
                        </Badge>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </CardContent>
        </Card>

        {/* Recent Mentions */}
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader>
            <CardTitle className="text-base flex items-center gap-2">
              <Inbox className="h-4 w-4" />
              Recent Mentions
              {aggregated.unreadCount > 0 && (
                <Badge variant="default" className="text-[10px] ml-1">{aggregated.unreadCount} new</Badge>
              )}
            </CardTitle>
          </CardHeader>
          <CardContent>
            {aggregated.mentions.length === 0 ? (
              <div className="text-center py-8 text-muted-foreground">
                <Inbox className="h-8 w-8 mx-auto mb-2 opacity-40" />
                <p>No mentions yet</p>
              </div>
            ) : (
              <ScrollArea className="h-[400px]">
                <div className="space-y-2 pr-3">
                  {aggregated.mentions.map((mention) => {
                    const PlatformIcon = PLATFORM_ICONS[mention.platform] || Globe;
                    const isUnread = mention.status === 'unread';
                    return (
                      <div
                        key={mention.id}
                        className={`p-3 rounded-lg border border-border/50 ${isUnread ? 'bg-accent/20' : ''}`}
                      >
                        <div className="flex items-start gap-2">
                          <PlatformIcon className="h-4 w-4 shrink-0 mt-0.5 text-muted-foreground" />
                          <div className="min-w-0 flex-1">
                            <div className="flex items-center gap-2">
                              <span className="text-xs font-medium">
                                {mention.author_display_name || mention.author_username || 'Unknown'}
                              </span>
                              <Badge variant="outline" className="text-[9px]">{mention.mention_type}</Badge>
                              {isUnread && <span className="h-1.5 w-1.5 rounded-full bg-blue-500 shrink-0" />}
                            </div>
                            {mention.content && (
                              <p className="text-xs text-muted-foreground line-clamp-2 mt-1">{mention.content}</p>
                            )}
                            <div className="flex items-center gap-2 mt-1">
                              <span className="text-[10px] text-muted-foreground">
                                {formatDate(mention.received_at)}
                              </span>
                              <Badge variant="outline" className="text-[9px]">{mention._projectName}</Badge>
                            </div>
                          </div>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </ScrollArea>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

// ── Integrations Tab ─────────────────────────────────────────────────────────

function IntegrationCard({
  accent,
  icon: Icon,
  name,
  description,
  status,
  statusLabel,
  actions,
  extra,
}: {
  accent: string;
  icon: React.ElementType;
  name: string;
  description: string;
  status: 'connected' | 'warning' | 'disconnected';
  statusLabel?: string;
  actions: React.ReactNode;
  extra?: React.ReactNode;
}) {
  const barColor = status === 'connected' ? '#22c55e' : status === 'warning' ? '#f59e0b' : '#6b728040';
  return (
    <Card className="border-border/60 bg-card/80 overflow-hidden">
      <div className="flex items-stretch">
        <div className="w-1 shrink-0" style={{ backgroundColor: barColor }} />
        <div className="flex-1 p-4 space-y-3">
          <div className="flex items-start justify-between gap-3">
            <div className="flex items-center gap-3">
              <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl" style={{ backgroundColor: `${accent}18`, border: `1px solid ${accent}30` }}>
                <Icon className="h-5 w-5" style={{ color: accent }} />
              </div>
              <div>
                <div className="flex items-center gap-2">
                  <span className="text-sm font-semibold">{name}</span>
                  {status === 'connected' ? (
                    <Badge className="text-[10px] px-1.5 py-0 bg-emerald-100 text-emerald-700 border-emerald-200">
                      <CheckCircle2 className="h-3 w-3 mr-1" />{statusLabel ?? 'Connected'}
                    </Badge>
                  ) : status === 'warning' ? (
                    <Badge className="text-[10px] px-1.5 py-0 bg-amber-100 text-amber-700 border-amber-200">
                      <AlertCircle className="h-3 w-3 mr-1" />{statusLabel ?? 'Reauthorize'}
                    </Badge>
                  ) : (
                    <Badge variant="secondary" className="text-[10px] px-1.5 py-0">Not connected</Badge>
                  )}
                </div>
                <p className="text-xs text-muted-foreground mt-0.5">{description}</p>
              </div>
            </div>
            <div className="flex items-center gap-2 shrink-0">{actions}</div>
          </div>
          {extra}
        </div>
      </div>
    </Card>
  );
}

function IntegrationsTab({ orgId }: { orgId: string }) {
  const queryClient = useQueryClient();
  const [connectingEmail, setConnectingEmail] = useState<string | null>(null);
  const [qbSyncing, setQbSyncing] = useState(false);
  const [qbDisconnecting, setQbDisconnecting] = useState(false);

  // ── Email accounts (Gmail / Zoho) ─────────────────────────────────────────
  const { data: emailAccounts = [], isLoading: emailLoading } = useQuery<EmailAccountRecord[]>({
    queryKey: ['email-accounts-org', orgId],
    queryFn: () => emailApi.listAccounts(undefined, undefined, 'organization', orgId),
    staleTime: 30_000,
  });

  const handleEmailConnect = async (provider: string) => {
    setConnectingEmail(provider);
    try {
      const redirectUri = `${window.location.origin}/oauth/${provider}/callback`;
      const { auth_url } = await emailApi.initiateOAuth(null, provider, redirectUri, 'organization', orgId);
      window.location.href = auth_url;
    } catch {
      setConnectingEmail(null);
    }
  };

  const handleEmailDisconnect = async (id: string) => {
    if (!confirm('Disconnect this email account?')) return;
    await emailApi.deleteAccount(id);
    queryClient.invalidateQueries({ queryKey: ['email-accounts-org', orgId] });
  };

  const handleEmailSync = async (id: string) => {
    await emailApi.triggerSync(id);
    queryClient.invalidateQueries({ queryKey: ['email-accounts-org', orgId] });
  };

  // ── QuickBooks ────────────────────────────────────────────────────────────
  const { data: qbStatus, isLoading: qbLoading, refetch: refetchQb } = useQuery({
    queryKey: ['qb-status-org', orgId],
    queryFn: () => quickbooksApi.getStatus(orgId),
    staleTime: 30_000,
    retry: false,
  });

  const handleQbConnect = () => { window.location.href = quickbooksApi.getConnectUrl(orgId); };

  const handleQbDisconnect = async () => {
    if (!qbStatus?.account?.id) return;
    if (!confirm('Disconnect QuickBooks? Entity mappings will be removed.')) return;
    setQbDisconnecting(true);
    try { await quickbooksApi.disconnect(qbStatus.account.id); refetchQb(); }
    finally { setQbDisconnecting(false); }
  };

  const handleQbSync = async () => {
    if (!qbStatus?.account?.id) return;
    setQbSyncing(true);
    try { await quickbooksApi.triggerSync(qbStatus?.account.id); refetchQb(); }
    finally { setQbSyncing(false); }
  };

  const handleQbRefresh = async () => {
    if (!qbStatus?.account?.id) return;
    try { await quickbooksApi.refreshToken(qbStatus.account.id); refetchQb(); }
    catch { /* ignore */ }
  };

  // ── Airtable ──────────────────────────────────────────────────────────────
  const { config, updateAndSaveConfig } = useUserSystem();
  const [airtableToken, setAirtableToken] = useState(config?.airtable?.token ?? '');
  const [airtableVerifying, setAirtableVerifying] = useState(false);
  const [airtableError, setAirtableError] = useState<string | null>(null);
  const isAirtableConnected = !!(config?.airtable?.token);

  const handleAirtableSave = async () => {
    if (!airtableToken) { setAirtableError('Enter your Personal Access Token'); return; }
    setAirtableVerifying(true);
    setAirtableError(null);
    try {
      const result = await airtableApi.verifyCredentials({ token: airtableToken });
      if (result.valid) {
        await updateAndSaveConfig({ airtable: { ...(config?.airtable ?? {}), token: airtableToken, user_email: result.user_email ?? null } as never });
      } else {
        setAirtableError('Token is invalid. Check permissions and try again.');
      }
    } catch (e: unknown) {
      setAirtableError(e instanceof Error ? e.message : 'Verification failed');
    } finally {
      setAirtableVerifying(false);
    }
  };

  const handleAirtableDisconnect = async () => {
    if (!confirm('Remove Airtable connection?')) return;
    await updateAndSaveConfig({ airtable: { ...(config?.airtable ?? {}), token: '', user_email: null } as never });
    setAirtableToken('');
  };

  const qbConnected = !qbLoading && !!qbStatus?.connected;
  const qbNeedsReauth = !qbLoading && !!qbStatus?.needs_reauth;

  const emailSections: { provider: string; label: string; accent: string; desc: string }[] = [
    { provider: 'gmail',  label: 'Gmail',      accent: '#EA4335', desc: 'Unified inbox, Nora email access, and contact sync.' },
    { provider: 'zoho',   label: 'Zoho Mail',  accent: '#C8202B', desc: 'Zoho Mail + CRM sync — operations and pipeline in lock-step.' },
  ];

  return (
    <div className="space-y-8">
      <div>
        <h2 className="text-lg font-semibold">Organization Integrations</h2>
        <p className="text-sm text-muted-foreground mt-1">
          Org-level connections shared across all projects. Projects, users, and agents have their own independently configurable integrations.
        </p>
      </div>

      {/* ── Email ── */}
      <section className="space-y-3">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Email</h3>
        {emailLoading && <div className="flex items-center gap-2 text-sm text-muted-foreground"><Loader2 className="h-4 w-4 animate-spin" />Loading…</div>}
        {emailSections.map(({ provider, label, accent, desc }) => {
          const account = emailAccounts.find(a => a.provider === provider);
          const isConn = account?.status === 'active';
          const isWarn = account?.status === 'needs_reauth';
          return (
            <IntegrationCard
              key={provider}
              accent={accent}
              icon={Mail}
              name={label}
              description={account ? account.email_address : desc}
              status={isConn ? 'connected' : isWarn ? 'warning' : 'disconnected'}
              actions={
                account ? (
                  <>
                    <button className="h-7 px-2.5 text-xs border rounded-md hover:bg-accent flex items-center gap-1" onClick={() => handleEmailSync(account.id)}>
                      <RefreshCw className="h-3 w-3" />Sync
                    </button>
                    <button className="h-7 px-2.5 text-xs text-muted-foreground hover:text-destructive flex items-center gap-1" onClick={() => handleEmailDisconnect(account.id)}>
                      <Trash2 className="h-3 w-3" />Remove
                    </button>
                  </>
                ) : (
                  <button
                    className="h-7 px-2.5 text-xs border rounded-md hover:bg-accent flex items-center gap-1.5 disabled:opacity-50"
                    disabled={connectingEmail === provider}
                    onClick={() => handleEmailConnect(provider)}
                  >
                    {connectingEmail === provider ? <Loader2 className="h-3 w-3 animate-spin" /> : <Plug className="h-3 w-3" />}
                    Connect
                  </button>
                )
              }
              extra={account?.last_sync_at ? <p className="text-[11px] text-muted-foreground">Last sync {new Date(account.last_sync_at).toLocaleString()}</p> : undefined}
            />
          );
        })}
      </section>

      {/* ── Accounting ── */}
      <section className="space-y-3">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Accounting & Finance</h3>
        <IntegrationCard
          accent="#2CA01C"
          icon={FileText}
          name="QuickBooks Online"
          description={qbStatus?.account?.company_name ?? 'Sync invoices, customers, payments, and expenses with QuickBooks.'}
          status={qbConnected ? 'connected' : qbNeedsReauth ? 'warning' : 'disconnected'}
          statusLabel={qbConnected ? qbStatus?.account?.company_name ? 'Connected' : 'Connected' : undefined}
          actions={
            qbLoading ? <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" /> :
            qbConnected ? (
              <>
                <button className="h-7 px-2.5 text-xs border rounded-md hover:bg-accent flex items-center gap-1 disabled:opacity-50" onClick={handleQbSync} disabled={qbSyncing}>
                  {qbSyncing ? <Loader2 className="h-3 w-3 animate-spin" /> : <RefreshCw className="h-3 w-3" />}Sync
                </button>
                <button className="h-7 px-2.5 text-xs text-muted-foreground hover:text-destructive flex items-center gap-1 disabled:opacity-50" onClick={handleQbDisconnect} disabled={qbDisconnecting}>
                  <Trash2 className="h-3 w-3" />Disconnect
                </button>
              </>
            ) : qbNeedsReauth ? (
              <button className="h-7 px-2.5 text-xs border rounded-md hover:bg-accent flex items-center gap-1" onClick={handleQbRefresh}>
                <RefreshCw className="h-3 w-3" />Reauthorize
              </button>
            ) : (
              <button className="h-7 px-2.5 text-xs border rounded-md hover:bg-accent flex items-center gap-1.5" onClick={handleQbConnect}>
                <Plug className="h-3 w-3" />Connect
              </button>
            )
          }
          extra={qbConnected && qbStatus?.account?.last_sync_at
            ? <p className="text-[11px] text-muted-foreground">Last sync {new Date(qbStatus.account.last_sync_at).toLocaleString()}</p>
            : undefined
          }
        />
      </section>

      {/* ── Productivity ── */}
      <section className="space-y-3">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Productivity & Data</h3>

        {/* Airtable */}
        <IntegrationCard
          accent="#FF0000"
          icon={FileText}
          name="Airtable"
          description={isAirtableConnected && config?.airtable?.user_email ? config.airtable.user_email : 'Connect Airtable with a Personal Access Token to give Nora and agents access to your bases.'}
          status={isAirtableConnected ? 'connected' : 'disconnected'}
          actions={
            isAirtableConnected ? (
              <button className="h-7 px-2.5 text-xs text-muted-foreground hover:text-destructive flex items-center gap-1" onClick={handleAirtableDisconnect}>
                <Trash2 className="h-3 w-3" />Remove
              </button>
            ) : null
          }
          extra={
            !isAirtableConnected ? (
              <div className="space-y-2 pt-1">
                <div className="flex gap-2">
                  <input
                    type="password"
                    value={airtableToken}
                    onChange={e => setAirtableToken(e.target.value)}
                    placeholder="patXXXXXXXXXXXXXX"
                    className="flex-1 h-8 px-3 text-xs border rounded-md bg-background focus:outline-none focus:ring-1 focus:ring-ring"
                  />
                  <button
                    className="h-8 px-3 text-xs border rounded-md hover:bg-accent flex items-center gap-1.5 disabled:opacity-50"
                    onClick={handleAirtableSave}
                    disabled={airtableVerifying || !airtableToken}
                  >
                    {airtableVerifying ? <Loader2 className="h-3 w-3 animate-spin" /> : <CheckCircle2 className="h-3 w-3" />}
                    Verify & Save
                  </button>
                </div>
                {airtableError && <p className="text-[11px] text-destructive">{airtableError}</p>}
                <p className="text-[11px] text-muted-foreground">
                  Create a token at <span className="text-primary">airtable.com/create/tokens</span> with <code className="bg-muted px-1 rounded">data.records:read</code> + <code className="bg-muted px-1 rounded">schema.bases:read</code> scopes.
                </p>
              </div>
            ) : undefined
          }
        />

        {/* Dropbox */}
        <IntegrationCard
          accent="#0061FF"
          icon={FileText}
          name="Dropbox"
          description="Connect Dropbox folders as knowledge sources and asset storage for projects."
          status="disconnected"
          statusLabel="Coming soon"
          actions={
            <span className="text-[11px] text-muted-foreground italic">Configure per-project</span>
          }
        />

        {/* OneDrive */}
        <IntegrationCard
          accent="#0078D4"
          icon={FileText}
          name="OneDrive"
          description="Access Microsoft OneDrive files as knowledge sources and shared asset storage across projects."
          status="disconnected"
          statusLabel="Coming soon"
          actions={
            <span className="text-[11px] text-muted-foreground italic">Not yet configured</span>
          }
        />

        {/* GitHub */}
        <IntegrationCard
          accent="#24292e"
          icon={FileText}
          name="GitHub"
          description="Link repositories to projects. Agents can read code, create PRs, and browse issues."
          status="disconnected"
          statusLabel="Coming soon"
          actions={
            <span className="text-[11px] text-muted-foreground italic">Configure per-project</span>
          }
        />
      </section>

      {/* ── Social ── */}
      <SocialSection />

      {/* ── Communication ── */}
      <CommunicationSection />

      {/* ── Commerce ── */}
      <section className="space-y-3">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Commerce</h3>
        <IntegrationCard
          accent="#635BFF"
          icon={DollarSign}
          name="Stripe"
          description="Sync payments, subscriptions, and invoices. Enable Stripe billing for clients."
          status="disconnected"
          statusLabel="Coming soon"
          actions={<span className="text-[11px] text-muted-foreground italic">Not yet configured</span>}
        />
        <IntegrationCard
          accent="#96BF48"
          icon={Boxes}
          name="Shopify"
          description="Connect your Shopify store for order and product data access by agents."
          status="disconnected"
          statusLabel="Coming soon"
          actions={<span className="text-[11px] text-muted-foreground italic">Not yet configured</span>}
        />
        <IntegrationCard
          accent="#7C3AED"
          icon={DollarSign}
          name="VIBE Wallet"
          description="Manage on-chain VIBE token balances and project funding via the Aptos network."
          status="connected"
          statusLabel="Active"
          actions={
            <Link to="/settings/wallet" className="h-7 px-2.5 text-xs border rounded-md hover:bg-accent flex items-center gap-1.5">
              <ExternalLink className="h-3 w-3" />Wallet Settings
            </Link>
          }
        />
      </section>

      {/* ── Development ── */}
      <DevelopmentSection />
    </div>
  );
}

// ── Social Section ────────────────────────────────────────────────────────────
function SocialSection() {
  const socialPlatforms: { name: string; icon: React.ElementType; accent: string; desc: string }[] = [
    { name: 'Instagram',  icon: Instagram,    accent: '#E1306C', desc: 'Schedule posts, track mentions, and monitor engagement.' },
    { name: 'LinkedIn',   icon: Linkedin,     accent: '#0A66C2', desc: 'Company page management, posts, and B2B lead tracking.' },
    { name: 'X / Twitter', icon: Twitter,    accent: '#000000', desc: 'Post scheduling, mention monitoring, and DM management.' },
    { name: 'Facebook',   icon: Facebook,     accent: '#1877F2', desc: 'Page management, ads integration, and audience insights.' },
    { name: 'YouTube',    icon: Youtube,      accent: '#FF0000', desc: 'Channel analytics, comment monitoring, and content sync.' },
    { name: 'TikTok',     icon: Share2,       accent: '#010101', desc: 'Video scheduling and performance analytics.' },
    { name: 'Threads',    icon: MessageSquare, accent: '#101010', desc: 'Thread management and audience engagement.' },
    { name: 'Bluesky',    icon: Share2,       accent: '#0085FF', desc: 'Decentralised social — post scheduling and monitoring.' },
    { name: 'Pinterest',  icon: Share2,       accent: '#E60023', desc: 'Pin management, board sync, and product catalogue.' },
  ];

  return (
    <section className="space-y-3">
      <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Social Media</h3>
      <p className="text-xs text-muted-foreground">
        Social accounts are connected per-project to keep brand identities scoped. Visit a project's settings to connect individual platforms.
      </p>
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
        {socialPlatforms.map(({ name, icon: Icon, accent, desc }) => (
          <div
            key={name}
            className="flex items-start gap-3 p-3 rounded-lg border border-border/60 bg-card/60"
          >
            <div className="w-1 self-stretch rounded-full shrink-0" style={{ background: accent }} />
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2 mb-0.5">
                <Icon className="h-4 w-4 shrink-0" style={{ color: accent }} />
                <span className="text-sm font-medium">{name}</span>
              </div>
              <p className="text-[11px] text-muted-foreground leading-relaxed">{desc}</p>
            </div>
            <span className="text-[11px] text-muted-foreground italic shrink-0 mt-0.5">Per-project</span>
          </div>
        ))}
      </div>
    </section>
  );
}

// ── Communication Section ────────────────────────────────────────────────────
function CommunicationSection() {
  const { data: discordSessions = [] } = useQuery<DiscordSessionSummary[]>({
    queryKey: ['discord-active-sessions'],
    queryFn: () => discordApi.activeSessions(),
    staleTime: 30_000,
    retry: false,
  });

  return (
    <section className="space-y-3">
      <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Communication</h3>

      {/* Discord */}
      <IntegrationCard
        accent="#5865F2"
        icon={MessageSquare}
        name="Discord"
        description={
          discordSessions.length > 0
            ? `${discordSessions.length} active voice session${discordSessions.length !== 1 ? 's' : ''} — Nora is listening`
            : 'Nora joins voice channels and transcribes meetings. Configure in bot settings.'
        }
        status={discordSessions.length > 0 ? 'connected' : 'disconnected'}
        statusLabel={discordSessions.length > 0 ? 'Active' : 'Idle'}
        actions={
          <Link to="/discord" className="h-7 px-2.5 text-xs border rounded-md hover:bg-accent flex items-center gap-1.5">
            <ExternalLink className="h-3 w-3" />Manage Sessions
          </Link>
        }
        extra={
          discordSessions.length > 0 ? (
            <div className="space-y-1 pt-1">
              {discordSessions.slice(0, 3).map(s => (
                <p key={s.meeting_session_id} className="text-[11px] text-muted-foreground">
                  #{s.channel_name} · {s.guild_id}
                </p>
              ))}
            </div>
          ) : undefined
        }
      />

      {/* Twilio */}
      <IntegrationCard
        accent="#F22F46"
        icon={Radio}
        name="Twilio (Nora Phone)"
        description="Nora answers inbound calls and SMS. Outbound calling for CRM outreach."
        status="connected"
        statusLabel="Active"
        actions={
          <span className="text-[11px] text-muted-foreground italic">Managed via environment config</span>
        }
      />
    </section>
  );
}

// ── Development Section ───────────────────────────────────────────────────────
function DevelopmentSection() {
  const { data: ghStatus } = useQuery<string>({
    queryKey: ['github-token-status'],
    queryFn: () => githubAuthApi.checkGithubToken() as unknown as Promise<string>,
    staleTime: 60_000,
  });

  const ghConnected = ghStatus === 'VALID';

  return (
    <section className="space-y-3">
      <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Development</h3>

      {/* GitHub */}
      <IntegrationCard
        accent="#24292e"
        icon={FileText}
        name="GitHub"
        description={ghConnected ? 'GitHub account connected — agents can read repos, create PRs, and browse issues.' : 'Link your GitHub account so agents can read repos, create PRs, and browse issues.'}
        status={ghConnected ? 'connected' : 'disconnected'}
        actions={
          ghConnected ? (
            <span className="text-[11px] text-muted-foreground italic">Connected via agent settings</span>
          ) : (
            <Link to="/settings/agents" className="h-7 px-2.5 text-xs border rounded-md hover:bg-accent flex items-center gap-1.5">
              <Plug className="h-3 w-3" />Connect in Agent Settings
            </Link>
          )
        }
      />

      {/* Virtual Environment */}
      <IntegrationCard
        accent="#06B6D4"
        icon={Boxes}
        name="Virtual Environment"
        description="Sandboxed containers for agent code execution, shell access, and file operations."
        status="connected"
        statusLabel="Active"
        actions={
          <Link to="/virtual-environment" className="h-7 px-2.5 text-xs border rounded-md hover:bg-accent flex items-center gap-1.5">
            <ExternalLink className="h-3 w-3" />Open
          </Link>
        }
      />

      {/* Zapier / Webhooks placeholder */}
      <IntegrationCard
        accent="#FF4A00"
        icon={Network}
        name="Zapier / Webhooks"
        description="Connect any external tool via Zapier automations or custom HTTP webhooks."
        status="disconnected"
        statusLabel="Coming soon"
        actions={<span className="text-[11px] text-muted-foreground italic">Not yet configured</span>}
      />
    </section>
  );
}

// ── Members Tab ───────────────────────────────────────────────────────────────

function MemberAssignments({ orgId, userId }: { orgId: string; userId: string }) {
  const queryClient = useQueryClient();
  const { data: assignments, isLoading } = useQuery({
    queryKey: ['member-assignments', orgId, userId],
    queryFn: () => organizationsApi.getMemberAssignments(orgId, userId),
  });

  // Fetch org projects and clients for assignment dropdowns
  const { data: orgClients = [] } = useQuery<ClientData[]>({
    queryKey: ['orgClients', orgId],
    queryFn: () => organizationsApi.getClients(orgId),
  });

  const [assignType, setAssignType] = useState<string>('');
  const [assignTargetId, setAssignTargetId] = useState('');
  const [assignRole, setAssignRole] = useState('editor');

  // Fetch org projects via API
  const { data: orgProjects = [] } = useQuery<any[]>({
    queryKey: ['org-projects-list', orgId],
    queryFn: async () => {
      const res = await fetch(resolveApiUrl(`/api/projects?organization_id=${orgId}`), { credentials: 'include' });
      if (!res.ok) return [];
      const data = await res.json();
      return data.data || [];
    },
  });

  const assignMutation = useMutation({
    mutationFn: () => organizationsApi.assignMember(orgId, userId, assignType, assignTargetId, assignRole),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['member-assignments', orgId, userId] });
      queryClient.invalidateQueries({ queryKey: ['sidebarTree'] });
      setAssignType('');
      setAssignTargetId('');
    },
  });

  const unassignProjectMutation = useMutation({
    mutationFn: (projectId: string) => organizationsApi.unassignProject(orgId, userId, projectId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['member-assignments', orgId, userId] });
      queryClient.invalidateQueries({ queryKey: ['sidebarTree'] });
    },
  });

  const unassignClientMutation = useMutation({
    mutationFn: (clientId: string) => organizationsApi.unassignClient(orgId, userId, clientId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['member-assignments', orgId, userId] });
      queryClient.invalidateQueries({ queryKey: ['sidebarTree'] });
    },
  });

  if (isLoading) return <div className="text-xs text-muted-foreground py-2">Loading assignments...</div>;

  const projectAssignments = assignments?.projects || [];
  const clientAssignments = assignments?.clients || [];
  const taskAssignments = assignments?.tasks || [];
  const watchedTasks = assignments?.watched_tasks || [];

  return (
    <div className="pl-11 pb-3 space-y-3">
      {/* Assigned projects */}
      {projectAssignments.length > 0 && (
        <div>
          <p className="text-xs font-medium text-muted-foreground mb-1">Projects</p>
          <div className="flex flex-wrap gap-1">
            {projectAssignments.map((p: any) => (
              <Badge key={p.project_id} variant="secondary" className="text-xs gap-1">
                <FolderOpen className="h-3 w-3" />
                {p.project_name} ({p.role})
                <button onClick={() => unassignProjectMutation.mutate(p.project_id)} className="ml-1 hover:text-destructive">
                  <X className="h-3 w-3" />
                </button>
              </Badge>
            ))}
          </div>
        </div>
      )}

      {/* Assigned clients */}
      {clientAssignments.length > 0 && (
        <div>
          <p className="text-xs font-medium text-muted-foreground mb-1">Clients</p>
          <div className="flex flex-wrap gap-1">
            {clientAssignments.map((c: any) => (
              <Badge key={c.client_id} variant="secondary" className="text-xs gap-1">
                <Briefcase className="h-3 w-3" />
                {c.client_name} ({c.role})
                <button onClick={() => unassignClientMutation.mutate(c.client_id)} className="ml-1 hover:text-destructive">
                  <X className="h-3 w-3" />
                </button>
              </Badge>
            ))}
          </div>
        </div>
      )}

      {/* Assigned tasks */}
      {taskAssignments.length > 0 && (
        <div>
          <p className="text-xs font-medium text-muted-foreground mb-1">Tasks (assignee)</p>
          <div className="flex flex-wrap gap-1">
            {taskAssignments.map((t: any) => (
              <Badge key={t.task_id} variant="outline" className="text-xs">
                {t.title} <span className="text-muted-foreground ml-1">({t.project_name})</span>
              </Badge>
            ))}
          </div>
        </div>
      )}

      {/* Watched tasks */}
      {watchedTasks.length > 0 && (
        <div>
          <p className="text-xs font-medium text-muted-foreground mb-1">Tasks (watching)</p>
          <div className="flex flex-wrap gap-1">
            {watchedTasks.map((t: any) => (
              <Badge key={t.task_id} variant="outline" className="text-xs">
                <Eye className="h-3 w-3 mr-1" />
                {t.title} <span className="text-muted-foreground ml-1">({t.project_name})</span>
              </Badge>
            ))}
          </div>
        </div>
      )}

      {projectAssignments.length === 0 && clientAssignments.length === 0 && taskAssignments.length === 0 && (
        <p className="text-xs text-muted-foreground">No assignments yet</p>
      )}

      {/* Quick assign */}
      <div className="flex items-center gap-2 pt-1">
        <Select value={assignType} onValueChange={(v) => { setAssignType(v); setAssignTargetId(''); }}>
          <SelectTrigger className="w-[120px] h-7 text-xs">
            <SelectValue placeholder="Assign to..." />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="project">Project</SelectItem>
            <SelectItem value="client">Client</SelectItem>
          </SelectContent>
        </Select>

        {assignType === 'project' && (
          <>
            <Select value={assignTargetId} onValueChange={setAssignTargetId}>
              <SelectTrigger className="w-[180px] h-7 text-xs">
                <SelectValue placeholder="Select project..." />
              </SelectTrigger>
              <SelectContent>
                {orgProjects
                  .filter((p: any) => !projectAssignments.some((a: any) => a.project_id === p.id))
                  .map((p: any) => (
                    <SelectItem key={p.id} value={p.id}>{p.name}</SelectItem>
                  ))}
              </SelectContent>
            </Select>
            <Select value={assignRole} onValueChange={setAssignRole}>
              <SelectTrigger className="w-[90px] h-7 text-xs">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="viewer">Viewer</SelectItem>
                <SelectItem value="editor">Editor</SelectItem>
                <SelectItem value="admin">Admin</SelectItem>
              </SelectContent>
            </Select>
          </>
        )}

        {assignType === 'client' && (
          <Select value={assignTargetId} onValueChange={setAssignTargetId}>
            <SelectTrigger className="w-[180px] h-7 text-xs">
              <SelectValue placeholder="Select client..." />
            </SelectTrigger>
            <SelectContent>
              {orgClients
                .filter((c: any) => !clientAssignments.some((a: any) => a.client_id === c.id))
                .map((c: any) => (
                  <SelectItem key={c.id} value={c.id}>{c.name}</SelectItem>
                ))}
            </SelectContent>
          </Select>
        )}

        {assignType && assignTargetId && (
          <Button size="sm" variant="outline" className="h-7 text-xs" onClick={() => assignMutation.mutate()} disabled={assignMutation.isPending}>
            <Plus className="h-3 w-3 mr-1" />
            Assign
          </Button>
        )}
      </div>
    </div>
  );
}

function MembersTab({ orgId, orgName }: { orgId: string; orgName: string }) {
  const queryClient = useQueryClient();
  const { data: members = [], isLoading } = useQuery<OrgMember[]>({
    queryKey: ['org-members', orgId],
    queryFn: () => organizationsApi.getMembers(orgId),
    enabled: !!orgId,
  });

  // All users for add-member dropdown
  const { data: allUsers = [] } = useQuery<any[]>({
    queryKey: ['all-users'],
    queryFn: async () => {
      const res = await fetch(resolveApiUrl('/api/users'), { credentials: 'include' });
      if (!res.ok) return [];
      const data = await res.json();
      return data.data || [];
    },
  });

  const [expandedMember, setExpandedMember] = useState<string | null>(null);
  const [showAddMember, setShowAddMember] = useState(false);
  const [addUserId, setAddUserId] = useState('');
  const [addRole, setAddRole] = useState('member');
  const [showInviteDialog, setShowInviteDialog] = useState(false);
  const [inviteRole, setInviteRole] = useState('member');
  const [inviteLink, setInviteLink] = useState('');
  const [copied, setCopied] = useState(false);

  const availableUsers = allUsers.filter(
    (u: any) => !members.some((m) => m.user_id === u.id)
  );

  const addMemberMutation = useMutation({
    mutationFn: () => organizationsApi.addMember(orgId, addUserId, addRole),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['org-members', orgId] });
      setAddUserId('');
      setAddRole('member');
      setShowAddMember(false);
    },
  });

  const removeMemberMutation = useMutation({
    mutationFn: (userId: string) => organizationsApi.removeMember(orgId, userId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['org-members', orgId] });
    },
  });

  const changeRoleMutation = useMutation({
    mutationFn: ({ userId, role }: { userId: string; role: string }) =>
      organizationsApi.changeMemberRole(orgId, userId, role),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['org-members', orgId] });
    },
  });

  const createInviteMutation = useMutation({
    mutationFn: () => organizationsApi.createInvitation(orgId, inviteRole),
    onSuccess: (data: any) => {
      setInviteLink(data.invite_url || '');
    },
  });

  const handleCopyLink = () => {
    navigator.clipboard.writeText(inviteLink);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <Card className="bg-card/80 backdrop-blur-sm border-border/50">
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle>Team Members</CardTitle>
            <CardDescription>Manage who has access to {orgName}</CardDescription>
          </div>
          <div className="flex gap-2">
            <Button size="sm" variant="outline" onClick={() => { setShowInviteDialog(true); setInviteLink(''); }}>
              <Link2 className="h-4 w-4 mr-2" />
              Invite Link
            </Button>
            <Button size="sm" onClick={() => setShowAddMember(true)}>
              <UserPlus className="h-4 w-4 mr-2" />
              Add Member
            </Button>
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-3">
        {/* Add member inline form */}
        {showAddMember && (
          <div className="flex items-center gap-2 p-3 border rounded-lg bg-muted/50">
            <Select value={addUserId} onValueChange={setAddUserId}>
              <SelectTrigger className="flex-1">
                <SelectValue placeholder="Select user..." />
              </SelectTrigger>
              <SelectContent>
                {availableUsers.map((u: any) => (
                  <SelectItem key={u.id} value={u.id}>
                    {u.full_name || u.username} <span className="text-muted-foreground ml-1">@{u.username}</span>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <Select value={addRole} onValueChange={setAddRole}>
              <SelectTrigger className="w-[120px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="viewer">Viewer</SelectItem>
                <SelectItem value="member">Member</SelectItem>
                <SelectItem value="admin">Admin</SelectItem>
              </SelectContent>
            </Select>
            <Button size="sm" onClick={() => addMemberMutation.mutate()} disabled={!addUserId || addMemberMutation.isPending}>
              Add
            </Button>
            <Button size="sm" variant="ghost" onClick={() => setShowAddMember(false)}>
              <X className="h-4 w-4" />
            </Button>
          </div>
        )}

        {/* Member list */}
        {isLoading ? (
          <div className="text-center py-8 text-muted-foreground">
            <Loader2 className="h-5 w-5 mx-auto mb-2 animate-spin" />
            Loading members...
          </div>
        ) : members.length === 0 ? (
          <div className="text-center py-8 text-muted-foreground">
            <Users className="h-8 w-8 mx-auto mb-2 opacity-40" />
            <p>No members yet. Add members or send an invite link.</p>
          </div>
        ) : (
          <div className="space-y-1">
            {members.map((m: OrgMember) => {
              const isExpanded = expandedMember === m.user_id;
              const displayName = m.user?.full_name || m.user?.username || m.user_id;
              return (
                <div key={m.id} className="border rounded-lg">
                  <div
                    className="flex items-center justify-between p-3 cursor-pointer hover:bg-muted/50"
                    onClick={() => setExpandedMember(isExpanded ? null : m.user_id)}
                  >
                    <div className="flex items-center gap-3">
                      {isExpanded ? <ChevronDown className="h-4 w-4 text-muted-foreground" /> : <ChevronRight className="h-4 w-4 text-muted-foreground" />}
                      <div className="h-8 w-8 rounded-full bg-muted flex items-center justify-center text-sm font-medium">
                        {displayName[0].toUpperCase()}
                      </div>
                      <div>
                        <p className="text-sm font-medium">{displayName}</p>
                        {m.user?.email && (
                          <p className="text-xs text-muted-foreground">{m.user.email}</p>
                        )}
                      </div>
                    </div>
                    <div className="flex items-center gap-2" onClick={(e) => e.stopPropagation()}>
                      <Select
                        value={m.role}
                        onValueChange={(role) => changeRoleMutation.mutate({ userId: m.user_id, role })}
                      >
                        <SelectTrigger className="w-[110px] h-7 text-xs">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="viewer"><div className="flex items-center gap-1"><Eye className="h-3 w-3" /> Viewer</div></SelectItem>
                          <SelectItem value="member"><div className="flex items-center gap-1"><Users className="h-3 w-3" /> Member</div></SelectItem>
                          <SelectItem value="admin"><div className="flex items-center gap-1"><Shield className="h-3 w-3" /> Admin</div></SelectItem>
                        </SelectContent>
                      </Select>
                      <span className="text-xs text-muted-foreground hidden sm:inline">
                        {formatDate(m.joined_at)}
                      </span>
                      <Button
                        size="sm"
                        variant="ghost"
                        className="h-7 w-7 p-0"
                        onClick={() => removeMemberMutation.mutate(m.user_id)}
                      >
                        <Trash2 className="h-3.5 w-3.5 text-destructive" />
                      </Button>
                    </div>
                  </div>
                  {isExpanded && (
                    <MemberAssignments orgId={orgId} userId={m.user_id} />
                  )}
                </div>
              );
            })}
          </div>
        )}
      </CardContent>

      {/* Invite Link Dialog */}
      <Dialog open={showInviteDialog} onOpenChange={setShowInviteDialog}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Create Invite Link</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <div>
              <Label>Role for new members</Label>
              <Select value={inviteRole} onValueChange={setInviteRole}>
                <SelectTrigger className="mt-1">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="viewer">Viewer</SelectItem>
                  <SelectItem value="member">Member</SelectItem>
                  <SelectItem value="admin">Admin</SelectItem>
                </SelectContent>
              </Select>
            </div>
            {!inviteLink ? (
              <Button onClick={() => createInviteMutation.mutate()} disabled={createInviteMutation.isPending} className="w-full">
                {createInviteMutation.isPending ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : <Link2 className="h-4 w-4 mr-2" />}
                Generate Link
              </Button>
            ) : (
              <div className="space-y-2">
                <div className="flex items-center gap-2">
                  <Input value={inviteLink} readOnly className="text-xs" />
                  <Button size="sm" variant="outline" onClick={handleCopyLink}>
                    <Copy className="h-4 w-4" />
                  </Button>
                </div>
                {copied && <p className="text-xs text-green-600">Copied to clipboard!</p>}
                <p className="text-xs text-muted-foreground">This link expires in 7 days.</p>
              </div>
            )}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowInviteDialog(false)}>Close</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </Card>
  );
}

// ── Main Page ─────────────────────────────────────────────────────────────────

// Map from path segments to tab names
const PATH_TO_TAB: Record<string, string> = {
  crm: 'overview',
  'crm/contacts': 'contacts',
  'crm/companies': 'companies',
  'crm/pipeline': 'pipelines',
  'crm/deliverables': 'deliverables',
  social: 'social',
  intelligence: 'knowledge',
  'intelligence/data-sources': 'knowledge',
  'intelligence/artifacts': 'knowledge',
  'intelligence/workflows': 'knowledge',
  'intelligence/pulse': 'knowledge',
  'intelligence/topology': 'knowledge',
  members: 'members',
  projects: 'projects',
  integrations: 'integrations',
};

// Map from path segments to intelligence view
const PATH_TO_VIEW: Record<string, string> = {
  'intelligence/data-sources': 'datasources',
  'intelligence/artifacts': 'artifacts',
  'intelligence/workflows': 'workflows',
  'intelligence/pulse': 'pulse',
  'intelligence/topology': 'topology',
};

export function OrganizationProfilePage({ defaultTab, defaultPipeline }: OrganizationProfilePageProps = {}) {
  const { orgId } = useParams<{ orgId: string }>();
  const [searchParams, setSearchParams] = useSearchParams();
  const location = useLocation();
  const navigate = useNavigate();

  // Derive tab from URL path (new route pattern) or search params (legacy)
  const orgBase = `/organizations/${orgId}`;
  const pathSuffix = location.pathname.startsWith(orgBase)
    ? location.pathname.slice(orgBase.length + 1) // strip leading "/"
    : '';

  const tabFromPath = pathSuffix ? PATH_TO_TAB[pathSuffix] : undefined;
  const viewFromPath = pathSuffix ? PATH_TO_VIEW[pathSuffix] : undefined;
  const usingPathRoutes = !!tabFromPath;

  const tabFromUrl = tabFromPath || searchParams.get('tab') || defaultTab || 'overview';
  const pipelineFromUrl = searchParams.get('pipeline') || defaultPipeline;
  const clientFilter = searchParams.get('client');
  const viewFromUrl = viewFromPath || searchParams.get('view');

  const clearClientFilter = () => {
    if (usingPathRoutes) {
      navigate(`${orgBase}/projects`);
    } else {
      const params = new URLSearchParams(searchParams);
      params.delete('client');
      setSearchParams(params, { replace: true });
    }
  };

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
    queryFn: () => organizationsApi.getClients(orgId!),
    enabled: !!orgId,
  });

  const { data: sidebarTree } = useQuery({
    queryKey: ['sidebarTree'],
    queryFn: () => organizationsApi.getSidebarTree(),
    staleTime: 60_000,
  });

  const { data: orgDeals = [] } = useQuery({
    queryKey: ['org-deals', orgId],
    queryFn: () => crmDealsApi.listOrgDeals(orgId!),
    enabled: !!orgId,
    staleTime: 60_000,
  });

  const { contacts } = useOrgContacts(orgId);

  const sidebarOrg = sidebarTree
    ? [...(sidebarTree.owned_orgs || []), ...(sidebarTree.member_orgs || [])].find(o => o.id === orgId)
    : undefined;

  const allProjects = useMemo(() => {
    if (!sidebarOrg) return [];
    const collectProjects = (projects: any[]): any[] =>
      projects.flatMap((p: any) => [p, ...collectProjects(p.children || [])]);
    return [
      ...collectProjects(sidebarOrg.internal_projects || []),
      ...(sidebarOrg.clients || []).flatMap((c: any) => collectProjects(c.projects || [])),
    ];
  }, [sidebarOrg]);

  const totalDealValue = useMemo(
    () => orgDeals.reduce((sum, d) => sum + (d.amount || 0), 0),
    [orgDeals]
  );

  if (orgLoading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-muted-foreground">Loading organisation...</div>
      </div>
    );
  }

  if (!org || !orgId) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-muted-foreground">Organisation not found.</div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full bg-background">
      {/* Header */}
      <div className="border-b bg-card/50 backdrop-blur-sm shadow-sm">
        <div className="max-w-[1600px] mx-auto px-6 py-5">
          <div className="flex items-center gap-4">
            <Link to="/projects" className="text-muted-foreground hover:text-foreground transition-colors">
              <ArrowLeft className="h-5 w-5" />
            </Link>
            <div className="p-2.5 bg-blue-100 dark:bg-blue-950 rounded-xl">
              <Building2 className="w-6 h-6 text-blue-600" />
            </div>
            <div className="min-w-0">
              <h1 className="text-2xl font-bold text-foreground">{org.name}</h1>
              {org.description && (
                <p className="text-sm text-muted-foreground mt-0.5 truncate">{org.description}</p>
              )}
            </div>
            <div className="ml-auto flex items-center gap-3">
              <Badge variant={org.is_active ? 'default' : 'secondary'} className="text-xs">
                {org.is_active ? 'Active' : 'Inactive'}
              </Badge>
            </div>
          </div>

          {/* Stat pills — overview only */}
          {tabFromUrl === 'overview' && (
            <div className="flex items-center gap-3 mt-4 flex-wrap">
              <StatPill icon={FolderOpen} label="Projects" value={allProjects.length} />
              <StatPill icon={Briefcase} label="Clients" value={clients.length} />
              <StatPill icon={Users} label="Members" value={members.length} />
              <StatPill icon={DollarSign} label="Pipeline" value={formatCurrency(totalDealValue)} />
            </div>
          )}
        </div>
      </div>

      {/* Tabs */}
      <div className="flex-1 overflow-auto">
        <div className="max-w-[1600px] mx-auto px-6 py-5">
          {tabFromUrl === 'overview' && (
              <OverviewTab
                orgId={orgId}
                projectEntries={allProjects.map(p => ({ id: p.id, name: p.name }))}
                projectCount={allProjects.length}
                clientCount={clients.length}
                memberCount={members.length}
                totalDealValue={totalDealValue}
                totalDeals={orgDeals.length}
                contactCount={contacts.length}
              />
            )}

            {tabFromUrl === 'pipelines' && (
              <PipelinesTab orgId={orgId} defaultPipeline={pipelineFromUrl || undefined} />
            )}

            {tabFromUrl === 'contacts' && (
              <ContactsTab orgId={orgId} />
            )}

            {tabFromUrl === 'companies' && (
              <CompaniesTab orgId={orgId} />
            )}

            {tabFromUrl === 'deliverables' && (
              <DeliverablesTab orgId={orgId} projectEntries={allProjects.map(p => ({ id: p.id, name: p.name }))} />
            )}

            {tabFromUrl === 'projects' && (
              <ProjectsTab
                orgId={orgId}
                sidebarOrg={sidebarOrg}
                clientFilter={clientFilter}
                onClearClientFilter={clearClientFilter}
              />
            )}

            {tabFromUrl === 'social' && (
              <SocialTab projectEntries={allProjects.map(p => ({ id: p.id, name: p.name }))} />
            )}

            {tabFromUrl === 'knowledge' && (
              <KnowledgeTab
                orgId={orgId!}
                projectEntries={allProjects.map(p => ({ id: p.id, name: p.name }))}
                view={viewFromUrl}
              />
            )}

            {tabFromUrl === 'members' && (
              <MembersTab orgId={orgId} orgName={org.name} />
            )}

            {tabFromUrl === 'integrations' && (
              <IntegrationsTab orgId={orgId} />
            )}
        </div>
      </div>
    </div>
  );
}

function StatPill({ icon: Icon, label, value }: { icon: any; label: string; value: string | number }) {
  return (
    <div className="flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-muted/50 text-sm">
      <Icon className="h-3.5 w-3.5 text-muted-foreground" />
      <span className="text-muted-foreground">{label}</span>
      <span className="font-semibold">{value}</span>
    </div>
  );
}
