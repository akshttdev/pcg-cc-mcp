import { useState } from 'react';
import { useParams } from 'react-router-dom';
import { useQuery, useMutation } from '@tanstack/react-query';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { Label } from '@/components/ui/label';
import { Badge } from '@/components/ui/badge';
import {
  Select, SelectContent, SelectItem, SelectTrigger, SelectValue
} from '@/components/ui/select';
import { CheckCircle2, Loader2, Palette, Building2, Globe, Users, Zap, Megaphone, Lightbulb } from 'lucide-react';
import { intakeApi } from '@/lib/api';
import { reviewKeys } from '@/lib/query-keys';

const VOICE_OPTIONS = ['formal', 'casual', 'playful', 'authoritative', 'bold', 'sophisticated'];
const ARCHETYPE_OPTIONS = ['Hero', 'Creator', 'Sage', 'Outlaw', 'Explorer', 'Ruler', 'Caregiver', 'Innocent', 'Jester', 'Lover', 'Magician', 'Regular Guy'];
const POSITION_OPTIONS = ['luxury', 'premium', 'mid-market', 'budget'];
const ICP_SIZE_OPTIONS = ['solo', 'startup', 'smb', 'mid-market', 'enterprise'];

type FormData = Record<string, string>;

const SECTIONS = [
  {
    id: 'foundation',
    icon: Building2,
    title: 'Foundation',
    description: 'The core identity of your brand.',
    fields: [
      { key: 'tagline', label: 'Tagline / Elevator Pitch', placeholder: 'One punchy sentence that captures who you are', multiline: false },
      { key: 'mission_statement', label: 'Mission Statement', placeholder: 'Why does your brand exist?', multiline: true },
      { key: 'vision_statement', label: 'Vision Statement', placeholder: 'Where is your brand going?', multiline: true },
      { key: 'unique_value_proposition', label: 'Unique Value Proposition', placeholder: 'What makes you irreplaceable?', multiline: false },
      { key: 'industry', label: 'Industry', placeholder: 'e.g. Luxury Hospitality, Creative Agency, SaaS', multiline: false },
    ],
  },
  {
    id: 'personality',
    icon: Palette,
    title: 'Brand Personality',
    description: 'How your brand sounds, feels, and presents itself.',
    fields: [],
    selects: [
      { key: 'brand_voice', label: 'Brand Voice', options: VOICE_OPTIONS },
      { key: 'brand_archetype', label: 'Brand Archetype', options: ARCHETYPE_OPTIONS },
      { key: 'market_position', label: 'Market Position', options: POSITION_OPTIONS },
    ],
    textFields: [
      { key: 'brand_values', label: 'Core Values', placeholder: 'e.g. Excellence, Integrity, Innovation (comma-separated)', multiline: false },
      { key: 'content_tone', label: 'Tone Notes', placeholder: '"We are bold but not reckless. Professional but never stuffy."', multiline: true },
    ],
  },
  {
    id: 'audience',
    icon: Users,
    title: 'Audience & ICP',
    description: 'Who you serve and who your ideal client is.',
    fields: [
      { key: 'target_audience', label: 'Target Audience', placeholder: 'Describe your primary and secondary audiences', multiline: true },
      { key: 'icp_description', label: 'Ideal Customer Profile', placeholder: 'Paint a detailed picture — their pain points, goals, budget, context', multiline: true },
      { key: 'icp_industries', label: 'ICP Industries', placeholder: 'e.g. Hospitality, Real Estate, Fashion (comma-separated)', multiline: false },
    ],
    selects: [
      { key: 'icp_company_size', label: 'ICP Company Size', options: ICP_SIZE_OPTIONS },
    ],
  },
  {
    id: 'competitive',
    icon: Zap,
    title: 'Competitive Landscape',
    description: 'Who you\'re up against and how you win.',
    fields: [
      { key: 'competitor_brands', label: 'Top Competitors', placeholder: 'Competitor A, Competitor B, Competitor C (comma-separated)', multiline: false },
      { key: 'differentiators', label: 'Your Differentiators', placeholder: 'What makes you win against these competitors? (comma-separated)', multiline: false },
    ],
  },
  {
    id: 'visual',
    icon: Lightbulb,
    title: 'Visual Identity',
    description: 'Colours, fonts, and logo — what you already have.',
    colorFields: [
      { key: 'primary_color', label: 'Primary Colour', default: '#000000' },
      { key: 'secondary_color', label: 'Secondary Colour', default: '#ffffff' },
      { key: 'accent_color', label: 'Accent Colour', default: '#AF9041' },
    ],
    fields: [
      { key: 'typography_heading', label: 'Heading Font', placeholder: 'e.g. Cinzel, Playfair Display, Montserrat', multiline: false },
      { key: 'typography_body', label: 'Body Font', placeholder: 'e.g. Inter, DM Sans, Source Serif Pro', multiline: false },
    ],
  },
  {
    id: 'online',
    icon: Globe,
    title: 'Online Presence',
    description: 'Where you live on the internet.',
    fields: [
      { key: 'website_url', label: 'Website URL', placeholder: 'https://yourbrand.com', multiline: false },
      { key: 'social_instagram', label: 'Instagram', placeholder: '@handle', multiline: false },
      { key: 'social_linkedin', label: 'LinkedIn', placeholder: 'company slug or @handle', multiline: false },
      { key: 'social_twitter', label: 'X / Twitter', placeholder: '@handle', multiline: false },
      { key: 'social_tiktok', label: 'TikTok', placeholder: '@handle', multiline: false },
      { key: 'social_youtube', label: 'YouTube', placeholder: '@channel', multiline: false },
      { key: 'social_facebook', label: 'Facebook', placeholder: 'page name', multiline: false },
    ],
  },
  {
    id: 'content',
    icon: Megaphone,
    title: 'Content Strategy',
    description: 'What you talk about and how often.',
    fields: [
      { key: 'content_pillars', label: 'Content Pillars', placeholder: 'e.g. Education, Behind the Scenes, Client Results, Culture (comma-separated)', multiline: false },
    ],
  },
];

