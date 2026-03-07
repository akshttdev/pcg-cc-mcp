import { useState, lazy, Suspense } from 'react';
const ProposalCreateModal = lazy(() =>
  import('@/components/dialogs/ProposalCreateModal').then((m) => ({ default: m.ProposalCreateModal }))
);
import { useParams, useNavigate, Link } from 'react-router-dom';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Skeleton } from '@/components/ui/skeleton';
import {
  ArrowLeft,
  Mail,
  Phone,
  Globe,
  Building2,
  Linkedin,
  Twitter,
  Instagram,
  Youtube,
  Github,
  Star,
  Sparkles,
  DollarSign,
  TrendingUp,
  Users,
  RefreshCw,
  ExternalLink,
  Plus,
  MessageSquare,
  Pencil,
  Save,
  X,
} from 'lucide-react';
import {
  personsApi,
  intelligenceApi,
  companiesApi,
  organizationsApi,
  type PersonWithSocials,
  type PersonSocialProfile,
  type PersonCompanyRole,
  type InvoiceRecord,
  type IntelligenceStatus,
  type UpdatePersonInput,
  type CompanyRecord,
  type OrganizationData,
} from '@/lib/api';
import { formatDistanceToNow } from 'date-fns';
import { toast } from 'sonner';

// ── Constants ───────────────────────────────────────────────────────────────

const PERSON_TYPE_INFO: Record<string, { label: string; color: string }> = {
  team:       { label: 'Team',       color: 'bg-violet-100 text-violet-700' },
  client:     { label: 'Client',     color: 'bg-green-100 text-green-700' },
  contractor: { label: 'Contractor', color: 'bg-amber-100 text-amber-700' },
  lead:       { label: 'Lead',       color: 'bg-blue-100 text-blue-700' },
  partner:    { label: 'Partner',    color: 'bg-pink-100 text-pink-700' },
  contact:    { label: 'Contact',    color: 'bg-gray-100 text-gray-600' },
};

const FINANCIAL_ROLE_INFO: Record<string, { label: string; description: string; color: string }> = {
  taker:   { label: 'Taker',   description: 'Client — spends VIBE for services', color: 'text-red-600 bg-red-50' },
  giver:   { label: 'Giver',   description: 'Contributor — earns VIBE for work', color: 'text-green-600 bg-green-50' },
  both:    { label: 'Both',    description: 'Spends and earns VIBE on the platform', color: 'text-purple-600 bg-purple-50' },
  neutral: { label: 'Neutral', description: 'No financial relationship yet', color: 'text-gray-500 bg-gray-50' },
};

const BUSINESS_STAGE_INFO: Record<string, { label: string; color: string; order: number }> = {
  idea:                  { label: '💡 Idea',           color: 'text-gray-500',  order: 0 },
  funding:               { label: '💰 Funding',         color: 'text-blue-600',  order: 1 },
  manufacturing:         { label: '🏭 Manufacturing',   color: 'text-amber-600', order: 2 },
  distribution:          { label: '🚚 Distribution',    color: 'text-orange-600',order: 3 },
  customer_acquisition:  { label: '📈 Customers',       color: 'text-green-600', order: 4 },
};

const LIFECYCLE_STAGES = ['subscriber','lead','marketing_qualified','sales_qualified','opportunity','customer','evangelist','other'];
const CHANNELS = ['email','sms','whatsapp','instagram','linkedin','twitter','phone','in_person'];

const PLATFORM_ICONS: Record<string, React.FC<{ className?: string }>> = {
  linkedin:  Linkedin,
  twitter:   Twitter,
  instagram: Instagram,
  youtube:   Youtube,
  github:    Github,
  facebook:  Globe,
  website:   Globe,
};

// ── Sub-components ───────────────────────────────────────────────────────────

function SocialRail({ profiles }: { profiles: PersonSocialProfile[] }) {
  if (profiles.length === 0) return null;

  return (
    <div className="flex items-center gap-3 flex-wrap">
      {profiles.map((p) => {
        const Icon = PLATFORM_ICONS[p.platform] ?? Globe;
        return (
          <a
            key={p.id}
            href={p.profile_url ?? undefined}
            target="_blank"
            rel="noopener noreferrer"
            className="flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors"
            title={p.platform}
          >
            <Icon className="h-4 w-4" />
            {p.handle && <span className="text-xs">@{p.handle}</span>}
            {p.follower_count != null && p.follower_count > 0 && (
              <span className="text-xs text-muted-foreground">
                {p.follower_count.toLocaleString()}
              </span>
            )}
          </a>
        );
      })}
    </div>
  );
}

