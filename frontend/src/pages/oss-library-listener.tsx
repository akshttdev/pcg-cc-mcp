import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { formatDistanceToNow } from 'date-fns';
import {
  AlertTriangle,
  Bot,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Clock,
  ExternalLink,
  Eye,
  Loader2,
  Package,
  Plus,
  RefreshCw,
  Trash2,
} from 'lucide-react';
import { useState } from 'react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import { IconButton } from '@/components/ui/icon-button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Skeleton } from '@/components/ui/skeleton';
import { Textarea } from '@/components/ui/textarea';
import { apiClient } from '@/lib/api';
import { ossKeys } from '@/lib/query-keys';

// ── Types ─────────────────────────────────────────────────────────────────────

interface OssLibrary {
  id: string;
  name: string;
  github_owner: string;
  github_repo: string;
  tracked_version: string | null;
  latest_version: string | null;
  last_checked_at: string | null;
  is_active: boolean;
  notes: string | null;
}

interface OssLibraryUpdate {
  id: string;
  library_id: string;
  version: string;
  release_url: string | null;
  release_notes: string | null;
  significance: 'patch' | 'minor' | 'major';
  agent_recommendation: string | null;
  recommendation_status: 'pending' | 'generating' | 'done' | 'failed' | 'dismissed';
  published_at: string | null;
  created_at: string;
}

// ── API helpers ───────────────────────────────────────────────────────────────

const ossApi = {
  listLibraries: () => apiClient.get<OssLibrary[]>('/oss-libraries').then((r) => r.data),
  createLibrary: (body: { name: string; github_owner: string; github_repo: string; notes?: string }) =>
    apiClient.post<OssLibrary>('/oss-libraries', body).then((r) => r.data),
  deleteLibrary: (id: string) => apiClient.delete(`/oss-libraries/${id}`),
  checkNow: (id: string) => apiClient.post(`/oss-libraries/${id}/check`),
  listUpdates: (id: string) => apiClient.get<OssLibraryUpdate[]>(`/oss-libraries/${id}/updates`).then((r) => r.data),
  recentUpdates: () => apiClient.get<OssLibraryUpdate[]>('/oss-updates/recent').then((r) => r.data),
  dismiss: (id: string) => apiClient.patch(`/oss-updates/${id}/dismiss`),
};

// ── Significance badge ────────────────────────────────────────────────────────

function SigBadge({ sig }: { sig: string }) {
  const map: Record<string, string> = {
    major: 'bg-red-100 text-red-700',
    minor: 'bg-yellow-100 text-yellow-700',
    patch: 'bg-blue-100 text-blue-700',
  };
  return (
    <Badge className={`text-xs border-0 ${map[sig] ?? 'bg-muted text-muted-foreground'}`}>
      {sig}
    </Badge>
  );
}

function StatusIcon({ status }: { status: string }) {
  if (status === 'generating') return <Loader2 className="h-3.5 w-3.5 animate-spin text-blue-500" />;
  if (status === 'done') return <CheckCircle2 className="h-3.5 w-3.5 text-green-500" />;
  if (status === 'failed') return <AlertTriangle className="h-3.5 w-3.5 text-red-500" />;
  if (status === 'dismissed') return <Eye className="h-3.5 w-3.5 text-muted-foreground" />;
  return <Clock className="h-3.5 w-3.5 text-muted-foreground" />;
}

// ── Update row with expandable recommendation ─────────────────────────────────

