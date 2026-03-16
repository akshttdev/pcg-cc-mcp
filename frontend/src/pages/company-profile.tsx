import { useEffect, useRef, useState, lazy, Suspense } from 'react';
const ProposalCreateModal = lazy(() =>
  import('@/components/dialogs/ProposalCreateModal').then((m) => ({ default: m.ProposalCreateModal }))
);
import { useParams, useNavigate, useSearchParams, Link } from 'react-router-dom';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import {
  ArrowLeft,
  Building2,
  Globe,
  MapPin,
  Briefcase,
  ExternalLink,
  Sparkles,
  RefreshCw,
  Users,
  FileText,
  Download,
  Plus,
  Mail,
  Phone,
  X,
  Save,
  Star,
  Instagram,
  Linkedin,
  Twitter,
  Facebook,
  MessageCircle,
  Clock,
  Calendar,
  Tag,
  CheckCircle2,
  ChevronRight,
  MessageSquare,
  Loader2,
  Layers,
} from 'lucide-react';
const LayersIcon = Layers;
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { Label } from '@/components/ui/label';
import { Skeleton } from '@/components/ui/skeleton';
import { EmptyState } from '@/components/ui/empty-state';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { toast } from 'sonner';
import { cn } from '@/lib/utils';
import { entityKeys, businessKeys } from '@/lib/query-keys';
import {
  companiesApi,
  type CompanyRecord,
  type ProposalRecord,
  type PersonRecord,
  type CompanyContactMethod,
} from '@/lib/api';

// ── Tab types ─────────────────────────────────────────────────────────────────

type Tab = 'overview' | 'proposals' | 'contacts' | 'intelligence' | 'edit';

const TABS: { key: Tab; label: string }[] = [
  { key: 'overview',     label: 'Overview' },
  { key: 'proposals',    label: 'Proposals' },
  { key: 'contacts',     label: 'Contacts' },
  { key: 'intelligence', label: 'Intelligence' },
  { key: 'edit',         label: 'Edit' },
];

const EMPLOYEE_OPTIONS = ['1-10', '11-50', '51-200', '201-500', '500+'];

// ── Helpers ───────────────────────────────────────────────────────────────────

function StarRating({ rating, count }: { rating?: number | null; count?: number | null }) {
  if (!rating) return null;
  const full = Math.floor(rating);
  const half = rating - full >= 0.5;
  return (
    <div className="flex items-center gap-1.5">
      <div className="flex items-center gap-0.5">
        {[1,2,3,4,5].map((i) => (
          <Star
            key={i}
            className={cn(
              'h-4 w-4',
              i <= full ? 'fill-amber-400 text-amber-400' :
              i === full + 1 && half ? 'fill-amber-200 text-amber-400' :
              'text-gray-200 fill-gray-200'
            )}
          />
        ))}
      </div>
      <span className="text-sm font-semibold">{rating.toFixed(1)}</span>
      {count != null && (
        <span className="text-xs text-muted-foreground">({count.toLocaleString()} reviews)</span>
      )}
    </div>
  );
}

function IntelBadge({ status }: { status: string }) {
  const cfg: Record<string, { label: string; cls: string }> = {
    idle:    { label: 'No Intel',  cls: 'bg-gray-100 text-gray-500' },
    queued:  { label: 'Queued',    cls: 'bg-yellow-100 text-yellow-700' },
    running: { label: 'Running…',  cls: 'bg-blue-100 text-blue-700 animate-pulse' },
    done:    { label: 'Intel ✓',   cls: 'bg-green-100 text-green-700' },
    failed:  { label: 'Failed',    cls: 'bg-red-100 text-red-700' },
  };
  const { label, cls } = cfg[status] ?? cfg['idle'];
  return <Badge className={`text-xs border-0 px-2 py-0.5 ${cls}`}>{label}</Badge>;
}

function ProposalStatusBadge({ status }: { status: string }) {
  const cls =
    status === 'contract_signed' ? 'bg-green-100 text-green-700' :
    status === 'declined'        ? 'bg-red-100 text-red-700' :
    status === 'verbal'          ? 'bg-orange-100 text-orange-700' :
    status === 'sent'            ? 'bg-purple-100 text-purple-700' :
    'bg-gray-100 text-gray-600';
  return (
    <Badge className={`text-xs border-0 px-1.5 py-0 ${cls}`}>
      {status.replace(/_/g, ' ')}
    </Badge>
  );
}

// ── GMB-style business hours parser ──────────────────────────────────────────

const DAYS = ['mon','tue','wed','thu','fri','sat','sun'] as const;
const DAY_LABELS: Record<string, string> = {
  mon: 'Monday', tue: 'Tuesday', wed: 'Wednesday', thu: 'Thursday',
  fri: 'Friday', sat: 'Saturday', sun: 'Sunday',
};