export function BrandIntakePage() {
  const { token } = useParams<{ token: string }>();
  const [form, setForm] = useState<FormData>({});
  const [submitted, setSubmitted] = useState(false);

  const { data: context, isLoading, error } = useQuery({
    queryKey: reviewKeys.intake(token!),
    queryFn: () => intakeApi.getContext(token!),
    enabled: !!token,
    retry: false,
  });

  const { mutate: submit, isPending: submitting } = useMutation({
    mutationFn: (data: FormData) => intakeApi.submit(token!, data),
    onSuccess: () => setSubmitted(true),
  });

  const set = (key: string, value: string) => setForm(f => ({ ...f, [key]: value }));
  const get = (key: string) => form[key] ?? (context?.existing?.[key] as string) ?? '';

  if (isLoading) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (error || !context) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <Card className="max-w-md w-full mx-4">
          <CardContent className="pt-8 pb-8 text-center">
            <div className="h-12 w-12 rounded-full bg-red-100 flex items-center justify-center mx-auto mb-4">
              <Zap className="h-6 w-6 text-red-500" />
            </div>
            <h2 className="text-lg font-semibold mb-2">Link not found</h2>
            <p className="text-sm text-muted-foreground">This intake link is invalid or has expired. Please request a new one from your contact at Powerclub Global.</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  if (submitted) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <Card className="max-w-lg w-full mx-4">
          <CardContent className="pt-10 pb-10 text-center">
            <div className="h-16 w-16 rounded-full bg-green-100 flex items-center justify-center mx-auto mb-5">
              <CheckCircle2 className="h-8 w-8 text-green-500" />
            </div>
            <h2 className="text-2xl font-semibold mb-2">You're all set!</h2>
            <p className="text-muted-foreground text-sm max-w-sm mx-auto">
              Your brand information has been received by the Powerclub Global team. We'll be in touch shortly.
            </p>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-background">
      {/* Header */}
      <div className="border-b bg-card/80 backdrop-blur-sm sticky top-0 z-10">
        <div className="max-w-3xl mx-auto px-6 py-4 flex items-center gap-3">
          <div className="h-8 w-8 rounded-lg bg-gradient-to-br from-black to-amber-700 flex items-center justify-center text-white text-xs font-semibold">
            PCG
          </div>
          <div>
            <p className="text-xs text-muted-foreground">Brand Discovery — Powerclub Global</p>
            <h1 className="text-sm font-bold">{context.orgName}</h1>
          </div>
          <Badge variant="outline" className="ml-auto text-xs">Brand Intake</Badge>
        </div>
      </div>

      <div className="max-w-3xl mx-auto px-6 py-8 space-y-8">
        {/* Intro */}
        <div>
          <h2 className="text-2xl font-semibold mb-2">Brand Discovery Questionnaire</h2>
          <p className="text-muted-foreground text-sm leading-relaxed">
            This questionnaire helps our team build a complete picture of your brand — identity, personality, audience, and online presence.
            Fill in what you know; leave anything blank if you're unsure. We'll fill the gaps together.
          </p>
        </div>

        {/* Sections */}
        {SECTIONS.map(section => {
          const Icon = section.icon;
          return (
            <Card key={section.id} className="border-border/50">
              <CardHeader className="pb-3">
                <CardTitle className="text-base flex items-center gap-2">
                  <Icon className="h-4 w-4 text-amber-600" />
                  {section.title}
                </CardTitle>
                <CardDescription>{section.description}</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                {/* Text fields */}
                {section.fields?.map(field => (
                  <div key={field.key}>
                    <Label className="text-xs font-medium">{field.label}</Label>
                    {field.multiline ? (
                      <Textarea
                        value={get(field.key)}
                        onChange={e => set(field.key, e.target.value)}
                        placeholder={field.placeholder}
                        rows={3}
                        className="mt-1"
                      />
                    ) : (
                      <Input
                        value={get(field.key)}
                        onChange={e => set(field.key, e.target.value)}
                        placeholder={field.placeholder}
                        className="mt-1"
                      />
                    )}
                  </div>
                ))}

                {/* Select fields */}
                {'selects' in section && section.selects?.map(sel => (
                  <div key={sel.key}>
                    <Label className="text-xs font-medium">{sel.label}</Label>
                    <Select value={get(sel.key)} onValueChange={v => set(sel.key, v)}>
                      <SelectTrigger className="mt-1">
                        <SelectValue placeholder={`Select ${sel.label.toLowerCase()}`} />
                      </SelectTrigger>
                      <SelectContent>
                        {sel.options.map(o => (
                          <SelectItem key={o} value={o.toLowerCase()} className="capitalize">{o}</SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                ))}

                {/* Additional text fields after selects */}
                {'textFields' in section && section.textFields?.map(field => (
                  <div key={field.key}>
                    <Label className="text-xs font-medium">{field.label}</Label>
                    {field.multiline ? (
                      <Textarea value={get(field.key)} onChange={e => set(field.key, e.target.value)} placeholder={field.placeholder} rows={3} className="mt-1" />
                    ) : (
                      <Input value={get(field.key)} onChange={e => set(field.key, e.target.value)} placeholder={field.placeholder} className="mt-1" />
                    )}
                  </div>
                ))}

                {/* Color pickers */}
                {'colorFields' in section && section.colorFields?.map(cf => (
                  <div key={cf.key}>
                    <Label className="text-xs font-medium">{cf.label}</Label>
                    <div className="flex gap-2 mt-1">
                      <input
                        type="color"
                        value={get(cf.key) || cf.default}
                        onChange={e => set(cf.key, e.target.value)}
                        className="h-9 w-12 rounded border cursor-pointer"
                      />
                      <Input
                        value={get(cf.key) || cf.default}
                        onChange={e => set(cf.key, e.target.value)}
                        placeholder={cf.default}
                        className="font-mono"
                      />
                    </div>
                  </div>
                ))}
              </CardContent>
            </Card>
          );
        })}

        {/* Submit */}
        <div className="pb-12">
          <Button
            onClick={() => submit(form)}
            disabled={submitting}
            className="w-full h-11 text-base"
          >
            {submitting
              ? <><Loader2 className="h-4 w-4 mr-2 animate-spin" />Submitting…</>
              : 'Submit Brand Questionnaire'
            }
          </Button>
          <p className="text-xs text-muted-foreground text-center mt-3">
            Your information is private and only accessible to the Powerclub Global team.
          </p>
        </div>
      </div>
    </div>
  );
}
