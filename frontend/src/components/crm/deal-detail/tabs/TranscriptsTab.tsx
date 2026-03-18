import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Loader2, Mic, Plus, Link2, Clock } from 'lucide-react';
import { toast } from 'sonner';
import { crmDealsApi } from '@/lib/api/crm';
import { crmKeys } from '@/lib/query-keys';
import type { CrmDealWithContact } from '@/types/crm';
import { formatDistanceToNow } from 'date-fns';

interface TranscriptsTabProps {
  deal: CrmDealWithContact;
}

export function TranscriptsTab({ deal }: TranscriptsTabProps) {
  const qc = useQueryClient();
  const [adding, setAdding] = useState(false);
  const [form, setForm] = useState({ transcript_text: '', summary: '', call_log_id: '' });

  const { data: transcripts, isLoading } = useQuery({
    queryKey: crmKeys.dealTranscripts(deal.id),
    queryFn: () => crmDealsApi.listTranscripts(deal.id),
    staleTime: 30000,
  });

  const link = useMutation({
    mutationFn: () => crmDealsApi.linkTranscript(deal.id, { ...form, matched_by: 'manual' }),
    onSuccess: () => {
      toast.success('Transcript linked');
      qc.invalidateQueries({ queryKey: crmKeys.dealTranscripts(deal.id) });
      setAdding(false);
      setForm({ transcript_text: '', summary: '', call_log_id: '' });
    },
    onError: () => toast.error('Failed to link transcript'),
  });

  return (
    <div className="p-5 space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Mic className="h-4 w-4 text-purple-400" />
          <h3 className="font-semibold text-sm">Discovery Transcripts</h3>
          {transcripts && <span className="text-xs text-muted-foreground">({transcripts.length})</span>}
        </div>
        <Button variant="ghost" size="sm" className="h-7 gap-1 text-xs" onClick={() => setAdding(!adding)}>
          <Plus className="h-3 w-3" /> Link
        </Button>
      </div>

      {/* Add form */}
      {adding && (
        <div className="rounded-lg border p-3 space-y-3 bg-muted/20">
          <p className="text-xs font-medium">Link Transcript</p>
          <div className="space-y-1.5">
            <Label className="text-xs">Call Log ID (optional)</Label>
            <Input
              className="h-7 text-xs"
              placeholder="call_log_id or external ref"
              value={form.call_log_id}
              onChange={(e) => setForm(f => ({ ...f, call_log_id: e.target.value }))}
            />
          </div>
          <div className="space-y-1.5">
            <Label className="text-xs">Summary</Label>
            <Textarea
              className="text-xs min-h-[60px]"
              placeholder="Brief summary of the call…"
              value={form.summary}
              onChange={(e) => setForm(f => ({ ...f, summary: e.target.value }))}
            />
          </div>
          <div className="space-y-1.5">
            <Label className="text-xs">Full Transcript (optional)</Label>
            <Textarea
              className="text-xs min-h-[80px] font-mono"
              placeholder="Paste full transcript here…"
              value={form.transcript_text}
              onChange={(e) => setForm(f => ({ ...f, transcript_text: e.target.value }))}
            />
          </div>
          <div className="flex gap-2">
            <Button size="sm" className="h-7 gap-1 text-xs" onClick={() => link.mutate()} disabled={link.isPending || (!form.summary && !form.transcript_text)}>
              {link.isPending ? <Loader2 className="h-3 w-3 animate-spin" /> : <Link2 className="h-3 w-3" />}
              Link
            </Button>
            <Button variant="ghost" size="sm" className="h-7 text-xs" onClick={() => setAdding(false)}>Cancel</Button>
          </div>
        </div>
      )}

      {/* Transcript list */}
      {isLoading && <div className="flex items-center gap-2 text-xs text-muted-foreground"><Loader2 className="h-3 w-3 animate-spin" /> Loading…</div>}

      {!isLoading && (!transcripts || transcripts.length === 0) && (
        <div className="rounded-lg border border-dashed border-purple-500/30 bg-purple-500/5 p-6 text-center space-y-2">
          <Mic className="h-6 w-6 mx-auto text-purple-400/60" />
          <p className="text-sm font-medium">No transcripts linked</p>
          <p className="text-xs text-muted-foreground">Nora auto-links call transcripts from email intake. You can also link manually above.</p>
        </div>
      )}

      {transcripts && transcripts.length > 0 && (
        <div className="space-y-2">
          {transcripts.map((t) => (
            <div key={t.id} className="rounded-lg border p-3 space-y-1.5">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-1.5 text-[11px] text-muted-foreground">
                  <Clock className="h-3 w-3" />
                  {formatDistanceToNow(new Date(t.created_at), { addSuffix: true })}
                  {t.matched_by && (
                    <span className="bg-purple-500/10 text-purple-400 px-1 rounded text-[10px]">
                      {t.matched_by}
                    </span>
                  )}
                </div>
                {t.call_log_id && (
                  <span className="text-[10px] text-muted-foreground font-mono">{t.call_log_id}</span>
                )}
              </div>
              {t.summary && (
                <p className="text-xs leading-relaxed text-foreground/80">{t.summary}</p>
              )}
              {t.transcript_text && (
                <details className="text-[11px] text-muted-foreground">
                  <summary className="cursor-pointer hover:text-foreground">Full transcript</summary>
                  <pre className="mt-2 whitespace-pre-wrap font-mono leading-relaxed bg-muted/30 rounded p-2 max-h-40 overflow-y-auto">
                    {t.transcript_text}
                  </pre>
                </details>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