function BusinessHoursCard({ hours }: { hours?: string | null }) {
  if (!hours) return null;
  let parsed: Record<string, string> = {};
  try { parsed = JSON.parse(hours); } catch { return null; }

  const today = ['sun','mon','tue','wed','thu','fri','sat'][new Date().getDay()];

  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="text-sm font-medium flex items-center gap-1.5">
          <Clock className="h-4 w-4 text-muted-foreground" />
          Business Hours
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-1">
          {DAYS.map((d) => {
            const val = parsed[d];
            const isToday = d === today;
            return (
              <div
                key={d}
                className={cn(
                  'flex justify-between text-sm py-0.5',
                  isToday && 'font-semibold text-foreground',
                  !isToday && 'text-muted-foreground'
                )}
              >
                <span className="w-24">{DAY_LABELS[d]}</span>
                <span>{val ?? 'Closed'}</span>
              </div>
            );
          })}
        </div>
      </CardContent>
    </Card>
  );
}

// ── Contact action rail ───────────────────────────────────────────────────────

function ContactRail({ company }: { company: CompanyRecord }) {
  const actions = [];

  if (company.phone) {
    actions.push(
      <a key="phone" href={`tel:${company.phone}`}
        className="flex flex-col items-center gap-1 p-3 rounded-xl bg-muted hover:bg-muted/70 transition-colors text-xs font-medium text-center min-w-[64px]"
      >
        <Phone className="h-5 w-5 text-green-600" />
        <span>Call</span>
      </a>
    );
  }
  if (company.whatsapp || company.phone) {
    const wa = company.whatsapp ?? company.phone ?? '';
    const clean = wa.replace(/\D/g, '');
    actions.push(
      <a key="wa" href={`https://wa.me/${clean}`} target="_blank" rel="noopener noreferrer"
        className="flex flex-col items-center gap-1 p-3 rounded-xl bg-muted hover:bg-muted/70 transition-colors text-xs font-medium text-center min-w-[64px]"
      >
        <MessageCircle className="h-5 w-5 text-green-500" />
        <span>WhatsApp</span>
      </a>
    );
  }
  if (company.email) {
    actions.push(
      <a key="email" href={`mailto:${company.email}`}
        className="flex flex-col items-center gap-1 p-3 rounded-xl bg-muted hover:bg-muted/70 transition-colors text-xs font-medium text-center min-w-[64px]"
      >
        <Mail className="h-5 w-5 text-blue-600" />
        <span>Email</span>
      </a>
    );
  }
  if (company.website) {
    actions.push(
      <a key="web" href={company.website} target="_blank" rel="noopener noreferrer"
        className="flex flex-col items-center gap-1 p-3 rounded-xl bg-muted hover:bg-muted/70 transition-colors text-xs font-medium text-center min-w-[64px]"
      >
        <Globe className="h-5 w-5 text-indigo-600" />
        <span>Website</span>
      </a>
    );
  }
  if (company.instagram_handle) {
    const handle = company.instagram_handle.replace(/^@/, '');
    actions.push(
      <a key="ig" href={`https://instagram.com/${handle}`} target="_blank" rel="noopener noreferrer"
        className="flex flex-col items-center gap-1 p-3 rounded-xl bg-muted hover:bg-muted/70 transition-colors text-xs font-medium text-center min-w-[64px]"
      >
        <Instagram className="h-5 w-5 text-pink-600" />
        <span>Instagram</span>
      </a>
    );
  }
  if (company.linkedin_url) {
    actions.push(
      <a key="li" href={company.linkedin_url} target="_blank" rel="noopener noreferrer"
        className="flex flex-col items-center gap-1 p-3 rounded-xl bg-muted hover:bg-muted/70 transition-colors text-xs font-medium text-center min-w-[64px]"
      >
        <Linkedin className="h-5 w-5 text-blue-700" />
        <span>LinkedIn</span>
      </a>
    );
  }
  if (company.twitter_handle) {
    const handle = company.twitter_handle.replace(/^@/, '');
    actions.push(
      <a key="tw" href={`https://twitter.com/${handle}`} target="_blank" rel="noopener noreferrer"
        className="flex flex-col items-center gap-1 p-3 rounded-xl bg-muted hover:bg-muted/70 transition-colors text-xs font-medium text-center min-w-[64px]"
      >
        <Twitter className="h-5 w-5 text-sky-500" />
        <span>Twitter</span>
      </a>
    );
  }
  if (company.facebook_url) {
    actions.push(
      <a key="fb" href={company.facebook_url} target="_blank" rel="noopener noreferrer"
        className="flex flex-col items-center gap-1 p-3 rounded-xl bg-muted hover:bg-muted/70 transition-colors text-xs font-medium text-center min-w-[64px]"
      >
        <Facebook className="h-5 w-5 text-blue-600" />
        <span>Facebook</span>
      </a>
    );
  }

  if (actions.length === 0) return null;

  return (
    <div className="flex gap-2 flex-wrap">
      {actions}
    </div>
  );
}

// ── Tab: Overview ─────────────────────────────────────────────────────────────