function BusinessStageTracker({ stage }: { stage: string }) {
  const stages = ['idea', 'funding', 'manufacturing', 'distribution', 'customer_acquisition'];
  const currentIdx = stages.indexOf(stage);

  return (
    <div className="flex items-center gap-1">
      {stages.map((s, idx) => {
        const info = BUSINESS_STAGE_INFO[s];
        const active = s === stage;
        const past = idx < currentIdx;
        return (
          <div
            key={s}
            className={`flex-1 h-1.5 rounded-full transition-all ${
              active ? 'bg-primary' : past ? 'bg-primary/40' : 'bg-border'
            }`}
            title={info.label}
          />
        );
      })}
    </div>
  );
}

function InvoiceRow({ invoice }: { invoice: InvoiceRecord }) {
  const isAr = invoice.invoice_type === 'ar';
  const statusColors: Record<string, string> = {
    draft:     'text-gray-500',
    sent:      'text-blue-600',
    viewed:    'text-blue-400',
    paid:      'text-green-600',
    partial:   'text-yellow-600',
    overdue:   'text-red-600',
    void:      'text-gray-400 line-through',
    cancelled: 'text-gray-400 line-through',
  };

  return (
    <div className="flex items-center justify-between py-2 border-b last:border-0 text-sm">
      <div>
        <span className="font-medium">{invoice.invoice_number}</span>
        {invoice.title && <span className="text-muted-foreground ml-2">{invoice.title}</span>}
      </div>
      <div className="flex items-center gap-3">
        <span className={`capitalize text-xs ${statusColors[invoice.status] ?? ''}`}>
          {invoice.status}
        </span>
        <span className={`font-medium ${isAr ? 'text-green-600' : 'text-amber-600'}`}>
          {isAr ? '+' : '-'}${invoice.amount_usd.toFixed(2)}
        </span>
      </div>
    </div>
  );
}

// ── Main page ────────────────────────────────────────────────────────────────

