import { useState } from 'react';
import { RefreshCw, Save } from 'lucide-react';
import { Button } from '@/components/ui/button';
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
import { toast } from 'sonner';
import { companiesApi, type CompanyRecord } from '@/lib/api';

const EMPLOYEE_OPTIONS = ['1-10', '11-50', '51-200', '201-500', '500+'];

export function EditTab({
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
      // Strip empty strings -> omit so backend treats as no-change
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
            <Label className="text-xs">Rating (0-5)</Label>
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
          placeholder="Private notes visible only to your team\u2026"
          value={val('notes')}
          onChange={e => set('notes', e.target.value)}
        />
      </div>

      <div className="flex gap-2 pb-6">
        <Button onClick={handleSave} disabled={saving}>
          {saving ? <RefreshCw className="h-4 w-4 mr-1 animate-spin" /> : <Save className="h-4 w-4 mr-1" />}
          {saving ? 'Saving\u2026' : 'Save Changes'}
        </Button>
        <Button variant="ghost" onClick={() => setForm({})}>Reset</Button>
      </div>
    </div>
  );
}
