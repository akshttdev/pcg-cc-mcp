import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { BookOpen, Brain, Pencil, Save, X } from 'lucide-react';
import { toast } from 'sonner';
import { organizationsApi, type OrgBrandProfile } from '@/lib/api';
import { Link } from 'react-router-dom';

export function OrgWikiTab({ orgId, orgName }: { orgId: string; orgName: string }) {
  const qc = useQueryClient();

  const { data: org } = useQuery({
    queryKey: ['org', orgId],
    queryFn: () => organizationsApi.getById(orgId),
    enabled: !!orgId,
  });

  const { data: profile } = useQuery<OrgBrandProfile | null>({
    queryKey: ['orgBrandProfile', orgId],
    queryFn: () => organizationsApi.getBrandProfile(orgId),
    enabled: !!orgId,
    staleTime: 5 * 60_000,
  });

  const [editing, setEditing] = useState(false);
  const [desc, setDesc] = useState('');
  const [saving, setSaving] = useState(false);

  const startEdit = () => {
    setDesc((org as any)?.description ?? '');
    setEditing(true);
  };

  const save = async () => {
    setSaving(true);
    try {
      await organizationsApi.update(orgId, { description: desc });
      qc.invalidateQueries({ queryKey: ['org', orgId] });
      qc.invalidateQueries({ queryKey: ['organization', orgId] });
      toast.success('Wiki updated');
      setEditing(false);
    } catch {
      toast.error('Failed to save');
    } finally {
      setSaving(false);
    }
  };

  const brandSections = profile ? [
    { label: 'Tagline', value: profile.tagline },
    { label: 'Mission', value: profile.missionStatement },
    { label: 'Vision', value: profile.visionStatement },
    { label: 'Brand Voice', value: profile.brandVoice },
    { label: 'Market Position', value: profile.marketPosition },
    { label: 'Unique Value', value: profile.uniqueValueProposition },
    { label: 'Target Audience', value: profile.targetAudience },
  ].filter(s => s.value) : [];

  return (
    <div className="space-y-5 max-w-3xl">
      {/* Brand Guide quick link */}
      <div className="flex items-center gap-3">
        <Link to={`/organizations/${orgId}/brand-guide`}>
          <Button variant="outline" size="sm" className="gap-1.5">
            <BookOpen className="h-4 w-4" />
            View Full Brand Guide
          </Button>
        </Link>
      </div>

      {/* Brand profile snapshot */}
      {brandSections.length > 0 && (
        <Card className="bg-muted/30 border-border/60">
          <CardHeader className="p-4 pb-2">
            <CardTitle className="text-sm font-semibold flex items-center gap-2">
              <Brain className="h-4 w-4 text-amber-500" />
              Brand Identity Snapshot
            </CardTitle>
          </CardHeader>
          <CardContent className="p-4 pt-0 space-y-3">
            {brandSections.map(s => (
              <div key={s.label}>
                <p className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-0.5">{s.label}</p>
                <p className="text-sm">{s.value}</p>
              </div>
            ))}
            {profile?.brandValues && (() => {
              try {
                const vals = JSON.parse(profile.brandValues) as string[];
                if (vals.length > 0) return (
                  <div>
                    <p className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-1">Brand Values</p>
                    <div className="flex flex-wrap gap-1.5">
                      {vals.map(v => <span key={v} className="px-2 py-0.5 rounded-full bg-amber-100 dark:bg-amber-950/40 text-amber-700 dark:text-amber-300 text-xs font-medium">{v}</span>)}
                    </div>
                  </div>
                );
              } catch { return null; }
              return null;
            })()}
          </CardContent>
        </Card>
      )}

      {/* Description / human notes */}
      <Card>
        <CardHeader className="p-4 pb-2">
          <div className="flex items-center justify-between">
            <CardTitle className="text-sm font-semibold">About {orgName}</CardTitle>
            {!editing && (
              <Button variant="ghost" size="sm" className="h-7 text-xs gap-1" onClick={startEdit}>
                <Pencil className="h-3.5 w-3.5" />
                Edit
              </Button>
            )}
          </div>
        </CardHeader>
        <CardContent className="p-4 pt-0">
          {editing ? (
            <div className="space-y-2">
              <Textarea
                value={desc}
                onChange={(e) => setDesc(e.target.value)}
                rows={8}
                placeholder="Describe this organization — its purpose, history, notes..."
                className="text-sm resize-none"
              />
              <div className="flex gap-2">
                <Button size="sm" className="h-7 text-xs gap-1" onClick={save} disabled={saving}>
                  <Save className="h-3.5 w-3.5" />
                  Save
                </Button>
                <Button variant="ghost" size="sm" className="h-7 text-xs gap-1" onClick={() => setEditing(false)}>
                  <X className="h-3.5 w-3.5" />
                  Cancel
                </Button>
              </div>
            </div>
          ) : (org as any)?.description ? (
            <p className="text-sm leading-relaxed whitespace-pre-wrap">{(org as any)?.description}</p>
          ) : (
            <p className="text-sm text-muted-foreground italic">No description yet. Click Edit to add one.</p>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
