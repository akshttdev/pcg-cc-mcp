import { Star, Phone, MessageCircle, Mail, Globe, Instagram, Linkedin, Twitter, Facebook, Clock } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { cn } from '@/lib/utils';
import { type CompanyRecord } from '@/lib/api';

// ── Constants ────────────────────────────────────────────────────────────────

export const DAYS = ['mon','tue','wed','thu','fri','sat','sun'] as const;
export const DAY_LABELS: Record<string, string> = {
  mon: 'Monday', tue: 'Tuesday', wed: 'Wednesday', thu: 'Thursday',
  fri: 'Friday', sat: 'Saturday', sun: 'Sunday',
};

// ── StarRating ───────────────────────────────────────────────────────────────

export function StarRating({ rating, count }: { rating?: number | null; count?: number | null }) {
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

// ── IntelBadge ───────────────────────────────────────────────────────────────

export function IntelBadge({ status }: { status: string }) {
  const cfg: Record<string, { label: string; cls: string }> = {
    idle:    { label: 'No Intel',  cls: 'bg-gray-100 text-gray-500' },
    queued:  { label: 'Queued',    cls: 'bg-yellow-100 text-yellow-700' },
    running: { label: 'Running\u2026',  cls: 'bg-blue-100 text-blue-700 animate-pulse' },
    done:    { label: 'Intel \u2713',   cls: 'bg-green-100 text-green-700' },
    failed:  { label: 'Failed',    cls: 'bg-red-100 text-red-700' },
  };
  const { label, cls } = cfg[status] ?? cfg['idle'];
  return <Badge className={`text-xs border-0 px-2 py-0.5 ${cls}`}>{label}</Badge>;
}

// ── ProposalStatusBadge ──────────────────────────────────────────────────────

export function ProposalStatusBadge({ status }: { status: string }) {
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

// ── BusinessHoursCard ────────────────────────────────────────────────────────

export function BusinessHoursCard({ hours }: { hours?: string | null }) {
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

// ── ContactRail ──────────────────────────────────────────────────────────────

export function ContactRail({ company }: { company: CompanyRecord }) {
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
