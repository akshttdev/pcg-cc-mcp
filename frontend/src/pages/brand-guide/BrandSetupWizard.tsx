import type { QueryKey } from '@tanstack/react-query';
import { Loader2, Palette, Sparkles, Wand2 } from 'lucide-react';
import { useState } from 'react';

import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { useMutationWithToast } from '@/hooks/useMutationWithToast';
import { organizationsApi, type OrgBrandProfile } from '@/lib/api';
import { organizationKeys } from '@/lib/query-keys';

export interface BrandApiOverride {
  upsertBrandProfile: (
    id: string,
    data: Partial<OrgBrandProfile>
  ) => Promise<OrgBrandProfile>;
  triggerBrandResearch?: (id: string) => Promise<unknown>;
  invalidateKey: QueryKey;
}

interface BrandSetupWizardProps {
  orgId: string;
  orgName: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  apiOverride?: BrandApiOverride;
}

export function BrandSetupWizard({
  orgId,
  orgName,
  open,
  onOpenChange,
  apiOverride,
}: BrandSetupWizardProps) {
  const [step, setStep] = useState(0);
  const [form, setForm] = useState({
    tagline: '',
    primaryColor: '#000000',
    secondaryColor: '#FFFFFF',
    accentColor: '#AF9041',
    typographyHeading: '',
    typographyBody: '',
    missionStatement: '',
    brandVoice: '',
    websiteUrl: '',
    industry: '',
  });

  const set =
    (field: string) =>
    (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) =>
      setForm((prev) => ({ ...prev, [field]: e.target.value }));

  const upsertFn =
    apiOverride?.upsertBrandProfile ?? organizationsApi.upsertBrandProfile;
  const invalidateKey =
    apiOverride?.invalidateKey ?? organizationKeys.brandProfile(orgId);

  const upsertMutation = useMutationWithToast({
    mutationFn: (data: Partial<OrgBrandProfile>) => upsertFn(orgId, data),
    successMessage: 'Brand profile created',
    errorMessage: 'Failed to create brand profile',
    invalidateKeys: [invalidateKey],
    onSuccess: () => {
      onOpenChange(false);
    },
  });

  const researchFn =
    apiOverride?.triggerBrandResearch ?? organizationsApi.triggerBrandResearch;

  const researchMutation = useMutationWithToast({
    mutationFn: () => researchFn(orgId),
    successMessage: 'Brand research started -- results will appear shortly',
    errorMessage: 'Failed to start brand research',
    invalidateKeys: [invalidateKey],
    onSuccess: () => {
      onOpenChange(false);
    },
  });

  const steps = [
    {
      title: 'Brand Identity',
      content: (
        <div className="space-y-4">
          <div>
            <Label htmlFor="tagline">Tagline</Label>
            <Input
              id="tagline"
              placeholder="Your brand in one line"
              value={form.tagline}
              onChange={set('tagline')}
            />
          </div>
          <div>
            <Label htmlFor="industry">Industry</Label>
            <Input
              id="industry"
              placeholder="e.g. Technology, Fashion, Finance"
              value={form.industry}
              onChange={set('industry')}
            />
          </div>
          <div>
            <Label htmlFor="mission">Mission Statement</Label>
            <Textarea
              id="mission"
              placeholder="Why does your brand exist?"
              value={form.missionStatement}
              onChange={set('missionStatement')}
              rows={3}
            />
          </div>
          <div>
            <Label htmlFor="voice">Brand Voice</Label>
            <Input
              id="voice"
              placeholder="e.g. Professional, friendly, bold"
              value={form.brandVoice}
              onChange={set('brandVoice')}
            />
          </div>
        </div>
      ),
    },
    {
      title: 'Colors & Typography',
      content: (
        <div className="space-y-4">
          <div className="grid grid-cols-3 gap-3">
            <div>
              <Label htmlFor="primary">Primary</Label>
              <div className="flex gap-2 items-center mt-1">
                <input
                  type="color"
                  id="primary"
                  value={form.primaryColor}
                  onChange={set('primaryColor')}
                  className="h-8 w-8 rounded cursor-pointer border"
                />
                <Input
                  value={form.primaryColor}
                  onChange={set('primaryColor')}
                  className="font-mono text-xs"
                />
              </div>
            </div>
            <div>
              <Label htmlFor="secondary">Secondary</Label>
              <div className="flex gap-2 items-center mt-1">
                <input
                  type="color"
                  id="secondary"
                  value={form.secondaryColor}
                  onChange={set('secondaryColor')}
                  className="h-8 w-8 rounded cursor-pointer border"
                />
                <Input
                  value={form.secondaryColor}
                  onChange={set('secondaryColor')}
                  className="font-mono text-xs"
                />
              </div>
            </div>
            <div>
              <Label htmlFor="accent">Accent</Label>
              <div className="flex gap-2 items-center mt-1">
                <input
                  type="color"
                  id="accent"
                  value={form.accentColor}
                  onChange={set('accentColor')}
                  className="h-8 w-8 rounded cursor-pointer border"
                />
                <Input
                  value={form.accentColor}
                  onChange={set('accentColor')}
                  className="font-mono text-xs"
                />
              </div>
            </div>
          </div>
          <div className="flex gap-2 mt-2">
            {[form.primaryColor, form.secondaryColor, form.accentColor].map(
              (c, i) => (
                <div
                  key={i}
                  className="h-12 flex-1 rounded-md border"
                  style={{ backgroundColor: c }}
                />
              )
            )}
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div>
              <Label htmlFor="headingFont">Heading Font</Label>
              <Input
                id="headingFont"
                placeholder="e.g. Playfair Display"
                value={form.typographyHeading}
                onChange={set('typographyHeading')}
              />
            </div>
            <div>
              <Label htmlFor="bodyFont">Body Font</Label>
              <Input
                id="bodyFont"
                placeholder="e.g. Inter, Roboto"
                value={form.typographyBody}
                onChange={set('typographyBody')}
              />
            </div>
          </div>
        </div>
      ),
    },
    {
      title: 'Web Presence',
      content: (
        <div className="space-y-4">
          <div>
            <Label htmlFor="website">Website URL</Label>
            <Input
              id="website"
              placeholder="https://example.com"
              value={form.websiteUrl}
              onChange={set('websiteUrl')}
            />
          </div>
          <div className="rounded-lg border border-dashed p-4 text-center space-y-3">
            <Sparkles className="h-8 w-8 mx-auto text-yellow-500" />
            <p className="text-sm font-medium">Auto-fill with AI Research</p>
            <p className="text-xs text-muted-foreground">
              Nora can analyze your website and public presence to automatically
              populate your brand profile with colors, fonts, positioning, and
              more.
            </p>
            <Button
              variant="outline"
              size="sm"
              onClick={() => researchMutation.mutate()}
              disabled={researchMutation.isPending || !form.websiteUrl}
            >
              {researchMutation.isPending ? (
                <>
                  <Loader2 className="h-3 w-3 animate-spin mr-1" />{' '}
                  Researching...
                </>
              ) : (
                <>
                  <Wand2 className="h-3 w-3 mr-1" /> Run Brand Research
                </>
              )}
            </Button>
          </div>
        </div>
      ),
    },
  ];

  const isLast = step === steps.length - 1;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Palette className="h-5 w-5" />
            Set Up Brand Guide -- {orgName}
          </DialogTitle>
          <DialogDescription>
            Step {step + 1} of {steps.length}: {steps[step].title}
          </DialogDescription>
        </DialogHeader>
        <div className="flex gap-1 mb-2">
          {steps.map((_, i) => (
            <div
              key={i}
              className={`h-1 flex-1 rounded-full transition-colors ${
                i <= step ? 'bg-primary' : 'bg-muted'
              }`}
            />
          ))}
        </div>
        {steps[step].content}
        <div className="flex justify-between pt-4">
          <Button
            variant="ghost"
            onClick={() => setStep((s) => s - 1)}
            disabled={step === 0}
          >
            Back
          </Button>
          <div className="flex gap-2">
            <Button variant="ghost" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            {isLast ? (
              <Button
                onClick={() => {
                  const data: Partial<OrgBrandProfile> = {};
                  if (form.tagline) (data as any).tagline = form.tagline;
                  if (form.primaryColor !== '#000000')
                    (data as any).primaryColor = form.primaryColor;
                  if (form.secondaryColor !== '#FFFFFF')
                    (data as any).secondaryColor = form.secondaryColor;
                  if (form.accentColor !== '#AF9041')
                    (data as any).accentColor = form.accentColor;
                  if (form.typographyHeading)
                    (data as any).typographyHeading = form.typographyHeading;
                  if (form.typographyBody)
                    (data as any).typographyBody = form.typographyBody;
                  if (form.missionStatement)
                    (data as any).missionStatement = form.missionStatement;
                  if (form.brandVoice)
                    (data as any).brandVoice = form.brandVoice;
                  if (form.websiteUrl)
                    (data as any).websiteUrl = form.websiteUrl;
                  if (form.industry) (data as any).industry = form.industry;
                  upsertMutation.mutate(data);
                }}
                disabled={upsertMutation.isPending}
              >
                {upsertMutation.isPending ? (
                  <>
                    <Loader2 className="h-3 w-3 animate-spin mr-1" /> Saving...
                  </>
                ) : (
                  'Create Brand Profile'
                )}
              </Button>
            ) : (
              <Button onClick={() => setStep((s) => s + 1)}>Next</Button>
            )}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
