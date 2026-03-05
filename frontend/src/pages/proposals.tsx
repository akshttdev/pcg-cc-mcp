import { useState, useMemo } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  DndContext,
  DragOverlay,
  closestCorners,
  PointerSensor,
  useSensor,
  useSensors,
  useDroppable,
  type DragEndEvent,
  type DragStartEvent,
} from '@dnd-kit/core';
import {
  SortableContext,
  useSortable,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { ScrollArea, ScrollBar } from '@/components/ui/scroll-area';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';
import {
  FileText,
  Plus,
  GripVertical,
  Coins,
  Users,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';
import {
  proposalsApi,
  type ProposalRecord,
  type ProposalStatus,
} from '@/lib/api';

// ── Pipeline stages ───────────────────────────────────────────────────────────

const STAGES: { key: ProposalStatus; label: string; color: string }[] = [
  { key: 'drafted',           label: 'Drafted',           color: 'bg-gray-100 text-gray-700' },
  { key: 'pending_approval',  label: 'Pending Approval',  color: 'bg-yellow-100 text-yellow-700' },
  { key: 'approved',          label: 'Approved',          color: 'bg-blue-100 text-blue-700' },
  { key: 'meeting_scheduled', label: 'Meeting Scheduled', color: 'bg-indigo-100 text-indigo-700' },
  { key: 'sent',              label: 'Sent',              color: 'bg-purple-100 text-purple-700' },
  { key: 'seen',              label: 'Seen',              color: 'bg-pink-100 text-pink-700' },
  { key: 'verbal',            label: 'Verbal',            color: 'bg-orange-100 text-orange-700' },
  { key: 'contract_signed',   label: 'Signed',            color: 'bg-green-100 text-green-700' },
];

const TERMINAL_STAGES: { key: ProposalStatus; label: string; color: string }[] = [
  { key: 'declined', label: 'Declined', color: 'bg-red-100 text-red-700' },
  { key: 'deferred', label: 'Deferred', color: 'bg-gray-100 text-gray-500' },
];

const ALL_STAGES = [...STAGES, ...TERMINAL_STAGES];

function fmtVibe(v: number) {
  if (v >= 1_000_000) return `${(v / 1_000_000).toFixed(1)}M ꝩ`;
  if (v >= 1_000) return `${(v / 1_000).toFixed(0)}k ꝩ`;
  return `${v} ꝩ`;
}

// ── Proposal card ─────────────────────────────────────────────────────────────

function ProposalCard({
  proposal,
  isDragging,
}: {
  proposal: ProposalRecord;
  isDragging?: boolean;
}) {
  const { attributes, listeners, setNodeRef, transform, transition } = useSortable({
    id: proposal.id,
    data: { type: 'proposal', proposal },
  });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.3 : 1,
  };

  const dealColor = {
    'one-off': 'bg-blue-50 text-blue-600',
    retainer: 'bg-purple-50 text-purple-600',
    hybrid: 'bg-teal-50 text-teal-600',
  }[proposal.deal_type] ?? '';

  return (
    <div
      ref={setNodeRef}
      style={style}
      className="bg-card border rounded-lg p-3 shadow-sm hover:shadow-md transition-shadow cursor-pointer group"
    >
      <div className="flex items-start gap-2">
        <div
          {...attributes}
          {...listeners}
          className="mt-0.5 cursor-grab opacity-0 group-hover:opacity-60 shrink-0"
        >
          <GripVertical className="h-4 w-4 text-muted-foreground" />
        </div>
        <div className="flex-1 min-w-0">
          <p className="text-sm font-medium truncate">{proposal.title}</p>
          <div className="flex items-center gap-2 mt-1 flex-wrap">
            <Badge className={`text-xs px-1.5 py-0 border-0 ${dealColor}`}>
              {proposal.deal_type}
            </Badge>
            {proposal.quote_amount_vibe > 0 && (
              <span className="flex items-center gap-0.5 text-xs text-muted-foreground">
                <Coins className="h-3 w-3" />
                {fmtVibe(proposal.quote_amount_vibe)}
              </span>
            )}
            {(() => {
              try {
                const contacts = JSON.parse(proposal.contact_ids || '[]');
                if (contacts.length > 0) {
                  return (
                    <span className="flex items-center gap-0.5 text-xs text-muted-foreground">
                      <Users className="h-3 w-3" />
                      {contacts.length}
                    </span>
                  );
                }
              } catch { /* ignore */ }
              return null;
            })()}
          </div>
          {proposal.description && (
            <p className="text-xs text-muted-foreground mt-1 line-clamp-2">
              {proposal.description}
            </p>
          )}
        </div>
      </div>
    </div>
  );
}

// ── Droppable column ──────────────────────────────────────────────────────────

function DroppableColumn({
  stageKey,
  label,
  color,
  proposals,
  activeId,
}: {
  stageKey: string;
  label: string;
  color: string;
  proposals: ProposalRecord[];
  activeId: string | null;
}) {
  const { setNodeRef, isOver } = useDroppable({
    id: stageKey,
    data: { type: 'column', stageKey },
  });

  const total = proposals.reduce((s, p) => s + p.quote_amount_vibe, 0);

  return (
    <div className="flex flex-col w-60 shrink-0">
      <div className="flex items-center justify-between mb-2 px-1">
        <div className="flex items-center gap-2">
          <Badge className={`text-xs px-2 py-0.5 border-0 ${color}`}>{label}</Badge>
          <span className="text-xs text-muted-foreground">{proposals.length}</span>
        </div>
        {total > 0 && (
          <span className="text-xs text-muted-foreground">{fmtVibe(total)}</span>
        )}
      </div>
      <div
        ref={setNodeRef}
        className={cn(
          'flex flex-col gap-2 min-h-[120px] p-2 rounded-lg transition-colors bg-muted/30',
          isOver && 'bg-primary/5 ring-1 ring-primary/30'
        )}
      >
        <SortableContext
          items={proposals.map((p) => p.id)}
          strategy={verticalListSortingStrategy}
        >
          {proposals.map((p) => (
            <ProposalCard key={p.id} proposal={p} isDragging={activeId === p.id} />
          ))}
        </SortableContext>
      </div>
    </div>
  );
}

// ── Page ──────────────────────────────────────────────────────────────────────

export function ProposalsPage() {
  const queryClient = useQueryClient();
  const [activeProposal, setActiveProposal] = useState<ProposalRecord | null>(null);

  const { data: proposals = [], isLoading } = useQuery({
    queryKey: ['proposals'],
    queryFn: () => proposalsApi.list({ limit: 500 }),
  });

  const moveStatus = useMutation({
    mutationFn: ({ id, status }: { id: string; status: ProposalStatus }) =>
      proposalsApi.moveStatus(id, status),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['proposals'] }),
    onError: () => toast.error('Failed to move proposal'),
  });

  // Group by status
  const grouped = useMemo(() => {
    const map: Record<string, ProposalRecord[]> = {};
    for (const s of ALL_STAGES) map[s.key] = [];
    for (const p of proposals) {
      if (map[p.status]) map[p.status].push(p);
      else map['drafted'] = [...(map['drafted'] ?? []), p];
    }
    return map;
  }, [proposals]);

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 8 } })
  );

  function handleDragStart(event: DragStartEvent) {
    const proposal = proposals.find((p) => p.id === event.active.id);
    setActiveProposal(proposal ?? null);
  }

  function handleDragEnd(event: DragEndEvent) {
    setActiveProposal(null);
    const { active, over } = event;
    if (!over) return;

    const targetStatus = (over.data.current?.stageKey ??
      // dropped on another card — find its column
      proposals.find((p) => p.id === over.id)?.status) as ProposalStatus | undefined;

    if (!targetStatus) return;

    const proposal = proposals.find((p) => p.id === active.id);
    if (!proposal || proposal.status === targetStatus) return;

    moveStatus.mutate({ id: proposal.id, status: targetStatus });
  }

  const totalSigned = proposals
    .filter((p) => p.status === 'contract_signed')
    .reduce((s, p) => s + p.quote_amount_vibe, 0);

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 border-b shrink-0">
        <div>
          <div className="flex items-center gap-2">
            <FileText className="h-5 w-5 text-muted-foreground" />
            <h1 className="text-xl font-semibold">Proposals</h1>
            <Badge variant="outline" className="text-xs">{proposals.length}</Badge>
          </div>
          <p className="text-sm text-muted-foreground mt-0.5">
            {totalSigned > 0 && (
              <span className="text-green-600 font-medium">{fmtVibe(totalSigned)} signed</span>
            )}
            {totalSigned === 0 && 'Pipeline overview — drag cards to advance stages'}
          </p>
        </div>
        <Button size="sm" onClick={() => toast.info('New proposal form coming soon')}>
          <Plus className="h-4 w-4 mr-1" />
          New Proposal
        </Button>
      </div>

      {/* Kanban */}
      {isLoading ? (
        <div className="flex gap-4 p-6">
          {STAGES.slice(0, 5).map((s) => (
            <div key={s.key} className="w-60 shrink-0 space-y-2">
              <Skeleton className="h-6 w-28 rounded-full" />
              <Skeleton className="h-24 w-full rounded-lg" />
              <Skeleton className="h-16 w-full rounded-lg" />
            </div>
          ))}
        </div>
      ) : (
        <ScrollArea className="flex-1">
          <DndContext
            sensors={sensors}
            collisionDetection={closestCorners}
            onDragStart={handleDragStart}
            onDragEnd={handleDragEnd}
          >
            <div className="flex gap-4 p-6 pb-8">
              {/* Active pipeline */}
              {STAGES.map((s) => (
                <DroppableColumn
                  key={s.key}
                  stageKey={s.key}
                  label={s.label}
                  color={s.color}
                  proposals={grouped[s.key] ?? []}
                  activeId={activeProposal?.id ?? null}
                />
              ))}
              {/* Divider */}
              <div className="w-px bg-border shrink-0 self-stretch mx-2" />
              {/* Terminal */}
              {TERMINAL_STAGES.map((s) => (
                <DroppableColumn
                  key={s.key}
                  stageKey={s.key}
                  label={s.label}
                  color={s.color}
                  proposals={grouped[s.key] ?? []}
                  activeId={activeProposal?.id ?? null}
                />
              ))}
            </div>

            <DragOverlay>
              {activeProposal && (
                <div className="opacity-95 rotate-1">
                  <ProposalCard proposal={activeProposal} />
                </div>
              )}
            </DragOverlay>
          </DndContext>
          <ScrollBar orientation="horizontal" />
        </ScrollArea>
      )}
    </div>
  );
}
