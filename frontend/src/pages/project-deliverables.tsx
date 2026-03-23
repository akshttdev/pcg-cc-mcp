import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  ArrowLeft,
  BookMarked,
  BookOpen,
  CheckCircle,
  Code,
  FileText,
  Image,
  Link,
  Music,
  Package,
  Plus,
  Video,
} from 'lucide-react';
import { useState } from 'react';
import { useNavigate,useParams } from 'react-router-dom';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { EmptyState } from '@/components/ui/empty-state';
import { FormField } from '@/components/ui/form-field';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import { type CreateDeliverableInput, type DeliverableRecord, deliverablesApi, type DeliverableStatus, type DeliverableType,projectsApi } from '@/lib/api';
import { businessKeys,projectKeys } from '@/lib/query-keys';

// ── Helpers ──────────────────────────────────────────────────────────────────

const TYPE_ICONS: Record<string, React.ElementType> = {
  video:    Video,
  audio:    Music,
  graphic:  Image,
  copy:     FileText,
  code:     Code,
  document: BookOpen,
  other:    Package,
};

const STATUS_STAGES: DeliverableStatus[] = [
  'working',
  'internal_review',
  'client_review',
  'revision',
  'client_revision',
  'done',
];

const STATUS_LABELS: Record<string, string> = {
  working:         'Working',
  internal_review: 'Internal Review',
  client_review:   'Client Review',
  revision:        'Revision',
  client_revision: 'Client Revision',
  done:            'Done',
};

const STATUS_COLORS: Record<string, string> = {
  working:         'bg-gray-100 text-gray-600',
  internal_review: 'bg-blue-100 text-blue-700',
  client_review:   'bg-purple-100 text-purple-700',
  revision:        'bg-orange-100 text-orange-700',
  client_revision: 'bg-amber-100 text-amber-700',
  done:            'bg-green-100 text-green-700',
};

const TYPES = ['video', 'audio', 'graphic', 'copy', 'code', 'document', 'other'];

// ── Deliverable Card ──────────────────────────────────────────────────────────

function DeliverableCard({ item, onStatusChange }: {
  item: DeliverableRecord;
  onStatusChange: (id: string, status: DeliverableStatus) => void;
}) {
  const Icon = TYPE_ICONS[item.deliverable_type] ?? Package;
  const currentIdx = STATUS_STAGES.indexOf(item.status as DeliverableStatus);
  const nextStatus = STATUS_STAGES[currentIdx + 1];

  return (
    <Card className="hover:shadow-md transition-shadow">
      <CardContent className="pt-4 space-y-3">
        <div className="flex items-start gap-2">
          <div className="p-1.5 rounded-md bg-muted">
            <Icon className="h-4 w-4 text-muted-foreground" />
          </div>
          <div className="flex-1 min-w-0">
            <p className="font-medium text-sm truncate">{item.title}</p>
            <p className="text-xs text-muted-foreground capitalize">{item.deliverable_type}</p>
          </div>
          <Badge className={`${STATUS_COLORS[item.status] ?? ''} border-0 text-xs shrink-0`}>
            {STATUS_LABELS[item.status] ?? item.status}
          </Badge>
        </div>

        {item.description && (
          <p className="text-xs text-muted-foreground line-clamp-2">{item.description}</p>
        )}

        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          {item.due_date && <span>Due {new Date(item.due_date).toLocaleDateString()}</span>}
          <span className="ml-auto">
            Rev: {item.revision_rounds_used}/{item.revision_rounds_allowed}
          </span>
        </div>

        {item.final_link && (
          <a
            href={item.final_link}
            target="_blank"
            rel="noopener noreferrer"
            className="flex items-center gap-1 text-xs text-blue-600 hover:underline"
          >
            <Link className="h-3 w-3" />
            View Final Deliverable
          </a>
        )}

        {/* Knowledge Graph badge when done */}
        {item.status === 'done' && (
          <div className="flex items-center gap-1 text-xs text-purple-600 bg-purple-50 rounded px-2 py-1">
            <BookMarked className="h-3 w-3" />
            Registered in knowledge graph
          </div>
        )}

        {/* Next stage action */}
        {nextStatus && (
          <Button
            size="sm"
            variant="outline"
            className="w-full h-7 text-xs"
            onClick={() => onStatusChange(item.id, nextStatus)}
          >
            {nextStatus === 'done' ? (
              <><CheckCircle className="h-3 w-3 mr-1.5 text-green-600" />Mark Done</>
            ) : (
              <>Move to {STATUS_LABELS[nextStatus]}</>
            )}
          </Button>
        )}
      </CardContent>
    </Card>
  );
}

// ── Create Dialog ─────────────────────────────────────────────────────────────