function UpdateRow({ update, libraryName }: { update: OssLibraryUpdate; libraryName?: string }) {
  const [expanded, setExpanded] = useState(false);
  const qc = useQueryClient();

  const dismiss = useMutation({
    mutationFn: () => ossApi.dismiss(update.id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ossKeys.recent() });
      qc.invalidateQueries({ queryKey: ossKeys.updates(update.library_id) });
    },
  });

  return (
    <div className="border rounded-lg overflow-hidden">
      <div
        className="flex items-center gap-3 px-4 py-3 cursor-pointer hover:bg-muted/40"
        onClick={() => setExpanded(e => !e)}
      >
        <div className="shrink-0">
          {expanded ? <ChevronDown className="h-4 w-4 text-muted-foreground" /> : <ChevronRight className="h-4 w-4 text-muted-foreground" />}
        </div>

        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 flex-wrap">
            {libraryName && <span className="font-medium text-sm">{libraryName}</span>}
            <code className="text-xs bg-muted px-1.5 py-0.5 rounded">{update.version}</code>
            <SigBadge sig={update.significance} />
          </div>
          <p className="text-xs text-muted-foreground mt-0.5">
            {update.published_at
              ? formatDistanceToNow(new Date(update.published_at), { addSuffix: true })
              : formatDistanceToNow(new Date(update.created_at), { addSuffix: true })}
          </p>
        </div>

        <div className="flex items-center gap-2 shrink-0">
          <StatusIcon status={update.recommendation_status} />
          {update.release_url && (
            <a
              href={update.release_url}
              target="_blank"
              rel="noreferrer"
              onClick={e => e.stopPropagation()}
              className="text-muted-foreground hover:text-foreground"
            >
              <ExternalLink className="h-3.5 w-3.5" />
            </a>
          )}
        </div>
      </div>

      {expanded && (
        <div className="border-t bg-muted/20 p-4 space-y-4">
          {update.release_notes && (
            <div>
              <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground mb-1">Release Notes</p>
              <pre className="text-xs whitespace-pre-wrap text-foreground/80 max-h-48 overflow-y-auto font-sans">
                {update.release_notes}
              </pre>
            </div>
          )}

          {update.recommendation_status === 'generating' && (
            <div className="flex items-center gap-2 text-sm text-blue-600">
              <Loader2 className="h-4 w-4 animate-spin" />
              Nora is generating the upgrade recommendation…
            </div>
          )}

          {update.recommendation_status === 'done' && update.agent_recommendation && (
            <div>
              <div className="flex items-center gap-2 mb-1">
                <Bot className="h-3.5 w-3.5 text-muted-foreground" />
                <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">Nora Recommendation</p>
              </div>
              <div className="prose prose-sm max-w-none">
                <Textarea
                  readOnly
                  value={update.agent_recommendation}
                  className="text-xs font-mono min-h-[200px] resize-none bg-background"
                />
              </div>
            </div>
          )}

          {update.recommendation_status !== 'dismissed' && (
            <div className="flex justify-end">
              <Button
                variant="ghost"
                size="sm"
                className="text-muted-foreground text-xs"
                onClick={() => dismiss.mutate()}
                disabled={dismiss.isPending}
              >
                Dismiss
              </Button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// ── Library card ──────────────────────────────────────────────────────────────

function LibraryCard({ lib }: { lib: OssLibrary }) {
  const qc = useQueryClient();

  const { data: updates = [] } = useQuery({
    queryKey: ossKeys.updates(lib.id),
    queryFn: () => ossApi.listUpdates(lib.id),
  });

  const checkNow = useMutation({
    mutationFn: () => ossApi.checkNow(lib.id),
    onSuccess: () => {
      setTimeout(() => {
        qc.invalidateQueries({ queryKey: ossKeys.libraries() });
        qc.invalidateQueries({ queryKey: ossKeys.updates(lib.id) });
        qc.invalidateQueries({ queryKey: ossKeys.recent() });
      }, 2000);
    },
  });

  const deleteLib = useMutation({
    mutationFn: () => ossApi.deleteLibrary(lib.id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ossKeys.libraries() }),
  });

  const pendingUpdates = updates.filter((u) => u.recommendation_status !== 'dismissed');
  const hasNew = lib.latest_version && lib.latest_version !== lib.tracked_version;

  return (
    <div className="border rounded-xl overflow-hidden">
      <div className="flex items-center gap-3 px-4 py-3 border-b bg-muted/30">
        <Package className="h-4 w-4 text-muted-foreground shrink-0" />
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className="font-medium text-sm">{lib.name}</span>
            {hasNew && (
              <Badge className="text-xs bg-amber-100 text-amber-700 border-0">new release</Badge>
            )}
            {!lib.is_active && (
              <Badge variant="outline" className="text-xs">paused</Badge>
            )}
          </div>
          <div className="flex items-center gap-2 text-xs text-muted-foreground mt-0.5">
            <a
              href={`https://github.com/${lib.github_owner}/${lib.github_repo}`}
              target="_blank"
              rel="noreferrer"
              className="hover:underline"
            >
              {lib.github_owner}/{lib.github_repo}
            </a>
            {lib.latest_version && (
              <span className="text-foreground/60">· latest: <code>{lib.latest_version}</code></span>
            )}
            {lib.last_checked_at && (
              <span>· checked {formatDistanceToNow(new Date(lib.last_checked_at), { addSuffix: true })}</span>
            )}
          </div>
        </div>

        <div className="flex items-center gap-1 shrink-0">
          <IconButton
            variant="ghost" className="h-7 w-7"
            onClick={() => checkNow.mutate()}
            disabled={checkNow.isPending}
            icon={RefreshCw}
            label="Check now"
            iconClassName={`h-3.5 w-3.5 ${checkNow.isPending ? 'animate-spin' : ''}`}
          />
          <IconButton
            variant="ghost" className="h-7 w-7 text-muted-foreground hover:text-destructive"
            onClick={() => deleteLib.mutate()}
            disabled={deleteLib.isPending}
            icon={Trash2}
            label="Remove"
            iconClassName="h-3.5 w-3.5"
          />
        </div>
      </div>

      {pendingUpdates.length === 0 ? (
        <div className="flex items-center gap-2 px-4 py-4 text-sm text-muted-foreground">
          <CheckCircle2 className="h-4 w-4 text-green-500" />
          Up to date
        </div>
      ) : (
        <div className="divide-y">
          {pendingUpdates.map((u) => (
            <UpdateRow key={u.id} update={u} />
          ))}
        </div>
      )}
    </div>
  );
}

// ── Add Library dialog ────────────────────────────────────────────────────────

function AddLibraryDialog() {
  const qc = useQueryClient();
  const [open, setOpen] = useState(false);
  const [form, setForm] = useState({ name: '', github_owner: '', github_repo: '', notes: '' });

  const create = useMutation({
    mutationFn: () => ossApi.createLibrary({
      name: form.name,
      github_owner: form.github_owner,
      github_repo: form.github_repo,
      notes: form.notes || undefined,
    }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ossKeys.libraries() });
      setOpen(false);
      setForm({ name: '', github_owner: '', github_repo: '', notes: '' });
    },
  });

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button size="sm" variant="outline" className="gap-2">
          <Plus className="h-4 w-4" />
          Add Library
        </Button>
      </DialogTrigger>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>Track a Library</DialogTitle>
        </DialogHeader>
        <div className="space-y-4 pt-2">
          <div className="space-y-1">
            <Label htmlFor="lib-name">Display Name</Label>
            <Input id="lib-name" placeholder="e.g. axum" value={form.name}
              onChange={e => setForm(f => ({ ...f, name: e.target.value }))} />
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1">
              <Label htmlFor="lib-owner">GitHub Owner</Label>
              <Input id="lib-owner" placeholder="tokio-rs" value={form.github_owner}
                onChange={e => setForm(f => ({ ...f, github_owner: e.target.value }))} />
            </div>
            <div className="space-y-1">
              <Label htmlFor="lib-repo">Repository</Label>
              <Input id="lib-repo" placeholder="axum" value={form.github_repo}
                onChange={e => setForm(f => ({ ...f, github_repo: e.target.value }))} />
            </div>
          </div>
          <div className="space-y-1">
            <Label htmlFor="lib-notes">Notes (optional)</Label>
            <Input id="lib-notes" placeholder="e.g. Web framework" value={form.notes}
              onChange={e => setForm(f => ({ ...f, notes: e.target.value }))} />
          </div>
          <DialogFooter className="pt-1">
            <Button variant="ghost" onClick={() => setOpen(false)}>Cancel</Button>
            <Button
              onClick={() => create.mutate()}
              disabled={!form.name || !form.github_owner || !form.github_repo || create.isPending}
            >
              {create.isPending ? <Loader2 className="h-4 w-4 animate-spin" /> : 'Add'}
            </Button>
          </DialogFooter>
        </div>
      </DialogContent>
    </Dialog>
  );
}

// ── Page ──────────────────────────────────────────────────────────────────────

export function OssLibraryListenerPage() {
  const { data: libraries = [], isLoading } = useQuery({
    queryKey: ossKeys.libraries(),
    queryFn: ossApi.listLibraries,
    refetchInterval: 60_000,
  });

  const pendingCount = libraries.filter(
    (l) => l.latest_version && l.latest_version !== l.tracked_version
  ).length;

  if (isLoading) {
    return (
      <div className="p-6 space-y-4 max-w-3xl mx-auto">
        <Skeleton className="h-10 w-72" />
        {[...Array(4)].map((_, i) => (
          <Skeleton key={i} className="h-24 w-full rounded-xl" />
        ))}
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6 max-w-3xl mx-auto">
      {/* Header */}
      <div className="flex items-start justify-between gap-4">
        <div className="flex items-center gap-3">
          <Package className="h-6 w-6 text-muted-foreground" />
          <div>
            <h1 className="text-2xl font-bold">Library Listener</h1>
            <p className="text-sm text-muted-foreground">
              {pendingCount > 0
                ? `${pendingCount} librar${pendingCount === 1 ? 'y has' : 'ies have'} new releases`
                : 'All tracked libraries are up to date'}
            </p>
          </div>
          {pendingCount > 0 && (
            <Badge className="bg-amber-100 text-amber-700 border-0">{pendingCount} pending</Badge>
          )}
        </div>
        <AddLibraryDialog />
      </div>

      {/* Agent workflow note */}
      <div className="flex items-start gap-3 px-4 py-3 bg-blue-50 border border-blue-100 rounded-lg text-sm text-blue-800">
        <Bot className="h-4 w-4 mt-0.5 shrink-0" />
        <p>
          Every new release triggers an agent workflow — Nora analyses the release notes and
          generates upgrade recommendations for the sovereign stack. Check the{' '}
          <span className="font-medium">Agent Flows</span> board to see active workflows.
        </p>
      </div>

      {/* Library cards */}
      {libraries.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-16 text-muted-foreground gap-2">
          <Package className="h-8 w-8" />
          <p className="text-sm">No libraries tracked yet.</p>
          <p className="text-xs">Click "Add Library" to start monitoring a GitHub repository.</p>
        </div>
      ) : (
        <div className="space-y-4">
          {libraries.map((lib) => (
            <LibraryCard key={lib.id} lib={lib} />
          ))}
        </div>
      )}
    </div>
  );
}
