import { useQuery } from '@tanstack/react-query';
import { formatDistanceToNow } from 'date-fns';
import { Clock, Database, Link2, Loader2, Mic, Plus } from 'lucide-react';
import { useState } from 'react';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { SectionHeader } from '@/components/ui/section-header';
import { Textarea } from '@/components/ui/textarea';
import { useMutationWithToast } from '@/hooks/useMutationWithToast';
import { crmDealsApi } from '@/lib/api/crm';
import { crmKeys } from '@/lib/query-keys';
import type { CrmDealWithContact } from '@/types/crm';

interface TranscriptsTabProps {
  deal: CrmDealWithContact;
}

export function TranscriptsTab({ deal }: TranscriptsTabProps) {
  const [adding, setAdding] = useState(false);
  const [form, setForm] = useState({
    transcript_text: '',
    summary: '',
    call_log_id: '',
  });

  const { data: transcripts, isLoading } = useQuery({
    queryKey: crmKeys.dealTranscripts(deal.id),
    queryFn: () => crmDealsApi.listTranscripts(deal.id),
    staleTime: 30000,
  });

  const link = useMutationWithToast({
    mutationFn: () =>
      crmDealsApi.linkTranscript(deal.id, { ...form, matched_by: 'manual' }),
    successMessage: 'Transcript linked',
    errorMessage: 'Failed to link transcript',
    invalidateKeys: [crmKeys.dealTranscripts(deal.id)],
    onSuccess: () => {
      setAdding(false);
      setForm({ transcript_text: '', summary: '', call_log_id: '' });
    },
  });

  return (
    <div className="p-5 space-y-5">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Mic className="h-4 w-4 text-purple-400" />
          <h3 className="font-semibold text-sm">Discovery Transcripts</h3>
          {transcripts && (
            <span className="text-xs text-muted-foreground">
              ({transcripts.length})
            </span>
          )}
        </div>
        <Button
          variant="ghost"
          size="sm"
          className="h-7 gap-1 text-xs"
          onClick={() => setAdding(!adding)}
        >
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
              onChange={(e) =>
                setForm((f) => ({ ...f, call_log_id: e.target.value }))
              }
            />
          </div>
          <div className="space-y-1.5">
            <Label className="text-xs">Summary</Label>
            <Textarea
              className="text-xs min-h-[60px]"
              placeholder="Brief summary of the call…"
              value={form.summary}
              onChange={(e) =>
                setForm((f) => ({ ...f, summary: e.target.value }))
              }
            />
          </div>
          <div className="space-y-1.5">
            <Label className="text-xs">Full Transcript (optional)</Label>
            <Textarea
              className="text-xs min-h-[80px] font-mono"
              placeholder="Paste full transcript here…"
              value={form.transcript_text}
              onChange={(e) =>
                setForm((f) => ({ ...f, transcript_text: e.target.value }))
              }
            />
          </div>
          <div className="flex gap-2">
            <Button
              size="sm"
              className="h-7 gap-1 text-xs"
              onClick={() => link.mutate()}
              disabled={
                link.isPending || (!form.summary && !form.transcript_text)
              }
            >
              {link.isPending ? (
                <Loader2 className="h-3 w-3 animate-spin" />
              ) : (
                <Link2 className="h-3 w-3" />
              )}
              Link
            </Button>
            <Button
              variant="ghost"
              size="sm"
              className="h-7 text-xs"
              onClick={() => setAdding(false)}
            >
              Cancel
            </Button>
          </div>
        </div>
      )}

      {/* Transcript list */}
      {isLoading && (
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <Loader2 className="h-3 w-3 animate-spin" /> Loading…
        </div>
      )}

      {!isLoading && (!transcripts || transcripts.length === 0) && (
        <div className="rounded-lg border border-dashed border-purple-500/30 bg-purple-500/5 p-6 text-center space-y-2">
          <Mic className="h-6 w-6 mx-auto text-purple-400/60" />
          <p className="text-sm font-medium">No transcripts linked</p>
          <p className="text-xs text-muted-foreground">
            Call transcripts are auto-linked from email intake. You can also
            link manually above.
          </p>
        </div>
      )}

      {transcripts && transcripts.length > 0 && (
        <div className="space-y-2">
          {transcripts.map((t) => (
            <div key={t.id} className="rounded-lg border p-3 space-y-1.5">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
                  <Clock className="h-3 w-3" />
                  {formatDistanceToNow(new Date(t.created_at), {
                    addSuffix: true,
                  })}
                  {t.matched_by && (
                    <span className="bg-purple-500/10 text-purple-400 px-1 rounded text-xs">
                      {t.matched_by}
                    </span>
                  )}
                </div>
                {t.call_log_id && (
                  <span className="text-xs text-muted-foreground font-mono">
                    {t.call_log_id}
                  </span>
                )}
              </div>
              {t.summary && (
                <p className="text-xs leading-relaxed text-foreground/80">
                  {t.summary}
                </p>
              )}
              {t.transcript_text && (
                <details className="text-xs text-muted-foreground">
                  <summary className="cursor-pointer hover:text-foreground">
                    Full transcript
                  </summary>
                  <pre className="mt-2 whitespace-pre-wrap font-mono leading-relaxed bg-muted/30 rounded p-2 max-h-40 overflow-y-auto">
                    {t.transcript_text}
                  </pre>
                </details>
              )}
            </div>
          ))}
        </div>
      )}

      {/* Divider */}
      <div className="border-t" />

      {/* Linked Data Sources */}
      <LinkedDataSourcesSection deal={deal} />
    </div>
  );
}

