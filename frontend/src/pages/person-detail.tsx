import { useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
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
} from 'lucide-react';
import { personsApi, type PersonWithSocials, type PersonSocialProfile, type InvoiceRecord } from '@/lib/api';
import { formatDistanceToNow } from 'date-fns';

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

  const updatePerson = useMutation({
    mutationFn: (data: Parameters<typeof personsApi.update>[1]) =>
      personsApi.update(personId!, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['persons', personId] });
    },
  });

  const handleResearch = async () => {
    setResearchLoading(true);
    try {
      // Trigger Nora intelligence research on this person
      // For now: clear flag + show placeholder — full Nora tool integration in next sprint
      await updatePerson.mutateAsync({
        intelligence_summary: `Research requested on ${new Date().toLocaleDateString()}. Nora will gather online presence data.`,
      });
    } finally {
      setResearchLoading(false);
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

  const typeInfo = PERSON_TYPE_INFO[person.person_type] ?? PERSON_TYPE_INFO.contact;
  const roleInfo = FINANCIAL_ROLE_INFO[person.financial_role] ?? FINANCIAL_ROLE_INFO.neutral;

  const initials = person.full_name
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
            <img src={person.avatar_url} alt={person.full_name} className="w-full h-full rounded-full object-cover" />
          ) : (
            initials
          )}
        </div>

        <div className="flex-1 min-w-0">
          <h1 className="text-2xl font-semibold truncate">{person.full_name}</h1>
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
            {person.company_name && (
              <span className="text-sm text-muted-foreground flex items-center gap-1">
                <Building2 className="h-3 w-3" />
                {person.company_name}
              </span>
            )}
          </div>

          {/* Social rail */}
          {person.social_profiles.length > 0 && (
            <div className="mt-2">
              <SocialRail profiles={person.social_profiles} />
            </div>
          )}
        </div>

        <Button
          variant="outline"
          size="sm"
          onClick={handleResearch}
          disabled={researchLoading}
          className="shrink-0"
        >
          {researchLoading ? (
            <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
          ) : (
            <Sparkles className="h-4 w-4 mr-2" />
          )}
          Research
        </Button>
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
              {person.intelligence_summary ? (
                <div className="space-y-3">
                  <p className="text-sm leading-relaxed">{person.intelligence_summary}</p>
                  <Button variant="outline" size="sm" onClick={handleResearch} disabled={researchLoading}>
                    <RefreshCw className="h-3.5 w-3.5 mr-1.5" />
                    Refresh Research
                  </Button>
                </div>
              ) : (
                <div className="flex flex-col items-center py-8 text-center text-muted-foreground">
                  <Sparkles className="h-8 w-8 mb-3 opacity-30" />
                  <p className="text-sm font-medium">No intelligence gathered yet</p>
                  <p className="text-xs mt-1 max-w-xs">
                    Click Research to have Nora gather this person's online presence, social media activity, and professional background.
                  </p>
                  <Button variant="outline" size="sm" className="mt-4" onClick={handleResearch} disabled={researchLoading}>
                    {researchLoading ? (
                      <RefreshCw className="h-3.5 w-3.5 mr-1.5 animate-spin" />
                    ) : (
                      <Sparkles className="h-3.5 w-3.5 mr-1.5" />
                    )}
                    Run One-Shot Research
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
    </div>
  );
}