export function PersonDetailPage() {
  const { personId } = useParams<{ personId: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [researchLoading, setResearchLoading] = useState(false);
  const [researchStatus, setResearchStatus] = useState<IntelligenceStatus | null>(null);
  const [showCreateProposal, setShowCreateProposal] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const [saving, setSaving] = useState(false);
  const [form, setForm] = useState<UpdatePersonInput>({});
  const [showAddCompany, setShowAddCompany] = useState(false);
  const [addCompanyId, setAddCompanyId] = useState('');
  const [addCompanyRole, setAddCompanyRole] = useState('contact');
  const [addCompanyTitle, setAddCompanyTitle] = useState('');
  const [showAddOrg, setShowAddOrg] = useState(false);
  const [addOrgId, setAddOrgId] = useState('');
  const [addOrgContext, setAddOrgContext] = useState('contact');
  const [affiliationSaving, setAffiliationSaving] = useState(false);

  const { data: person, isLoading } = useQuery<PersonWithSocials>({
    queryKey: ['persons', personId],
    queryFn: () => personsApi.get(personId!),
    enabled: !!personId,
  });

  const { data: invoices = [] } = useQuery<InvoiceRecord[]>({
    queryKey: ['persons', personId, 'invoices'],
    queryFn: () => personsApi.listInvoices(personId!),
    enabled: !!personId,
  });

  const { data: allCompanies = [] } = useQuery<CompanyRecord[]>({
    queryKey: ['companies'],
    queryFn: () => companiesApi.list({ limit: 200 }),
    enabled: showAddCompany,
  });

  const { data: allOrgs = [] } = useQuery<OrganizationData[]>({
    queryKey: ['organizations'],
    queryFn: () => organizationsApi.getAll(),
    enabled: showAddOrg,
  });

  const handleResearch = async () => {
    if (!personId) return;
    setResearchLoading(true);
    setResearchStatus(null);
    try {
      await intelligenceApi.triggerResearch(personId);
      const pollInterval = setInterval(async () => {
        try {
          const status = await intelligenceApi.getStatus(personId);
          setResearchStatus(status);
          if (status.status === 'done' || status.status === 'failed') {
            clearInterval(pollInterval);
            setResearchLoading(false);
            queryClient.invalidateQueries({ queryKey: ['persons', personId] });
          }
        } catch {
          clearInterval(pollInterval);
          setResearchLoading(false);
        }
      }, 3000);
      setTimeout(() => {
        clearInterval(pollInterval);
        setResearchLoading(false);
      }, 90000);
    } catch {
      setResearchLoading(false);
    }
  };

  const startEditing = () => {
    if (!person) return;
    setForm({
      full_name: person.full_name,
      email: person.email ?? '',
      phone: person.phone ?? '',
      job_title: person.job_title ?? '',
      company_name: person.company_name ?? '',
      website: person.website ?? '',
      notes: person.notes ?? '',
      person_type: person.person_type,
      financial_role: person.financial_role,
      lifecycle_stage: person.lifecycle_stage,
      lead_score: person.lead_score,
      onboarding_channel: person.onboarding_channel ?? '',
      preferred_contact: person.preferred_contact ?? '',
    });
    setIsEditing(true);
  };

  const cancelEditing = () => {
    setIsEditing(false);
    setForm({});
  };

  const handleSave = async () => {
    if (!personId) return;
    setSaving(true);
    try {
      // Strip empty strings → undefined so backend ignores them
      const payload: UpdatePersonInput = {};
      for (const [k, v] of Object.entries(form)) {
        if (v !== '' && v !== undefined) {
          (payload as any)[k] = v;
        }
      }
      await personsApi.update(personId, payload);
      await queryClient.invalidateQueries({ queryKey: ['persons', personId] });
      queryClient.invalidateQueries({ queryKey: ['org-persons'] });
      setIsEditing(false);
      setForm({});
      toast.success('Contact updated');
    } catch {
      toast.error('Failed to save changes');
    } finally {
      setSaving(false);
    }
  };

  const set = (field: keyof UpdatePersonInput, value: any) =>
    setForm(prev => ({ ...prev, [field]: value }));

  const handleAddCompany = async () => {
    if (!personId || !addCompanyId) return;
    setAffiliationSaving(true);
    try {
      await personsApi.addCompany(personId, {
        company_id: addCompanyId,
        role: addCompanyRole,
        title: addCompanyTitle || undefined,
      });
      await queryClient.invalidateQueries({ queryKey: ['persons', personId] });
      setShowAddCompany(false);
      setAddCompanyId('');
      setAddCompanyTitle('');
      setAddCompanyRole('contact');
      toast.success('Company linked');
    } catch {
      toast.error('Failed to link company');
    } finally {
      setAffiliationSaving(false);
    }
  };

  const handleRemoveCompany = async (company_id: string) => {
    if (!personId) return;
    try {
      await personsApi.removeCompany(personId, company_id);
      await queryClient.invalidateQueries({ queryKey: ['persons', personId] });
      toast.success('Company removed');
    } catch {
      toast.error('Failed to remove company');
    }
  };

  const handleTogglePrimary = async (role: PersonCompanyRole) => {
    if (!personId) return;
    try {
      await personsApi.updateCompanyRole(personId, role.company_id, {
        is_primary: role.is_primary === 0,
      });
      await queryClient.invalidateQueries({ queryKey: ['persons', personId] });
    } catch {
      toast.error('Failed to update primary');
    }
  };

  const handleAddOrg = async () => {
    if (!personId || !addOrgId) return;
    setAffiliationSaving(true);
    try {
      await personsApi.addOrg(personId, {
        organization_id: addOrgId,
        context: addOrgContext,
      });
      await queryClient.invalidateQueries({ queryKey: ['persons', personId] });
      setShowAddOrg(false);
      setAddOrgId('');
      setAddOrgContext('contact');
      toast.success('Organization linked');
    } catch {
      toast.error('Failed to link organization');
    } finally {
      setAffiliationSaving(false);
    }
  };

  const handleRemoveOrg = async (org_id: string) => {
    if (!personId) return;
    try {
      await personsApi.removeOrg(personId, org_id);
      await queryClient.invalidateQueries({ queryKey: ['persons', personId] });
      toast.success('Organization removed');
    } catch {
      toast.error('Failed to remove organization');
    }
  };

  if (!personId) {
    return <div className="p-6 text-muted-foreground">Person not found</div>;
  }

  if (isLoading || !person) {
    return (
      <div className="p-6 space-y-6 max-w-4xl mx-auto">
        <Skeleton className="h-8 w-48" />
        <div className="grid gap-6 md:grid-cols-3">
          <div className="md:col-span-2 space-y-4">
            <Skeleton className="h-48" />
            <Skeleton className="h-64" />
          </div>
          <Skeleton className="h-80" />
        </div>
      </div>
    );
  }

  const typeInfo = PERSON_TYPE_INFO[isEditing ? (form.person_type ?? person.person_type) : person.person_type] ?? PERSON_TYPE_INFO.contact;
  const roleInfo = FINANCIAL_ROLE_INFO[isEditing ? (form.financial_role ?? person.financial_role) : person.financial_role] ?? FINANCIAL_ROLE_INFO.neutral;

  const displayName = isEditing ? (form.full_name ?? person.full_name) : person.full_name;
  const initials = displayName
    .split(' ')
    .map((n) => n[0])
    .slice(0, 2)
    .join('')
    .toUpperCase();

  const totalAr = invoices.filter(i => i.invoice_type === 'ar' && i.status === 'paid')
    .reduce((s, i) => s + i.amount_usd, 0);
  const totalAp = invoices.filter(i => i.invoice_type === 'ap' && i.status === 'paid')
    .reduce((s, i) => s + i.amount_usd, 0);

  return (
    <div className="p-6 space-y-6 max-w-4xl mx-auto">
      {/* Header */}
      <div className="flex items-center gap-4">
        <Button variant="ghost" size="icon" onClick={() => navigate(-1)}>
          <ArrowLeft className="h-4 w-4" />
        </Button>

        <div className="w-14 h-14 rounded-full bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center text-white text-xl font-semibold shrink-0">
          {person.avatar_url ? (
            <img src={person.avatar_url} alt={displayName} className="w-full h-full rounded-full object-cover" />
          ) : (
            initials
          )}
        </div>

        <div className="flex-1 min-w-0">
          {isEditing ? (
            <Input
              className="text-xl font-semibold h-9 mb-1"
              value={form.full_name ?? ''}
              onChange={e => set('full_name', e.target.value)}
              placeholder="Full name"
            />
          ) : (
            <h1 className="text-2xl font-semibold truncate">{person.full_name}</h1>
          )}
          <div className="flex items-center gap-2 flex-wrap mt-1">
            <Badge className={`${typeInfo.color} border-0`}>{typeInfo.label}</Badge>
            <span className={`text-xs px-2 py-0.5 rounded-full font-medium ${roleInfo.color}`}>
              {roleInfo.label}
            </span>
            {person.lead_score > 0 && (
              <Badge className="bg-yellow-100 text-yellow-700 border-0">
                <Star className="h-3 w-3 mr-1" />
                Score: {person.lead_score}
              </Badge>
            )}
            {person.company_name && !isEditing && (
              <span className="text-sm text-muted-foreground flex items-center gap-1">
                <Building2 className="h-3 w-3" />
                {person.company_name}
              </span>
            )}
          </div>

          {!isEditing && person.social_profiles.length > 0 && (
            <div className="mt-2">
              <SocialRail profiles={person.social_profiles} />
            </div>
          )}
        </div>

        <div className="flex items-center gap-2 shrink-0">
          {isEditing ? (
            <>
              <Button variant="ghost" size="sm" onClick={cancelEditing} disabled={saving}>
                <X className="h-4 w-4 mr-1" />
                Cancel
              </Button>
              <Button size="sm" onClick={handleSave} disabled={saving}>
                <Save className="h-4 w-4 mr-1" />
                {saving ? 'Saving…' : 'Save'}
              </Button>
            </>
          ) : (
            <>
              <Button variant="outline" size="sm" onClick={startEditing}>
                <Pencil className="h-4 w-4 mr-1" />
                Edit
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={() => setShowCreateProposal(true)}
              >
                <Plus className="h-4 w-4 mr-1" />
                Proposal
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={handleResearch}
                disabled={researchLoading}
              >
                {researchLoading ? (
                  <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
                ) : (
                  <Sparkles className="h-4 w-4 mr-2" />
                )}
                Research
              </Button>
            </>
          )}
        </div>
      </div>

      <div className="grid gap-6 md:grid-cols-3">
        {/* Left — main content */}
        <div className="md:col-span-2 space-y-6">

          {/* Contact info */}
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Contact Information</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              {isEditing ? (
                <div className="space-y-4">
                  <div className="grid gap-4 sm:grid-cols-2">
                    <div className="space-y-1">
                      <Label>Email</Label>
                      <Input
                        type="email"
                        value={form.email ?? ''}
                        onChange={e => set('email', e.target.value)}
                        placeholder="email@example.com"
                      />
                    </div>
                    <div className="space-y-1">
                      <Label>Phone</Label>
                      <Input
                        value={form.phone ?? ''}
                        onChange={e => set('phone', e.target.value)}
                        placeholder="+1 555 000 0000"
                      />
                    </div>
                    <div className="space-y-1">
                      <Label>Job Title</Label>
                      <Input
                        value={form.job_title ?? ''}
                        onChange={e => set('job_title', e.target.value)}
                        placeholder="Founder, Manager…"
                      />
                    </div>
                    <div className="space-y-1">
                      <Label>Company</Label>
                      <Input
                        value={form.company_name ?? ''}
                        onChange={e => set('company_name', e.target.value)}
                        placeholder="Company name"
                      />
                    </div>
                    <div className="space-y-1 sm:col-span-2">
                      <Label>Website</Label>
                      <Input
                        value={form.website ?? ''}
                        onChange={e => set('website', e.target.value)}
                        placeholder="https://…"
                      />
                    </div>
                  </div>

                  <div className="grid gap-4 sm:grid-cols-2 pt-1 border-t">
                    <div className="space-y-1">
                      <Label>Contact Type</Label>
                      <Select value={form.person_type ?? person.person_type} onValueChange={v => set('person_type', v)}>
                        <SelectTrigger><SelectValue /></SelectTrigger>
                        <SelectContent>
                          {Object.entries(PERSON_TYPE_INFO).map(([v, { label }]) => (
                            <SelectItem key={v} value={v}>{label}</SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    </div>
                    <div className="space-y-1">
                      <Label>Financial Role</Label>
                      <Select value={form.financial_role ?? person.financial_role} onValueChange={v => set('financial_role', v)}>
                        <SelectTrigger><SelectValue /></SelectTrigger>
                        <SelectContent>
                          {Object.entries(FINANCIAL_ROLE_INFO).map(([v, { label }]) => (
                            <SelectItem key={v} value={v}>{label}</SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    </div>
                    <div className="space-y-1">
                      <Label>Lifecycle Stage</Label>
                      <Select value={form.lifecycle_stage ?? person.lifecycle_stage} onValueChange={v => set('lifecycle_stage', v)}>
                        <SelectTrigger><SelectValue /></SelectTrigger>
                        <SelectContent>
                          {LIFECYCLE_STAGES.map(s => (
                            <SelectItem key={s} value={s}>{s.replace(/_/g, ' ')}</SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    </div>
                    <div className="space-y-1">
                      <Label>Lead Score</Label>
                      <Input
                        type="number"
                        min={0}
                        max={100}
                        value={form.lead_score ?? person.lead_score}
                        onChange={e => set('lead_score', parseInt(e.target.value) || 0)}
                      />
                    </div>
                    <div className="space-y-1">
                      <Label>Onboarding Channel</Label>
                      <Select value={form.onboarding_channel ?? person.onboarding_channel ?? ''} onValueChange={v => set('onboarding_channel', v)}>
                        <SelectTrigger><SelectValue placeholder="Select channel" /></SelectTrigger>
                        <SelectContent>
                          {CHANNELS.map(c => (
                            <SelectItem key={c} value={c}>{c.replace(/_/g, ' ')}</SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    </div>
                    <div className="space-y-1">
                      <Label>Preferred Contact</Label>
                      <Select value={form.preferred_contact ?? person.preferred_contact ?? ''} onValueChange={v => set('preferred_contact', v)}>
                        <SelectTrigger><SelectValue placeholder="Select channel" /></SelectTrigger>
                        <SelectContent>
                          {CHANNELS.map(c => (
                            <SelectItem key={c} value={c}>{c.replace(/_/g, ' ')}</SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    </div>
                  </div>

                  <div className="space-y-1 pt-1 border-t">
                    <Label>Notes</Label>
                    <Textarea
                      rows={3}
                      value={form.notes ?? ''}
                      onChange={e => set('notes', e.target.value)}
                      placeholder="Internal notes…"
                    />
                  </div>
                </div>
              ) : (
                <>
                  <div className="grid gap-3 sm:grid-cols-2">
                    {person.email && (
                      <div className="flex items-center gap-2 text-sm">
                        <Mail className="h-4 w-4 text-muted-foreground" />
                        <a href={`mailto:${person.email}`} className="text-blue-600 hover:underline truncate">
                          {person.email}
                        </a>
                      </div>
                    )}
                    {person.phone && (
                      <div className="flex items-center gap-2 text-sm">
                        <Phone className="h-4 w-4 text-muted-foreground" />
                        <a href={`tel:${person.phone}`} className="hover:underline">{person.phone}</a>
                      </div>
                    )}
                    {person.job_title && (
                      <div className="flex items-center gap-2 text-sm">
                        <Building2 className="h-4 w-4 text-muted-foreground" />
                        <span>{person.job_title}{person.company_name ? ` at ${person.company_name}` : ''}</span>
                      </div>
                    )}
                    {person.website && (
                      <div className="flex items-center gap-2 text-sm">
                        <Globe className="h-4 w-4 text-muted-foreground" />
                        <a href={person.website} target="_blank" rel="noopener noreferrer" className="text-blue-600 hover:underline truncate flex items-center gap-1">
                          {person.website}
                          <ExternalLink className="h-3 w-3 inline" />
                        </a>
                      </div>
                    )}
                  </div>

                  {person.notes && (
                    <p className="text-sm text-muted-foreground pt-2 border-t">{person.notes}</p>
                  )}

                  {(person.onboarding_channel || person.preferred_contact) && (
                    <div className="flex flex-wrap gap-3 pt-2 border-t text-xs text-muted-foreground">
                      {person.onboarding_channel && (
                        <span className="flex items-center gap-1">
                          <MessageSquare className="h-3 w-3" />
                          Onboarded via <span className="capitalize font-medium text-foreground">{person.onboarding_channel.replace(/_/g, ' ')}</span>
                        </span>
                      )}
                      {person.preferred_contact && (
                        <span className="flex items-center gap-1">
                          Preferred: <span className="capitalize font-medium text-foreground">{person.preferred_contact.replace(/_/g, ' ')}</span>
                        </span>
                      )}
                    </div>
                  )}

                  <div className="text-xs text-muted-foreground pt-1 border-t flex gap-4">
                    <span>
                      Lifecycle: <span className="capitalize">{person.lifecycle_stage.replace(/_/g, ' ')}</span>
                    </span>
                    <span>
                      Added {formatDistanceToNow(new Date(person.created_at), { addSuffix: true })}
                    </span>
                    {person.intelligence_last_run_at && (
                      <span>
                        Researched {formatDistanceToNow(new Date(person.intelligence_last_run_at), { addSuffix: true })}
                      </span>
                    )}
                  </div>
                </>
              )}
            </CardContent>
          </Card>

          {/* Intelligence */}
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between">
                <CardTitle className="text-base flex items-center gap-2">
                  <Sparkles className="h-4 w-4 text-purple-500" />
                  Contact Intelligence
                </CardTitle>
                {person.intelligence_confidence > 0 && (
                  <span className="text-xs text-muted-foreground">
                    {Math.round(person.intelligence_confidence * 100)}% confidence
                  </span>
                )}
              </div>
            </CardHeader>
            <CardContent>
              {researchLoading && researchStatus && (
                <div className="mb-3 px-3 py-2 rounded-md bg-purple-50 text-purple-700 text-xs flex items-center gap-2">
                  <RefreshCw className="h-3.5 w-3.5 animate-spin" />
                  <span>
                    {researchStatus.status === 'queued' && 'Research queued — Scout will start shortly…'}
                    {researchStatus.status === 'running' && 'Scout is researching — gathering online presence data…'}
                  </span>
                </div>
              )}
              {person.intelligence_summary ? (
                <div className="space-y-3">
                  <p className="text-sm leading-relaxed">{person.intelligence_summary}</p>
                  {person.intelligence_agent && (
                    <p className="text-xs text-muted-foreground">Via {person.intelligence_agent} agent</p>
                  )}
                  <Button variant="outline" size="sm" onClick={handleResearch} disabled={researchLoading}>
                    <RefreshCw className={`h-3.5 w-3.5 mr-1.5 ${researchLoading ? 'animate-spin' : ''}`} />
                    Refresh Research
                  </Button>
                </div>
              ) : (
                <div className="flex flex-col items-center py-8 text-center text-muted-foreground">
                  <Sparkles className="h-8 w-8 mb-3 opacity-30" />
                  <p className="text-sm font-medium">No intelligence gathered yet</p>
                  <p className="text-xs mt-1 max-w-xs">
                    Nora delegates to Scout (social intelligence) or Astra (market research) to gather this contact's online presence, company background, and professional profile.
                  </p>
                  <Button variant="outline" size="sm" className="mt-4" onClick={handleResearch} disabled={researchLoading}>
                    {researchLoading ? (
                      <RefreshCw className="h-3.5 w-3.5 mr-1.5 animate-spin" />
                    ) : (
                      <Sparkles className="h-3.5 w-3.5 mr-1.5" />
                    )}
                    {researchLoading ? 'Researching…' : 'Research via Scout/Astra'}
                  </Button>
                </div>
              )}
            </CardContent>
          </Card>

          {/* Invoices */}
          {invoices.length > 0 && (
            <Card>
              <CardHeader>
                <div className="flex items-center justify-between">
                  <CardTitle className="text-base flex items-center gap-2">
                    <DollarSign className="h-4 w-4" />
                    Financials
                  </CardTitle>
                  <div className="flex gap-3 text-xs">
                    {totalAr > 0 && (
                      <span className="text-green-600 font-medium">AR: +${totalAr.toFixed(2)}</span>
                    )}
                    {totalAp > 0 && (
                      <span className="text-amber-600 font-medium">AP: -${totalAp.toFixed(2)}</span>
                    )}
                  </div>
                </div>
              </CardHeader>
              <CardContent>
                {invoices.map((inv) => (
                  <InvoiceRow key={inv.id} invoice={inv} />
                ))}
              </CardContent>
            </Card>
          )}
        </div>

        {/* Right sidebar */}
        <div className="space-y-4">
          {/* Financial role */}
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Role</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3 text-sm">
              <div className={`p-3 rounded-lg ${roleInfo.color}`}>
                <p className="font-medium">{roleInfo.label}</p>
                <p className="text-xs opacity-80 mt-0.5">{roleInfo.description}</p>
              </div>
              {person.client_profile && (
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Profile</span>
                  <span className="capitalize font-medium">{person.client_profile}</span>
                </div>
              )}
            </CardContent>
          </Card>

          {/* Affiliations */}
          {true && (
            <Card>
              <CardHeader>
                <CardTitle className="text-base flex items-center gap-2">
                  <Building2 className="h-4 w-4" />
                  Affiliations
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-3 text-sm">
                {/* Companies */}
                <div>
                  <p className="text-xs text-muted-foreground font-medium uppercase tracking-wide mb-1.5">Companies</p>
                  {person.company_roles.map((r) => (
                    <div key={r.company_id} className="flex items-center justify-between py-1 group">
                      <div className="flex items-center gap-1.5 min-w-0">
                        <button
                          className={`shrink-0 ${r.is_primary ? 'text-yellow-500' : 'text-gray-300 hover:text-yellow-400'}`}
                          onClick={() => handleTogglePrimary(r)}
                          title={r.is_primary ? 'Primary (click to unset)' : 'Set as primary'}
                        >
                          <Star className="h-3 w-3 fill-current" />
                        </button>
                        <Link
                          to={`/companies/${r.company_id}`}
                          className="truncate font-medium hover:underline hover:text-primary"
                        >
                          {r.company_name ?? r.company_id}
                        </Link>
                        {r.title && (
                          <span className="text-xs text-muted-foreground truncate">{r.title}</span>
                        )}
                        {!r.title && r.role !== 'contact' && (
                          <span className="text-xs text-muted-foreground capitalize">{r.role}</span>
                        )}
                      </div>
                      <button
                        className="opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-destructive transition-opacity"
                        onClick={() => handleRemoveCompany(r.company_id)}
                      >
                        <X className="h-3.5 w-3.5" />
                      </button>
                    </div>
                  ))}
                  {showAddCompany ? (
                    <div className="mt-2 space-y-2 p-2 rounded-md border bg-muted/40">
                      <Select value={addCompanyId} onValueChange={setAddCompanyId}>
                        <SelectTrigger className="h-7 text-xs">
                          <SelectValue placeholder="Select company…" />
                        </SelectTrigger>
                        <SelectContent>
                          {allCompanies.map(c => (
                            <SelectItem key={c.id} value={c.id}>{c.name}</SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                      <div className="flex gap-1.5">
                        <Select value={addCompanyRole} onValueChange={setAddCompanyRole}>
                          <SelectTrigger className="h-7 text-xs flex-1">
                            <SelectValue />
                          </SelectTrigger>
                          <SelectContent>
                            {['contact','employee','founder','advisor','consultant','board','investor'].map(r => (
                              <SelectItem key={r} value={r} className="capitalize">{r}</SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                        <Input
                          className="h-7 text-xs flex-1"
                          placeholder="Title (opt)"
                          value={addCompanyTitle}
                          onChange={e => setAddCompanyTitle(e.target.value)}
                        />
                      </div>
                      <div className="flex gap-1.5">
                        <Button size="sm" className="h-7 text-xs flex-1" onClick={handleAddCompany} disabled={!addCompanyId || affiliationSaving}>
                          {affiliationSaving ? 'Linking…' : 'Link'}
                        </Button>
                        <Button size="sm" variant="ghost" className="h-7 text-xs" onClick={() => setShowAddCompany(false)}>
                          Cancel
                        </Button>
                      </div>
                    </div>
                  ) : (
                    <button
                      className="text-xs text-muted-foreground hover:text-foreground flex items-center gap-1 mt-1"
                      onClick={() => setShowAddCompany(true)}
                    >
                      <Plus className="h-3 w-3" />
                      Link Company
                    </button>
                  )}
                </div>

                {/* Orgs */}
                <div className="border-t pt-2">
                  <p className="text-xs text-muted-foreground font-medium uppercase tracking-wide mb-1.5">Organizations</p>
                  {person.org_contacts.map((o) => (
                    <div key={o.organization_id} className="flex items-center justify-between py-1 group">
                      <div className="flex items-center gap-1.5 min-w-0">
                        <span className="truncate font-medium">{o.org_name ?? o.organization_id}</span>
                        <Badge className="text-[10px] px-1 py-0 border-0 bg-gray-100 text-gray-600 capitalize">
                          {o.context}
                        </Badge>
                      </div>
                      <button
                        className="opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-destructive transition-opacity"
                        onClick={() => handleRemoveOrg(o.organization_id)}
                      >
                        <X className="h-3.5 w-3.5" />
                      </button>
                    </div>
                  ))}
                  {showAddOrg ? (
                    <div className="mt-2 space-y-2 p-2 rounded-md border bg-muted/40">
                      <Select value={addOrgId} onValueChange={setAddOrgId}>
                        <SelectTrigger className="h-7 text-xs">
                          <SelectValue placeholder="Select organization…" />
                        </SelectTrigger>
                        <SelectContent>
                          {allOrgs.map(o => (
                            <SelectItem key={o.id} value={o.id}>{o.name}</SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                      <Select value={addOrgContext} onValueChange={setAddOrgContext}>
                        <SelectTrigger className="h-7 text-xs">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          {['contact','client','vendor','partner','prospect'].map(c => (
                            <SelectItem key={c} value={c} className="capitalize">{c}</SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                      <div className="flex gap-1.5">
                        <Button size="sm" className="h-7 text-xs flex-1" onClick={handleAddOrg} disabled={!addOrgId || affiliationSaving}>
                          {affiliationSaving ? 'Linking…' : 'Link'}
                        </Button>
                        <Button size="sm" variant="ghost" className="h-7 text-xs" onClick={() => setShowAddOrg(false)}>
                          Cancel
                        </Button>
                      </div>
                    </div>
                  ) : (
                    <button
                      className="text-xs text-muted-foreground hover:text-foreground flex items-center gap-1 mt-1"
                      onClick={() => setShowAddOrg(true)}
                    >
                      <Plus className="h-3 w-3" />
                      Link Org
                    </button>
                  )}
                </div>
              </CardContent>
            </Card>
          )}

          {/* Business stage (for startup clients) */}
          {person.business_stage && (
            <Card>
              <CardHeader>
                <CardTitle className="text-base flex items-center gap-2">
                  <TrendingUp className="h-4 w-4" />
                  Business Stage
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-3">
                <p className={`text-sm font-medium ${BUSINESS_STAGE_INFO[person.business_stage]?.color ?? ''}`}>
                  {BUSINESS_STAGE_INFO[person.business_stage]?.label ?? person.business_stage}
                </p>
                <BusinessStageTracker stage={person.business_stage} />
                <div className="flex justify-between text-xs text-muted-foreground">
                  <span>Idea</span>
                  <span>Customers</span>
                </div>
              </CardContent>
            </Card>
          )}

          {/* Social profiles */}
          {person.social_profiles.length > 0 && (
            <Card>
              <CardHeader>
                <CardTitle className="text-base flex items-center gap-2">
                  <Users className="h-4 w-4" />
                  Social Profiles
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-2">
                {person.social_profiles.map((p) => {
                  const Icon = PLATFORM_ICONS[p.platform] ?? Globe;
                  return (
                    <div key={p.id} className="flex items-center justify-between text-sm">
                      <div className="flex items-center gap-2">
                        <Icon className="h-4 w-4 text-muted-foreground" />
                        <span className="capitalize">{p.platform}</span>
                        {p.handle && <span className="text-muted-foreground text-xs">@{p.handle}</span>}
                      </div>
                      {p.follower_count != null && (
                        <span className="text-xs text-muted-foreground">
                          {p.follower_count.toLocaleString()} followers
                        </span>
                      )}
                    </div>
                  );
                })}
              </CardContent>
            </Card>
          )}

          {/* Bridge links */}
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Linked Records</CardTitle>
            </CardHeader>
            <CardContent className="space-y-2 text-sm">
              {person.user_id && (
                <div className="flex items-center justify-between">
                  <span className="text-muted-foreground">Platform User</span>
                  <Badge variant="outline" className="text-xs">Linked</Badge>
                </div>
              )}
              {person.crm_contact_id && (
                <div className="flex items-center justify-between">
                  <span className="text-muted-foreground">CRM Contact</span>
                  <Badge variant="outline" className="text-xs">Linked</Badge>
                </div>
              )}
              {!person.user_id && !person.crm_contact_id && (
                <p className="text-xs text-muted-foreground">No linked platform records yet.</p>
              )}
            </CardContent>
          </Card>
        </div>
      </div>

      <Suspense fallback={null}>
        <ProposalCreateModal
          open={showCreateProposal}
          onClose={() => setShowCreateProposal(false)}
          defaultLeadId={personId}
        />
      </Suspense>
    </div>
  );
}