// ── Linked Data Sources Section ────────────────────────────────────────────

function LinkedDataSourcesSection({ deal }: { deal: CrmDealWithContact }) {
  const [linking, setLinking] = useState(false);
  const [sourceId, setSourceId] = useState('');

  const { data: sources, isLoading } = useQuery({
    queryKey: crmKeys.dealDataSources(deal.id),
    queryFn: () => crmDealsApi.listDataSources(deal.id),
    staleTime: 30000,
  });

  const linkSource = useMutationWithToast({
    mutationFn: () =>
      crmDealsApi.linkDataSource(deal.id, { data_source_id: sourceId }),
    successMessage: 'Data source linked',
    errorMessage: 'Failed to link data source',
    invalidateKeys: [crmKeys.dealDataSources(deal.id)],
    onSuccess: () => {
      setLinking(false);
      setSourceId('');
    },
  });

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <SectionHeader icon={Database} title="Linked Sources" />
        <Button
          variant="ghost"
          size="sm"
          className="h-7 gap-1 text-xs"
          onClick={() => setLinking(!linking)}
        >
          <Plus className="h-3 w-3" /> Link Source
        </Button>
      </div>

      {linking && (
        <div className="rounded-lg border p-3 space-y-3 bg-muted/20">
          <p className="text-xs font-medium">Link Data Source by ID</p>
          <div className="space-y-1.5">
            <Label className="text-xs">Data Source ID</Label>
            <Input
              className="h-7 text-xs"
              placeholder="Paste data source UUID…"
              value={sourceId}
              onChange={(e) => setSourceId(e.target.value)}
            />
          </div>
          <div className="flex gap-2">
            <Button
              size="sm"
              className="h-7 gap-1 text-xs"
              onClick={() => linkSource.mutate()}
              disabled={linkSource.isPending || !sourceId.trim()}
            >
              {linkSource.isPending ? <Loader2 className="h-3 w-3 animate-spin" /> : <Link2 className="h-3 w-3" />}
              Link
            </Button>
            <Button variant="ghost" size="sm" className="h-7 text-xs" onClick={() => setLinking(false)}>
              Cancel
            </Button>
          </div>
        </div>
      )}

      {isLoading && (
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <Loader2 className="h-3 w-3 animate-spin" /> Loading…
        </div>
      )}

      {!isLoading && (!sources || sources.length === 0) && !linking && (
        <p className="text-xs text-muted-foreground">
          No data sources linked. Link sources from the data library to enrich agent context.
        </p>
      )}

      {sources && sources.length > 0 && (
        <div className="space-y-2">
          {sources.map((s) => (
            <div key={s.id} className="rounded-lg border p-3 flex items-center gap-2">
              <Database className="h-3.5 w-3.5 text-blue-400 shrink-0" />
              <span className="text-xs font-mono text-muted-foreground truncate">{s.data_source_id}</span>
              <span className="text-xs text-muted-foreground ml-auto">
                {formatDistanceToNow(new Date(s.created_at), { addSuffix: true })}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
