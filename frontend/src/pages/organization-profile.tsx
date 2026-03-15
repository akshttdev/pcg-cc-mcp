import { useCallback, useMemo, useState } from 'react';
import type React from 'react';
import { useParams, useSearchParams, useLocation, useNavigate, Link } from 'react-router-dom';
import { PageErrorBoundary } from '@/components/PageErrorBoundary';
import { toast } from 'sonner';
import { useQuery, useQueries, useMutation, useQueryClient } from '@tanstack/react-query';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
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
  MapPin,
  Users,
  FolderOpen,
  Briefcase,
  Target,
  TrendingUp,
  ArrowLeft,
  Globe,
  ExternalLink,
  LayoutGrid,
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
  Palette,
  Mic,
  Type,
  Zap,
  Heart,
  Link as LinkIcon,
  Tag,
  Megaphone,
  Lightbulb,
  Crosshair,
  CalendarDays,
  CheckCircle,
  BarChart2,
  ChevronLeft,
  CalendarRange,
  List,
  Play,
  Phone,
  UserPlus,
  Link2,
  Eye,
  Shield,
  Copy,
} from 'lucide-react';
import {
  organizationsApi,
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
  type SocialPostRecord,
  type SocialMentionRecord,
  dataSourcesApi,
  workflowsApi,
  resolveApiUrl,
  type DataSourceRecord,
  type UpdateDataSourceRequest,
  type ExecutionArtifact,
  DATA_TYPE_OPTIONS,
  type EmailAccountRecord,
  type OrgBrandProfile,
  companiesApi,
  type CompanyRecord,
  crmApi,
  type CreateCrmContactRequest,
  type CrmContactRecord,
} from '@/lib/api';
import { useUserSystem } from '@/components/config-provider';
import { WorkflowEditor as WorkflowEditorComponent } from '@/components/workflows/WorkflowEditor';
import type { WorkflowDefinition } from '@/lib/api';
import { CrmPipelineBoard } from '@/components/crm/CrmPipelineBoard';

import { LIFECYCLE_STAGE_INFO, type LifecycleStage } from '@/types/crm';
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

// ── Brand Identity Card ───────────────────────────────────────────────────────

const BRAND_VOICE_OPTIONS = ['formal', 'casual', 'playful', 'authoritative', 'bold', 'sophisticated'];
const BRAND_ARCHETYPE_OPTIONS = ['Hero', 'Creator', 'Sage', 'Outlaw', 'Explorer', 'Ruler', 'Caregiver', 'Innocent', 'Jester', 'Lover', 'Magician', 'Regular Guy'];
const MARKET_POSITION_OPTIONS = ['luxury', 'premium', 'mid-market', 'budget'];
const ICP_COMPANY_SIZE_OPTIONS = ['solo', 'startup', 'smb', 'mid-market', 'enterprise'];

function parseJsonArray(val: string | null | undefined): string[] {
  if (!val) return [];
  try { return JSON.parse(val); } catch { return []; }
}

