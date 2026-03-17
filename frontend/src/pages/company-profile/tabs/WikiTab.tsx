import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Brain, Pencil, Save, X } from 'lucide-react';
import { toast } from 'sonner';
import { companiesApi, type CompanyRecord } from '@/lib/api';

export function WikiTab({
  company,
  onSaved,
}: {
  company: CompanyRecord;
  onSaved: () => void;
}) {
  const [editingNotes, setEditingNotes] = useState(false);
  const [notes, setNotes] = useState(company.notes ?? '');
  const [saving, setSaving] = useState(false);

  const saveNotes = async () => {
    setSaving(true);
    try {
      await companiesApi.update(company.id, { notes });
      toast.success('Wiki notes saved');
      setEditingNotes(false);
      onSaved();
    } catch {
      toast.error('Failed to save notes');
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="space-y-5 max-w-3xl">
      {/* AI Intelligence Summary */}
      {company.intelligence_summary && (
        <Card className="bg-muted/30 border-border/60">
          <CardHeader className="p-4 pb-2">
            <CardTitle className="text-sm font-semibold flex items-center gap-2">
              <Brain className="h-4 w-4 text-blue-500" />
              AI Intelligence Summary
            </CardTitle>
          </CardHeader>
          <CardContent className="p-4 pt-0">
            <p className="text-sm leading-relaxed whitespace-pre-wrap text-muted-foreground">
              {company.intelligence_summary}
            </p>
          </CardContent>
        </Card>
      )}

      {/* Raw facts extracted */}
      {(() => {
        try {
          const raw = JSON.parse(company.intelligence_raw ?? '{}');
          const sections = [
            { key: 'services', label: 'Services / Offerings' },
            { key: 'positioning', label: 'Market Positioning' },
            { key: 'target_audience', label: 'Target Audience' },
            { key: 'differentiators', label: 'Differentiators' },
            { key: 'tone', label: 'Brand Voice / Tone' },
          ];
          const filtered = sections.filter(s => raw[s.key]);
          if (filtered.length === 0) return null;
          return (
            <Card className="border-border/40">
              <CardHeader className="p-4 pb-2">
                <CardTitle className="text-sm font-semibold">Extracted Brand Facts</CardTitle>
              </CardHeader>
              <CardContent className="p-4 pt-0 space-y-3">
                {filtered.map(s => (
                  <div key={s.key}>
                    <p className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-1">{s.label}</p>
                    <p className="text-sm">{Array.isArray(raw[s.key]) ? (raw[s.key] as string[]).join(', ') : String(raw[s.key])}</p>
                  </div>
                ))}
              </CardContent>
            </Card>
          );
        } catch { return null; }
      })()}

      {/* Human-curated notes */}
      <Card>
        <CardHeader className="p-4 pb-2">
          <div className="flex items-center justify-between">
            <CardTitle className="text-sm font-semibold">Human Context & Notes</CardTitle>
            {!editingNotes && (
              <Button variant="ghost" size="sm" className="h-7 text-xs gap-1" onClick={() => setEditingNotes(true)}>
                <Pencil className="h-3.5 w-3.5" />
                Edit
              </Button>
            )}
          </div>
        </CardHeader>
        <CardContent className="p-4 pt-0">
          {editingNotes ? (
            <div className="space-y-2">
              <Textarea
                value={notes}
                onChange={(e) => setNotes(e.target.value)}
                rows={8}
                placeholder="Add context, observations, strategic notes about this company..."
                className="text-sm resize-none"
              />
              <div className="flex gap-2">
                <Button size="sm" className="h-7 text-xs gap-1" onClick={saveNotes} disabled={saving}>
                  <Save className="h-3.5 w-3.5" />
                  Save
                </Button>
                <Button variant="ghost" size="sm" className="h-7 text-xs gap-1" onClick={() => { setEditingNotes(false); setNotes(company.notes ?? ''); }}>
                  <X className="h-3.5 w-3.5" />
                  Cancel
                </Button>
              </div>
            </div>
          ) : notes ? (
            <p className="text-sm leading-relaxed whitespace-pre-wrap">{notes}</p>
          ) : (
            <p className="text-sm text-muted-foreground italic">No additional context added yet. Click Edit to add human-curated notes.</p>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