function CreateDeliverableDialog({ projectId, open, onClose, onCreated }: {
  projectId: string;
  open: boolean;
  onClose: () => void;
  onCreated: () => void;
}) {
  const [form, setForm] = useState<Partial<CreateDeliverableInput>>({
    project_id: projectId,
    deliverable_type: 'video',
    revision_rounds_allowed: 2,
  });
  const [saving, setSaving] = useState(false);

  const handleSubmit = async () => {
    if (!form.title) return;
    setSaving(true);
    try {
      await deliverablesApi.create({ ...form, project_id: projectId } as CreateDeliverableInput);
      onCreated();
      onClose();
      setForm({ project_id: projectId, deliverable_type: 'video', revision_rounds_allowed: 2 });
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>New Deliverable</DialogTitle>
        </DialogHeader>
        <div className="space-y-4 py-2">
          <FormField label="Title">
            <Input
              className="h-8 text-sm"
              placeholder="e.g. Hero Reel — Sirak Studios"
              value={form.title ?? ''}
              onChange={e => setForm(f => ({ ...f, title: e.target.value }))}
            />
          </FormField>

          <div className="grid grid-cols-2 gap-3">
            <FormField label="Type">
              <Select
                value={form.deliverable_type}
                onValueChange={v => setForm(f => ({ ...f, deliverable_type: v as DeliverableType }))}
              >
                <SelectTrigger className="h-8 text-sm">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {TYPES.map(t => (
                    <SelectItem key={t} value={t} className="capitalize">{t}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </FormField>
            <FormField label="Revisions Allowed">
              <Input
                className="h-8 text-sm"
                type="number"
                min="0"
                max="10"
                value={form.revision_rounds_allowed ?? 2}
                onChange={e => setForm(f => ({ ...f, revision_rounds_allowed: parseInt(e.target.value) }))}
              />
            </FormField>
          </div>

          <FormField label="Description">
            <Textarea
              className="text-sm min-h-[60px]"
              placeholder="Brief for this deliverable…"
              value={form.description ?? ''}
              onChange={e => setForm(f => ({ ...f, description: e.target.value }))}
            />
          </FormField>

          <FormField label="Due Date">
            <Input
              className="h-8 text-sm"
              type="date"
              value={form.due_date ?? ''}
              onChange={e => setForm(f => ({ ...f, due_date: e.target.value }))}
            />
          </FormField>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>Cancel</Button>
          <Button onClick={handleSubmit} disabled={saving || !form.title}>
            {saving ? 'Creating…' : 'Create Deliverable'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// ── Page ──────────────────────────────────────────────────────────────────────

export function ProjectDeliverablesPage() {
  const { projectId } = useParams<{ projectId: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [createOpen, setCreateOpen] = useState(false);
  const [filterStatus, setFilterStatus] = useState<string>('');

  const { data: project } = useQuery({
    queryKey: projectKeys.detail(projectId!),
    queryFn: () => projectsApi.getById(projectId!),
    enabled: !!projectId,
  });

  const { data: deliverables = [], isLoading } = useQuery<DeliverableRecord[]>({
    queryKey: businessKeys.deliverables(projectId!),
    queryFn: () => deliverablesApi.listForProject(projectId!),
    enabled: !!projectId,
  });

  const moveStatus = useMutation({
    mutationFn: ({ id, status }: { id: string; status: DeliverableStatus }) =>
      deliverablesApi.moveStatus(id, status),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: businessKeys.deliverables(projectId!) }),
  });

  if (!projectId) return null;

  const filtered = filterStatus ? deliverables.filter(d => d.status === filterStatus) : deliverables;
  const doneCount = deliverables.filter(d => d.status === 'done').length;

  return (
    <div className="p-6 max-w-5xl mx-auto space-y-4">
      {/* Header */}
      <div className="flex items-center gap-3">
        <Button variant="ghost" size="sm" onClick={() => navigate(-1)}>
          <ArrowLeft className="h-4 w-4 mr-1" />
          Back
        </Button>
        <div className="flex-1">
          <h1 className="text-xl font-bold flex items-center gap-2">
            <FileText className="h-5 w-5 text-blue-600" />
            Deliverables
            {project?.name && <span className="text-muted-foreground font-normal">— {project.name}</span>}
          </h1>
          <p className="text-xs text-muted-foreground mt-0.5">
            {doneCount}/{deliverables.length} complete · Done items auto-register in the knowledge graph
          </p>
        </div>
        <Button size="sm" onClick={() => setCreateOpen(true)}>
          <Plus className="h-3.5 w-3.5 mr-1.5" />
          New Deliverable
        </Button>
      </div>

      {/* Stage filter */}
      <div className="flex gap-2 flex-wrap">
        <Button
          size="sm"
          variant={filterStatus === '' ? 'default' : 'outline'}
          className="h-7 text-xs"
          onClick={() => setFilterStatus('')}
        >
          All ({deliverables.length})
        </Button>
        {STATUS_STAGES.map(s => {
          const count = deliverables.filter(d => d.status === s).length;
          return (
            <Button
              key={s}
              size="sm"
              variant={filterStatus === s ? 'default' : 'outline'}
              className="h-7 text-xs"
              onClick={() => setFilterStatus(filterStatus === s ? '' : s)}
            >
              {STATUS_LABELS[s]} ({count})
            </Button>
          );
        })}
      </div>

      {/* Grid */}
      {isLoading ? (
        <p className="text-sm text-muted-foreground">Loading…</p>
      ) : filtered.length === 0 ? (
        <EmptyState
          icon={FileText}
          title="No deliverables yet"
          description="Create the first deliverable to start tracking outputs"
          action={{ label: 'New Deliverable', onClick: () => setCreateOpen(true) }}
        />
      ) : (
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
          {filtered.map(item => (
            <DeliverableCard
              key={item.id}
              item={item}
              onStatusChange={(id, status) => moveStatus.mutate({ id, status })}
            />
          ))}
        </div>
      )}

      <CreateDeliverableDialog
        projectId={projectId}
        open={createOpen}
        onClose={() => setCreateOpen(false)}
        onCreated={() => queryClient.invalidateQueries({ queryKey: businessKeys.deliverables(projectId!) })}
      />
    </div>
  );
}