function OverviewTab({
  company,
  proposals,
  contacts,
  contactMethods,
  onNavigate,
  onRemoveMethod,
}: {
  company: CompanyRecord;
  proposals: ProposalRecord[];
  contacts: PersonRecord[];
  contactMethods: CompanyContactMethod[];
  onNavigate: (tab: Tab) => void;
  onRemoveMethod: (id: string) => Promise<void>;
}) {
  const tags: string[] = (() => { try { return JSON.parse(company.tags ?? '[]'); } catch { return []; } })();

  return (
    <div className="grid grid-cols-1 lg:grid-cols-3 gap-5">
      {/* ── Left column (2/3) ── */}
      <div className="lg:col-span-2 space-y-4">

        {/* Contact action rail */}
        <ContactRail company={company} />

        {/* Star rating + GMB */}
        {(company.gmb_rating != null || company.gmb_review_count != null) && (
          <div className="flex items-center gap-3 flex-wrap">
            <StarRating rating={company.gmb_rating} count={company.gmb_review_count} />
            {company.gmb_place_id && (
              <a
                href={`https://maps.google.com/?cid=${company.gmb_place_id}`}
                target="_blank"
                rel="noopener noreferrer"
                className="text-xs text-blue-500 hover:underline flex items-center gap-0.5"
              >
                View on Google Maps
                <ExternalLink className="h-3 w-3 ml-0.5" />
              </a>
            )}
          </div>
        )}

        {/* About */}
        {company.description && (
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium">About</CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-sm text-muted-foreground leading-relaxed whitespace-pre-line">
                {company.description}
              </p>
            </CardContent>
          </Card>
        )}

        {/* Internal notes */}
        {company.notes && (
          <Card className="border-amber-200 bg-amber-50/50">
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium text-amber-800 flex items-center gap-1.5">
                <MessageSquare className="h-4 w-4" />
                Internal Notes
              </CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-sm text-amber-900 whitespace-pre-line">{company.notes}</p>
            </CardContent>
          </Card>
        )}

        {/* Stats row */}
        <div className="grid grid-cols-3 gap-3">
          <button
            onClick={() => onNavigate('proposals')}
            className="flex flex-col items-center gap-1 p-4 rounded-xl bg-muted hover:bg-muted/70 transition-colors text-center"
          >
            <span className="text-2xl font-bold">{proposals.length}</span>
            <span className="text-xs text-muted-foreground">Proposal{proposals.length !== 1 ? 's' : ''}</span>
          </button>
          <button
            onClick={() => onNavigate('contacts')}
            className="flex flex-col items-center gap-1 p-4 rounded-xl bg-muted hover:bg-muted/70 transition-colors text-center"
          >
            <span className="text-2xl font-bold">{contacts.length}</span>
            <span className="text-xs text-muted-foreground">Contact{contacts.length !== 1 ? 's' : ''}</span>
          </button>
          <div className="flex flex-col items-center gap-1 p-4 rounded-xl bg-muted text-center">
            <span className="text-2xl font-bold">
              {proposals.filter(p => p.status === 'contract_signed').length}
            </span>
            <span className="text-xs text-muted-foreground">Won Deals</span>
          </div>
        </div>

        {/* Recent proposals */}
        {proposals.length > 0 && (
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium flex items-center justify-between">
                <span>Recent Proposals</span>
                <button onClick={() => onNavigate('proposals')} className="text-xs text-muted-foreground hover:text-foreground flex items-center gap-0.5">
                  View all <ChevronRight className="h-3 w-3" />
                </button>
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-2">
              {proposals.slice(0, 3).map((p) => (
                <div key={p.id} className="flex items-center justify-between text-sm py-1 border-b last:border-0">
                  <span className="truncate flex-1">{p.title}</span>
                  <ProposalStatusBadge status={p.status} />
                </div>
              ))}
            </CardContent>
          </Card>
        )}

        {/* Recent contacts */}
        {contacts.length > 0 && (
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium flex items-center justify-between">
                <span>Contacts</span>
                <button onClick={() => onNavigate('contacts')} className="text-xs text-muted-foreground hover:text-foreground flex items-center gap-0.5">
                  View all <ChevronRight className="h-3 w-3" />
                </button>
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-2">
              {contacts.slice(0, 4).map((c) => (
                <Link
                  key={c.id}
                  to={`/people/${c.id}`}
                  className="flex items-center gap-2 text-sm py-1 border-b last:border-0 hover:text-primary transition-colors"
                >
                  <div className="h-7 w-7 rounded-full bg-primary/10 flex items-center justify-center text-xs font-semibold text-primary shrink-0">
                    {c.full_name.slice(0,2).toUpperCase()}
                  </div>
                  <div className="min-w-0 flex-1">
                    <p className="font-medium truncate">{c.full_name}</p>
                    {c.job_title && <p className="text-xs text-muted-foreground truncate">{c.job_title}</p>}
                  </div>
                </Link>
              ))}
            </CardContent>
          </Card>
        )}
      </div>

      {/* ── Right column (1/3) ── */}
      <div className="space-y-4">

        {/* Location + contact details */}
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Details</CardTitle>
          </CardHeader>
          <CardContent className="space-y-2.5 text-sm">
            {(company.address || company.city || company.headquarters) && (
              <div className="flex gap-2">
                <MapPin className="h-4 w-4 text-muted-foreground shrink-0 mt-0.5" />
                <div>
                  {company.address && <p>{company.address}</p>}
                  {(company.city || company.country) && (
                    <p className="text-muted-foreground">{[company.city, company.country].filter(Boolean).join(', ')}</p>
                  )}
                  {!company.address && !company.city && company.headquarters && (
                    <p className="text-muted-foreground">{company.headquarters}</p>
                  )}
                </div>
              </div>
            )}
            {company.phone && (
              <div className="flex items-center gap-2">
                <Phone className="h-4 w-4 text-muted-foreground shrink-0" />
                <a href={`tel:${company.phone}`} className="hover:text-primary transition-colors">{company.phone}</a>
              </div>
            )}
            {company.email && (
              <div className="flex items-center gap-2">
                <Mail className="h-4 w-4 text-muted-foreground shrink-0" />
                <a href={`mailto:${company.email}`} className="hover:text-primary transition-colors truncate">{company.email}</a>
              </div>
            )}
            {company.website && (
              <div className="flex items-center gap-2">
                <Globe className="h-4 w-4 text-muted-foreground shrink-0" />
                <a href={company.website} target="_blank" rel="noopener noreferrer"
                  className="text-blue-500 hover:underline truncate"
                >
                  {company.website.replace(/^https?:\/\/(www\.)?/, '')}
                </a>
              </div>
            )}
            {company.industry && (
              <div className="flex items-center gap-2">
                <Briefcase className="h-4 w-4 text-muted-foreground shrink-0" />
                <span>{company.industry}</span>
              </div>
            )}
            {company.founded_year && (
              <div className="flex items-center gap-2">
                <Calendar className="h-4 w-4 text-muted-foreground shrink-0" />
                <span>Founded {company.founded_year}</span>
              </div>
            )}
            {company.employee_count && (
              <div className="flex items-center gap-2">
                <Users className="h-4 w-4 text-muted-foreground shrink-0" />
                <span>{company.employee_count} employees</span>
              </div>
            )}
            {company.organization_id && (
              <div className="flex items-center gap-2">
                <CheckCircle2 className="h-4 w-4 text-green-600 shrink-0" />
                <span className="text-green-700 font-medium">Platform Organisation</span>
              </div>
            )}
          </CardContent>
        </Card>

        {/* Tags */}
        {tags.length > 0 && (
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium flex items-center gap-1.5">
                <Tag className="h-4 w-4 text-muted-foreground" />
                Tags
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="flex flex-wrap gap-1.5">
                {tags.map((t) => (
                  <Badge key={t} variant="secondary" className="text-xs font-normal capitalize">
                    {t}
                  </Badge>
                ))}
              </div>
            </CardContent>
          </Card>
        )}

        {/* Business Hours */}
        <BusinessHoursCard hours={company.business_hours} />

        {/* Extra contact methods from junction table */}
        {contactMethods.length > 0 && (
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium">Additional Contacts</CardTitle>
            </CardHeader>
            <CardContent className="space-y-2">
              {contactMethods.map((m) => (
                <div key={m.id} className="flex items-center justify-between text-sm group">
                  <div className="flex items-center gap-2 min-w-0">
                    <span className="capitalize text-muted-foreground text-xs w-16 shrink-0">{m.method_type}</span>
                    <span className="truncate">{m.value}</span>
                    {m.label && <span className="text-xs text-muted-foreground">({m.label})</span>}
                  </div>
                  <button
                    className="opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-destructive transition-opacity shrink-0"
                    onClick={() => onRemoveMethod(m.id)}
                  >
                    <X className="h-3.5 w-3.5" />
                  </button>
                </div>
              ))}
            </CardContent>
          </Card>
        )}
      </div>
    </div>
  );
}

