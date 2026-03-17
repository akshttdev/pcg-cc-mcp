import { useState } from 'react';
import {
  RefreshCw, Sparkles, Building2, Users, Globe, MapPin,
  Target, Mic2, TrendingUp, ChevronDown, ChevronUp,
  Instagram, Twitter, Linkedin, Facebook, Phone, Mail,
  Star, MessageSquare,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { EmptyState } from '@/components/ui/empty-state';
import { type CompanyRecord } from '@/lib/api';
import { IntelBadge } from '../components/helpers';

// ─── helpers ────────────────────────────────────────────────────────────────

function parseRaw(raw?: string | null): Record<string, unknown> {
  if (!raw) return {};
  try { return JSON.parse(raw) as Record<string, unknown>; } catch { return {}; }
}

function asStrings(val: unknown): string[] {
  if (Array.isArray(val)) return val.filter((v): v is string => typeof v === 'string');
  if (typeof val === 'string' && val.trim()) return [val];
  return [];
}

function Section({ icon: Icon, title, children }: { icon: React.FC<{ className?: string }>; title: string; children: React.ReactNode }) {
  return (
    <Card>
      <CardHeader className="pb-2 pt-4">
        <CardTitle className="text-sm font-semibold flex items-center gap-2">
          <Icon className="h-4 w-4 text-primary" />
          {title}
        </CardTitle>
      </CardHeader>
      <CardContent className="pb-4">{children}</CardContent>
    </Card>
  );
}

function Chip({ label }: { label: string }) {
  return (
    <Badge variant="secondary" className="text-xs font-normal">
      {label}
    </Badge>
  );
}

function SocialLink({ href, label, icon: Icon, color }: { href: string; label: string; icon: React.FC<{ className?: string }>; color: string }) {
  return (
    <a href={href} target="_blank" rel="noopener noreferrer"
      className="flex items-center gap-1.5 text-xs hover:underline"
      style={{ color }}>
      <Icon className="h-3.5 w-3.5" />
      {label}
    </a>
  );
}

// ─── main component ──────────────────────────────────────────────────────────

export function IntelligenceTab({
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
  const [rawOpen, setRawOpen] = useState(false);
  const raw = parseRaw(company.intelligence_raw);
  const hasRaw = Object.keys(raw).length > 0 && !(Object.keys(raw).length === 1 && raw[''] === undefined);

  // Pull structured sections from intelligence_raw if populated by research agent
  const services       = asStrings(raw.services       ?? raw.offerings       ?? raw.products);
  const differentiators= asStrings(raw.differentiators ?? raw.unique_value    ?? raw.competitive_advantages);
  const targetAudience = asStrings(raw.target_audience ?? raw.ideal_customer  ?? raw.icp);
  const brandVoice     = asStrings(raw.tone            ?? raw.brand_voice     ?? raw.content_style);
  const competitors    = asStrings(raw.competitors     ?? raw.competition);
  const keyFacts       = asStrings(raw.key_facts       ?? raw.highlights);

  // Company baseline fields
  const hasSocials = company.instagram_handle || company.twitter_handle || company.linkedin_url || company.facebook_url;
  const hasContact = company.phone || company.email || company.whatsapp;
  const hasLocation = company.city || company.country || company.headquarters || company.address;
  const hasOverviewData = company.industry || company.founded_year || company.employee_count || hasLocation;

  const hasAnyStructured = services.length || differentiators.length || targetAudience.length ||
    brandVoice.length || competitors.length || keyFacts.length;

  const noData = !intel?.summary && !hasAnyStructured && !hasOverviewData && !hasSocials;

  return (
    <div className="space-y-4">
      {/* ── toolbar ── */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2 flex-wrap">
          <IntelBadge status={intel?.status ?? 'idle'} />
          {intel?.last_run_at && (
            <span className="text-xs text-muted-foreground">
              Last run: {new Date(intel.last_run_at).toLocaleDateString()}
            </span>
          )}
          {intel?.agent && (
            <span className="text-xs text-muted-foreground">via {intel.agent}</span>
          )}
        </div>
        <Button
          size="sm"
          variant="outline"
          onClick={onRun}
          disabled={isPolling || intel?.status === 'running' || intel?.status === 'queued'}
        >
          {isPolling
            ? <RefreshCw className="h-3.5 w-3.5 mr-1 animate-spin" />
            : <Sparkles className="h-3.5 w-3.5 mr-1" />}
          Run Research
        </Button>
      </div>

      {/* ── confidence bar ── */}
      {(intel?.confidence ?? 0) > 0 && (
        <div className="space-y-1">
          <div className="flex justify-between text-xs text-muted-foreground">
            <span>Research confidence</span>
            <span>{Math.round((intel?.confidence ?? 0) * 100)}%</span>
          </div>
          <div className="h-1.5 bg-muted rounded-full overflow-hidden">
            <div className="h-full bg-green-500 rounded-full transition-all"
              style={{ width: `${(intel?.confidence ?? 0) * 100}%` }} />
          </div>
        </div>
      )}

      {noData ? (
        <EmptyState
          icon={Sparkles}
          title="No intelligence gathered yet"
          description="Run research to populate company profile"
          className="h-32 border rounded-lg"
        />
      ) : (
        <>
          {/* ── Brand Overview ── */}
          {hasOverviewData && (
            <Section icon={Building2} title="Brand Overview">
              <dl className="grid grid-cols-2 gap-x-6 gap-y-2 text-sm">
                {company.industry && (
                  <>
                    <dt className="text-muted-foreground">Industry</dt>
                    <dd className="font-medium">{company.industry}</dd>
                  </>
                )}
                {company.founded_year && (
                  <>
                    <dt className="text-muted-foreground">Founded</dt>
                    <dd className="font-medium">{company.founded_year}</dd>
                  </>
                )}
                {company.employee_count && (
                  <>
                    <dt className="text-muted-foreground">Team size</dt>
                    <dd className="font-medium">{company.employee_count}</dd>
                  </>
                )}
                {(company.city || company.country) && (
                  <>
                    <dt className="text-muted-foreground">Location</dt>
                    <dd className="font-medium flex items-center gap-1">
                      <MapPin className="h-3 w-3 text-muted-foreground" />
                      {[company.city, company.country].filter(Boolean).join(', ')}
                    </dd>
                  </>
                )}
                {company.headquarters && !company.city && (
                  <>
                    <dt className="text-muted-foreground">HQ</dt>
                    <dd className="font-medium">{company.headquarters}</dd>
                  </>
                )}
                {company.gmb_rating && (
                  <>
                    <dt className="text-muted-foreground">Rating</dt>
                    <dd className="font-medium flex items-center gap-1">
                      <Star className="h-3 w-3 text-yellow-500" />
                      {company.gmb_rating.toFixed(1)}
                      {company.gmb_review_count && (
                        <span className="text-muted-foreground text-xs">({company.gmb_review_count} reviews)</span>
                      )}
                    </dd>
                  </>
                )}
              </dl>
              {company.description && (
                <p className="mt-3 text-sm text-muted-foreground leading-relaxed border-t pt-3">
                  {company.description}
                </p>
              )}
            </Section>
          )}

          {/* ── Key Highlights (raw key_facts) ── */}
          {keyFacts.length > 0 && (
            <Section icon={TrendingUp} title="Key Highlights">
              <ul className="space-y-1">
                {keyFacts.map((f, i) => (
                  <li key={i} className="text-sm text-muted-foreground flex gap-2">
                    <span className="text-primary mt-0.5">•</span>
                    <span>{f}</span>
                  </li>
                ))}
              </ul>
            </Section>
          )}

          {/* ── Services & Offerings ── */}
          {services.length > 0 && (
            <Section icon={Building2} title="Services & Offerings">
              <div className="flex flex-wrap gap-2">
                {services.map((s, i) => <Chip key={i} label={s} />)}
              </div>
            </Section>
          )}

          {/* ── Target Market ── */}
          {targetAudience.length > 0 && (
            <Section icon={Target} title="Target Market & Audience">
              <div className="flex flex-wrap gap-2">
                {targetAudience.map((t, i) => <Chip key={i} label={t} />)}
              </div>
            </Section>
          )}

          {/* ── Brand Voice ── */}
          {brandVoice.length > 0 && (
            <Section icon={Mic2} title="Brand Voice & Personality">
              <div className="flex flex-wrap gap-2">
                {brandVoice.map((v, i) => <Chip key={i} label={v} />)}
              </div>
            </Section>
          )}

          {/* ── Competitive Differentiators ── */}
          {differentiators.length > 0 && (
            <Section icon={TrendingUp} title="Competitive Differentiators">
              <ul className="space-y-1">
                {differentiators.map((d, i) => (
                  <li key={i} className="text-sm text-muted-foreground flex gap-2">
                    <span className="text-green-500 mt-0.5">✓</span>
                    <span>{d}</span>
                  </li>
                ))}
              </ul>
            </Section>
          )}

          {/* ── Competitors ── */}
          {competitors.length > 0 && (
            <Section icon={Users} title="Competitive Landscape">
              <div className="flex flex-wrap gap-2">
                {competitors.map((c, i) => <Chip key={i} label={c} />)}
              </div>
            </Section>
          )}

          {/* ── Social Presence ── */}
          {hasSocials && (
            <Section icon={Globe} title="Social Presence">
              <div className="flex flex-wrap gap-4">
                {company.instagram_handle && (
                  <SocialLink
                    href={`https://instagram.com/${company.instagram_handle.replace('@', '')}`}
                    label={company.instagram_handle}
                    icon={Instagram}
                    color="#E1306C"
                  />
                )}
                {company.twitter_handle && (
                  <SocialLink
                    href={`https://twitter.com/${company.twitter_handle.replace('@', '')}`}
                    label={company.twitter_handle}
                    icon={Twitter}
                    color="#1DA1F2"
                  />
                )}
                {company.linkedin_url && (
                  <SocialLink
                    href={company.linkedin_url}
                    label="LinkedIn"
                    icon={Linkedin}
                    color="#0A66C2"
                  />
                )}
                {company.facebook_url && (
                  <SocialLink
                    href={company.facebook_url}
                    label="Facebook"
                    icon={Facebook}
                    color="#1877F2"
                  />
                )}
              </div>
            </Section>
          )}

          {/* ── Contact ── */}
          {hasContact && (
            <Section icon={MessageSquare} title="Contact Information">
              <div className="space-y-1.5">
                {company.phone && (
                  <a href={`tel:${company.phone}`} className="flex items-center gap-2 text-sm hover:underline">
                    <Phone className="h-3.5 w-3.5 text-muted-foreground" />
                    {company.phone}
                  </a>
                )}
                {company.email && (
                  <a href={`mailto:${company.email}`} className="flex items-center gap-2 text-sm hover:underline">
                    <Mail className="h-3.5 w-3.5 text-muted-foreground" />
                    {company.email}
                  </a>
                )}
                {company.whatsapp && (
                  <span className="flex items-center gap-2 text-sm text-muted-foreground">
                    <MessageSquare className="h-3.5 w-3.5" />
                    WhatsApp: {company.whatsapp}
                  </span>
                )}
              </div>
            </Section>
          )}

          {/* ── AI Summary ── */}
          {intel?.summary && (
            <Section icon={Sparkles} title="AI Research Summary">
              <p className="text-sm text-muted-foreground leading-relaxed whitespace-pre-line">
                {intel.summary}
              </p>
            </Section>
          )}

          {/* ── Raw Data (collapsible) ── */}
          {hasRaw && (
            <Card>
              <button
                className="w-full flex items-center justify-between px-4 py-3 text-sm font-medium hover:bg-muted/50 transition-colors rounded-t-lg"
                onClick={() => setRawOpen(v => !v)}
              >
                <span className="flex items-center gap-2 text-muted-foreground">
                  Raw Research Data
                </span>
                {rawOpen
                  ? <ChevronUp className="h-4 w-4 text-muted-foreground" />
                  : <ChevronDown className="h-4 w-4 text-muted-foreground" />}
              </button>
              {rawOpen && (
                <CardContent className="pt-0">
                  <pre className="text-xs text-muted-foreground overflow-auto max-h-64 bg-muted rounded p-2">
                    {JSON.stringify(raw, null, 2)}
                  </pre>
                </CardContent>
              )}
            </Card>
          )}
        </>
      )}
    </div>
  );
}
