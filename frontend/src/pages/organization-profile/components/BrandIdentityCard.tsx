import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
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
import {
  Globe,
  Pencil,
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
  Linkedin,
  Instagram,
  Twitter,
  Facebook,
  Youtube,
} from 'lucide-react';
import {
  organizationsApi,
  type OrgBrandProfile,
} from '@/lib/api';
import { organizationKeys } from '@/lib/query-keys';
import { parseJsonArray } from '../helpers';
import {
  BRAND_VOICE_OPTIONS,
  BRAND_ARCHETYPE_OPTIONS,
  MARKET_POSITION_OPTIONS,
  ICP_COMPANY_SIZE_OPTIONS,
} from '../constants';

export function BrandIdentityCard({ orgId, orgName }: { orgId: string; orgName: string }) {
  const qc = useQueryClient();
  const { data: profile, isLoading } = useQuery<OrgBrandProfile | null>({
    queryKey: organizationKeys.brandProfile(orgId),
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
      qc.invalidateQueries({ queryKey: organizationKeys.brandProfile(orgId) });
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
                className="h-16 w-16 rounded-xl flex items-center justify-center text-white text-xl font-semibold shrink-0 shadow"
                style={{ background: `linear-gradient(135deg, ${profile.primaryColor}, ${profile.secondaryColor})` }}
              >
                {initials}
              </div>
              <div className="min-w-0">
                <p className="font-semibold text-lg leading-tight">{orgName}</p>
                {profile.tagline && <p className="text-muted-foreground text-sm mt-0.5 italic">"{profile.tagline}"</p>}
                <div className="flex items-center gap-2 flex-wrap mt-1.5">
                  {profile.industry && <Badge variant="secondary" className="text-xs">{profile.industry}</Badge>}
                  {profile.marketPosition && <Badge variant="outline" className="text-xs capitalize">{profile.marketPosition}</Badge>}
                  {profile.brandArchetype && <Badge className="text-xs bg-[hsl(var(--brand))]/10 text-[hsl(var(--brand))] border-[hsl(var(--brand))]/20">{profile.brandArchetype}</Badge>}
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
                    <p className="text-xs text-muted-foreground">Primary</p>
                    <p className="text-xs font-mono font-medium">{profile.primaryColor}</p>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <div className="h-8 w-8 rounded-md border shadow-sm shrink-0" style={{ backgroundColor: profile.secondaryColor }} />
                  <div>
                    <p className="text-xs text-muted-foreground">Secondary</p>
                    <p className="text-xs font-mono font-medium">{profile.secondaryColor}</p>
                  </div>
                </div>
                {profile.accentColor && (
                  <div className="flex items-center gap-2">
                    <div className="h-8 w-8 rounded-md border shadow-sm shrink-0" style={{ backgroundColor: profile.accentColor }} />
                    <div>
                      <p className="text-xs text-muted-foreground">Accent</p>
                      <p className="text-xs font-mono font-medium">{profile.accentColor}</p>
                    </div>
                  </div>
                )}
                {profile.typographyHeading && (
                  <div className="flex items-center gap-2 pt-1">
                    <Type className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                    <div>
                      <p className="text-xs text-muted-foreground">Heading Font</p>
                      <p className="text-xs font-medium">{profile.typographyHeading}</p>
                    </div>
                  </div>
                )}
                {profile.typographyBody && (
                  <div className="flex items-center gap-2">
                    <Type className="h-3.5 w-3.5 text-muted-foreground shrink-0 opacity-60" />
                    <div>
                      <p className="text-xs text-muted-foreground">Body Font</p>
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
                    <p className="text-xs text-muted-foreground mb-0.5">Unique Value Proposition</p>
                    <p className="text-xs leading-relaxed">{profile.uniqueValueProposition}</p>
                  </div>
                )}
                {profile.missionStatement && (
                  <div>
                    <p className="text-xs text-muted-foreground mb-0.5">Mission</p>
                    <p className="text-xs leading-relaxed">{profile.missionStatement}</p>
                  </div>
                )}
                {profile.visionStatement && (
                  <div>
                    <p className="text-xs text-muted-foreground mb-0.5">Vision</p>
                    <p className="text-xs leading-relaxed">{profile.visionStatement}</p>
                  </div>
                )}
                {values.length > 0 && (
                  <div>
                    <p className="text-xs text-muted-foreground mb-1">Brand Values</p>
                    <div className="flex flex-wrap gap-1">
                      {values.map(v => <Badge key={v} variant="outline" className="text-xs">{v}</Badge>)}
                    </div>
                  </div>
                )}
                {profile.brandVoice && (
                  <div className="flex items-center gap-2">
                    <Mic className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                    <div>
                      <p className="text-xs text-muted-foreground">Brand Voice</p>
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
                    <p className="text-xs text-muted-foreground mb-0.5">Target Audience</p>
                    <p className="text-xs leading-relaxed">{profile.targetAudience}</p>
                  </div>
                )}
                {profile.icpDescription && (
                  <div>
                    <p className="text-xs text-muted-foreground mb-0.5">Ideal Customer Profile</p>
                    <p className="text-xs leading-relaxed">{profile.icpDescription}</p>
                  </div>
                )}
                {profile.icpCompanySize && (
                  <div className="flex items-center gap-2">
                    <Tag className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                    <div>
                      <p className="text-xs text-muted-foreground">Company Size</p>
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
                      <p className="text-xs text-muted-foreground mb-1">Content Pillars</p>
                      <div className="flex flex-wrap gap-1">
                        {pillars.map(p => <Badge key={p} variant="secondary" className="text-xs">{p}</Badge>)}
                      </div>
                    </div>
                  )}
                  {profile.contentTone && (
                    <div>
                      <p className="text-xs text-muted-foreground mb-0.5">Tone Notes</p>
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
                      <p className="text-xs text-muted-foreground mb-1">Competitors</p>
                      <div className="flex flex-wrap gap-1">
                        {competitors.map(c => <Badge key={c} variant="outline" className="text-xs border-destructive/30 text-destructive">{c}</Badge>)}
                      </div>
                    </div>
                  )}
                  {differentiators.length > 0 && (
                    <div>
                      <p className="text-xs text-muted-foreground mb-1">Differentiators</p>
                      <div className="flex flex-wrap gap-1">
                        {differentiators.map(d => <Badge key={d} variant="outline" className="text-xs border-emerald-500/30 text-emerald-600">{d}</Badge>)}
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