export function BrandIdentityCard({ orgId, orgName }: { orgId: string; orgName: string }) {
  const qc = useQueryClient();
  const { data: profile, isLoading } = useQuery<OrgBrandProfile | null>({
    queryKey: ['orgBrandProfile', orgId],
    queryFn: () => organizationsApi.getBrandProfile(orgId),
    staleTime: 5 * 60_000,
  });

  const [editing, setEditing] = useState(false);
  const [form, setForm] = useState<Partial<OrgBrandProfile>>({});
  const [saving, setSaving] = useState(false);

  const openEdit = () => {
    setForm({
      tagline: profile?.tagline ?? '',
      primaryColor: profile?.primaryColor ?? '#2563EB',
      secondaryColor: profile?.secondaryColor ?? '#EC4899',
      accentColor: profile?.accentColor ?? '',
      typographyHeading: profile?.typographyHeading ?? '',
      typographyBody: profile?.typographyBody ?? '',
      logoUrl: profile?.logoUrl ?? '',
      industry: profile?.industry ?? '',
      marketPosition: profile?.marketPosition ?? '',
      uniqueValueProposition: profile?.uniqueValueProposition ?? '',
      missionStatement: profile?.missionStatement ?? '',
      visionStatement: profile?.visionStatement ?? '',
      brandValues: profile?.brandValues ?? '[]',
      brandVoice: profile?.brandVoice ?? '',
      brandArchetype: profile?.brandArchetype ?? '',
      targetAudience: profile?.targetAudience ?? '',
      icpDescription: profile?.icpDescription ?? '',
      icpCompanySize: profile?.icpCompanySize ?? '',
      icpIndustries: profile?.icpIndustries ?? '[]',
      competitorBrands: profile?.competitorBrands ?? '[]',
      differentiators: profile?.differentiators ?? '[]',
      contentPillars: profile?.contentPillars ?? '[]',
      contentTone: profile?.contentTone ?? '',
      websiteUrl: profile?.websiteUrl ?? '',
      socialInstagram: profile?.socialInstagram ?? '',
      socialTwitter: profile?.socialTwitter ?? '',
      socialLinkedin: profile?.socialLinkedin ?? '',
      socialFacebook: profile?.socialFacebook ?? '',
      socialYoutube: profile?.socialYoutube ?? '',
      socialTiktok: profile?.socialTiktok ?? '',
    });
    setEditing(true);
  };

  const save = async () => {
    setSaving(true);
    try {
      await organizationsApi.upsertBrandProfile(orgId, form);
      qc.invalidateQueries({ queryKey: ['orgBrandProfile', orgId] });
      setEditing(false);
    } finally {
      setSaving(false);
    }
  };

  const set = (k: keyof OrgBrandProfile, v: string) => setForm(f => ({ ...f, [k]: v }));

  // helper: comma-separated ↔ JSON array
  const getArr = (k: keyof OrgBrandProfile) => parseJsonArray(form[k] as string).join(', ');
  const setArr = (k: keyof OrgBrandProfile, v: string) =>
    set(k, JSON.stringify(v.split(',').map(s => s.trim()).filter(Boolean)));

  const values = parseJsonArray(profile?.brandValues);
  const pillars = parseJsonArray(profile?.contentPillars);
  const competitors = parseJsonArray(profile?.competitorBrands);
  const differentiators = parseJsonArray(profile?.differentiators);

  const initials = orgName.split(' ').slice(0, 2).map(w => w[0]).join('').toUpperCase();

  return (
    <>
      <Card className="bg-card/80 backdrop-blur-sm border-border/50">
        <CardHeader className="flex flex-row items-start justify-between pb-2">
          <div>
            <CardTitle className="text-base flex items-center gap-2">
              <Palette className="h-4 w-4 text-[hsl(var(--brand))]" />
              Brand Identity
            </CardTitle>
            <CardDescription>Visual language, positioning, audience, and content strategy</CardDescription>
          </div>
          <Button variant="outline" size="sm" onClick={openEdit} className="shrink-0">
            <Pencil className="h-3 w-3 mr-1.5" />
            {profile ? 'Edit' : 'Set up brand'}
          </Button>
        </CardHeader>

        {isLoading ? (
          <CardContent className="flex items-center gap-2 text-muted-foreground py-6">
            <Loader2 className="h-4 w-4 animate-spin" /> Loading…
          </CardContent>
        ) : !profile ? (
          <CardContent className="py-8 text-center text-muted-foreground">
            <Palette className="h-8 w-8 mx-auto mb-2 opacity-30" />
            <p className="text-sm">No brand profile yet. Click <strong>Set up brand</strong> to get started.</p>
          </CardContent>
        ) : (
          <CardContent className="space-y-6">
            {/* Hero row */}
            <div className="flex items-start gap-4">
              <div
                className="h-16 w-16 rounded-xl flex items-center justify-center text-white text-xl font-bold shrink-0 shadow"
                style={{ background: `linear-gradient(135deg, ${profile.primaryColor}, ${profile.secondaryColor})` }}
              >
                {initials}
              </div>
              <div className="min-w-0">
                <p className="font-semibold text-lg leading-tight">{orgName}</p>
                {profile.tagline && <p className="text-muted-foreground text-sm mt-0.5 italic">"{profile.tagline}"</p>}
                <div className="flex items-center gap-2 flex-wrap mt-1.5">
                  {profile.industry && <Badge variant="secondary" className="text-[10px]">{profile.industry}</Badge>}
                  {profile.marketPosition && <Badge variant="outline" className="text-[10px] capitalize">{profile.marketPosition}</Badge>}
                  {profile.brandArchetype && <Badge className="text-[10px] bg-[hsl(var(--brand))]/10 text-[hsl(var(--brand))] border-[hsl(var(--brand))]/20">{profile.brandArchetype}</Badge>}
                </div>
              </div>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
              {/* Visual Identity */}
              <div className="space-y-3">
                <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground flex items-center gap-1.5">
                  <Palette className="h-3 w-3" /> Visual Identity
                </p>
                <div className="flex items-center gap-2">
                  <div className="h-8 w-8 rounded-md border shadow-sm shrink-0" style={{ backgroundColor: profile.primaryColor }} />
                  <div>
                    <p className="text-[10px] text-muted-foreground">Primary</p>
                    <p className="text-xs font-mono font-medium">{profile.primaryColor}</p>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <div className="h-8 w-8 rounded-md border shadow-sm shrink-0" style={{ backgroundColor: profile.secondaryColor }} />
                  <div>
                    <p className="text-[10px] text-muted-foreground">Secondary</p>
                    <p className="text-xs font-mono font-medium">{profile.secondaryColor}</p>
                  </div>
                </div>
                {profile.accentColor && (
                  <div className="flex items-center gap-2">
                    <div className="h-8 w-8 rounded-md border shadow-sm shrink-0" style={{ backgroundColor: profile.accentColor }} />
                    <div>
                      <p className="text-[10px] text-muted-foreground">Accent</p>
                      <p className="text-xs font-mono font-medium">{profile.accentColor}</p>
                    </div>
                  </div>
                )}
                {profile.typographyHeading && (
                  <div className="flex items-center gap-2 pt-1">
                    <Type className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                    <div>
                      <p className="text-[10px] text-muted-foreground">Heading Font</p>
                      <p className="text-xs font-medium">{profile.typographyHeading}</p>
                    </div>
                  </div>
                )}
                {profile.typographyBody && (
                  <div className="flex items-center gap-2">
                    <Type className="h-3.5 w-3.5 text-muted-foreground shrink-0 opacity-60" />
                    <div>
                      <p className="text-[10px] text-muted-foreground">Body Font</p>
                      <p className="text-xs font-medium">{profile.typographyBody}</p>
                    </div>
                  </div>
                )}
              </div>

              {/* Positioning */}
              <div className="space-y-3">
                <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground flex items-center gap-1.5">
                  <Lightbulb className="h-3 w-3" /> Positioning
                </p>
                {profile.uniqueValueProposition && (
                  <div>
                    <p className="text-[10px] text-muted-foreground mb-0.5">Unique Value Proposition</p>
                    <p className="text-xs leading-relaxed">{profile.uniqueValueProposition}</p>
                  </div>
                )}
                {profile.missionStatement && (
                  <div>
                    <p className="text-[10px] text-muted-foreground mb-0.5">Mission</p>
                    <p className="text-xs leading-relaxed">{profile.missionStatement}</p>
                  </div>
                )}
                {profile.visionStatement && (
                  <div>
                    <p className="text-[10px] text-muted-foreground mb-0.5">Vision</p>
                    <p className="text-xs leading-relaxed">{profile.visionStatement}</p>
                  </div>
                )}
                {values.length > 0 && (
                  <div>
                    <p className="text-[10px] text-muted-foreground mb-1">Brand Values</p>
                    <div className="flex flex-wrap gap-1">
                      {values.map(v => <Badge key={v} variant="outline" className="text-[10px]">{v}</Badge>)}
                    </div>
                  </div>
                )}
                {profile.brandVoice && (
                  <div className="flex items-center gap-2">
                    <Mic className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                    <div>
                      <p className="text-[10px] text-muted-foreground">Brand Voice</p>
                      <p className="text-xs font-medium capitalize">{profile.brandVoice}</p>
                    </div>
                  </div>
                )}
              </div>

              {/* Audience */}
              <div className="space-y-3">
                <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground flex items-center gap-1.5">
                  <Crosshair className="h-3 w-3" /> Audience & ICP
                </p>
                {profile.targetAudience && (
                  <div>
                    <p className="text-[10px] text-muted-foreground mb-0.5">Target Audience</p>
                    <p className="text-xs leading-relaxed">{profile.targetAudience}</p>
                  </div>
                )}
                {profile.icpDescription && (
                  <div>
                    <p className="text-[10px] text-muted-foreground mb-0.5">Ideal Customer Profile</p>
                    <p className="text-xs leading-relaxed">{profile.icpDescription}</p>
                  </div>
                )}
                {profile.icpCompanySize && (
                  <div className="flex items-center gap-2">
                    <Tag className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                    <div>
                      <p className="text-[10px] text-muted-foreground">Company Size</p>
                      <p className="text-xs font-medium capitalize">{profile.icpCompanySize}</p>
                    </div>
                  </div>
                )}
              </div>

              {/* Content Strategy */}
              {(pillars.length > 0 || profile.contentTone) && (
                <div className="space-y-3">
                  <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground flex items-center gap-1.5">
                    <Megaphone className="h-3 w-3" /> Content Strategy
                  </p>
                  {pillars.length > 0 && (
                    <div>
                      <p className="text-[10px] text-muted-foreground mb-1">Content Pillars</p>
                      <div className="flex flex-wrap gap-1">
                        {pillars.map(p => <Badge key={p} variant="secondary" className="text-[10px]">{p}</Badge>)}
                      </div>
                    </div>
                  )}
                  {profile.contentTone && (
                    <div>
                      <p className="text-[10px] text-muted-foreground mb-0.5">Tone Notes</p>
                      <p className="text-xs leading-relaxed">{profile.contentTone}</p>
                    </div>
                  )}
                </div>
              )}

              {/* Competitive */}
              {(competitors.length > 0 || differentiators.length > 0) && (
                <div className="space-y-3">
                  <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground flex items-center gap-1.5">
                    <Zap className="h-3 w-3" /> Competitive
                  </p>
                  {competitors.length > 0 && (
                    <div>
                      <p className="text-[10px] text-muted-foreground mb-1">Competitors</p>
                      <div className="flex flex-wrap gap-1">
                        {competitors.map(c => <Badge key={c} variant="outline" className="text-[10px] border-destructive/30 text-destructive">{c}</Badge>)}
                      </div>
                    </div>
                  )}
                  {differentiators.length > 0 && (
                    <div>
                      <p className="text-[10px] text-muted-foreground mb-1">Differentiators</p>
                      <div className="flex flex-wrap gap-1">
                        {differentiators.map(d => <Badge key={d} variant="outline" className="text-[10px] border-emerald-500/30 text-emerald-600">{d}</Badge>)}
                      </div>
                    </div>
                  )}
                </div>
              )}

              {/* Online Presence */}
              {(profile.websiteUrl || profile.socialInstagram || profile.socialLinkedin || profile.socialTwitter || profile.socialTiktok || profile.socialYoutube) && (
                <div className="space-y-3">
                  <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground flex items-center gap-1.5">
                    <Globe className="h-3 w-3" /> Online Presence
                  </p>
                  {profile.websiteUrl && (
                    <a href={profile.websiteUrl} target="_blank" rel="noopener noreferrer"
                      className="flex items-center gap-1.5 text-xs text-primary hover:underline">
                      <LinkIcon className="h-3 w-3" /> {profile.websiteUrl.replace(/^https?:\/\//, '')}
                    </a>
                  )}
                  <div className="flex flex-wrap gap-2">
                    {profile.socialInstagram && <a href={`https://instagram.com/${profile.socialInstagram.replace('@','')}`} target="_blank" rel="noopener noreferrer" className="text-xs text-muted-foreground hover:text-foreground flex items-center gap-1"><Instagram className="h-3.5 w-3.5" />{profile.socialInstagram}</a>}
                    {profile.socialLinkedin && <a href={`https://linkedin.com/in/${profile.socialLinkedin.replace('@','')}`} target="_blank" rel="noopener noreferrer" className="text-xs text-muted-foreground hover:text-foreground flex items-center gap-1"><Linkedin className="h-3.5 w-3.5" />{profile.socialLinkedin}</a>}
                    {profile.socialTwitter && <a href={`https://twitter.com/${profile.socialTwitter.replace('@','')}`} target="_blank" rel="noopener noreferrer" className="text-xs text-muted-foreground hover:text-foreground flex items-center gap-1"><Twitter className="h-3.5 w-3.5" />{profile.socialTwitter}</a>}
                    {profile.socialTiktok && <a href={`https://tiktok.com/@${profile.socialTiktok.replace('@','')}`} target="_blank" rel="noopener noreferrer" className="text-xs text-muted-foreground hover:text-foreground flex items-center gap-1"><Heart className="h-3.5 w-3.5" />{profile.socialTiktok}</a>}
                    {profile.socialYoutube && <a href={`https://youtube.com/@${profile.socialYoutube.replace('@','')}`} target="_blank" rel="noopener noreferrer" className="text-xs text-muted-foreground hover:text-foreground flex items-center gap-1"><Youtube className="h-3.5 w-3.5" />{profile.socialYoutube}</a>}
                    {profile.socialFacebook && <a href={`https://facebook.com/${profile.socialFacebook.replace('@','')}`} target="_blank" rel="noopener noreferrer" className="text-xs text-muted-foreground hover:text-foreground flex items-center gap-1"><Facebook className="h-3.5 w-3.5" />{profile.socialFacebook}</a>}
                  </div>
                </div>
              )}
            </div>
          </CardContent>
        )}
      </Card>

      {/* Edit Dialog */}
      <Dialog open={editing} onOpenChange={setEditing}>
        <DialogContent className="max-w-3xl max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Palette className="h-5 w-5 text-[hsl(var(--brand))]" />
              Brand Profile — {orgName}
            </DialogTitle>
          </DialogHeader>

          <div className="space-y-6 py-2">
            {/* Visual */}
            <div>
              <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-3 flex items-center gap-1.5"><Palette className="h-3 w-3" /> Visual Identity</p>
              <div className="grid grid-cols-2 gap-3">
                <div><Label className="text-xs">Tagline</Label><Input value={form.tagline ?? ''} onChange={e => set('tagline', e.target.value)} placeholder="One-liner that captures the brand" /></div>
                <div><Label className="text-xs">Industry</Label><Input value={form.industry ?? ''} onChange={e => set('industry', e.target.value)} placeholder="e.g. Luxury Agency, SaaS" /></div>
                <div>
                  <Label className="text-xs">Primary Color</Label>
                  <div className="flex gap-2 mt-1">
                    <input type="color" value={form.primaryColor ?? '#2563EB'} onChange={e => set('primaryColor', e.target.value)} className="h-9 w-12 rounded border cursor-pointer" />
                    <Input value={form.primaryColor ?? ''} onChange={e => set('primaryColor', e.target.value)} placeholder="#2563EB" className="font-mono" />
                  </div>
                </div>
                <div>
                  <Label className="text-xs">Secondary Color</Label>
                  <div className="flex gap-2 mt-1">
                    <input type="color" value={form.secondaryColor ?? '#EC4899'} onChange={e => set('secondaryColor', e.target.value)} className="h-9 w-12 rounded border cursor-pointer" />
                    <Input value={form.secondaryColor ?? ''} onChange={e => set('secondaryColor', e.target.value)} placeholder="#EC4899" className="font-mono" />
                  </div>
                </div>
                <div>
                  <Label className="text-xs">Accent Color</Label>
                  <div className="flex gap-2 mt-1">
                    <input type="color" value={form.accentColor ?? '#000000'} onChange={e => set('accentColor', e.target.value)} className="h-9 w-12 rounded border cursor-pointer" />
                    <Input value={form.accentColor ?? ''} onChange={e => set('accentColor', e.target.value)} placeholder="#000000" className="font-mono" />
                  </div>
                </div>
                <div><Label className="text-xs">Heading Font</Label><Input value={form.typographyHeading ?? ''} onChange={e => set('typographyHeading', e.target.value)} placeholder="e.g. Anton, Playfair Display" /></div>
                <div><Label className="text-xs">Body Font</Label><Input value={form.typographyBody ?? ''} onChange={e => set('typographyBody', e.target.value)} placeholder="e.g. Inter, DM Sans" /></div>
                <div><Label className="text-xs">Logo URL</Label><Input value={form.logoUrl ?? ''} onChange={e => set('logoUrl', e.target.value)} placeholder="https://..." /></div>
              </div>
            </div>

            {/* Positioning */}
            <div>
              <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-3 flex items-center gap-1.5"><Lightbulb className="h-3 w-3" /> Positioning</p>
              <div className="grid grid-cols-2 gap-3">
                <div className="col-span-2"><Label className="text-xs">Unique Value Proposition</Label><Input value={form.uniqueValueProposition ?? ''} onChange={e => set('uniqueValueProposition', e.target.value)} placeholder="What makes this brand irreplaceable?" /></div>
                <div className="col-span-2"><Label className="text-xs">Mission Statement</Label><Textarea value={form.missionStatement ?? ''} onChange={e => set('missionStatement', e.target.value)} placeholder="Why does this brand exist?" rows={2} /></div>
                <div className="col-span-2"><Label className="text-xs">Vision Statement</Label><Textarea value={form.visionStatement ?? ''} onChange={e => set('visionStatement', e.target.value)} placeholder="Where is this brand going?" rows={2} /></div>
                <div>
                  <Label className="text-xs">Brand Voice</Label>
                  <Select value={form.brandVoice ?? ''} onValueChange={v => set('brandVoice', v)}>
                    <SelectTrigger><SelectValue placeholder="Select voice" /></SelectTrigger>
                    <SelectContent>{BRAND_VOICE_OPTIONS.map(o => <SelectItem key={o} value={o} className="capitalize">{o}</SelectItem>)}</SelectContent>
                  </Select>
                </div>
                <div>
                  <Label className="text-xs">Brand Archetype</Label>
                  <Select value={form.brandArchetype ?? ''} onValueChange={v => set('brandArchetype', v)}>
                    <SelectTrigger><SelectValue placeholder="Select archetype" /></SelectTrigger>
                    <SelectContent>{BRAND_ARCHETYPE_OPTIONS.map(o => <SelectItem key={o} value={o}>{o}</SelectItem>)}</SelectContent>
                  </Select>
                </div>
                <div>
                  <Label className="text-xs">Market Position</Label>
                  <Select value={form.marketPosition ?? ''} onValueChange={v => set('marketPosition', v)}>
                    <SelectTrigger><SelectValue placeholder="Select position" /></SelectTrigger>
                    <SelectContent>{MARKET_POSITION_OPTIONS.map(o => <SelectItem key={o} value={o} className="capitalize">{o}</SelectItem>)}</SelectContent>
                  </Select>
                </div>
                <div><Label className="text-xs">Brand Values <span className="text-muted-foreground font-normal">(comma-separated)</span></Label><Input value={getArr('brandValues')} onChange={e => setArr('brandValues', e.target.value)} placeholder="Integrity, Innovation, Excellence" /></div>
              </div>
            </div>

            {/* Audience */}
            <div>
              <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-3 flex items-center gap-1.5"><Crosshair className="h-3 w-3" /> Audience & ICP</p>
              <div className="grid grid-cols-2 gap-3">
                <div className="col-span-2"><Label className="text-xs">Target Audience</Label><Textarea value={form.targetAudience ?? ''} onChange={e => set('targetAudience', e.target.value)} placeholder="Who is this brand speaking to?" rows={2} /></div>
                <div className="col-span-2"><Label className="text-xs">Ideal Customer Profile (ICP)</Label><Textarea value={form.icpDescription ?? ''} onChange={e => set('icpDescription', e.target.value)} placeholder="Describe the perfect client in detail — pain points, goals, context" rows={3} /></div>
                <div>
                  <Label className="text-xs">ICP Company Size</Label>
                  <Select value={form.icpCompanySize ?? ''} onValueChange={v => set('icpCompanySize', v)}>
                    <SelectTrigger><SelectValue placeholder="Select size" /></SelectTrigger>
                    <SelectContent>{ICP_COMPANY_SIZE_OPTIONS.map(o => <SelectItem key={o} value={o} className="capitalize">{o}</SelectItem>)}</SelectContent>
                  </Select>
                </div>
                <div><Label className="text-xs">ICP Industries <span className="text-muted-foreground font-normal">(comma-separated)</span></Label><Input value={getArr('icpIndustries')} onChange={e => setArr('icpIndustries', e.target.value)} placeholder="Hospitality, Real Estate, Fashion" /></div>
              </div>
            </div>

            {/* Competitive */}
            <div>
              <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-3 flex items-center gap-1.5"><Zap className="h-3 w-3" /> Competitive Intelligence</p>
              <div className="grid grid-cols-2 gap-3">
                <div><Label className="text-xs">Competitors <span className="text-muted-foreground font-normal">(comma-separated)</span></Label><Input value={getArr('competitorBrands')} onChange={e => setArr('competitorBrands', e.target.value)} placeholder="Competitor A, Competitor B" /></div>
                <div><Label className="text-xs">Differentiators <span className="text-muted-foreground font-normal">(comma-separated)</span></Label><Input value={getArr('differentiators')} onChange={e => setArr('differentiators', e.target.value)} placeholder="End-to-end, Luxury positioning, Speed" /></div>
              </div>
            </div>

            {/* Content & Social */}
            <div>
              <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-3 flex items-center gap-1.5"><Megaphone className="h-3 w-3" /> Content & Social</p>
              <div className="grid grid-cols-2 gap-3">
                <div className="col-span-2"><Label className="text-xs">Content Pillars <span className="text-muted-foreground font-normal">(comma-separated)</span></Label><Input value={getArr('contentPillars')} onChange={e => setArr('contentPillars', e.target.value)} placeholder="Education, Behind the scenes, Client results, Culture" /></div>
                <div className="col-span-2"><Label className="text-xs">Tone Notes</Label><Textarea value={form.contentTone ?? ''} onChange={e => set('contentTone', e.target.value)} placeholder="Nuances about how this brand communicates across channels" rows={2} /></div>
                <div className="col-span-2"><Label className="text-xs">Website URL</Label><Input value={form.websiteUrl ?? ''} onChange={e => set('websiteUrl', e.target.value)} placeholder="https://brand.com" /></div>
                <div><Label className="text-xs">Instagram</Label><Input value={form.socialInstagram ?? ''} onChange={e => set('socialInstagram', e.target.value)} placeholder="@handle" /></div>
                <div><Label className="text-xs">LinkedIn</Label><Input value={form.socialLinkedin ?? ''} onChange={e => set('socialLinkedin', e.target.value)} placeholder="@handle or company slug" /></div>
                <div><Label className="text-xs">X (Twitter)</Label><Input value={form.socialTwitter ?? ''} onChange={e => set('socialTwitter', e.target.value)} placeholder="@handle" /></div>
                <div><Label className="text-xs">TikTok</Label><Input value={form.socialTiktok ?? ''} onChange={e => set('socialTiktok', e.target.value)} placeholder="@handle" /></div>
                <div><Label className="text-xs">YouTube</Label><Input value={form.socialYoutube ?? ''} onChange={e => set('socialYoutube', e.target.value)} placeholder="@channel" /></div>
                <div><Label className="text-xs">Facebook</Label><Input value={form.socialFacebook ?? ''} onChange={e => set('socialFacebook', e.target.value)} placeholder="page name or handle" /></div>
              </div>
            </div>
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={() => setEditing(false)}>Cancel</Button>
            <Button onClick={save} disabled={saving}>
              {saving ? <><Loader2 className="h-3 w-3 mr-1.5 animate-spin" />Saving…</> : 'Save Brand Profile'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

// ── Overview Tab ──────────────────────────────────────────────────────────────

function OverviewTab({
  orgId,
  orgName: _orgName,
  projectEntries,
  projectCount,
  clientCount: _clientCount,
  memberCount: _memberCount,
  totalDealValue,
  totalDeals,
  contactCount,
}: {
  orgId: string;
  orgName: string;
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
              <p className="text-xs mt-1">Activity from tasks, deals, and contacts will appear here.</p>
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

// ── Leads Pipeline Panel ──────────────────────────────────────────────────────

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

// ── Contacts Tab (with Companies sub-section) ─────────────────────────────────

function ContactsTab({ orgId }: { orgId: string }) {
  const [crmView, setCrmView] = useState<'contacts' | 'companies'>('contacts');
  const [searchQuery, setSearchQuery] = useState('');
  const [stageFilter, setStageFilter] = useState<string>('all');
  const [showAddContact, setShowAddContact] = useState(false);
  const [showAddCompany, setShowAddCompany] = useState(false);
  const [selectedContactId, setSelectedContactId] = useState<string | null>(null);
  const [contactForm, setContactForm] = useState({ first_name: '', last_name: '', email: '', phone: '', company_name: '', job_title: '', lifecycle_stage: 'lead' });
  const [companyForm, setCompanyForm] = useState({ name: '', website: '', industry: '' });
  const queryClient = useQueryClient();

  const { data: contacts = [], isLoading } = useQuery({
    queryKey: ['crm-contacts', orgId],
    queryFn: () => crmApi.listContacts(orgId),
    enabled: !!orgId,
    staleTime: 30_000,
  });

  const selectedContact = useMemo(
    () => contacts.find((c: CrmContactRecord) => c.id === selectedContactId),
    [contacts, selectedContactId]
  );

  const createContactMutation = useMutation({
    mutationFn: (data: CreateCrmContactRequest) => crmApi.createContact(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['crm-contacts', orgId] });
      setShowAddContact(false);
      setContactForm({ first_name: '', last_name: '', email: '', phone: '', company_name: '', job_title: '', lifecycle_stage: 'lead' });
      toast.success('Contact created');
    },
    onError: () => toast.error('Failed to create contact'),
  });

  const handleCreateContact = useCallback((e: React.FormEvent) => {
    e.preventDefault();
    createContactMutation.mutate({
      organization_id: orgId,
      ...contactForm,
      first_name: contactForm.first_name || undefined,
      last_name: contactForm.last_name || undefined,
      email: contactForm.email || undefined,
      phone: contactForm.phone || undefined,
      company_name: contactForm.company_name || undefined,
      job_title: contactForm.job_title || undefined,
    });
  }, [contactForm, orgId, createContactMutation]);

  const createCompanyMutation = useMutation({
    mutationFn: (data: { name: string; website?: string; industry?: string; created_by_org_id?: string }) => companiesApi.create(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['org-companies', orgId] });
      setShowAddCompany(false);
      setCompanyForm({ name: '', website: '', industry: '' });
      toast.success('Company created');
    },
    onError: () => toast.error('Failed to create company'),
  });

  const handleCreateCompany = useCallback((e: React.FormEvent) => {
    e.preventDefault();
    if (!companyForm.name.trim()) return;
    createCompanyMutation.mutate({
      name: companyForm.name,
      website: companyForm.website || undefined,
      industry: companyForm.industry || undefined,
      created_by_org_id: orgId,
    });
  }, [companyForm, orgId, createCompanyMutation]);

  const { data: companies = [], isLoading: companiesLoading } = useQuery<CompanyRecord[]>({
    queryKey: ['org-companies', orgId],
    queryFn: () => companiesApi.list({ created_by_org_id: orgId, limit: 500 }),
    enabled: !!orgId,
    staleTime: 60_000,
  });

  const filteredContacts = useMemo(() => {
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

  const filteredCompanies = useMemo(() => {
    if (!searchQuery) return companies;
    const q = searchQuery.toLowerCase();
    return companies.filter(c =>
      c.name.toLowerCase().includes(q) ||
      (c.industry && c.industry.toLowerCase().includes(q)) ||
      (c.headquarters && c.headquarters.toLowerCase().includes(q))
    );
  }, [companies, searchQuery]);

  const stageInfo = LIFECYCLE_STAGE_INFO;

  return (
    <div className="space-y-4">
      {/* Sub-tab toggle */}
      <div className="flex items-center gap-1 p-1 bg-muted rounded-lg w-fit">
        <button
          onClick={() => setCrmView('contacts')}
          className={`flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md transition-all ${
            crmView === 'contacts' ? 'bg-background text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'
          }`}
        >
          <Contact2 className="h-3.5 w-3.5" />
          Contacts
          <span className="text-xs text-muted-foreground">({contacts.length})</span>
        </button>
        <button
          onClick={() => setCrmView('companies')}
          className={`flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md transition-all ${
            crmView === 'companies' ? 'bg-background text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'
          }`}
        >
          <Building2 className="h-3.5 w-3.5" />
          Companies
          <span className="text-xs text-muted-foreground">({companies.length})</span>
        </button>
      </div>

      {/* Search + filter bar */}
      <div className="flex items-center gap-3">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder={crmView === 'contacts' ? 'Search contacts...' : 'Search companies...'}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9"
          />
        </div>
        {crmView === 'contacts' && (
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
        )}
        {(isLoading || companiesLoading) && (
          <span className="text-xs text-muted-foreground">Loading…</span>
        )}
        {crmView === 'contacts' ? (
          <Button size="sm" onClick={() => setShowAddContact(true)}>
            <Plus className="h-4 w-4 mr-1" />
            Add Contact
          </Button>
        ) : (
          <Button size="sm" onClick={() => setShowAddCompany(true)}>
            <Plus className="h-4 w-4 mr-1" />
            Add Company
          </Button>
        )}
      </div>

      {/* Add Contact Dialog */}
      <Dialog open={showAddContact} onOpenChange={setShowAddContact}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Add Contact</DialogTitle>
          </DialogHeader>
          <form onSubmit={handleCreateContact} className="space-y-3">
            <div className="grid grid-cols-2 gap-3">
              <div>
                <Label>First Name</Label>
                <Input value={contactForm.first_name} onChange={e => setContactForm(f => ({ ...f, first_name: e.target.value }))} placeholder="Jane" />
              </div>
              <div>
                <Label>Last Name</Label>
                <Input value={contactForm.last_name} onChange={e => setContactForm(f => ({ ...f, last_name: e.target.value }))} placeholder="Doe" />
              </div>
            </div>
            <div>
              <Label>Email</Label>
              <Input type="email" value={contactForm.email} onChange={e => setContactForm(f => ({ ...f, email: e.target.value }))} placeholder="jane@example.com" />
            </div>
            <div>
              <Label>Phone</Label>
              <Input value={contactForm.phone} onChange={e => setContactForm(f => ({ ...f, phone: e.target.value }))} placeholder="+1 555-0123" />
            </div>
            <div>
              <Label>Company</Label>
              <Input value={contactForm.company_name} onChange={e => setContactForm(f => ({ ...f, company_name: e.target.value }))} placeholder="Acme Corp" />
            </div>
            <div>
              <Label>Job Title</Label>
              <Input value={contactForm.job_title} onChange={e => setContactForm(f => ({ ...f, job_title: e.target.value }))} placeholder="CTO" />
            </div>
            <div>
              <Label>Lifecycle Stage</Label>
              <Select value={contactForm.lifecycle_stage} onValueChange={v => setContactForm(f => ({ ...f, lifecycle_stage: v }))}>
                <SelectTrigger><SelectValue /></SelectTrigger>
                <SelectContent>
                  {Object.entries(LIFECYCLE_STAGE_INFO).map(([key, info]) => (
                    <SelectItem key={key} value={key}>{info.label}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => setShowAddContact(false)}>Cancel</Button>
              <Button type="submit" disabled={createContactMutation.isPending}>
                {createContactMutation.isPending ? 'Creating...' : 'Create Contact'}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      {/* Add Company Dialog */}
      <Dialog open={showAddCompany} onOpenChange={setShowAddCompany}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Add Company</DialogTitle>
          </DialogHeader>
          <form onSubmit={handleCreateCompany} className="space-y-3">
            <div>
              <Label>Company Name *</Label>
              <Input value={companyForm.name} onChange={e => setCompanyForm(f => ({ ...f, name: e.target.value }))} placeholder="Acme Corp" />
            </div>
            <div>
              <Label>Website</Label>
              <Input value={companyForm.website} onChange={e => setCompanyForm(f => ({ ...f, website: e.target.value }))} placeholder="https://acme.com" />
            </div>
            <div>
              <Label>Industry</Label>
              <Input value={companyForm.industry} onChange={e => setCompanyForm(f => ({ ...f, industry: e.target.value }))} placeholder="Technology" />
            </div>
            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => setShowAddCompany(false)}>Cancel</Button>
              <Button type="submit" disabled={createCompanyMutation.isPending || !companyForm.name.trim()}>
                {createCompanyMutation.isPending ? 'Creating...' : 'Add Company'}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      {/* Contacts view */}
      {crmView === 'contacts' && (
        filteredContacts.length === 0 ? (
          <div className="text-center py-12 text-muted-foreground">
            <Contact2 className="h-8 w-8 mx-auto mb-2 opacity-40" />
            <p>{isLoading ? 'Loading contacts...' : 'No contacts found'}</p>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
            {filteredContacts.map((contact) => (
              <ContactCard key={contact.id} contact={contact} onClick={() => setSelectedContactId(contact.id)} />
            ))}
          </div>
        )
      )}

      {/* Companies view */}
      {crmView === 'companies' && (
        filteredCompanies.length === 0 ? (
          <div className="text-center py-12 text-muted-foreground">
            <Building2 className="h-8 w-8 mx-auto mb-2 opacity-40" />
            <p>{companiesLoading ? 'Loading companies...' : 'No companies found'}</p>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
            {filteredCompanies.map(company => (
              <Link
                key={company.id}
                to={`/companies/${company.id}`}
                className="block p-4 rounded-lg border bg-card hover:border-primary/40 hover:shadow-sm transition-all group"
              >
                <div className="flex items-start gap-3">
                  {company.logo_url ? (
                    <img src={company.logo_url} alt="" className="w-9 h-9 rounded object-cover shrink-0" />
                  ) : (
                    <div className="w-9 h-9 rounded bg-muted flex items-center justify-center shrink-0">
                      <Building2 className="h-4 w-4 text-muted-foreground" />
                    </div>
                  )}
                  <div className="min-w-0 flex-1">
                    <p className="text-sm font-semibold truncate group-hover:text-primary transition-colors">{company.name}</p>
                    {company.industry && (
                      <p className="text-xs text-muted-foreground truncate">{company.industry}</p>
                    )}
                    {company.headquarters && (
                      <p className="text-xs text-muted-foreground flex items-center gap-1 mt-0.5">
                        <MapPin className="h-3 w-3 shrink-0" />{company.headquarters}
                      </p>
                    )}
                  </div>
                  {company.intelligence_status && company.intelligence_status !== 'idle' && (
                    <span className={`w-2 h-2 rounded-full shrink-0 mt-1 ${
                      company.intelligence_status === 'done' ? 'bg-green-500' :
                      company.intelligence_status === 'running' ? 'bg-blue-500 animate-pulse' :
                      'bg-yellow-500'
                    }`} />
                  )}
                </div>
                {company.intelligence_summary && (
                  <p className="text-xs text-muted-foreground mt-2 line-clamp-2">{company.intelligence_summary}</p>
                )}
                {company.website && (
                  <p className="text-xs text-primary/70 mt-1 truncate flex items-center gap-1">
                    <Globe className="h-3 w-3 shrink-0" />{company.website.replace(/^https?:\/\//, '')}
                  </p>
                )}
              </Link>
            ))}
          </div>
        )
      )}

      {/* Contact Detail Modal */}
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

// Preserved for future PersonRecord-based contact intelligence features:
// const CONTEXT_COLORS: Record<string, string> = {
//   client:  'bg-green-100 text-green-700',
//   vendor:  'bg-orange-100 text-orange-700',
//   partner: 'bg-purple-100 text-purple-700',
//   prospect:'bg-blue-100 text-blue-700',
//   contact: 'bg-gray-100 text-gray-600',
// };
// const RESEARCH_DEPTH_COLOR: Record<string, string> = {
//   shallow:  'bg-gray-100 text-gray-600',
//   moderate: 'bg-yellow-100 text-yellow-700',
//   deep:     'bg-green-100 text-green-700',
// };
// const INTEL_STATUS_DOT: Record<string, string> = {
//   queued:  'bg-yellow-400',
//   running: 'bg-blue-400 animate-pulse',
//   done:    'bg-green-400',
//   failed:  'bg-red-400',
// };

function ContactCard({ contact, onClick }: { contact: CrmContactRecord; onClick?: () => void }) {
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
      <div className="flex items-center gap-2 mt-3 flex-wrap">
        {stageInfo && (
          <Badge
            variant="secondary"
            className="text-[10px]"
            style={{ backgroundColor: stageInfo.color + '20', color: stageInfo.color }}
          >
            {stageInfo.label}
          </Badge>
        )}
        {contact.source && (
          <Badge variant="outline" className="text-[10px] capitalize">{contact.source}</Badge>
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
  contact: CrmContactRecord;
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
      queryClient.invalidateQueries({ queryKey: ['crm-contacts'] });
      setIsEditing(false);
      toast.success('Contact updated');
    },
    onError: () => toast.error('Failed to update contact'),
  });

  const deleteMutation = useMutation({
    mutationFn: () => crmApi.deleteContact(contact.id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['crm-contacts'] });
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

function ProjectRow({ project, folderName, depth = 0 }: { project: any; folderName?: string; depth?: number }) {
  const hasChildren = project.children && project.children.length > 0;
  const [expanded, setExpanded] = useState(false);

  return (
    <div>
      <div className="flex items-center gap-1">
        {hasChildren && (
          <button
            onClick={() => setExpanded(!expanded)}
            className="p-0.5 hover:bg-muted rounded shrink-0"
          >
            {expanded ? <ChevronDown className="h-3 w-3 text-muted-foreground" /> : <ChevronRight className="h-3 w-3 text-muted-foreground" />}
          </button>
        )}
        <Link
          to={`/projects/${project.id}`}
          className="flex items-center justify-between p-2 rounded-md hover:bg-muted transition-colors flex-1 min-w-0"
          style={hasChildren ? undefined : { marginLeft: depth > 0 ? '0' : '1.25rem' }}
        >
          <div className="flex items-center gap-2 min-w-0">
            <FolderOpen className="h-4 w-4 text-muted-foreground shrink-0" />
            <span className="text-sm font-medium truncate">{project.name}</span>
            {folderName && <Badge variant="secondary" className="text-xs shrink-0">{folderName}</Badge>}
            {hasChildren && <Badge variant="outline" className="text-[10px] shrink-0">{project.children.length}</Badge>}
          </div>
          <ExternalLink className="h-3 w-3 text-muted-foreground shrink-0" />
        </Link>
      </div>
      {hasChildren && expanded && (
        <div className="pl-4 border-l border-border/50 ml-3 mt-0.5 space-y-0.5">
          {project.children.map((child: any) => (
            <ProjectRow key={child.id} project={child} depth={depth + 1} />
          ))}
        </div>
      )}
    </div>
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
  const [folder, setFolder] = useState((editingSource as any)?.folder ?? '');

  // Reset form when dialog opens/closes or editingSource changes
  const resetForm = () => {
    setSelectedProject(editingSource?.project_id ?? '__none__');
    setSourceType(editingSource?.source_type ?? 'text');
    setDataType(editingSource?.data_type ?? 'conversation');
    setTitle(editingSource?.title ?? '');
    setDescription(editingSource?.description ?? '');
    setContent(editingSource?.content ?? '');
    setFolder((editingSource as any)?.folder ?? '');
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
        if (folder.trim()) formData.append('folder', folder.trim());
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
      } as any);
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
      } as any);
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

          <div className="space-y-2">
            <Label>Folder (optional)</Label>
            <Input
              placeholder="e.g. Meetings or Meetings/Google Meet"
              value={folder}
              onChange={(e) => setFolder(e.target.value)}
            />
            <p className="text-xs text-muted-foreground">Use / to create subfolders. Leave blank to file under "Unfiled".</p>
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
  const [selectedFolder, setSelectedFolder] = useState<string | null>(null);
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set(['Meetings', 'Documents']));
  const [search, setSearch] = useState('');

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

  // Build folder tree from sources
  const folderTree = useMemo(() => {
    const tree: Record<string, Record<string, number>> = {}; // root → { sub: count }
    sources.forEach(s => {
      const f = (s as any).folder || 'Unfiled';
      const parts = f.split('/');
      const root = parts[0];
      const sub = parts[1] || null;
      if (!tree[root]) tree[root] = {};
      if (sub) {
        tree[root][sub] = (tree[root][sub] || 0) + 1;
      } else {
        tree[root]['__self__'] = (tree[root]['__self__'] || 0) + 1;
      }
    });
    return tree;
  }, [sources]);

  const allFolderCount = sources.length;

  const sorted = useMemo(() => {
    let arr = [...sources];
    // Folder filter
    if (selectedFolder) {
      arr = arr.filter(s => {
        const f = (s as any).folder || 'Unfiled';
        return f === selectedFolder || f.startsWith(selectedFolder + '/');
      });
    }
    // Search filter
    if (search.trim()) {
      const q = search.toLowerCase();
      arr = arr.filter(s => s.title.toLowerCase().includes(q) || (s.description || '').toLowerCase().includes(q));
    }
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
  }, [sources, sortField, sortDir, selectedFolder, search]);

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

  const toggleFolder = (f: string) => {
    setExpandedFolders(prev => {
      const next = new Set(prev);
      if (next.has(f)) next.delete(f); else next.add(f);
      return next;
    });
  };

  const folderCount = (f: string) =>
    sources.filter(s => {
      const sf = (s as any).folder || 'Unfiled';
      return sf === f || sf.startsWith(f + '/');
    }).length;

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
      ) : sources.length === 0 ? (
        <div className="text-center py-12 text-muted-foreground">
          <Database className="h-8 w-8 mx-auto mb-2 opacity-40" />
          <p>No data sources yet</p>
          <Button variant="outline" size="sm" className="mt-4 gap-1" onClick={() => setAddOpen(true)}>
            <Plus className="h-3.5 w-3.5" />
            Add your first data source
          </Button>
        </div>
      ) : (
        <div className="flex gap-4">
          {/* Folder sidebar */}
          <div className="w-48 shrink-0">
            <div className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2 px-1">Folders</div>
            <nav className="space-y-0.5">
              {/* All */}
              <button
                onClick={() => setSelectedFolder(null)}
                className={`w-full flex items-center justify-between px-2 py-1.5 rounded text-sm hover:bg-muted/60 transition-colors ${!selectedFolder ? 'bg-muted font-medium' : ''}`}
              >
                <span className="flex items-center gap-1.5"><FolderOpen className="h-3.5 w-3.5 text-muted-foreground" />All</span>
                <span className="text-xs text-muted-foreground">{allFolderCount}</span>
              </button>
              {/* Root folders */}
              {Object.entries(folderTree).sort(([a],[b]) => a.localeCompare(b)).map(([root, subs]) => {
                const subKeys = Object.keys(subs).filter(k => k !== '__self__');
                const hasChildren = subKeys.length > 0;
                const isExpanded = expandedFolders.has(root);
                const cnt = folderCount(root);
                return (
                  <div key={root}>
                    <button
                      onClick={() => { if (hasChildren) toggleFolder(root); setSelectedFolder(root); }}
                      className={`w-full flex items-center justify-between px-2 py-1.5 rounded text-sm hover:bg-muted/60 transition-colors ${selectedFolder === root ? 'bg-muted font-medium' : ''}`}
                    >
                      <span className="flex items-center gap-1.5 min-w-0">
                        {hasChildren
                          ? (isExpanded ? <ChevronDown className="h-3 w-3 shrink-0 text-muted-foreground" /> : <ChevronRight className="h-3 w-3 shrink-0 text-muted-foreground" />)
                          : <span className="w-3 shrink-0" />}
                        <FolderOpen className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                        <span className="truncate">{root}</span>
                      </span>
                      <span className="text-xs text-muted-foreground shrink-0 ml-1">{cnt}</span>
                    </button>
                    {hasChildren && isExpanded && (
                      <div className="pl-4 space-y-0.5">
                        {subKeys.sort().map(sub => {
                          const fullPath = `${root}/${sub}`;
                          const subCnt = folderCount(fullPath);
                          return (
                            <button
                              key={sub}
                              onClick={() => setSelectedFolder(fullPath)}
                              className={`w-full flex items-center justify-between px-2 py-1 rounded text-xs hover:bg-muted/60 transition-colors ${selectedFolder === fullPath ? 'bg-muted font-medium' : ''}`}
                            >
                              <span className="flex items-center gap-1.5 min-w-0">
                                <FolderOpen className="h-3 w-3 shrink-0 text-muted-foreground" />
                                <span className="truncate">{sub}</span>
                              </span>
                              <span className="text-xs text-muted-foreground">{subCnt}</span>
                            </button>
                          );
                        })}
                      </div>
                    )}
                  </div>
                );
              })}
            </nav>
          </div>

          {/* Main content */}
          <div className="flex-1 min-w-0 space-y-3">
            {/* Search */}
            <div className="relative">
              <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
              <input
                type="text"
                placeholder="Search sources…"
                value={search}
                onChange={e => setSearch(e.target.value)}
                className="w-full pl-8 pr-3 py-1.5 text-sm border rounded bg-background focus:outline-none focus:ring-1 focus:ring-ring"
              />
            </div>
            {sorted.length === 0 ? (
              <div className="text-center py-8 text-sm text-muted-foreground">No sources in this folder.</div>
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
          </div>
        </div>
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
    queryKey: ['recentArtifacts'],
    queryFn: () => workflowsApi.listRecentArtifacts(),
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
  // Org-level knowledge (brand research, intelligence)
  const { data: orgKnowledge } = useQuery({
    queryKey: ['orgKnowledge', orgId],
    queryFn: () => organizationsApi.getKnowledge(orgId),
    staleTime: 60_000,
    enabled: !!orgId,
  });

  const knowledgeQueries = useQueries({
    queries: projectEntries.map((entry) => ({
      queryKey: ['projectKnowledge', entry.id],
      queryFn: () => knowledgeApi.getProjectKnowledge(entry.id),
      staleTime: 60_000,
      enabled: projectEntries.length > 0,
    })),
  });

  const isLoading = knowledgeQueries.some(q => q.isLoading);
  const loadedCount = knowledgeQueries.filter(q => q.isSuccess).length;

  const aggregated = useMemo(() => {
    let totalSources = 0;
    let staleSources = 0;
    let totalCoverage = 0;
    let coverageCount = 0;
    const sourcesByProject: { projectName: string; projectId: string; data: ProjectKnowledgeResponse }[] = [];

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
    });

    const orgEntryCount = orgKnowledge?.knowledge_entries?.length ?? 0;
    totalSources += orgEntryCount;
    const avgCompleteness = coverageCount > 0 ? Math.round((totalCoverage / coverageCount) * 100) : 0;
    return { totalSources, staleSources, avgCompleteness, sourcesByProject };
  }, [knowledgeQueries, projectEntries, orgKnowledge]);

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
      return <TopologyView projectEntries={projectEntries} aggregated={aggregated as any} isLoading={isLoading} loadedCount={loadedCount} />;
    }

    // Generic fallback for other knowledge source types (e.g. conversations)
    const typeKey =
      view === 'conversations' ? 'conversation'
      : null;
    const items = typeKey ? ((aggregated as any).byType?.[typeKey] || []) : [];
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
            {items.map(({ source, projectName, projectId }: { source: any; projectName: string; projectId: string }) => (
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
              <span className="text-2xl font-bold">{aggregated.totalSources}</span>
              <span className="text-sm text-muted-foreground">across {projectEntries.length} projects</span>
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

      {isLoading && (
        <p className="text-xs text-muted-foreground">Loading {loadedCount}/{projectEntries.length} projects...</p>
      )}

      {orgKnowledge?.knowledge_entries && orgKnowledge.knowledge_entries.length > 0 && (
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-3">
            <CardTitle className="text-base flex items-center gap-2">
              <Brain className="h-4 w-4 text-[hsl(var(--brand))]" />
              Organization Intelligence
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex flex-wrap gap-2">
              {orgKnowledge.knowledge_entries.map((entry: any) => (
                <Badge key={entry.source_id} variant="secondary" className="text-xs gap-1 cursor-default" title={entry.source_summary || entry.source_title}>
                  <Brain className="h-3 w-3" />
                  {entry.source_title}
                  {entry.coverage_score != null && (
                    <span className="opacity-60 ml-1">{Math.round(entry.coverage_score * 100)}%</span>
                  )}
                </Badge>
              ))}
            </div>
          </CardContent>
        </Card>
      )}

      {aggregated.sourcesByProject.length === 0 && !isLoading && !(orgKnowledge?.knowledge_entries?.length) ? (
        <div className="text-center py-12 text-muted-foreground">
          <BookOpen className="h-8 w-8 mx-auto mb-2 opacity-40" />
          <p>No knowledge sources indexed yet</p>
          <p className="text-xs mt-1">Connect a data source in one of your projects to start building intelligence.</p>
        </div>
      ) : aggregated.sourcesByProject.length > 0 ? (
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
      ) : null}
    </div>
  );
}

// ── Pulse Section ─────────────────────────────────────────────────────────────

function PulseSection({ projectEntries }: { projectEntries: { id: string; name: string }[] }) {
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
      q.data.forEach((a: any) => allAlerts.push({ ...a, _projectName: entry.name }));
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
                              {formatDate(alert.triggered_at || alert.created_at)}
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
                          {formatDate(item.collected_at || item.created_at)}
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

// ── Intelligence Tab (Social + Knowledge + Pulse combined) ────────────────────
// Org-level intelligence view: system automations, pipeline blueprints, data sources, artifacts.
// The "Workflows" sub-view here shows org-scoped automations and blueprints, NOT user-created
// extraction pipelines (those live in /workflows "My Workflows" page).
// TODO: Surface org-specific staging records here so users can review CRM imports
// without switching to the global /workflows page.

function IntelligenceTab({ projectEntries, orgId }: { projectEntries: { id: string; name: string }[]; orgId: string }) {
  const [searchParams] = useSearchParams();
  const location = useLocation();

  // Derive view from pathname (sidebar links) or query param (tab clicks)
  const pathSegment = location.pathname.match(/\/intelligence\/([^/]+)/)?.[1];
  const PATH_TO_VIEW: Record<string, string> = {
    'data-sources': 'datasources',
    'artifacts': 'artifacts',
    'workflows': 'workflows',
    'pulse': 'pulse',
    'topology': 'topology',
  };
  const viewFromUrl = searchParams.get('view') || (pathSegment ? PATH_TO_VIEW[pathSegment] : null) || 'overview';

  const views = [
    { key: 'overview',    label: 'Overview',      icon: Brain },
    { key: 'datasources', label: 'Data Sources',  icon: Database },
    { key: 'artifacts',   label: 'Artifacts',     icon: FileText },
    { key: 'workflows',   label: 'Workflows',     icon: GitBranch },
    { key: 'pulse',       label: 'Pulse',         icon: Radio },
    { key: 'topology',    label: 'Topology',      icon: Network },
  ];

  const navigate = useNavigate();
  const VIEW_PATHS: Record<string, string> = {
    overview: '',
    datasources: '/data-sources',
    artifacts: '/artifacts',
    workflows: '/workflows',
    pulse: '/pulse',
    topology: '/topology',
  };

  const setView = (view: string) => {
    const basePath = `/organizations/${orgId}/intelligence`;
    const viewPath = VIEW_PATHS[view] ?? '';
    navigate(`${basePath}${viewPath}`, { replace: true });
  };

  return (
    <div className="space-y-6">
      <div className="flex gap-1 p-1 bg-muted/50 rounded-lg w-fit flex-wrap">
        {views.map(({ key, label, icon: Icon }) => (
          <button
            key={key}
            onClick={() => setView(key)}
            className={`flex items-center gap-1.5 px-4 py-1.5 text-sm font-medium rounded-md transition-all ${
              viewFromUrl === key
                ? 'bg-background text-foreground shadow-sm'
                : 'text-muted-foreground hover:text-foreground'
            }`}
          >
            <Icon className="h-3.5 w-3.5" />
            {label}
          </button>
        ))}
      </div>

      {viewFromUrl === 'overview'    && <KnowledgeTab orgId={orgId} projectEntries={projectEntries} />}
      {viewFromUrl === 'datasources' && (
        <div className="space-y-6">
          <DataSourcesIntelView orgId={orgId} projectEntries={projectEntries} />
          <DataSourcesView orgId={orgId} projectEntries={projectEntries} />
        </div>
      )}
      {viewFromUrl === 'artifacts'   && <ArtifactsIntelView projectEntries={projectEntries} />}
      {viewFromUrl === 'workflows'   && <WorkflowsIntelView orgId={orgId} />}
      {viewFromUrl === 'pulse'       && <PulseSection projectEntries={projectEntries} />}
      {viewFromUrl === 'topology'    && <TopologyIntelView projectEntries={projectEntries} />}
    </div>
  );
}

function WorkflowsIntelView({ orgId }: { orgId: string }) {
  const { data: automations = [] } = useQuery({
    queryKey: ['system-automations'],
    queryFn: async () => {
      const r = await fetch('/api/automations', { credentials: 'include' });
      const d = await r.json();
      return d.data || [];
    },
    staleTime: 5 * 60_000,
  });

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium text-muted-foreground">Workflows & Automations</h3>
        <Link
          to="/workflows"
          className="inline-flex items-center gap-1.5 text-sm text-primary hover:underline"
        >
          <GitBranch className="h-3.5 w-3.5" />
          Live Workflow Monitor
          <ExternalLink className="h-3 w-3" />
        </Link>
      </div>

      {/* System Automations */}
      {automations.length > 0 && (
        <div>
          <p className="text-xs font-medium text-muted-foreground uppercase tracking-wider mb-2">System Automations ({automations.length} active)</p>
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
      )}

      {/* Pipeline Blueprints — Conference, Editron, and API templates */}
      <div>
        <p className="text-xs font-medium text-muted-foreground uppercase tracking-wider mb-3">Pipeline Blueprints</p>
        <LegacyPipelinesView orgId={orgId} />
      </div>
    </div>
  );
}

function TopologyIntelView({ projectEntries }: { projectEntries: { id: string; name: string }[] }) {
  const knowledgeQueries = useQueries({
    queries: projectEntries.map((entry) => ({
      queryKey: ['projectKnowledge', entry.id],
      queryFn: () => knowledgeApi.getProjectKnowledge(entry.id),
      staleTime: 60_000,
      enabled: projectEntries.length > 0,
    })),
  });

  const topologyItems = useMemo(() => {
    const all: { title: string; summary?: string; coverage?: number; projectName: string; projectId: string; isStale: boolean }[] = [];
    knowledgeQueries.forEach((q, i) => {
      if (!q.data) return;
      const entry = projectEntries[i];
      ((q.data as any).sources_by_type?.topology_snapshot || []).forEach((src: any) => {
        all.push({ title: src.source_title, summary: src.source_summary, coverage: src.coverage_score, projectName: entry.name, projectId: entry.id, isStale: src.is_stale || false });
      });
    });
    return all;
  }, [knowledgeQueries, projectEntries]);

  const isLoading = knowledgeQueries.some(q => q.isLoading);

  if (isLoading) return (
    <div className="flex items-center gap-2 text-muted-foreground text-sm py-4">
      <Loader2 className="h-4 w-4 animate-spin" />Loading topology data...
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
      <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
        {topologyItems.map((t, i) => (
          <Card key={i} className={`bg-card/80 ${t.isStale ? 'border-yellow-500/30' : 'border-border/50'}`}>
            <CardContent className="pt-4 pb-4">
              <div className="flex items-start gap-2">
                <Network className="h-4 w-4 text-[hsl(var(--info))] shrink-0 mt-0.5" />
                <div className="min-w-0 flex-1">
                  <p className="text-sm font-medium truncate">{t.title}</p>
                  {t.summary && <p className="text-xs text-muted-foreground mt-0.5 line-clamp-2">{t.summary}</p>}
                  <div className="flex items-center justify-between mt-1.5">
                    <Link to={`/projects/${t.projectId}`} className="text-[10px] text-muted-foreground hover:text-foreground">{t.projectName}</Link>
                    {t.coverage != null && <span className="text-[10px] text-muted-foreground">{Math.round(t.coverage * 100)}% coverage</span>}
                  </div>
                  {t.isStale && <Badge variant="outline" className="text-[9px] mt-1 text-yellow-600 border-yellow-600">Stale</Badge>}
                </div>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}


// ── Social Tab ────────────────────────────────────────────────────────────────

const PLATFORM_ICONS: Record<string, React.ComponentType<{ className?: string }>> = {
  linkedin: Linkedin,
  instagram: Instagram,
  twitter: Twitter,
  facebook: Facebook,
  youtube: Youtube,
  tiktok: Globe,
  threads: Globe,
  bluesky: Globe,
  pinterest: Globe,
};

const PLATFORM_COLORS: Record<string, string> = {
  linkedin: 'text-[#0A66C2]',
  instagram: 'text-[#E4405F]',
  twitter: 'text-foreground',
  facebook: 'text-[#1877F2]',
  youtube: 'text-[#FF0000]',
  tiktok: 'text-foreground',
  threads: 'text-foreground',
  bluesky: 'text-sky-500',
  pinterest: 'text-[#E60023]',
};

const PLATFORM_BG: Record<string, string> = {
  linkedin: 'bg-[#0A66C2]/10',
  instagram: 'bg-[#E4405F]/10',
  twitter: 'bg-foreground/10',
  facebook: 'bg-[#1877F2]/10',
  youtube: 'bg-[#FF0000]/10',
  tiktok: 'bg-muted',
  bluesky: 'bg-sky-500/10',
  pinterest: 'bg-[#E60023]/10',
};

const STATUS_COLORS: Record<string, string> = {
  draft: 'text-muted-foreground border-border',
  pending_review: 'text-amber-600 border-amber-300 bg-amber-50 dark:bg-amber-950/20',
  approved: 'text-blue-600 border-blue-300 bg-blue-50 dark:bg-blue-950/20',
  scheduled: 'text-purple-600 border-purple-300 bg-purple-50 dark:bg-purple-950/20',
  published: 'text-green-600 border-green-300 bg-green-50 dark:bg-green-950/20',
  failed: 'text-destructive border-destructive/30 bg-destructive/10',
  cancelled: 'text-muted-foreground border-border',
};

const SENTIMENT_COLORS: Record<string, string> = {
  positive: 'text-green-600',
  negative: 'text-destructive',
  neutral: 'text-muted-foreground',
  unknown: 'text-muted-foreground',
};

const PRIORITY_COLORS: Record<string, string> = {
  urgent: 'text-destructive',
  high: 'text-amber-600',
  normal: 'text-muted-foreground',
  low: 'text-muted-foreground/60',
};

// ── Social: Overview ──────────────────────────────────────────────────────────

function SocialOverviewView({
  projectEntries,
  orgId,
  onSwitchView,
}: {
  projectEntries: { id: string; name: string }[];
  orgId: string;
  onSwitchView: (v: string) => void;
}) {
  const { data: brandProfile } = useQuery({
    queryKey: ['brandProfile', orgId],
    queryFn: () => organizationsApi.getBrandProfile(orgId),
    staleTime: 300_000,
    enabled: !!orgId,
  });

  const accountQueries = useQueries({
    queries: projectEntries.map(e => ({
      queryKey: ['social-accounts', e.id],
      queryFn: () => socialApi.listAccounts(e.id),
      staleTime: 60_000,
    })),
  });

  const postQueries = useQueries({
    queries: projectEntries.map(e => ({
      queryKey: ['social-posts', e.id],
      queryFn: () => socialApi.listPostsFiltered({ projectId: e.id, limit: 20 }),
      staleTime: 60_000,
    })),
  });

  const mentionQueries = useQueries({
    queries: projectEntries.map(e => ({
      queryKey: ['social-mentions-ov', e.id],
      queryFn: () => socialApi.listMentions(e.id, { limit: 10 }),
      staleTime: 60_000,
    })),
  });

  const agg = useMemo(() => {
    const allAccounts: (SocialAccountRecord & { _project: string })[] = [];
    const allPosts: (SocialPostRecord & { _project: string })[] = [];
    const allMentions: (SocialMentionRecord & { _project: string })[] = [];

    accountQueries.forEach((q, i) => {
      q.data?.forEach(a => allAccounts.push({ ...a, _project: projectEntries[i].name }));
    });
    postQueries.forEach((q, i) => {
      q.data?.forEach(p => allPosts.push({ ...p, _project: projectEntries[i].name }));
    });
    mentionQueries.forEach((q, i) => {
      q.data?.forEach(m => allMentions.push({ ...m, _project: projectEntries[i].name }));
    });

    const totalFollowers = allAccounts.reduce((s, a) => s + (a.follower_count || 0), 0);
    const scheduled = allPosts.filter(p => p.status === 'scheduled').length;
    const published = allPosts.filter(p => p.status === 'published').length;
    const unread = allMentions.filter(m => m.status === 'unread').length;
    const urgent = allMentions.filter(m => m.priority === 'urgent' || m.priority === 'high').length;

    const recentPosts = [...allPosts]
      .filter(p => p.status === 'published' || p.status === 'scheduled')
      .sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime())
      .slice(0, 6);

    const recentMentions = [...allMentions]
      .sort((a, b) => new Date(b.received_at).getTime() - new Date(a.received_at).getTime())
      .slice(0, 5);

    // Platform breakdown
    const byPlatform: Record<string, { accounts: number; followers: number }> = {};
    allAccounts.forEach(a => {
      if (!byPlatform[a.platform]) byPlatform[a.platform] = { accounts: 0, followers: 0 };
      byPlatform[a.platform].accounts++;
      byPlatform[a.platform].followers += a.follower_count || 0;
    });

    // Brand profile platforms (if no connected accounts)
    const brandHandles: { platform: string; handle: string }[] = [];
    if (allAccounts.length === 0 && brandProfile) {
      if (brandProfile.socialInstagram) brandHandles.push({ platform: 'instagram', handle: brandProfile.socialInstagram });
      if (brandProfile.socialLinkedin) brandHandles.push({ platform: 'linkedin', handle: brandProfile.socialLinkedin });
      if (brandProfile.socialTwitter) brandHandles.push({ platform: 'twitter', handle: brandProfile.socialTwitter });
      if (brandProfile.socialTiktok) brandHandles.push({ platform: 'tiktok', handle: brandProfile.socialTiktok });
      if (brandProfile.socialYoutube) brandHandles.push({ platform: 'youtube', handle: brandProfile.socialYoutube });
    }

    return { totalFollowers, scheduled, published, unread, urgent, recentPosts, recentMentions, byPlatform, brandHandles, totalAccounts: allAccounts.length };
  }, [accountQueries, postQueries, mentionQueries, projectEntries, brandProfile]);

  const fmtFollowers = (n: number) => n >= 1_000_000 ? `${(n/1_000_000).toFixed(1)}M` : n >= 1000 ? `${(n/1000).toFixed(1)}k` : n.toString();

  return (
    <div className="space-y-6">
      {/* KPI row */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        {[
          { label: 'Total Followers', value: fmtFollowers(agg.totalFollowers), icon: Users, color: 'text-blue-500', action: () => onSwitchView('accounts') },
          { label: 'Scheduled Posts', value: agg.scheduled, icon: CalendarDays, color: 'text-purple-500', action: () => onSwitchView('content') },
          { label: 'Unread Mentions', value: agg.unread, icon: Inbox, color: agg.unread > 0 ? 'text-amber-500' : 'text-muted-foreground', action: () => onSwitchView('inbox') },
          { label: 'Published', value: agg.published, icon: CheckCircle, color: 'text-green-500', action: () => onSwitchView('content') },
        ].map(({ label, value, icon: Icon, color, action }) => (
          <Card key={label} className="bg-card/80 backdrop-blur-sm border-border/50 cursor-pointer hover:border-border transition-colors" onClick={action}>
            <CardContent className="pt-5">
              <div className="flex items-start justify-between">
                <div>
                  <p className="text-xs text-muted-foreground mb-1">{label}</p>
                  <p className="text-2xl font-bold">{value}</p>
                </div>
                <Icon className={`h-5 w-5 ${color} mt-0.5`} />
              </div>
            </CardContent>
          </Card>
        ))}
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Platform breakdown */}
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium flex items-center gap-2">
              <Share2 className="h-4 w-4 text-pink-500" />
              Platforms
            </CardTitle>
          </CardHeader>
          <CardContent>
            {Object.keys(agg.byPlatform).length === 0 && agg.brandHandles.length === 0 ? (
              <div className="text-center py-6 text-muted-foreground">
                <Share2 className="h-6 w-6 mx-auto mb-2 opacity-40" />
                <p className="text-xs">No accounts connected</p>
                <button onClick={() => onSwitchView('accounts')} className="mt-2 text-xs text-primary hover:underline">Connect accounts →</button>
              </div>
            ) : agg.brandHandles.length > 0 ? (
              <div className="space-y-2">
                <p className="text-[10px] text-muted-foreground mb-2">From brand profile</p>
                {agg.brandHandles.map(({ platform, handle }) => {
                  const Icon = PLATFORM_ICONS[platform] || Globe;
                  return (
                    <div key={platform} className="flex items-center gap-2.5 p-2 rounded-lg bg-muted/40">
                      <Icon className={`h-4 w-4 shrink-0 ${PLATFORM_COLORS[platform] || 'text-muted-foreground'}`} />
                      <div className="min-w-0 flex-1">
                        <p className="text-xs font-medium truncate">{handle}</p>
                        <p className="text-[10px] text-muted-foreground capitalize">{platform}</p>
                      </div>
                    </div>
                  );
                })}
              </div>
            ) : (
              <div className="space-y-2">
                {Object.entries(agg.byPlatform).map(([platform, data]) => {
                  const Icon = PLATFORM_ICONS[platform] || Globe;
                  return (
                    <div key={platform} className={`flex items-center gap-2.5 p-2 rounded-lg ${PLATFORM_BG[platform] || 'bg-muted/40'}`}>
                      <Icon className={`h-4 w-4 shrink-0 ${PLATFORM_COLORS[platform] || 'text-muted-foreground'}`} />
                      <div className="min-w-0 flex-1">
                        <p className="text-xs font-medium capitalize">{platform}</p>
                        <p className="text-[10px] text-muted-foreground">{fmtFollowers(data.followers)} followers</p>
                      </div>
                      <span className="text-[10px] text-muted-foreground">{data.accounts} acct{data.accounts !== 1 ? 's' : ''}</span>
                    </div>
                  );
                })}
              </div>
            )}
          </CardContent>
        </Card>

        {/* Recent posts */}
        <Card className="bg-card/80 backdrop-blur-sm border-border/50 lg:col-span-2">
          <CardHeader className="pb-3">
            <div className="flex items-center justify-between">
              <CardTitle className="text-sm font-medium flex items-center gap-2">
                <FileText className="h-4 w-4 text-purple-500" />
                Recent Content
              </CardTitle>
              <button onClick={() => onSwitchView('content')} className="text-xs text-primary hover:underline">View all →</button>
            </div>
          </CardHeader>
          <CardContent>
            {agg.recentPosts.length === 0 ? (
              <div className="text-center py-6 text-muted-foreground">
                <FileText className="h-6 w-6 mx-auto mb-2 opacity-40" />
                <p className="text-xs">No posts yet</p>
                <button onClick={() => onSwitchView('content')} className="mt-2 text-xs text-primary hover:underline">Create first post →</button>
              </div>
            ) : (
              <div className="space-y-2">
                {agg.recentPosts.map(post => {
                  const platforms: string[] = (() => { try { return JSON.parse(post.platforms); } catch { return [post.platforms]; } })();
                  return (
                    <div key={post.id} className="flex items-start gap-3 p-2.5 rounded-lg border border-border/40 hover:bg-muted/30 transition-colors">
                      <div className="flex gap-1 mt-0.5">
                        {platforms.slice(0, 3).map(p => {
                          const Icon = PLATFORM_ICONS[p] || Globe;
                          return <Icon key={p} className={`h-3.5 w-3.5 ${PLATFORM_COLORS[p] || 'text-muted-foreground'}`} />;
                        })}
                      </div>
                      <div className="min-w-0 flex-1">
                        <p className="text-xs line-clamp-1 font-medium">{post.caption || '(no caption)'}</p>
                        <div className="flex items-center gap-2 mt-0.5">
                          <span className={`text-[10px] px-1.5 py-0.5 rounded border ${STATUS_COLORS[post.status] || ''}`}>{post.status}</span>
                          {post.scheduled_for && <span className="text-[10px] text-muted-foreground">{formatDate(post.scheduled_for)}</span>}
                          <span className="text-[10px] text-muted-foreground">{post._project}</span>
                        </div>
                      </div>
                      {(post.likes > 0 || post.impressions > 0) && (
                        <div className="text-right shrink-0">
                          {post.impressions > 0 && <p className="text-[10px] text-muted-foreground">{fmtFollowers(post.impressions)} views</p>}
                          {post.likes > 0 && <p className="text-[10px] text-muted-foreground">{post.likes} ♥</p>}
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      {/* Recent inbox */}
      {agg.recentMentions.length > 0 && (
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-3">
            <div className="flex items-center justify-between">
              <CardTitle className="text-sm font-medium flex items-center gap-2">
                <Inbox className="h-4 w-4 text-amber-500" />
                Recent Inbox
                {agg.unread > 0 && <Badge variant="default" className="text-[10px]">{agg.unread} unread</Badge>}
                {agg.urgent > 0 && <Badge variant="destructive" className="text-[10px]">{agg.urgent} urgent</Badge>}
              </CardTitle>
              <button onClick={() => onSwitchView('inbox')} className="text-xs text-primary hover:underline">View all →</button>
            </div>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-2">
              {agg.recentMentions.map(m => {
                const Icon = PLATFORM_ICONS[m.platform] || Globe;
                return (
                  <div key={m.id} className={`p-2.5 rounded-lg border border-border/40 ${m.status === 'unread' ? 'bg-accent/20' : ''}`}>
                    <div className="flex items-center gap-2 mb-1">
                      <Icon className={`h-3.5 w-3.5 shrink-0 ${PLATFORM_COLORS[m.platform] || 'text-muted-foreground'}`} />
                      <span className="text-xs font-medium truncate flex-1">{m.author_display_name || m.author_username || 'Unknown'}</span>
                      {m.status === 'unread' && <span className="h-1.5 w-1.5 rounded-full bg-blue-500 shrink-0" />}
                    </div>
                    {m.content && <p className="text-[11px] text-muted-foreground line-clamp-2">{m.content}</p>}
                    <div className="flex items-center gap-1.5 mt-1">
                      <Badge variant="outline" className="text-[9px]">{m.mention_type}</Badge>
                      {m.sentiment && m.sentiment !== 'unknown' && (
                        <span className={`text-[9px] ${SENTIMENT_COLORS[m.sentiment]}`}>{m.sentiment}</span>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}

// ── Social: Accounts ──────────────────────────────────────────────────────────

function SocialAccountsView({
  projectEntries,
  orgId,
}: {
  projectEntries: { id: string; name: string }[];
  orgId: string;
}) {
  const { data: brandProfile } = useQuery({
    queryKey: ['brandProfile', orgId],
    queryFn: () => organizationsApi.getBrandProfile(orgId),
    staleTime: 300_000,
    enabled: !!orgId,
  });

  const accountQueries = useQueries({
    queries: projectEntries.map(e => ({
      queryKey: ['social-accounts', e.id],
      queryFn: () => socialApi.listAccounts(e.id),
      staleTime: 60_000,
    })),
  });

  const allAccounts = useMemo(() => {
    const list: (SocialAccountRecord & { _project: string; _projectId: string })[] = [];
    accountQueries.forEach((q, i) => {
      q.data?.forEach(a => list.push({ ...a, _project: projectEntries[i].name, _projectId: projectEntries[i].id }));
    });
    return list.sort((a, b) => a.platform.localeCompare(b.platform));
  }, [accountQueries, projectEntries]);

  const queryClient = useQueryClient();
  const deleteMut = useMutation({
    mutationFn: (id: string) => socialApi.deleteAccount(id),
    onSuccess: () => {
      accountQueries.forEach((_, i) => {
        queryClient.invalidateQueries({ queryKey: ['social-accounts', projectEntries[i].id] });
      });
    },
  });

  const fmtFollowers = (n?: number | null) => !n ? '—' : n >= 1_000_000 ? `${(n/1_000_000).toFixed(1)}M` : n >= 1000 ? `${(n/1000).toFixed(1)}k` : n.toString();

  const brandHandles: { platform: string; handle: string }[] = [];
  if (brandProfile) {
    if (brandProfile.socialInstagram) brandHandles.push({ platform: 'instagram', handle: brandProfile.socialInstagram });
    if (brandProfile.socialLinkedin) brandHandles.push({ platform: 'linkedin', handle: brandProfile.socialLinkedin });
    if (brandProfile.socialTwitter) brandHandles.push({ platform: 'twitter', handle: brandProfile.socialTwitter });
    if (brandProfile.socialTiktok) brandHandles.push({ platform: 'tiktok', handle: brandProfile.socialTiktok });
    if (brandProfile.socialYoutube) brandHandles.push({ platform: 'youtube', handle: brandProfile.socialYoutube });
    if (brandProfile.socialFacebook) brandHandles.push({ platform: 'facebook', handle: brandProfile.socialFacebook });
  }

  return (
    <div className="space-y-6">
      {/* Brand profile handles */}
      {brandHandles.length > 0 && (
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium flex items-center gap-2">
              <Share2 className="h-4 w-4 text-pink-500" />
              Brand Profile Handles
              <Badge variant="outline" className="text-[10px] ml-1">From brand research</Badge>
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
              {brandHandles.map(({ platform, handle }) => {
                const Icon = PLATFORM_ICONS[platform] || Globe;
                const isConnected = allAccounts.some(a => a.platform === platform);
                return (
                  <div key={platform} className={`flex items-center gap-3 p-3 rounded-lg border ${isConnected ? 'border-green-300 bg-green-50 dark:bg-green-950/20' : 'border-border/50 bg-muted/30'}`}>
                    <div className={`h-9 w-9 rounded-full flex items-center justify-center ${PLATFORM_BG[platform] || 'bg-muted'}`}>
                      <Icon className={`h-4.5 w-4.5 ${PLATFORM_COLORS[platform] || 'text-muted-foreground'}`} />
                    </div>
                    <div className="min-w-0 flex-1">
                      <p className="text-xs font-medium truncate">{handle}</p>
                      <p className="text-[10px] text-muted-foreground capitalize">{platform}</p>
                    </div>
                    {isConnected
                      ? <Badge variant="outline" className="text-[9px] text-green-600 border-green-300">connected</Badge>
                      : <Badge variant="outline" className="text-[9px]">not connected</Badge>
                    }
                  </div>
                );
              })}
            </div>
          </CardContent>
        </Card>
      )}

      {/* Connected accounts */}
      <Card className="bg-card/80 backdrop-blur-sm border-border/50">
        <CardHeader className="pb-3">
          <div className="flex items-center justify-between">
            <CardTitle className="text-sm font-medium flex items-center gap-2">
              <CheckCircle className="h-4 w-4 text-green-500" />
              Connected Accounts
              <Badge variant="secondary" className="text-[10px]">{allAccounts.length}</Badge>
            </CardTitle>
          </div>
        </CardHeader>
        <CardContent>
          {allAccounts.length === 0 ? (
            <div className="text-center py-10 text-muted-foreground">
              <Share2 className="h-8 w-8 mx-auto mb-3 opacity-40" />
              <p className="text-sm font-medium mb-1">No accounts connected yet</p>
              <p className="text-xs">Connect social accounts to your projects to start tracking and publishing</p>
            </div>
          ) : (
            <div className="space-y-3">
              {allAccounts.map(account => {
                const Icon = PLATFORM_ICONS[account.platform] || Globe;
                return (
                  <div key={account.id} className="flex items-center gap-4 p-3 rounded-lg border border-border/50 hover:bg-muted/30 transition-colors">
                    <div className={`h-10 w-10 rounded-full flex items-center justify-center shrink-0 ${PLATFORM_BG[account.platform] || 'bg-muted'}`}>
                      {account.avatar_url
                        ? <img src={account.avatar_url} className="h-10 w-10 rounded-full object-cover" alt="" />
                        : <Icon className={`h-5 w-5 ${PLATFORM_COLORS[account.platform] || 'text-muted-foreground'}`} />
                      }
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-2">
                        <p className="text-sm font-medium truncate">{account.display_name || account.username || account.platform}</p>
                        <Badge variant={account.status === 'active' ? 'default' : account.status === 'error' ? 'destructive' : 'secondary'} className="text-[10px]">
                          {account.status}
                        </Badge>
                      </div>
                      <div className="flex items-center gap-3 text-xs text-muted-foreground mt-0.5">
                        <span className="capitalize">{account.platform}</span>
                        <span>·</span>
                        <span>{fmtFollowers(account.follower_count)} followers</span>
                        {account.post_count != null && <><span>·</span><span>{account.post_count} posts</span></>}
                        <span>·</span>
                        <span>{account._project}</span>
                      </div>
                    </div>
                    <div className="flex items-center gap-1 shrink-0">
                      {account.profile_url && (
                        <a href={account.profile_url} target="_blank" rel="noopener noreferrer"
                          className="p-1.5 rounded hover:bg-muted transition-colors" title="View profile">
                          <ExternalLink className="h-3.5 w-3.5 text-muted-foreground" />
                        </a>
                      )}
                      <button
                        onClick={() => { if (confirm('Disconnect this account?')) deleteMut.mutate(account.id); }}
                        className="p-1.5 rounded hover:bg-destructive/10 transition-colors" title="Disconnect">
                        <Trash2 className="h-3.5 w-3.5 text-muted-foreground hover:text-destructive" />
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

// ── Social: Content ───────────────────────────────────────────────────────────

// Helper: get all days in a month grid (6 rows × 7 cols, padded with prev/next month days)
function buildMonthGrid(year: number, month: number) {
  const firstDay = new Date(year, month, 1).getDay(); // 0=Sun
  const daysInMonth = new Date(year, month + 1, 0).getDate();
  const daysInPrev = new Date(year, month, 0).getDate();
  const cells: { date: Date; inMonth: boolean }[] = [];
  for (let i = firstDay - 1; i >= 0; i--) {
    cells.push({ date: new Date(year, month - 1, daysInPrev - i), inMonth: false });
  }
  for (let d = 1; d <= daysInMonth; d++) {
    cells.push({ date: new Date(year, month, d), inMonth: true });
  }
  while (cells.length % 7 !== 0) {
    cells.push({ date: new Date(year, month + 1, cells.length - daysInMonth - firstDay + 1), inMonth: false });
  }
  return cells;
}

// Helper: get 7 days of a week starting from a given date
function buildWeekDays(anchor: Date) {
  const start = new Date(anchor);
  start.setDate(anchor.getDate() - anchor.getDay()); // Sunday
  return Array.from({ length: 7 }, (_, i) => {
    const d = new Date(start);
    d.setDate(start.getDate() + i);
    return d;
  });
}

function isSameDay(a: Date, b: Date) {
  return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
}

const MONTH_NAMES = ['January','February','March','April','May','June','July','August','September','October','November','December'];
const DAY_NAMES = ['Sun','Mon','Tue','Wed','Thu','Fri','Sat'];

function SocialContentView({ projectEntries }: { projectEntries: { id: string; name: string }[] }) {
  const [calView, setCalView] = useState<'list' | 'week' | 'month'>('list');
  const [statusFilter, setStatusFilter] = useState<string>('all');
  const [projectFilter, setProjectFilter] = useState<string>('all');
  const [showCreate, setShowCreate] = useState(false);
  const [newCaption, setNewCaption] = useState('');
  const [newPlatforms, setNewPlatforms] = useState<string[]>([]);
  const [newStatus, setNewStatus] = useState('draft');
  const [newProjectId, setNewProjectId] = useState(projectEntries[0]?.id || '');
  const [newScheduled, setNewScheduled] = useState('');
  const [calDate, setCalDate] = useState(new Date()); // anchor for week/month navigation
  const [selectedDay, setSelectedDay] = useState<Date | null>(null);

  const postQueries = useQueries({
    queries: projectEntries.map(e => ({
      queryKey: ['social-posts', e.id],
      queryFn: () => socialApi.listPostsFiltered({ projectId: e.id, limit: 100 }),
      staleTime: 60_000,
    })),
  });

  const allPosts = useMemo(() => {
    const list: (SocialPostRecord & { _project: string; _projectId: string })[] = [];
    postQueries.forEach((q, i) => {
      q.data?.forEach(p => list.push({ ...p, _project: projectEntries[i].name, _projectId: projectEntries[i].id }));
    });
    return list.sort((a, b) => {
      if (a.scheduled_for && b.scheduled_for) return new Date(a.scheduled_for).getTime() - new Date(b.scheduled_for).getTime();
      return new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime();
    });
  }, [postQueries, projectEntries]);

  const filtered = useMemo(() => {
    return allPosts.filter(p => {
      if (statusFilter !== 'all' && p.status !== statusFilter) return false;
      if (projectFilter !== 'all' && p._projectId !== projectFilter) return false;
      return true;
    });
  }, [allPosts, statusFilter, projectFilter]);

  const counts = useMemo(() => {
    const c: Record<string, number> = { all: allPosts.length };
    allPosts.forEach(p => { c[p.status] = (c[p.status] || 0) + 1; });
    return c;
  }, [allPosts]);

  const queryClient = useQueryClient();
  const createMut = useMutation({
    mutationFn: () => socialApi.createPost({
      project_id: newProjectId,
      caption: newCaption,
      platforms: JSON.stringify(newPlatforms),
      status: newStatus,
      scheduled_for: newScheduled || undefined,
    }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['social-posts'] });
      setShowCreate(false);
      setNewCaption('');
      setNewPlatforms([]);
      setNewScheduled('');
    },
  });

  const deleteMut = useMutation({
    mutationFn: (id: string) => socialApi.deletePost(id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['social-posts'] }),
  });

  // Helpers shared across views
  const STATUS_TABS = ['all', 'draft', 'pending_review', 'scheduled', 'published', 'failed'];
  const parsePlatforms = (p: string): string[] => { try { return JSON.parse(p); } catch { return [p].filter(Boolean); } };
  const fmtFollowers = (n: number) => n >= 1_000_000 ? `${(n/1_000_000).toFixed(1)}M` : n >= 1000 ? `${(n/1000).toFixed(1)}k` : n.toString();

  // Calendar navigation
  const monthYear = `${MONTH_NAMES[calDate.getMonth()]} ${calDate.getFullYear()}`;
  const weekDays = buildWeekDays(calDate);
  const monthGrid = buildMonthGrid(calDate.getFullYear(), calDate.getMonth());

  // Posts indexed by date string "YYYY-MM-DD" using scheduled_for or published_at
  const postsByDate = useMemo(() => {
    const map: Record<string, typeof allPosts> = {};
    allPosts.forEach(p => {
      const d = p.scheduled_for || p.published_at;
      if (!d) return;
      const key = d.slice(0, 10);
      if (!map[key]) map[key] = [];
      map[key].push(p);
    });
    return map;
  }, [allPosts]);

  const dateKey = (d: Date) => `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,'0')}-${String(d.getDate()).padStart(2,'0')}`;
  const today = new Date();

  // Post pill used in calendar cells
  const PostPill = ({ post, compact = false }: { post: typeof allPosts[0]; compact?: boolean }) => {
    const platforms = parsePlatforms(post.platforms);
    const Icon = PLATFORM_ICONS[platforms[0]] || Globe;
    const statusColor =
      post.status === 'published' ? 'bg-green-500/15 text-green-700 dark:text-green-400 border-green-300/50' :
      post.status === 'scheduled' ? 'bg-purple-500/15 text-purple-700 dark:text-purple-400 border-purple-300/50' :
      post.status === 'failed'    ? 'bg-destructive/15 text-destructive border-destructive/30' :
      'bg-muted text-muted-foreground border-border/50';
    return (
      <div className={`flex items-center gap-1 px-1.5 py-0.5 rounded border text-[10px] leading-tight truncate ${statusColor}`}>
        <Icon className={`h-2.5 w-2.5 shrink-0 ${PLATFORM_COLORS[platforms[0]] || ''}`} />
        {!compact && <span className="truncate">{post.caption?.slice(0, 28) || '(no caption)'}</span>}
        {compact && <span className="truncate">{platforms[0]}</span>}
      </div>
    );
  };

  // Day detail panel (shown when a day is selected in month view)
  const DayDetail = ({ day }: { day: Date }) => {
    const posts = postsByDate[dateKey(day)] || [];
    return (
      <Card className="bg-card/80 backdrop-blur-sm border-border/50 mt-4">
        <CardHeader className="pb-2">
          <div className="flex items-center justify-between">
            <CardTitle className="text-sm font-medium">
              {day.toLocaleDateString('en-US', { weekday: 'long', month: 'long', day: 'numeric' })}
            </CardTitle>
            <button onClick={() => setSelectedDay(null)} className="text-muted-foreground hover:text-foreground">
              <X className="h-4 w-4" />
            </button>
          </div>
        </CardHeader>
        <CardContent>
          {posts.length === 0 ? (
            <p className="text-xs text-muted-foreground py-3 text-center">No posts this day</p>
          ) : (
            <div className="space-y-2">
              {posts.map(post => {
                const platforms = parsePlatforms(post.platforms);
                return (
                  <div key={post.id} className="flex items-start gap-2 p-2 rounded-lg border border-border/40 hover:bg-muted/30">
                    <div className="flex gap-0.5 mt-0.5">
                      {platforms.slice(0,3).map(p => {
                        const Icon = PLATFORM_ICONS[p] || Globe;
                        return <Icon key={p} className={`h-3.5 w-3.5 ${PLATFORM_COLORS[p] || ''}`} />;
                      })}
                    </div>
                    <div className="min-w-0 flex-1">
                      <p className="text-xs line-clamp-2">{post.caption || <span className="italic text-muted-foreground">No caption</span>}</p>
                      <div className="flex items-center gap-2 mt-1">
                        <span className={`text-[10px] px-1.5 py-0.5 rounded border ${STATUS_COLORS[post.status] || ''}`}>{post.status.replace('_',' ')}</span>
                        {(post.scheduled_for || post.published_at) && (
                          <span className="text-[10px] text-muted-foreground">
                            {new Date(post.scheduled_for || post.published_at!).toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' })}
                          </span>
                        )}
                        <span className="text-[10px] text-muted-foreground">{post._project}</span>
                      </div>
                    </div>
                    <button onClick={() => { if (confirm('Delete?')) deleteMut.mutate(post.id); }}
                      className="p-1 rounded hover:bg-destructive/10 shrink-0">
                      <Trash2 className="h-3 w-3 text-muted-foreground hover:text-destructive" />
                    </button>
                  </div>
                );
              })}
            </div>
          )}
        </CardContent>
      </Card>
    );
  };

  // Shared create form
  const CreateForm = () => (
    <Card className="bg-card/80 backdrop-blur-sm border-border/50">
      <CardHeader className="pb-3">
        <CardTitle className="text-sm font-medium flex items-center gap-2">
          <Plus className="h-4 w-4" /> Create Post
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        <div>
          <label className="text-xs font-medium mb-1 block">Caption</label>
          <textarea value={newCaption} onChange={e => setNewCaption(e.target.value)}
            placeholder="Write your caption..." rows={3}
            className="w-full text-sm border border-border rounded-md p-2 bg-background resize-none focus:outline-none focus:ring-1 focus:ring-ring" />
        </div>
        <div className="grid grid-cols-2 gap-3">
          <div>
            <label className="text-xs font-medium mb-1 block">Platforms</label>
            <div className="flex flex-wrap gap-1.5">
              {['instagram','linkedin','twitter','facebook','youtube','tiktok'].map(p => {
                const Icon = PLATFORM_ICONS[p] || Globe;
                const selected = newPlatforms.includes(p);
                return (
                  <button key={p} onClick={() => setNewPlatforms(prev => selected ? prev.filter(x => x !== p) : [...prev, p])}
                    className={`flex items-center gap-1 px-2 py-1 rounded border text-xs transition-colors ${selected ? 'border-primary bg-primary/10 text-primary' : 'border-border text-muted-foreground hover:border-border/80'}`}>
                    <Icon className={`h-3 w-3 ${selected ? '' : PLATFORM_COLORS[p]}`} />
                    <span className="capitalize">{p}</span>
                  </button>
                );
              })}
            </div>
          </div>
          <div className="space-y-2">
            <div>
              <label className="text-xs font-medium mb-1 block">Status</label>
              <select value={newStatus} onChange={e => setNewStatus(e.target.value)}
                className="w-full text-xs border border-border rounded-md px-2 py-1.5 bg-background">
                <option value="draft">Draft</option>
                <option value="scheduled">Scheduled</option>
                <option value="pending_review">Pending Review</option>
              </select>
            </div>
            {projectEntries.length > 1 && (
              <div>
                <label className="text-xs font-medium mb-1 block">Project</label>
                <select value={newProjectId} onChange={e => setNewProjectId(e.target.value)}
                  className="w-full text-xs border border-border rounded-md px-2 py-1.5 bg-background">
                  {projectEntries.map(e => <option key={e.id} value={e.id}>{e.name}</option>)}
                </select>
              </div>
            )}
          </div>
        </div>
        <div>
          <label className="text-xs font-medium mb-1 block">Schedule for {newStatus !== 'scheduled' && <span className="text-muted-foreground">(optional)</span>}</label>
          <input type="datetime-local" value={newScheduled} onChange={e => setNewScheduled(e.target.value)}
            className="text-xs border border-border rounded-md px-2 py-1.5 bg-background" />
        </div>
        <div className="flex gap-2 pt-1">
          <Button size="sm" onClick={() => createMut.mutate()} disabled={createMut.isPending || !newCaption || newPlatforms.length === 0}
            className="text-xs gap-1 h-8">
            {createMut.isPending ? 'Creating…' : 'Create Post'}
          </Button>
          <Button size="sm" variant="ghost" onClick={() => setShowCreate(false)} className="text-xs h-8">Cancel</Button>
        </div>
      </CardContent>
    </Card>
  );

  return (
    <div className="space-y-4">
      {/* Top toolbar */}
      <div className="flex items-center justify-between gap-3 flex-wrap">
        {/* View toggle */}
        <div className="flex gap-0.5 p-0.5 bg-muted/60 rounded-lg border border-border/40">
          {([['list', 'List', List], ['week', 'Week', CalendarDays], ['month', 'Month', CalendarRange]] as const).map(([v, label, Icon]) => (
            <button key={v} onClick={() => { setCalView(v); setSelectedDay(null); }}
              className={`flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-md transition-all ${calView === v ? 'bg-background text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'}`}>
              <Icon className="h-3.5 w-3.5" />{label}
            </button>
          ))}
        </div>

        <div className="flex items-center gap-2">
          {/* Calendar navigation (week/month only) */}
          {calView !== 'list' && (
            <div className="flex items-center gap-1">
              <button onClick={() => {
                  const d = new Date(calDate);
                  if (calView === 'month') d.setMonth(d.getMonth() - 1);
                  else d.setDate(d.getDate() - 7);
                  setCalDate(d); setSelectedDay(null);
                }} className="p-1.5 rounded hover:bg-muted transition-colors">
                <ChevronLeft className="h-4 w-4" />
              </button>
              <span className="text-sm font-medium min-w-[140px] text-center">
                {calView === 'month' ? monthYear : `${weekDays[0].toLocaleDateString('en-US',{month:'short',day:'numeric'})} – ${weekDays[6].toLocaleDateString('en-US',{month:'short',day:'numeric', year:'numeric'})}`}
              </span>
              <button onClick={() => {
                  const d = new Date(calDate);
                  if (calView === 'month') d.setMonth(d.getMonth() + 1);
                  else d.setDate(d.getDate() + 7);
                  setCalDate(d); setSelectedDay(null);
                }} className="p-1.5 rounded hover:bg-muted transition-colors">
                <ChevronRight className="h-4 w-4" />
              </button>
              <button onClick={() => { setCalDate(new Date()); setSelectedDay(null); }}
                className="text-xs px-2 py-1 rounded border border-border hover:bg-muted transition-colors">
                Today
              </button>
            </div>
          )}

          {/* List-only filters */}
          {calView === 'list' && (
            <>
              {projectEntries.length > 1 && (
                <select value={projectFilter} onChange={e => setProjectFilter(e.target.value)}
                  className="text-xs border border-border rounded-md px-2 py-1.5 bg-background">
                  <option value="all">All projects</option>
                  {projectEntries.map(e => <option key={e.id} value={e.id}>{e.name}</option>)}
                </select>
              )}
            </>
          )}

          <Button size="sm" className="gap-1.5 text-xs h-8" onClick={() => setShowCreate(v => !v)}>
            <Plus className="h-3.5 w-3.5" /> New Post
          </Button>
        </div>
      </div>

      {/* Create form */}
      {showCreate && <CreateForm />}

      {/* ── LIST VIEW ── */}
      {calView === 'list' && (
        <>
          <div className="flex gap-1 p-1 bg-muted/50 rounded-lg flex-wrap">
            {STATUS_TABS.map(s => (
              <button key={s} onClick={() => setStatusFilter(s)}
                className={`px-3 py-1 text-xs font-medium rounded-md transition-all capitalize ${statusFilter === s ? 'bg-background text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'}`}>
                {s === 'all' ? 'All' : s.replace('_', ' ')}
                {counts[s] != null && counts[s] > 0 && <span className="ml-1.5 text-[10px] text-muted-foreground">{counts[s]}</span>}
              </button>
            ))}
          </div>

          {filtered.length === 0 ? (
            <div className="text-center py-12 text-muted-foreground">
              <FileText className="h-8 w-8 mx-auto mb-2 opacity-40" />
              <p>{statusFilter === 'all' ? 'No posts yet' : `No ${statusFilter.replace('_',' ')} posts`}</p>
            </div>
          ) : (
            <div className="space-y-2">
              {filtered.map(post => {
                const platforms = parsePlatforms(post.platforms);
                const hashtags: string[] = (() => { try { return post.hashtags ? JSON.parse(post.hashtags) : []; } catch { return []; } })();
                return (
                  <Card key={post.id} className="bg-card/80 backdrop-blur-sm border-border/50 hover:border-border transition-colors">
                    <CardContent className="pt-4 pb-3">
                      <div className="flex items-start gap-3">
                        <div className="flex gap-1 mt-0.5 shrink-0">
                          {platforms.map(p => {
                            const Icon = PLATFORM_ICONS[p] || Globe;
                            return <div key={p} className={`h-7 w-7 rounded-full flex items-center justify-center ${PLATFORM_BG[p] || 'bg-muted'}`}>
                              <Icon className={`h-3.5 w-3.5 ${PLATFORM_COLORS[p] || 'text-muted-foreground'}`} />
                            </div>;
                          })}
                        </div>
                        <div className="min-w-0 flex-1">
                          <p className="text-sm line-clamp-2">{post.caption || <span className="text-muted-foreground italic">No caption</span>}</p>
                          {hashtags.length > 0 && <p className="text-xs text-blue-500 mt-1 line-clamp-1">{hashtags.map(h => `#${h}`).join(' ')}</p>}
                          <div className="flex items-center gap-2 mt-2 flex-wrap">
                            <span className={`text-[10px] px-1.5 py-0.5 rounded border ${STATUS_COLORS[post.status] || ''}`}>{post.status.replace('_',' ')}</span>
                            {post.scheduled_for && <span className="text-[10px] text-muted-foreground flex items-center gap-1"><CalendarDays className="h-3 w-3" />{formatDate(post.scheduled_for)}</span>}
                            {post.published_at && <span className="text-[10px] text-muted-foreground">Published {formatDate(post.published_at)}</span>}
                            <span className="text-[10px] text-muted-foreground">{post._project}</span>
                          </div>
                          {post.status === 'published' && (post.impressions > 0 || post.likes > 0) && (
                            <div className="flex gap-3 mt-2 text-[11px] text-muted-foreground">
                              {post.impressions > 0 && <span>👁 {fmtFollowers(post.impressions)}</span>}
                              {post.reach > 0 && <span>📡 {fmtFollowers(post.reach)}</span>}
                              {post.likes > 0 && <span>♥ {post.likes}</span>}
                              {post.comments > 0 && <span>💬 {post.comments}</span>}
                              {post.engagement_rate > 0 && <span>{(post.engagement_rate * 100).toFixed(1)}% eng</span>}
                            </div>
                          )}
                        </div>
                        <div className="flex items-center gap-1 shrink-0">
                          {post.platform_url && <a href={post.platform_url} target="_blank" rel="noopener noreferrer" className="p-1.5 rounded hover:bg-muted"><ExternalLink className="h-3.5 w-3.5 text-muted-foreground" /></a>}
                          <button onClick={() => { if (confirm('Delete this post?')) deleteMut.mutate(post.id); }} className="p-1.5 rounded hover:bg-destructive/10">
                            <Trash2 className="h-3.5 w-3.5 text-muted-foreground hover:text-destructive" />
                          </button>
                        </div>
                      </div>
                    </CardContent>
                  </Card>
                );
              })}
            </div>
          )}
        </>
      )}

      {/* ── WEEK VIEW ── */}
      {calView === 'week' && (
        <Card className="bg-card/80 backdrop-blur-sm border-border/50 overflow-hidden">
          <div className="grid grid-cols-7 border-b border-border/50">
            {weekDays.map((day, i) => {
              const isToday = isSameDay(day, today);
              const dayPosts = postsByDate[dateKey(day)] || [];
              return (
                <div key={i} className={`border-r border-border/40 last:border-r-0 min-h-[520px] flex flex-col ${isToday ? 'bg-primary/5' : ''}`}>
                  {/* Day header */}
                  <div className={`px-2 py-2 border-b border-border/40 ${isToday ? 'bg-primary/10' : 'bg-muted/20'}`}>
                    <p className="text-[10px] font-medium text-muted-foreground uppercase">{DAY_NAMES[day.getDay()]}</p>
                    <p className={`text-lg font-bold leading-none mt-0.5 ${isToday ? 'text-primary' : ''}`}>{day.getDate()}</p>
                    {dayPosts.length > 0 && (
                      <p className="text-[9px] text-muted-foreground mt-1">{dayPosts.length} post{dayPosts.length !== 1 ? 's' : ''}</p>
                    )}
                  </div>
                  {/* Posts */}
                  <div className="p-1.5 space-y-1 flex-1">
                    {dayPosts.map(post => {
                      const platforms = parsePlatforms(post.platforms);
                      const Icon = PLATFORM_ICONS[platforms[0]] || Globe;
                      const timeStr = (post.scheduled_for || post.published_at)
                        ? new Date(post.scheduled_for || post.published_at!).toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' })
                        : null;
                      const statusColor =
                        post.status === 'published' ? 'bg-green-500/10 border-green-300/40 text-green-700 dark:text-green-400' :
                        post.status === 'scheduled' ? 'bg-purple-500/10 border-purple-300/40 text-purple-700 dark:text-purple-400' :
                        post.status === 'failed'    ? 'bg-destructive/10 border-destructive/30 text-destructive' :
                        'bg-muted/60 border-border/40 text-muted-foreground';
                      return (
                        <div key={post.id} className={`rounded border p-1.5 cursor-default group relative ${statusColor}`}>
                          <div className="flex items-center gap-1 mb-0.5">
                            <Icon className={`h-2.5 w-2.5 shrink-0 ${PLATFORM_COLORS[platforms[0]] || ''}`} />
                            {timeStr && <span className="text-[9px] font-mono opacity-70">{timeStr}</span>}
                          </div>
                          <p className="text-[10px] leading-tight line-clamp-3">{post.caption || '(no caption)'}</p>
                          {platforms.length > 1 && (
                            <div className="flex gap-0.5 mt-1">
                              {platforms.slice(1).map(p => {
                                const I = PLATFORM_ICONS[p] || Globe;
                                return <I key={p} className={`h-2 w-2 ${PLATFORM_COLORS[p] || ''} opacity-60`} />;
                              })}
                            </div>
                          )}
                          <button
                            onClick={() => { if (confirm('Delete?')) deleteMut.mutate(post.id); }}
                            className="absolute top-1 right-1 opacity-0 group-hover:opacity-100 p-0.5 rounded hover:bg-destructive/20 transition-opacity">
                            <Trash2 className="h-2.5 w-2.5 text-destructive" />
                          </button>
                        </div>
                      );
                    })}
                    {dayPosts.length === 0 && (
                      <button
                        onClick={() => { setNewScheduled(`${dateKey(day)}T09:00`); setNewStatus('scheduled'); setShowCreate(true); }}
                        className="w-full h-8 rounded border border-dashed border-border/30 text-[10px] text-muted-foreground/40 hover:border-border/60 hover:text-muted-foreground transition-colors flex items-center justify-center gap-1">
                        <Plus className="h-2.5 w-2.5" /> Add
                      </button>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
          {/* Week summary bar */}
          <div className="px-4 py-2 bg-muted/20 border-t border-border/40 flex items-center gap-4 text-xs text-muted-foreground">
            <span>{weekDays.reduce((s, d) => s + (postsByDate[dateKey(d)]?.length || 0), 0)} posts this week</span>
            <span>·</span>
            <span>{weekDays.reduce((s, d) => s + (postsByDate[dateKey(d)]?.filter(p => p.status === 'scheduled').length || 0), 0)} scheduled</span>
            <span>·</span>
            <span>{weekDays.reduce((s, d) => s + (postsByDate[dateKey(d)]?.filter(p => p.status === 'published').length || 0), 0)} published</span>
          </div>
        </Card>
      )}

      {/* ── MONTH VIEW ── */}
      {calView === 'month' && (
        <>
          <Card className="bg-card/80 backdrop-blur-sm border-border/50 overflow-hidden">
            {/* Day-of-week headers */}
            <div className="grid grid-cols-7 bg-muted/30 border-b border-border/40">
              {DAY_NAMES.map(d => (
                <div key={d} className="py-2 text-center text-[11px] font-semibold text-muted-foreground uppercase tracking-wide">{d}</div>
              ))}
            </div>
            {/* Calendar grid */}
            <div className="grid grid-cols-7">
              {monthGrid.map((cell, idx) => {
                const key = dateKey(cell.date);
                const dayPosts = postsByDate[key] || [];
                const isToday = isSameDay(cell.date, today);
                const isSelected = selectedDay ? isSameDay(cell.date, selectedDay) : false;
                const MAX_VISIBLE = 3;
                return (
                  <div
                    key={idx}
                    onClick={() => setSelectedDay(isSelected ? null : cell.date)}
                    className={[
                      'min-h-[100px] p-1.5 border-b border-r border-border/30 cursor-pointer transition-colors',
                      !cell.inMonth && 'opacity-40',
                      isToday && 'bg-primary/5',
                      isSelected && 'ring-2 ring-inset ring-primary/40 bg-primary/5',
                      'hover:bg-muted/30',
                    ].filter(Boolean).join(' ')}
                  >
                    <div className="flex items-center justify-between mb-1">
                      <span className={[
                        'text-xs font-medium w-6 h-6 flex items-center justify-center rounded-full leading-none',
                        isToday ? 'bg-primary text-primary-foreground' : '',
                        !cell.inMonth ? 'text-muted-foreground/50' : '',
                      ].filter(Boolean).join(' ')}>
                        {cell.date.getDate()}
                      </span>
                      {dayPosts.length > 0 && (
                        <span className="text-[9px] text-muted-foreground">{dayPosts.length}</span>
                      )}
                    </div>
                    <div className="space-y-0.5">
                      {dayPosts.slice(0, MAX_VISIBLE).map(post => (
                        <PostPill key={post.id} post={post} />
                      ))}
                      {dayPosts.length > MAX_VISIBLE && (
                        <div className="text-[9px] text-muted-foreground pl-1">+{dayPosts.length - MAX_VISIBLE} more</div>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
            {/* Month legend */}
            <div className="px-4 py-2 bg-muted/20 border-t border-border/40 flex items-center gap-4 text-[11px]">
              {[['bg-purple-500/15 border-purple-300/50','scheduled'],['bg-green-500/15 border-green-300/50','published'],['bg-destructive/15 border-destructive/30','failed'],['bg-muted border-border/50','draft']].map(([cls, lbl]) => (
                <span key={lbl} className="flex items-center gap-1.5">
                  <span className={`w-2.5 h-2.5 rounded border ${cls}`} />
                  <span className="text-muted-foreground capitalize">{lbl}</span>
                </span>
              ))}
            </div>
          </Card>

          {/* Day detail panel */}
          {selectedDay && <DayDetail day={selectedDay} />}
        </>
      )}
    </div>
  );
}

// ── Social: Inbox ─────────────────────────────────────────────────────────────

function SocialInboxView({ projectEntries }: { projectEntries: { id: string; name: string }[] }) {
  const [statusFilter, setStatusFilter] = useState<string>('all');
  const [platformFilter, setPlatformFilter] = useState<string>('all');

  const mentionQueries = useQueries({
    queries: projectEntries.map(e => ({
      queryKey: ['social-mentions', e.id],
      queryFn: () => socialApi.listMentions(e.id, { limit: 50 }),
      staleTime: 30_000,
    })),
  });

  const allMentions = useMemo(() => {
    const list: (SocialMentionRecord & { _project: string })[] = [];
    mentionQueries.forEach((q, i) => {
      q.data?.forEach(m => list.push({ ...m, _project: projectEntries[i].name }));
    });
    return list.sort((a, b) => new Date(b.received_at).getTime() - new Date(a.received_at).getTime());
  }, [mentionQueries, projectEntries]);

  const platforms = useMemo(() => [...new Set(allMentions.map(m => m.platform))], [allMentions]);

  const filtered = useMemo(() => {
    return allMentions.filter(m => {
      if (statusFilter === 'unread' && m.status !== 'unread') return false;
      if (statusFilter === 'urgent' && m.priority !== 'urgent' && m.priority !== 'high') return false;
      if (statusFilter === 'replied' && m.status !== 'replied') return false;
      if (statusFilter === 'archived' && m.status !== 'archived') return false;
      if (platformFilter !== 'all' && m.platform !== platformFilter) return false;
      return true;
    });
  }, [allMentions, statusFilter, platformFilter]);

  const counts = useMemo(() => ({
    all: allMentions.length,
    unread: allMentions.filter(m => m.status === 'unread').length,
    urgent: allMentions.filter(m => m.priority === 'urgent' || m.priority === 'high').length,
    replied: allMentions.filter(m => m.status === 'replied').length,
    archived: allMentions.filter(m => m.status === 'archived').length,
  }), [allMentions]);

  const queryClient = useQueryClient();
  const updateMut = useMutation({
    mutationFn: ({ id, data }: { id: string; data: Parameters<typeof socialApi.updateMention>[1] }) =>
      socialApi.updateMention(id, data),
    onSuccess: () => {
      projectEntries.forEach(e => queryClient.invalidateQueries({ queryKey: ['social-mentions', e.id] }));
    },
  });

  const INBOX_TABS = [
    { key: 'all', label: 'All' },
    { key: 'unread', label: 'Unread' },
    { key: 'urgent', label: 'Urgent' },
    { key: 'replied', label: 'Replied' },
    { key: 'archived', label: 'Archived' },
  ];

  return (
    <div className="space-y-5">
      <div className="flex items-center justify-between gap-4 flex-wrap">
        <div className="flex gap-1 p-1 bg-muted/50 rounded-lg">
          {INBOX_TABS.map(({ key, label }) => (
            <button key={key} onClick={() => setStatusFilter(key)}
              className={`px-3 py-1 text-xs font-medium rounded-md transition-all ${statusFilter === key ? 'bg-background text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'}`}>
              {label}
              {(counts as any)[key] > 0 && <span className="ml-1.5 text-[10px] text-muted-foreground">{(counts as any)[key]}</span>}
            </button>
          ))}
        </div>
        {platforms.length > 1 && (
          <select value={platformFilter} onChange={e => setPlatformFilter(e.target.value)}
            className="text-xs border border-border rounded-md px-2 py-1.5 bg-background">
            <option value="all">All platforms</option>
            {platforms.map(p => <option key={p} value={p} className="capitalize">{p}</option>)}
          </select>
        )}
      </div>

      {filtered.length === 0 ? (
        <div className="text-center py-12 text-muted-foreground">
          <Inbox className="h-8 w-8 mx-auto mb-2 opacity-40" />
          <p>{statusFilter === 'all' ? 'No mentions yet' : `No ${statusFilter} mentions`}</p>
        </div>
      ) : (
        <div className="space-y-2">
          {filtered.map(mention => {
            const Icon = PLATFORM_ICONS[mention.platform] || Globe;
            const isUnread = mention.status === 'unread';
            return (
              <Card key={mention.id} className={`backdrop-blur-sm border-border/50 transition-colors ${isUnread ? 'bg-accent/10 border-accent/30' : 'bg-card/80'}`}>
                <CardContent className="pt-3 pb-3">
                  <div className="flex items-start gap-3">
                    <div className={`h-8 w-8 rounded-full flex items-center justify-center shrink-0 mt-0.5 ${PLATFORM_BG[mention.platform] || 'bg-muted'}`}>
                      <Icon className={`h-4 w-4 ${PLATFORM_COLORS[mention.platform] || 'text-muted-foreground'}`} />
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-2 flex-wrap">
                        <span className="text-sm font-medium">{mention.author_display_name || mention.author_username || 'Unknown'}</span>
                        {mention.author_is_verified && <span className="text-[10px] text-blue-500">✓ verified</span>}
                        <Badge variant="outline" className="text-[9px]">{mention.mention_type}</Badge>
                        <span className={`text-[10px] font-medium ${PRIORITY_COLORS[mention.priority]}`}>{mention.priority !== 'normal' ? mention.priority : ''}</span>
                        {isUnread && <span className="h-1.5 w-1.5 rounded-full bg-blue-500" />}
                      </div>
                      {mention.content && <p className="text-sm text-muted-foreground mt-1 line-clamp-3">{mention.content}</p>}
                      {mention.reply_content && (
                        <div className="mt-2 pl-3 border-l-2 border-primary/30">
                          <p className="text-xs text-muted-foreground">Reply: {mention.reply_content}</p>
                        </div>
                      )}
                      <div className="flex items-center gap-2 mt-2 flex-wrap">
                        {mention.sentiment && mention.sentiment !== 'unknown' && (
                          <span className={`text-[10px] ${SENTIMENT_COLORS[mention.sentiment]}`}>● {mention.sentiment}</span>
                        )}
                        <span className="text-[10px] text-muted-foreground">{formatDate(mention.received_at)}</span>
                        <Badge variant="outline" className="text-[9px]">{mention._project}</Badge>
                        <span className={`text-[10px] px-1.5 py-0.5 rounded border ${STATUS_COLORS[mention.status] || 'border-border text-muted-foreground'}`}>{mention.status}</span>
                      </div>
                    </div>
                    <div className="flex flex-col gap-1 shrink-0">
                      {isUnread && (
                        <button onClick={() => updateMut.mutate({ id: mention.id, data: { status: 'read' } })}
                          className="text-[10px] px-2 py-1 rounded border border-border hover:bg-muted transition-colors" title="Mark read">
                          Mark read
                        </button>
                      )}
                      {mention.status !== 'archived' && (
                        <button onClick={() => updateMut.mutate({ id: mention.id, data: { status: 'archived' } })}
                          className="text-[10px] px-2 py-1 rounded border border-border hover:bg-muted transition-colors">
                          Archive
                        </button>
                      )}
                      {mention.status !== 'replied' && (
                        <button onClick={() => updateMut.mutate({ id: mention.id, data: { status: 'flagged', priority: 'high' } })}
                          className="text-[10px] px-2 py-1 rounded border border-amber-300 hover:bg-amber-50 dark:hover:bg-amber-950/20 text-amber-600 transition-colors">
                          Flag
                        </button>
                      )}
                    </div>
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

// ── Social: Analytics ─────────────────────────────────────────────────────────

function SocialAnalyticsView({ projectEntries }: { projectEntries: { id: string; name: string }[] }) {
  const postQueries = useQueries({
    queries: projectEntries.map(e => ({
      queryKey: ['social-posts', e.id],
      queryFn: () => socialApi.listPostsFiltered({ projectId: e.id, status: 'published', limit: 100 }),
      staleTime: 120_000,
    })),
  });

  const agg = useMemo(() => {
    const posts: (SocialPostRecord & { _project: string })[] = [];
    postQueries.forEach((q, i) => {
      q.data?.forEach(p => posts.push({ ...p, _project: projectEntries[i].name }));
    });

    const totalImpressionsN = posts.reduce((s, p) => s + (p.impressions || 0), 0);
    const totalReachN = posts.reduce((s, p) => s + (p.reach || 0), 0);
    const totalLikes = posts.reduce((s, p) => s + (p.likes || 0), 0);
    const totalComments = posts.reduce((s, p) => s + (p.comments || 0), 0);
    const totalShares = posts.reduce((s, p) => s + (p.shares || 0), 0);
    const totalSaves = posts.reduce((s, p) => s + (p.saves || 0), 0);
    const avgEngagement = posts.length > 0 ? posts.reduce((s, p) => s + (p.engagement_rate || 0), 0) / posts.length : 0;

    const topPosts = [...posts]
      .sort((a, b) => (b.impressions || 0) - (a.impressions || 0))
      .slice(0, 10);

    const byPlatform: Record<string, { posts: number; impressions: number; likes: number; engagement: number }> = {};
    posts.forEach(p => {
      const platforms: string[] = (() => { try { return JSON.parse(p.platforms); } catch { return [p.platforms].filter(Boolean); } })();
      platforms.forEach(pl => {
        if (!byPlatform[pl]) byPlatform[pl] = { posts: 0, impressions: 0, likes: 0, engagement: 0 };
        byPlatform[pl].posts++;
        byPlatform[pl].impressions += p.impressions || 0;
        byPlatform[pl].likes += p.likes || 0;
        byPlatform[pl].engagement += p.engagement_rate || 0;
      });
    });
    Object.keys(byPlatform).forEach(pl => {
      if (byPlatform[pl].posts > 0) byPlatform[pl].engagement /= byPlatform[pl].posts;
    });

    return { totalImpressionsN, totalReachN, totalLikes, totalComments, totalShares, totalSaves, avgEngagement, topPosts, byPlatform, totalPosts: posts.length };
  }, [postQueries, projectEntries]);

  const fmt = (n: number) => n >= 1_000_000 ? `${(n/1_000_000).toFixed(1)}M` : n >= 1000 ? `${(n/1000).toFixed(1)}k` : n.toString();

  if (agg.totalPosts === 0) {
    return (
      <div className="text-center py-16 text-muted-foreground">
        <BarChart2 className="h-10 w-10 mx-auto mb-3 opacity-40" />
        <p className="font-medium mb-1">No analytics data yet</p>
        <p className="text-xs">Publish posts to start tracking engagement metrics</p>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Summary KPIs */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        {[
          { label: 'Total Impressions', value: fmt(agg.totalImpressionsN), sub: `${agg.totalPosts} posts` },
          { label: 'Total Reach', value: fmt(agg.totalReachN), sub: 'unique accounts' },
          { label: 'Total Likes', value: fmt(agg.totalLikes), sub: `+ ${fmt(agg.totalComments)} comments` },
          { label: 'Avg Engagement', value: `${(agg.avgEngagement * 100).toFixed(2)}%`, sub: `${fmt(agg.totalShares)} shares` },
        ].map(({ label, value, sub }) => (
          <Card key={label} className="bg-card/80 backdrop-blur-sm border-border/50">
            <CardContent className="pt-5">
              <p className="text-xs text-muted-foreground mb-1">{label}</p>
              <p className="text-2xl font-bold">{value}</p>
              <p className="text-[11px] text-muted-foreground mt-0.5">{sub}</p>
            </CardContent>
          </Card>
        ))}
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Platform breakdown */}
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium flex items-center gap-2">
              <BarChart2 className="h-4 w-4 text-blue-500" />
              Platform Performance
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              {Object.entries(agg.byPlatform).sort((a, b) => b[1].impressions - a[1].impressions).map(([platform, data]) => {
                const Icon = PLATFORM_ICONS[platform] || Globe;
                const maxImpressions = Math.max(...Object.values(agg.byPlatform).map(d => d.impressions));
                const pct = maxImpressions > 0 ? (data.impressions / maxImpressions) * 100 : 0;
                return (
                  <div key={platform}>
                    <div className="flex items-center gap-2 mb-1">
                      <Icon className={`h-3.5 w-3.5 shrink-0 ${PLATFORM_COLORS[platform] || 'text-muted-foreground'}`} />
                      <span className="text-xs capitalize flex-1">{platform}</span>
                      <span className="text-xs text-muted-foreground">{data.posts} posts</span>
                      <span className="text-xs font-medium">{fmt(data.impressions)} imp</span>
                      <span className="text-xs text-muted-foreground">{(data.engagement * 100).toFixed(1)}%</span>
                    </div>
                    <div className="h-1.5 rounded-full bg-muted overflow-hidden">
                      <div className={`h-full rounded-full ${PLATFORM_COLORS[platform]?.replace('text-', 'bg-') || 'bg-primary'}`} style={{ width: `${pct}%` }} />
                    </div>
                  </div>
                );
              })}
            </div>
          </CardContent>
        </Card>

        {/* Top performing posts */}
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium flex items-center gap-2">
              <TrendingUp className="h-4 w-4 text-green-500" />
              Top Posts
            </CardTitle>
          </CardHeader>
          <CardContent>
            <ScrollArea className="h-[280px]">
              <div className="space-y-2 pr-2">
                {agg.topPosts.map((post, i) => {
                  const platforms: string[] = (() => { try { return JSON.parse(post.platforms); } catch { return [post.platforms].filter(Boolean); } })();
                  return (
                    <div key={post.id} className="flex items-start gap-2 p-2 rounded-lg hover:bg-muted/30 transition-colors">
                      <span className="text-xs text-muted-foreground font-mono w-4 shrink-0 mt-0.5">{i + 1}</span>
                      <div className="flex gap-0.5 shrink-0 mt-0.5">
                        {platforms.slice(0, 2).map(p => {
                          const Icon = PLATFORM_ICONS[p] || Globe;
                          return <Icon key={p} className={`h-3 w-3 ${PLATFORM_COLORS[p] || 'text-muted-foreground'}`} />;
                        })}
                      </div>
                      <div className="min-w-0 flex-1">
                        <p className="text-xs line-clamp-2">{post.caption || '(no caption)'}</p>
                        <div className="flex gap-2 mt-0.5 text-[10px] text-muted-foreground">
                          <span>👁 {fmt(post.impressions)}</span>
                          {post.likes > 0 && <span>♥ {post.likes}</span>}
                          {post.engagement_rate > 0 && <span>{(post.engagement_rate * 100).toFixed(1)}%</span>}
                        </div>
                      </div>
                    </div>
                  );
                })}
              </div>
            </ScrollArea>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

// ── Social Tab (Router) ───────────────────────────────────────────────────────

function SocialTab({ projectEntries, orgId }: { projectEntries: { id: string; name: string }[]; orgId: string }) {
  const [searchParams, setSearchParams] = useSearchParams();
  const socialView = searchParams.get('sv') || 'overview';

  const views = [
    { key: 'overview',  label: 'Overview',  icon: LayoutGrid },
    { key: 'accounts',  label: 'Accounts',  icon: Share2 },
    { key: 'content',   label: 'Content',   icon: FileText },
    { key: 'inbox',     label: 'Inbox',     icon: Inbox },
    { key: 'analytics', label: 'Analytics', icon: BarChart2 },
  ];

  const setSocialView = (v: string) => {
    const params = new URLSearchParams(searchParams);
    if (v === 'overview') params.delete('sv'); else params.set('sv', v);
    setSearchParams(params, { replace: true });
  };

  return (
    <div className="space-y-6">
      <div className="flex gap-1 p-1 bg-muted/50 rounded-lg w-fit flex-wrap">
        {views.map(({ key, label, icon: Icon }) => (
          <button key={key} onClick={() => setSocialView(key)}
            className={`flex items-center gap-1.5 px-4 py-1.5 text-sm font-medium rounded-md transition-all ${
              socialView === key ? 'bg-background text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'
            }`}>
            <Icon className="h-3.5 w-3.5" />
            {label}
          </button>
        ))}
      </div>

      {socialView === 'overview'  && <SocialOverviewView projectEntries={projectEntries} orgId={orgId} onSwitchView={setSocialView} />}
      {socialView === 'accounts'  && <SocialAccountsView projectEntries={projectEntries} orgId={orgId} />}
      {socialView === 'content'   && <SocialContentView projectEntries={projectEntries} />}
      {socialView === 'inbox'     && <SocialInboxView projectEntries={projectEntries} />}
      {socialView === 'analytics' && <SocialAnalyticsView projectEntries={projectEntries} />}
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

// ── Member Assignments (expandable per-member) ───────────────────────────────

function MemberAssignments({ orgId, userId }: { orgId: string; userId: string }) {
  const queryClient = useQueryClient();
  const { data: assignments, isLoading } = useQuery({
    queryKey: ['member-assignments', orgId, userId],
    queryFn: () => organizationsApi.getMemberAssignments(orgId, userId),
  });

  const { data: orgClients = [] } = useQuery<ClientData[]>({
    queryKey: ['orgClients', orgId],
    queryFn: () => organizationsApi.getClients(orgId),
  });

  const [assignType, setAssignType] = useState<string>('');
  const [assignTargetId, setAssignTargetId] = useState('');
  const [assignRole, setAssignRole] = useState('editor');

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

// ── Members Tab ───────────────────────────────────────────────────────────────

function MembersTab({ orgId, orgName }: { orgId: string; orgName: string }) {
  const queryClient = useQueryClient();
  const { data: members = [], isLoading } = useQuery<OrgMember[]>({
    queryKey: ['org-members', orgId],
    queryFn: () => organizationsApi.getMembers(orgId),
    enabled: !!orgId,
  });

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
      toast.success('Member added');
    },
    onError: () => toast.error('Failed to add member'),
  });

  const removeMemberMutation = useMutation({
    mutationFn: (userId: string) => organizationsApi.removeMember(orgId, userId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['org-members', orgId] });
      toast.success('Member removed');
    },
    onError: () => toast.error('Failed to remove member'),
  });

  const changeRoleMutation = useMutation({
    mutationFn: ({ userId, role }: { userId: string; role: string }) =>
      organizationsApi.changeMemberRole(orgId, userId, role),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['org-members', orgId] });
      toast.success('Role updated');
    },
    onError: () => toast.error('Failed to update role'),
  });

  const createInviteMutation = useMutation({
    mutationFn: () => organizationsApi.createInvitation(orgId, inviteRole),
    onSuccess: (data: any) => {
      setInviteLink(data.invite_url || '');
    },
    onError: () => toast.error('Failed to create invite'),
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

export function OrganizationProfilePage({ defaultTab, defaultPipeline }: OrganizationProfilePageProps = {}) {
  const { orgId } = useParams<{ orgId: string }>();
  const [searchParams, setSearchParams] = useSearchParams();
  const navigate = useNavigate();

  const tabFromUrl = searchParams.get('tab') || defaultTab || 'overview';
  const pipelineFromUrl = searchParams.get('pipeline') || defaultPipeline;
  const clientFilter = searchParams.get('client');
  // viewFromUrl kept for potential future deep-link use
  const _viewFromUrl = searchParams.get('view'); void _viewFromUrl;

  const TAB_PATHS: Record<string, string> = {
    overview: '',
    pipelines: '/crm/pipeline',
    contacts: '/crm/contacts',
    projects: '/projects',
    social: '/social',
    intelligence: '/intelligence',
    members: '/members',
    integrations: '/integrations',
  };

  const setTab = (tab: string) => {
    const basePath = `/organizations/${orgId}`;
    const tabPath = TAB_PATHS[tab] ?? '';
    const params = new URLSearchParams();
    // Carry over relevant query params
    if (tab === 'pipelines' && pipelineFromUrl) params.set('pipeline', pipelineFromUrl);
    if (tab === 'projects' && clientFilter) params.set('client', clientFilter);
    const qs = params.toString();
    navigate(`${basePath}${tabPath}${qs ? `?${qs}` : ''}`, { replace: true });
  };

  const clearClientFilter = () => {
    const params = new URLSearchParams(searchParams);
    params.delete('client');
    setSearchParams(params, { replace: true });
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

  const { data: crmContacts = [] } = useQuery<CrmContactRecord[]>({
    queryKey: ['crm-contacts', orgId],
    queryFn: () => crmApi.listContacts(orgId!),
    enabled: !!orgId,
    staleTime: 30_000,
  });

  const qc = useQueryClient();
  const { data: brandProfile } = useQuery<OrgBrandProfile | null>({
    queryKey: ['orgBrandProfile', orgId],
    queryFn: () => organizationsApi.getBrandProfile(orgId!),
    enabled: !!orgId,
    staleTime: 5 * 60_000,
  });
  const [brandEditing, setBrandEditing] = useState(false);
  const [brandForm, setBrandForm] = useState<Partial<OrgBrandProfile>>({});
  const [brandSaving, setBrandSaving] = useState(false);

  const openBrandEdit = () => {
    setBrandForm({
      tagline: brandProfile?.tagline ?? '',
      primaryColor: brandProfile?.primaryColor ?? '#2563EB',
      secondaryColor: brandProfile?.secondaryColor ?? '#EC4899',
      accentColor: brandProfile?.accentColor ?? '',
      typographyHeading: brandProfile?.typographyHeading ?? '',
      typographyBody: brandProfile?.typographyBody ?? '',
      logoUrl: brandProfile?.logoUrl ?? '',
      industry: brandProfile?.industry ?? '',
      marketPosition: brandProfile?.marketPosition ?? '',
      uniqueValueProposition: brandProfile?.uniqueValueProposition ?? '',
      missionStatement: brandProfile?.missionStatement ?? '',
      visionStatement: brandProfile?.visionStatement ?? '',
      brandValues: brandProfile?.brandValues ?? '[]',
      brandVoice: brandProfile?.brandVoice ?? '',
      brandArchetype: brandProfile?.brandArchetype ?? '',
      targetAudience: brandProfile?.targetAudience ?? '',
      icpDescription: brandProfile?.icpDescription ?? '',
      icpCompanySize: brandProfile?.icpCompanySize ?? '',
      icpIndustries: brandProfile?.icpIndustries ?? '[]',
      competitorBrands: brandProfile?.competitorBrands ?? '[]',
      differentiators: brandProfile?.differentiators ?? '[]',
      contentPillars: brandProfile?.contentPillars ?? '[]',
      contentTone: brandProfile?.contentTone ?? '',
      websiteUrl: brandProfile?.websiteUrl ?? '',
      socialInstagram: brandProfile?.socialInstagram ?? '',
      socialTwitter: brandProfile?.socialTwitter ?? '',
      socialLinkedin: brandProfile?.socialLinkedin ?? '',
      socialFacebook: brandProfile?.socialFacebook ?? '',
      socialYoutube: brandProfile?.socialYoutube ?? '',
      socialTiktok: brandProfile?.socialTiktok ?? '',
    });
    setBrandEditing(true);
  };
  const saveBrand = async () => {
    setBrandSaving(true);
    try {
      await organizationsApi.upsertBrandProfile(orgId!, brandForm);
      qc.invalidateQueries({ queryKey: ['orgBrandProfile', orgId] });
      setBrandEditing(false);
    } finally {
      setBrandSaving(false);
    }
  };
  const bset = (k: keyof OrgBrandProfile, v: string) => setBrandForm(f => ({ ...f, [k]: v }));
  const bgetArr = (k: keyof OrgBrandProfile) => parseJsonArray(brandForm[k] as string).join(', ');
  const bsetArr = (k: keyof OrgBrandProfile, v: string) =>
    bset(k, JSON.stringify(v.split(',').map((s: string) => s.trim()).filter(Boolean)));

  // ── Brand research ──────────────────────────────────────────────────────────
  const [brandResearching, setBrandResearching] = useState(false);
  const [intakeUrl, setIntakeUrl] = useState<string | null>(null);
  const [intakeCopied, setIntakeCopied] = useState(false);

  const triggerResearch = async () => {
    if (!orgId) return;
    setBrandResearching(true);
    try {
      await organizationsApi.triggerBrandResearch(orgId);
      // Poll until done
      const poll = setInterval(async () => {
        const status = await organizationsApi.getBrandResearchStatus(orgId);
        if (status.status === 'done' || status.status === 'failed') {
          clearInterval(poll);
          setBrandResearching(false);
          qc.invalidateQueries({ queryKey: ['orgBrandProfile', orgId] });
        }
      }, 3000);
      // Safety timeout
      setTimeout(() => { clearInterval(poll); setBrandResearching(false); }, 120_000);
    } catch {
      setBrandResearching(false);
    }
  };

  const generateIntake = async () => {
    if (!orgId) return;
    const result = await organizationsApi.generateIntakeToken(orgId);
    setIntakeUrl(result.url);
  };

  const copyIntake = () => {
    if (!intakeUrl) return;
    navigator.clipboard.writeText(intakeUrl);
    setIntakeCopied(true);
    setTimeout(() => setIntakeCopied(false), 2000);
  };

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
      <div className="border-b shadow-sm overflow-hidden">
        {/* Brand accent bar */}
        {brandProfile && (
          <div
            className="h-[3px]"
            style={{ background: `linear-gradient(90deg, ${brandProfile.primaryColor} 0%, ${brandProfile.accentColor ?? brandProfile.secondaryColor} 100%)` }}
          />
        )}
        <div className="bg-card/50 backdrop-blur-sm">
          <div className="max-w-[1600px] mx-auto px-6 py-4">
            <div className="flex items-center gap-4">
              <Link to="/projects" className="text-muted-foreground hover:text-foreground transition-colors shrink-0">
                <ArrowLeft className="h-5 w-5" />
              </Link>

              {/* Brand avatar */}
              {brandProfile?.logoUrl ? (
                <div
                  className="h-11 w-11 rounded-xl shrink-0 shadow-sm overflow-hidden flex items-center justify-center"
                  style={{ backgroundColor: brandProfile.primaryColor }}
                >
                  <img
                    src={resolveApiUrl(brandProfile.logoUrl)}
                    alt={org.name}
                    className="h-full w-full object-contain p-1"
                  />
                </div>
              ) : brandProfile ? (
                <div
                  className="h-11 w-11 rounded-xl flex items-center justify-center text-white text-sm font-bold shrink-0 shadow-sm select-none"
                  style={{ background: `linear-gradient(135deg, ${brandProfile.primaryColor}, ${brandProfile.accentColor ?? brandProfile.secondaryColor})` }}
                >
                  {org.name.split(' ').slice(0, 2).map((w: string) => w[0]).join('').toUpperCase()}
                </div>
              ) : (
                <div className="p-2.5 bg-blue-100 dark:bg-blue-950 rounded-xl shrink-0">
                  <Building2 className="w-6 h-6 text-blue-600" />
                </div>
              )}

              <div className="min-w-0 flex-1">
                <div className="flex items-baseline gap-2.5 flex-wrap">
                  <h1 className="text-2xl font-bold text-foreground leading-none">{org.name}</h1>
                  {brandProfile?.typographyHeading && (
                    <span className="text-xs text-muted-foreground tracking-widest uppercase font-medium">{brandProfile.typographyHeading}</span>
                  )}
                </div>
                {brandProfile?.tagline ? (
                  <p className="text-sm text-muted-foreground mt-0.5 italic truncate">"{brandProfile.tagline}"</p>
                ) : org.description ? (
                  <p className="text-sm text-muted-foreground mt-0.5 truncate">{org.description}</p>
                ) : null}
                {(org as any).address && (
                  <p className="flex items-center gap-1 text-xs text-muted-foreground mt-0.5">
                    <MapPin className="h-3 w-3 shrink-0" />
                    {(org as any).address}
                  </p>
                )}
                {brandProfile && (
                  <div className="flex items-center gap-2.5 mt-1.5 flex-wrap">
                    {/* Colour swatches */}
                    <div className="flex items-center gap-1">
                      <div className="h-3.5 w-3.5 rounded-full border border-border/60 shadow-sm" style={{ backgroundColor: brandProfile.primaryColor }} title={`Primary: ${brandProfile.primaryColor}`} />
                      <div className="h-3.5 w-3.5 rounded-full border border-border/60 shadow-sm" style={{ backgroundColor: brandProfile.secondaryColor }} title={`Secondary: ${brandProfile.secondaryColor}`} />
                      {brandProfile.accentColor && <div className="h-3.5 w-3.5 rounded-full border border-border/60 shadow-sm" style={{ backgroundColor: brandProfile.accentColor }} title={`Accent: ${brandProfile.accentColor}`} />}
                    </div>
                    {brandProfile.industry && <Badge variant="secondary" className="text-[10px] h-4 py-0">{brandProfile.industry}</Badge>}
                    {brandProfile.marketPosition && <Badge variant="outline" className="text-[10px] h-4 py-0 capitalize">{brandProfile.marketPosition}</Badge>}
                    {brandProfile.brandArchetype && <Badge className="text-[10px] h-4 py-0 bg-amber-500/10 text-amber-700 dark:text-amber-400 border-amber-500/20">{brandProfile.brandArchetype}</Badge>}
                  </div>
                )}
              </div>

              <div className="ml-auto flex items-center gap-2 shrink-0">
                <Badge variant={org.is_active ? 'default' : 'secondary'} className="text-xs">
                  {org.is_active ? 'Active' : 'Inactive'}
                </Badge>

                {/* Triage completeness ring */}
                {brandProfile && (() => {
                  const scores = [
                    brandProfile.logoUrl ? 3 : 0,
                    brandProfile.accentColor ? 3 : brandProfile.primaryColor && brandProfile.secondaryColor ? 2 : 1,
                    brandProfile.typographyBody ? 3 : brandProfile.typographyHeading ? 2 : 0,
                    brandProfile.brandVoice && brandProfile.brandArchetype && brandProfile.contentTone ? 3 : brandProfile.brandVoice ? 1 : 0,
                    brandProfile.missionStatement && brandProfile.uniqueValueProposition && brandProfile.brandValues ? 3 : brandProfile.missionStatement ? 1 : 0,
                    brandProfile.targetAudience && brandProfile.icpDescription ? 3 : brandProfile.targetAudience ? 1 : 0,
                    brandProfile.websiteUrl && brandProfile.socialLinkedin && brandProfile.socialInstagram ? 3 : brandProfile.websiteUrl ? 1 : 0,
                  ];
                  const pct = Math.round(scores.reduce((a, b) => a + b, 0) / (scores.length * 3) * 100);
                  const color = pct >= 80 ? '#22c55e' : pct >= 50 ? '#f59e0b' : '#ef4444';
                  const r = 10; const circ = 2 * Math.PI * r;
                  return (
                    <button onClick={openBrandEdit} title={`Brand completeness: ${pct}%`} className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground transition-colors">
                      <svg width="28" height="28" viewBox="0 0 28 28">
                        <circle cx="14" cy="14" r={r} fill="none" stroke="currentColor" strokeWidth="3" className="text-border" />
                        <circle cx="14" cy="14" r={r} fill="none" stroke={color} strokeWidth="3"
                          strokeDasharray={`${circ * pct / 100} ${circ}`}
                          strokeLinecap="round"
                          transform="rotate(-90 14 14)" />
                        <text x="14" y="14" textAnchor="middle" dominantBaseline="central" fontSize="7" fontWeight="600" fill={color}>{pct}%</text>
                      </svg>
                    </button>
                  );
                })()}

                {/* Research button */}
                <Button
                  variant="outline" size="sm"
                  onClick={brandResearching ? undefined : triggerResearch}
                  disabled={brandResearching}
                  className="text-xs gap-1.5"
                  title="Research brand presence with Exa"
                >
                  {brandResearching
                    ? <><Loader2 className="h-3.5 w-3.5 animate-spin" />Researching…</>
                    : <><RefreshCw className="h-3.5 w-3.5" />Research</>
                  }
                </Button>

                {brandProfile ? (
                  <>
                    <Link to={`/organizations/${orgId}/brand-guide`}>
                      <Button variant="outline" size="sm" className="text-xs gap-1.5">
                        <BookOpen className="h-3.5 w-3.5" />
                        View Guide
                      </Button>
                    </Link>
                    <Button variant="ghost" size="sm" onClick={openBrandEdit} className="text-xs gap-1.5 text-muted-foreground">
                      <Pencil className="h-3.5 w-3.5" />
                    </Button>
                  </>
                ) : (
                  <Button variant="outline" size="sm" onClick={openBrandEdit} className="text-xs gap-1.5">
                    <Palette className="h-3.5 w-3.5" />
                    Set Up Brand
                  </Button>
                )}
              </div>
            </div>

            {/* Stat pills */}
            <div className="flex items-center gap-3 mt-4 flex-wrap">
              <StatPill icon={FolderOpen} label="Projects" value={allProjects.length} />
              <StatPill icon={Briefcase} label="Clients" value={clients.length} />
              <StatPill icon={Users} label="Members" value={members.length} />
              <StatPill icon={DollarSign} label="Pipeline" value={formatCurrency(totalDealValue)} />
            </div>
          </div>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex-1 overflow-auto">
        <div className="max-w-[1600px] mx-auto px-6 py-5">
          <Tabs value={tabFromUrl} onValueChange={setTab}>
            <div className="flex items-center justify-between mb-6">
            <TabsList>
              <TabsTrigger value="overview">
                <LayoutGrid className="h-4 w-4 mr-2" />
                Overview
              </TabsTrigger>
              <TabsTrigger value="pipelines">
                <Target className="h-4 w-4 mr-2" />
                Pipelines
              </TabsTrigger>
              <TabsTrigger value="contacts">
                <Contact2 className="h-4 w-4 mr-2" />
                Contacts
              </TabsTrigger>
              <TabsTrigger value="projects">
                <FolderOpen className="h-4 w-4 mr-2" />
                Projects
              </TabsTrigger>
              <TabsTrigger value="social">
                <Share2 className="h-4 w-4 mr-2" />
                Social
              </TabsTrigger>
              <TabsTrigger value="intelligence">
                <Brain className="h-4 w-4 mr-2" />
                Intelligence
              </TabsTrigger>
              <TabsTrigger value="members">
                <Users className="h-4 w-4 mr-2" />
                Members
              </TabsTrigger>
              <TabsTrigger value="integrations">
                <Plug className="h-4 w-4 mr-2" />
                Integrations
              </TabsTrigger>
            </TabsList>
            <Link
              to={`/organizations/${orgId}/data-sources`}
              className="inline-flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground border rounded-md px-3 py-1.5 transition-colors"
            >
              <Database className="h-3.5 w-3.5" />
              Data Library
            </Link>
            </div>

            <TabsContent value="overview">
              <PageErrorBoundary label="Overview">
                <OverviewTab
                  orgId={orgId}
                  orgName={org.name}
                  projectEntries={allProjects.map(p => ({ id: p.id, name: p.name }))}
                  projectCount={allProjects.length}
                  clientCount={clients.length}
                  memberCount={members.length}
                  totalDealValue={totalDealValue}
                  totalDeals={orgDeals.length}
                  contactCount={crmContacts.length}
                />
              </PageErrorBoundary>
            </TabsContent>

            <TabsContent value="pipelines">
              <PageErrorBoundary label="Pipelines">
                <PipelinesTab orgId={orgId} defaultPipeline={pipelineFromUrl || undefined} />
              </PageErrorBoundary>
            </TabsContent>

            <TabsContent value="contacts">
              <PageErrorBoundary label="Contacts">
                <ContactsTab orgId={orgId} />
              </PageErrorBoundary>
            </TabsContent>

            <TabsContent value="projects">
              <PageErrorBoundary label="Projects">
                <ProjectsTab
                  orgId={orgId}
                  sidebarOrg={sidebarOrg}
                  clientFilter={clientFilter}
                  onClearClientFilter={clearClientFilter}
                />
              </PageErrorBoundary>
            </TabsContent>

            <TabsContent value="social">
              <PageErrorBoundary label="Social">
                <SocialTab projectEntries={allProjects.map(p => ({ id: p.id, name: p.name }))} orgId={orgId} />
              </PageErrorBoundary>
            </TabsContent>

            <TabsContent value="intelligence">
              <PageErrorBoundary label="Intelligence">
                <IntelligenceTab
                  projectEntries={allProjects.map(p => ({ id: p.id, name: p.name }))}
                  orgId={orgId}
                />
              </PageErrorBoundary>
            </TabsContent>

            <TabsContent value="members">
              <PageErrorBoundary label="Members">
                <MembersTab orgId={orgId} orgName={org.name} />
              </PageErrorBoundary>
            </TabsContent>

            <TabsContent value="integrations">
              <PageErrorBoundary label="Integrations">
                <IntegrationsTab orgId={orgId} />
              </PageErrorBoundary>
            </TabsContent>
          </Tabs>
        </div>
      </div>

      {/* Brand Guide Edit Dialog */}
      <Dialog open={brandEditing} onOpenChange={setBrandEditing}>
        <DialogContent className="max-w-3xl max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Palette className="h-5 w-5 text-amber-500" />
              Brand Guide — {org.name}
            </DialogTitle>
          </DialogHeader>
          <div className="space-y-6 py-2">
            {/* Visual */}
            <div>
              <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-3 flex items-center gap-1.5"><Palette className="h-3 w-3" /> Visual Identity</p>
              <div className="grid grid-cols-2 gap-3">
                <div><Label className="text-xs">Tagline</Label><Input value={brandForm.tagline ?? ''} onChange={e => bset('tagline', e.target.value)} placeholder="One-liner that captures the brand" /></div>
                <div><Label className="text-xs">Industry</Label><Input value={brandForm.industry ?? ''} onChange={e => bset('industry', e.target.value)} placeholder="e.g. Luxury Agency, SaaS" /></div>
                <div>
                  <Label className="text-xs">Primary Color</Label>
                  <div className="flex gap-2 mt-1">
                    <input type="color" value={brandForm.primaryColor ?? '#2563EB'} onChange={e => bset('primaryColor', e.target.value)} className="h-9 w-12 rounded border cursor-pointer" />
                    <Input value={brandForm.primaryColor ?? ''} onChange={e => bset('primaryColor', e.target.value)} placeholder="#2563EB" className="font-mono" />
                  </div>
                </div>
                <div>
                  <Label className="text-xs">Secondary Color</Label>
                  <div className="flex gap-2 mt-1">
                    <input type="color" value={brandForm.secondaryColor ?? '#EC4899'} onChange={e => bset('secondaryColor', e.target.value)} className="h-9 w-12 rounded border cursor-pointer" />
                    <Input value={brandForm.secondaryColor ?? ''} onChange={e => bset('secondaryColor', e.target.value)} placeholder="#EC4899" className="font-mono" />
                  </div>
                </div>
                <div>
                  <Label className="text-xs">Accent Color</Label>
                  <div className="flex gap-2 mt-1">
                    <input type="color" value={brandForm.accentColor ?? '#000000'} onChange={e => bset('accentColor', e.target.value)} className="h-9 w-12 rounded border cursor-pointer" />
                    <Input value={brandForm.accentColor ?? ''} onChange={e => bset('accentColor', e.target.value)} placeholder="#000000" className="font-mono" />
                  </div>
                </div>
                <div><Label className="text-xs">Heading Font</Label><Input value={brandForm.typographyHeading ?? ''} onChange={e => bset('typographyHeading', e.target.value)} placeholder="e.g. Cinzel, Playfair Display" /></div>
                <div><Label className="text-xs">Body Font</Label><Input value={brandForm.typographyBody ?? ''} onChange={e => bset('typographyBody', e.target.value)} placeholder="e.g. Inter, DM Sans" /></div>
                <div><Label className="text-xs">Logo URL</Label><Input value={brandForm.logoUrl ?? ''} onChange={e => bset('logoUrl', e.target.value)} placeholder="https://..." /></div>
              </div>
            </div>
            {/* Positioning */}
            <div>
              <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-3 flex items-center gap-1.5"><Lightbulb className="h-3 w-3" /> Positioning</p>
              <div className="grid grid-cols-2 gap-3">
                <div className="col-span-2"><Label className="text-xs">Unique Value Proposition</Label><Input value={brandForm.uniqueValueProposition ?? ''} onChange={e => bset('uniqueValueProposition', e.target.value)} placeholder="What makes this brand irreplaceable?" /></div>
                <div className="col-span-2"><Label className="text-xs">Mission Statement</Label><Textarea value={brandForm.missionStatement ?? ''} onChange={e => bset('missionStatement', e.target.value)} placeholder="Why does this brand exist?" rows={2} /></div>
                <div className="col-span-2"><Label className="text-xs">Vision Statement</Label><Textarea value={brandForm.visionStatement ?? ''} onChange={e => bset('visionStatement', e.target.value)} placeholder="Where is this brand going?" rows={2} /></div>
                <div>
                  <Label className="text-xs">Brand Voice</Label>
                  <Select value={brandForm.brandVoice ?? ''} onValueChange={v => bset('brandVoice', v)}>
                    <SelectTrigger><SelectValue placeholder="Select voice" /></SelectTrigger>
                    <SelectContent>{BRAND_VOICE_OPTIONS.map(o => <SelectItem key={o} value={o} className="capitalize">{o}</SelectItem>)}</SelectContent>
                  </Select>
                </div>
                <div>
                  <Label className="text-xs">Brand Archetype</Label>
                  <Select value={brandForm.brandArchetype ?? ''} onValueChange={v => bset('brandArchetype', v)}>
                    <SelectTrigger><SelectValue placeholder="Select archetype" /></SelectTrigger>
                    <SelectContent>{BRAND_ARCHETYPE_OPTIONS.map(o => <SelectItem key={o} value={o}>{o}</SelectItem>)}</SelectContent>
                  </Select>
                </div>
                <div>
                  <Label className="text-xs">Market Position</Label>
                  <Select value={brandForm.marketPosition ?? ''} onValueChange={v => bset('marketPosition', v)}>
                    <SelectTrigger><SelectValue placeholder="Select position" /></SelectTrigger>
                    <SelectContent>{MARKET_POSITION_OPTIONS.map(o => <SelectItem key={o} value={o} className="capitalize">{o}</SelectItem>)}</SelectContent>
                  </Select>
                </div>
                <div><Label className="text-xs">Brand Values <span className="text-muted-foreground font-normal">(comma-separated)</span></Label><Input value={bgetArr('brandValues')} onChange={e => bsetArr('brandValues', e.target.value)} placeholder="Integrity, Innovation, Excellence" /></div>
              </div>
            </div>
            {/* Audience */}
            <div>
              <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-3 flex items-center gap-1.5"><Crosshair className="h-3 w-3" /> Audience & ICP</p>
              <div className="grid grid-cols-2 gap-3">
                <div className="col-span-2"><Label className="text-xs">Target Audience</Label><Textarea value={brandForm.targetAudience ?? ''} onChange={e => bset('targetAudience', e.target.value)} placeholder="Who is this brand speaking to?" rows={2} /></div>
                <div className="col-span-2"><Label className="text-xs">Ideal Customer Profile (ICP)</Label><Textarea value={brandForm.icpDescription ?? ''} onChange={e => bset('icpDescription', e.target.value)} placeholder="Describe the perfect client in detail" rows={3} /></div>
                <div>
                  <Label className="text-xs">ICP Company Size</Label>
                  <Select value={brandForm.icpCompanySize ?? ''} onValueChange={v => bset('icpCompanySize', v)}>
                    <SelectTrigger><SelectValue placeholder="Select size" /></SelectTrigger>
                    <SelectContent>{ICP_COMPANY_SIZE_OPTIONS.map(o => <SelectItem key={o} value={o} className="capitalize">{o}</SelectItem>)}</SelectContent>
                  </Select>
                </div>
                <div><Label className="text-xs">ICP Industries <span className="text-muted-foreground font-normal">(comma-separated)</span></Label><Input value={bgetArr('icpIndustries')} onChange={e => bsetArr('icpIndustries', e.target.value)} placeholder="Hospitality, Real Estate, Fashion" /></div>
              </div>
            </div>
            {/* Competitive */}
            <div>
              <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-3 flex items-center gap-1.5"><Zap className="h-3 w-3" /> Competitive Intelligence</p>
              <div className="grid grid-cols-2 gap-3">
                <div><Label className="text-xs">Competitors <span className="text-muted-foreground font-normal">(comma-separated)</span></Label><Input value={bgetArr('competitorBrands')} onChange={e => bsetArr('competitorBrands', e.target.value)} placeholder="Competitor A, Competitor B" /></div>
                <div><Label className="text-xs">Differentiators <span className="text-muted-foreground font-normal">(comma-separated)</span></Label><Input value={bgetArr('differentiators')} onChange={e => bsetArr('differentiators', e.target.value)} placeholder="End-to-end, Luxury positioning, Speed" /></div>
              </div>
            </div>
            {/* Content & Social */}
            <div>
              <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-3 flex items-center gap-1.5"><Megaphone className="h-3 w-3" /> Content & Social</p>
              <div className="grid grid-cols-2 gap-3">
                <div className="col-span-2"><Label className="text-xs">Content Pillars <span className="text-muted-foreground font-normal">(comma-separated)</span></Label><Input value={bgetArr('contentPillars')} onChange={e => bsetArr('contentPillars', e.target.value)} placeholder="Education, Behind the scenes, Client results" /></div>
                <div className="col-span-2"><Label className="text-xs">Tone Notes</Label><Textarea value={brandForm.contentTone ?? ''} onChange={e => bset('contentTone', e.target.value)} placeholder="Nuances about how this brand communicates" rows={2} /></div>
                <div className="col-span-2"><Label className="text-xs">Website URL</Label><Input value={brandForm.websiteUrl ?? ''} onChange={e => bset('websiteUrl', e.target.value)} placeholder="https://brand.com" /></div>
                <div><Label className="text-xs">Instagram</Label><Input value={brandForm.socialInstagram ?? ''} onChange={e => bset('socialInstagram', e.target.value)} placeholder="@handle" /></div>
                <div><Label className="text-xs">LinkedIn</Label><Input value={brandForm.socialLinkedin ?? ''} onChange={e => bset('socialLinkedin', e.target.value)} placeholder="@handle or company slug" /></div>
                <div><Label className="text-xs">X (Twitter)</Label><Input value={brandForm.socialTwitter ?? ''} onChange={e => bset('socialTwitter', e.target.value)} placeholder="@handle" /></div>
                <div><Label className="text-xs">TikTok</Label><Input value={brandForm.socialTiktok ?? ''} onChange={e => bset('socialTiktok', e.target.value)} placeholder="@handle" /></div>
                <div><Label className="text-xs">YouTube</Label><Input value={brandForm.socialYoutube ?? ''} onChange={e => bset('socialYoutube', e.target.value)} placeholder="@channel" /></div>
                <div><Label className="text-xs">Facebook</Label><Input value={brandForm.socialFacebook ?? ''} onChange={e => bset('socialFacebook', e.target.value)} placeholder="page name or handle" /></div>
              </div>
            </div>
          </div>
          {/* Intake link generator */}
          <div className="border-t pt-4 mt-2">
            <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-2 flex items-center gap-1.5">
              <LinkIcon className="h-3 w-3" /> Client Intake Link
            </p>
            <p className="text-xs text-muted-foreground mb-3">Generate a shareable link so your client can fill in this questionnaire directly.</p>
            {intakeUrl ? (
              <div className="flex items-center gap-2">
                <Input value={intakeUrl} readOnly className="text-xs font-mono h-8 flex-1" />
                <Button size="sm" variant="outline" onClick={copyIntake} className="text-xs shrink-0 gap-1">
                  <CheckCircle2 className={`h-3 w-3 ${intakeCopied ? 'text-green-500' : ''}`} />
                  {intakeCopied ? 'Copied!' : 'Copy'}
                </Button>
              </div>
            ) : (
              <Button size="sm" variant="outline" onClick={generateIntake} className="text-xs gap-1.5">
                <LinkIcon className="h-3.5 w-3.5" /> Generate Intake Link
              </Button>
            )}
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={() => setBrandEditing(false)}>Cancel</Button>
            <Button onClick={saveBrand} disabled={brandSaving}>
              {brandSaving ? <><Loader2 className="h-3 w-3 mr-1.5 animate-spin" />Saving…</> : 'Save Brand Guide'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
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