// ── Tab: Proposals ────────────────────────────────────────────────────────────

function ProposalsTab({ proposals, onNewProposal }: { proposals: ProposalRecord[]; onNewProposal: () => void }) {
  if (proposals.length === 0) {
    return (
      <EmptyState
        icon={FileText}
        title="No proposals yet"
        action={{ label: "Create Proposal", onClick: onNewProposal }}
        className="h-32"
      />
    );
  }
  return (
    <div className="space-y-2">
      {proposals.map((p) => (
        <Card key={p.id} className="hover:shadow-sm transition-shadow">
          <CardContent className="pt-3 pb-3">
            <div className="flex items-start justify-between gap-3">
              <div className="flex-1 min-w-0">
                <p className="font-medium text-sm">{p.title}</p>
                {p.description && (
                  <p className="text-xs text-muted-foreground mt-0.5 line-clamp-1">{p.description}</p>
                )}
                {p.quote_amount_vibe > 0 && (
                  <p className="text-xs text-muted-foreground mt-0.5">
                    ${(p.quote_amount_vibe / 100).toLocaleString()}
                  </p>
                )}
              </div>
              <ProposalStatusBadge status={p.status} />
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}

// ── Tab: Contacts ─────────────────────────────────────────────────────────────

function ContactsTab({ contacts, onResearch }: { contacts: PersonRecord[]; onResearch: (id: string) => void }) {
  const [runningDeep, setRunningDeep] = useState<Set<string>>(new Set());

  const handleDeepResearch = async (personId: string) => {
    setRunningDeep(prev => new Set([...prev, personId]));
    try {
      const { intelligenceApi } = await import('@/lib/api');
      await intelligenceApi.triggerNextPass(personId);
      toast.success('Deep research pass queued');
    } catch { toast.error('Failed to queue deep research'); }
    finally { setRunningDeep(prev => { const s = new Set(prev); s.delete(personId); return s; }); }
  };

  if (contacts.length === 0) {
    return (
      <EmptyState
        icon={Users}
        title="No contacts linked"
        description="Research this company to auto-discover contacts"
        className="h-32"
      />
    );
  }
  return (
    <div className="grid gap-2 sm:grid-cols-2">
      {contacts.map((c) => (
        <Card key={c.id} className="hover:shadow-sm transition-shadow">
          <CardContent className="pt-3 pb-3">
            <div className="flex items-center gap-3">
              <div className="h-9 w-9 rounded-full bg-primary/10 flex items-center justify-center text-sm font-semibold text-primary shrink-0">
                {c.full_name.slice(0,2).toUpperCase()}
              </div>
              <div className="flex-1 min-w-0">
                <Link to={`/people/${c.id}`} className="font-medium text-sm hover:text-primary transition-colors truncate block">
                  {c.full_name}
                </Link>
                <p className="text-xs text-muted-foreground truncate">
                  {c.job_title ?? c.person_type}
                  {c.email ? ` · ${c.email}` : ''}
                </p>
                {(c.research_pass_count ?? 0) > 0 && (
                  <p className="text-xs text-blue-500/70 mt-0.5">
                    {c.research_pass_count} research {(c.research_pass_count ?? 0) === 1 ? 'pass' : 'passes'} · {c.research_depth ?? 'shallow'}
                  </p>
                )}
              </div>
              <div className="flex items-center gap-1 shrink-0">
                {c.intelligence_status === 'done' && (
                  <Badge className="text-xs border-0 bg-green-100 text-green-700 px-1.5 py-0">Intel ✓</Badge>
                )}
                <Button size="sm" variant="ghost" className="h-7 w-7 p-0" onClick={() => onResearch(c.id)} title="Quick Research">
                  <Sparkles className="h-3.5 w-3.5" />
                </Button>
                <Button
                  size="sm" variant="ghost"
                  className="h-7 w-7 p-0 text-blue-500 hover:text-blue-400"
                  disabled={runningDeep.has(c.id)}
                  onClick={() => handleDeepResearch(c.id)}
                  title="Run next deep research pass">
                  {runningDeep.has(c.id)
                    ? <Loader2 className="h-3.5 w-3.5 animate-spin" />
                    : <LayersIcon className="h-3.5 w-3.5" />}
                </Button>
              </div>
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}

// ── Tab: Intelligence ─────────────────────────────────────────────────────────

function IntelligenceTab({
  company,
  intel,
  onRun,
  isPolling,
}: {
  company: CompanyRecord;
  intel: { status: string; summary?: string; confidence: number; agent?: string; last_run_at?: string } | null;
  onRun: () => void;
  isPolling: boolean;
}) {
  const rawData = (() => {
    try { return JSON.parse(company.intelligence_raw ?? '{}'); } catch { return {}; }
  })();

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <IntelBadge status={intel?.status ?? 'idle'} />
          {intel?.last_run_at && (
            <span className="text-xs text-muted-foreground">
              Last run: {new Date(intel.last_run_at).toLocaleDateString()}
            </span>
          )}
          {intel?.agent && (
            <span className="text-xs text-muted-foreground">by {intel.agent}</span>
          )}
        </div>
        <Button
          size="sm"
          variant="outline"
          onClick={onRun}
          disabled={isPolling || intel?.status === 'running' || intel?.status === 'queued'}
        >
          {isPolling ? (
            <RefreshCw className="h-3.5 w-3.5 mr-1 animate-spin" />
          ) : (
            <Sparkles className="h-3.5 w-3.5 mr-1" />
          )}
          Run Research
        </Button>
      </div>

      {(intel?.confidence ?? 0) > 0 && (
        <div className="space-y-1">
          <div className="flex justify-between text-xs text-muted-foreground">
            <span>Confidence</span>
            <span>{Math.round((intel?.confidence ?? 0) * 100)}%</span>
          </div>
          <div className="h-1.5 bg-muted rounded-full overflow-hidden">
            <div className="h-full bg-green-500 rounded-full" style={{ width: `${(intel?.confidence ?? 0) * 100}%` }} />
          </div>
        </div>
      )}

      {intel?.summary ? (
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium flex items-center gap-1.5">
              <Sparkles className="h-4 w-4 text-yellow-500" />
              AI Summary
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-muted-foreground leading-relaxed">{intel.summary}</p>
          </CardContent>
        </Card>
      ) : (
        <EmptyState
          icon={Sparkles}
          title="No intelligence gathered yet"
          description="Run research to populate"
          className="h-24 border rounded-lg"
        />
      )}

      {/* Raw intel fields (populated by research) */}
      {Object.keys(rawData).length > 0 && (
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Research Data</CardTitle>
          </CardHeader>
          <CardContent>
            <pre className="text-xs text-muted-foreground overflow-auto max-h-64 bg-muted rounded p-2">
              {JSON.stringify(rawData, null, 2)}
            </pre>
          </CardContent>
        </Card>
      )}
    </div>
  );
}

// ── Tab: Edit ─────────────────────────────────────────────────────────────────

function EditTab({
  company,
  onSaved,
}: {
  company: CompanyRecord;
  onSaved: () => void;
}) {
  const [form, setForm] = useState<Partial<CompanyRecord>>({});
  const [saving, setSaving] = useState(false);

  const set = (k: keyof CompanyRecord, v: unknown) => setForm(f => ({ ...f, [k]: v }));
  const val = <K extends keyof CompanyRecord>(k: K): string =>
    ((form[k] ?? company[k]) as string | null | undefined) ?? '';

  const handleSave = async () => {
    setSaving(true);
    try {
      // Strip empty strings → omit so backend treats as no-change
      const payload: Record<string, unknown> = {};
      for (const [k, v] of Object.entries(form)) {
        if (v !== '') payload[k] = v;
      }
      await companiesApi.update(company.id, payload as Partial<CompanyRecord>);
      toast.success('Company saved');
      onSaved();
    } catch {
      toast.error('Save failed');
    } finally {
      setSaving(false);
    }
  };

  const field = (label: string, key: keyof CompanyRecord, type: string = 'text', placeholder?: string) => (
    <div className="space-y-1">
      <Label className="text-xs">{label}</Label>
      <Input
        type={type}
        className="h-8 text-sm"
        placeholder={placeholder}
        value={val(key)}
        onChange={e => set(key, e.target.value)}
      />
    </div>
  );

  return (
    <div className="space-y-6 max-w-2xl">
      {/* Identity */}
      <div>
        <h3 className="text-sm font-semibold mb-3 text-muted-foreground uppercase tracking-wide">Identity</h3>
        <div className="grid grid-cols-2 gap-3">
          {field('Company Name', 'name')}
          {field('Industry', 'industry', 'text', 'e.g. Events, Hospitality')}
          {field('Logo URL', 'logo_url', 'url')}
          {field('Cover Image URL', 'cover_image_url', 'url')}
          <div className="col-span-2 space-y-1">
            <Label className="text-xs">Description / About</Label>
            <Textarea
              className="text-sm min-h-[80px]"
              value={val('description')}
              onChange={e => set('description', e.target.value)}
            />
          </div>
          <div className="space-y-1">
            <Label className="text-xs">Founded Year</Label>
            <Input
              type="number"
              className="h-8 text-sm"
              placeholder="e.g. 2015"
              value={val('founded_year')}
              onChange={e => set('founded_year', e.target.value ? Number(e.target.value) : '')}
            />
          </div>
          <div className="space-y-1">
            <Label className="text-xs">Employee Count</Label>
            <Select value={val('employee_count')} onValueChange={v => set('employee_count', v)}>
              <SelectTrigger className="h-8 text-sm"><SelectValue placeholder="Select range" /></SelectTrigger>
              <SelectContent>
                {EMPLOYEE_OPTIONS.map(o => <SelectItem key={o} value={o}>{o}</SelectItem>)}
              </SelectContent>
            </Select>
          </div>
        </div>
      </div>

      {/* Location */}
      <div>
        <h3 className="text-sm font-semibold mb-3 text-muted-foreground uppercase tracking-wide">Location</h3>
        <div className="grid grid-cols-2 gap-3">
          <div className="col-span-2">{field('Street Address', 'address', 'text', '123 Main St')}</div>
          {field('City', 'city')}
          {field('Country', 'country')}
          {field('Headquarters (short)', 'headquarters', 'text', 'Miami, FL')}
        </div>
      </div>

      {/* Contact */}
      <div>
        <h3 className="text-sm font-semibold mb-3 text-muted-foreground uppercase tracking-wide">Contact</h3>
        <div className="grid grid-cols-2 gap-3">
          {field('Phone', 'phone', 'tel', '+1 305...')}
          {field('WhatsApp', 'whatsapp', 'tel')}
          {field('Email', 'email', 'email')}
          {field('Website', 'website', 'url', 'https://')}
        </div>
      </div>

      {/* Social */}
      <div>
        <h3 className="text-sm font-semibold mb-3 text-muted-foreground uppercase tracking-wide">Social Media</h3>
        <div className="grid grid-cols-2 gap-3">
          {field('Instagram Handle', 'instagram_handle', 'text', '@handle')}
          {field('Twitter Handle', 'twitter_handle', 'text', '@handle')}
          {field('LinkedIn URL', 'linkedin_url', 'url')}
          {field('Facebook URL', 'facebook_url', 'url')}
        </div>
      </div>

      {/* GMB */}
      <div>
        <h3 className="text-sm font-semibold mb-3 text-muted-foreground uppercase tracking-wide">Google My Business</h3>
        <div className="grid grid-cols-3 gap-3">
          <div className="space-y-1">
            <Label className="text-xs">Rating (0–5)</Label>
            <Input
              type="number"
              min="0"
              max="5"
              step="0.1"
              className="h-8 text-sm"
              value={val('gmb_rating')}
              onChange={e => set('gmb_rating', e.target.value ? Number(e.target.value) : '')}
            />
          </div>
          <div className="space-y-1">
            <Label className="text-xs">Review Count</Label>
            <Input
              type="number"
              className="h-8 text-sm"
              value={val('gmb_review_count')}
              onChange={e => set('gmb_review_count', e.target.value ? Number(e.target.value) : '')}
            />
          </div>
          {field('Place ID', 'gmb_place_id', 'text')}
        </div>
      </div>

      {/* Tags */}
      <div>
        <h3 className="text-sm font-semibold mb-3 text-muted-foreground uppercase tracking-wide">Tags</h3>
        <div className="space-y-1">
          <Label className="text-xs">Comma-separated tags</Label>
          <Input
            className="h-8 text-sm"
            placeholder="events, luxury, miami, hospitality"
            value={(() => {
              try { return JSON.parse(val('tags') || '[]').join(', '); } catch { return val('tags'); }
            })()}
            onChange={e => {
              const tags = e.target.value.split(',').map(t => t.trim()).filter(Boolean);
              set('tags', JSON.stringify(tags));
            }}
          />
        </div>
      </div>

      {/* Internal Notes */}
      <div>
        <h3 className="text-sm font-semibold mb-3 text-muted-foreground uppercase tracking-wide">Internal Notes</h3>
        <Textarea
          className="text-sm min-h-[80px]"
          placeholder="Private notes visible only to your team…"
          value={val('notes')}
          onChange={e => set('notes', e.target.value)}
        />
      </div>

      <div className="flex gap-2 pb-6">
        <Button onClick={handleSave} disabled={saving}>
          {saving ? <RefreshCw className="h-4 w-4 mr-1 animate-spin" /> : <Save className="h-4 w-4 mr-1" />}
          {saving ? 'Saving…' : 'Save Changes'}
        </Button>
        <Button variant="ghost" onClick={() => setForm({})}>Reset</Button>
      </div>
    </div>
  );
}

// ── Page ──────────────────────────────────────────────────────────────────────

export function CompanyProfilePage() {
  const { companyId } = useParams<{ companyId: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [searchParams, setSearchParams] = useSearchParams();
  const activeTab = (searchParams.get('tab') as Tab) ?? 'overview';
  const [isPolling, setIsPolling] = useState(false);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const [showCreateProposal, setShowCreateProposal] = useState(false);
  const [isExporting, setIsExporting] = useState(false);

  const { data: company, isLoading, refetch: refetchCompany } = useQuery({
    queryKey: entityKeys.company(companyId!),
    queryFn: () => companiesApi.get(companyId!),
    enabled: !!companyId,
  });

  const { data: proposals = [] } = useQuery({
    queryKey: businessKeys.companyProposals(companyId!),
    queryFn: () => companiesApi.listProposals(companyId!),
    enabled: !!companyId,
  });

  const { data: contacts = [] } = useQuery({
    queryKey: entityKeys.companyContacts(companyId!),
    queryFn: () => companiesApi.listPersons(companyId!),
    enabled: !!companyId,
  });

  const { data: intel, refetch: refetchIntel } = useQuery({
    queryKey: entityKeys.companyIntel(companyId!),
    queryFn: () => companiesApi.getIntelligenceStatus(companyId!),
    enabled: !!companyId,
  });

  const { data: contactMethods = [], refetch: refetchMethods } = useQuery({
    queryKey: entityKeys.companyContactMethods(companyId!),
    queryFn: () => companiesApi.listContactMethods(companyId!),
    enabled: !!companyId,
  });

  async function handleRemoveMethod(methodId: string) {
    if (!companyId) return;
    await companiesApi.removeContactMethod(companyId, methodId);
    refetchMethods();
  }

  function setTab(tab: Tab) { setSearchParams({ tab }); }

  // Auto-poll during research
  useEffect(() => {
    if (intel?.status === 'running' || intel?.status === 'queued') {
      if (!pollRef.current) {
        setIsPolling(true);
        pollRef.current = setInterval(async () => {
          const updated = await refetchIntel();
          if (updated.data?.status !== 'running' && updated.data?.status !== 'queued') {
            clearInterval(pollRef.current!);
            pollRef.current = null;
            setIsPolling(false);
            queryClient.invalidateQueries({ queryKey: entityKeys.company(companyId!) });
          }
        }, 3000);
      }
    } else {
      if (pollRef.current) { clearInterval(pollRef.current); pollRef.current = null; setIsPolling(false); }
    }
    return () => { if (pollRef.current) clearInterval(pollRef.current); };
  }, [intel?.status, companyId, refetchIntel, queryClient]);

  async function handleRunResearch() {
    if (!companyId) return;
    try {
      await companiesApi.research(companyId);
      toast.success('Research queued');
      refetchIntel();
    } catch { toast.error('Failed to queue research'); }
  }

  async function handleExportAnalysis() {
    if (!companyId || !company) return;
    setIsExporting(true);
    try { await companiesApi.exportAnalysis(companyId, company.name); }
    catch { toast.error('Export failed'); }
    finally { setIsExporting(false); }
  }

  async function handlePersonResearch(personId: string) {
    try {
      const { intelligenceApi } = await import('@/lib/api');
      await intelligenceApi.triggerResearch(personId);
      toast.success('Contact research queued');
    } catch { toast.error('Failed to queue contact research'); }
  }

  if (isLoading) {
    return (
      <div className="p-6 space-y-4">
        <Skeleton className="h-48 w-full rounded-xl" />
        <Skeleton className="h-8 w-64" />
        <Skeleton className="h-48 w-full" />
      </div>
    );
  }

  if (!company) {
    return <div className="flex items-center justify-center h-full text-muted-foreground">Company not found.</div>;
  }

  const totalRevenue = proposals
    .filter(p => p.status === 'contract_signed')
    .reduce((sum, p) => sum + (p.quote_amount_vibe ?? 0) / 100, 0);

  return (
    <div className="flex flex-col h-full">
      {/* ── Cover + Header ── */}
      <div className="shrink-0">
        {/* Cover image */}
        <div
          className="h-32 w-full bg-gradient-to-br from-slate-200 to-slate-300 relative"
          style={company.cover_image_url ? {
            backgroundImage: `url(${company.cover_image_url})`,
            backgroundSize: 'cover',
            backgroundPosition: 'center',
          } : {}}
        >
          <div className="absolute inset-0 bg-black/20" />
          <button
            className="absolute top-3 left-3 flex items-center gap-1 px-2 py-1 rounded-md bg-black/40 hover:bg-black/60 text-white text-xs transition-colors"
            onClick={() => navigate('/companies')}
          >
            <ArrowLeft className="h-3.5 w-3.5" />
            Companies
          </button>
          {/* Action buttons top-right */}
          <div className="absolute top-3 right-3 flex gap-1.5">
            {totalRevenue > 0 && (
              <span className="px-2 py-1 rounded-md bg-green-500/90 text-white text-xs font-semibold">
                ${totalRevenue.toLocaleString()} earned
              </span>
            )}
            <Button size="sm" variant="secondary" className="h-7 text-xs" onClick={handleExportAnalysis} disabled={isExporting}>
              {isExporting ? <RefreshCw className="h-3 w-3 animate-spin" /> : <Download className="h-3 w-3" />}
            </Button>
            <Button size="sm" className="h-7 text-xs" onClick={() => setShowCreateProposal(true)}>
              <Plus className="h-3 w-3 mr-0.5" />
              Proposal
            </Button>
          </div>
        </div>

        {/* Logo + name row */}
        <div className="px-6 pb-0">
          <div className="flex items-end gap-4 -mt-8">
            {/* Logo */}
            <div className="h-16 w-16 rounded-xl border-2 border-background bg-white shadow-sm flex items-center justify-center overflow-hidden shrink-0">
              {company.logo_url ? (
                <img src={company.logo_url} alt={company.name} className="h-full w-full object-contain p-1" />
              ) : (
                <Building2 className="h-8 w-8 text-muted-foreground" />
              )}
            </div>
            <div className="pb-1 flex-1 min-w-0">
              <div className="flex items-center gap-2 flex-wrap">
                <h1 className="text-xl font-bold">{company.name}</h1>
                <IntelBadge status={company.intelligence_status} />
                {company.organization_id && (
                  <button
                    onClick={() => navigate(`/organizations/${company.organization_id}`)}
                    className="text-xs px-2 py-0.5 rounded-full bg-indigo-100 text-indigo-700 hover:bg-indigo-200 transition-colors font-medium"
                  >
                    View Org →
                  </button>
                )}
              </div>
              <div className="flex items-center gap-2 text-sm text-muted-foreground flex-wrap mt-0.5">
                {company.industry && <span>{company.industry}</span>}
                {company.city && <><span>·</span><span>{company.city}</span></>}
                {company.headquarters && !company.city && <><span>·</span><span>{company.headquarters}</span></>}
                {company.gmb_rating != null && (
                  <><span>·</span><StarRating rating={company.gmb_rating} count={company.gmb_review_count} /></>
                )}
              </div>
            </div>
          </div>

          {/* Tabs */}
          <div className="flex gap-0 mt-4 border-b">
            {TABS.map((t) => (
              <button
                key={t.key}
                onClick={() => setTab(t.key)}
                className={cn(
                  'px-4 py-2 text-sm font-medium border-b-2 transition-colors',
                  activeTab === t.key
                    ? 'border-primary text-primary'
                    : 'border-transparent text-muted-foreground hover:text-foreground',
                )}
              >
                {t.label}
                {t.key === 'proposals' && proposals.length > 0 && (
                  <span className="ml-1.5 text-xs text-muted-foreground">({proposals.length})</span>
                )}
                {t.key === 'contacts' && contacts.length > 0 && (
                  <span className="ml-1.5 text-xs text-muted-foreground">({contacts.length})</span>
                )}
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* ── Tab content ── */}
      <div className="flex-1 overflow-auto px-6 py-5">
        {activeTab === 'overview' && (
          <OverviewTab
            company={company}
            proposals={proposals}
            contacts={contacts}
            contactMethods={contactMethods}
            onNavigate={setTab}
            onRemoveMethod={handleRemoveMethod}
          />
        )}
        {activeTab === 'proposals' && (
          <ProposalsTab proposals={proposals} onNewProposal={() => setShowCreateProposal(true)} />
        )}
        {activeTab === 'contacts' && (
          <ContactsTab contacts={contacts} onResearch={handlePersonResearch} />
        )}
        {activeTab === 'intelligence' && (
          <IntelligenceTab
            company={company}
            intel={intel ?? null}
            onRun={handleRunResearch}
            isPolling={isPolling}
          />
        )}
        {activeTab === 'edit' && (
          <EditTab
            company={company}
            onSaved={() => {
              refetchCompany();
              setTab('overview');
            }}
          />
        )}
      </div>

      <Suspense fallback={null}>
        <ProposalCreateModal
          open={showCreateProposal}
          onClose={() => setShowCreateProposal(false)}
          defaultCompanyId={companyId}
        />
      </Suspense>
    </div>
  );
}
