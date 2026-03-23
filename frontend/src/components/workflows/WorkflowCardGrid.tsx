// ─── WorkflowCardGrid ──────────────────────────────────────────────────────
//
// Shared workflow card grid used in both My Workflows (BuilderTab)
// and Intelligence (WorkflowsView).

import { formatDistanceToNow } from 'date-fns';
import {
  Bot,
  Building2,
  Hammer,
  MoreHorizontal,
  Pencil,
  Play,
  Trash2,
  User,
} from 'lucide-react';

import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardDescription,CardHeader, CardTitle } from '@/components/ui/card';
import { CardGrid } from '@/components/ui/card-grid';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { EmptyState } from '@/components/ui/empty-state';
import { StatusBadge } from '@/components/ui/status-badge';
import { getNodeTypeDef } from '@/components/workflows/WorkflowEditor';
import type { WorkflowDefinition, WorkflowRun } from '@/lib/api';
import { getStatusInfo } from '@/lib/status-utils';
import { cn } from '@/lib/utils';

// ── Ownership Badge ────────────────────────────────────────────────────────

const OWNER_BADGE_CONFIG: Record<string, { label: string; icon: typeof User; className: string }> = {
  user: {
    label: 'Personal',
    icon: User,
    className: 'bg-blue-500/10 text-blue-700 border-blue-500/20 dark:text-blue-400',
  },
  organization: {
    label: 'Organization',
    icon: Building2,
    className: 'bg-green-500/10 text-green-700 border-green-500/20 dark:text-green-400',
  },
  system: {
    label: 'System',
    icon: Bot,
    className: 'bg-slate-500/10 text-slate-700 border-slate-500/20 dark:text-slate-400',
  },
};

function OwnershipBadge({ ownerType }: { ownerType: string }) {
  const config = OWNER_BADGE_CONFIG[ownerType] ?? OWNER_BADGE_CONFIG.system;
  const Icon = config.icon;
  return (
    <Badge variant="outline" className={cn('text-[10px] gap-1', config.className)}>
      <Icon className="h-2.5 w-2.5" />
      {config.label}
    </Badge>
  );
}

// ── Node color badge mapping ───────────────────────────────────────────────

const NODE_BADGE_COLORS: Record<string, string> = {
  'bg-blue-500': 'bg-blue-500/10 text-blue-700 border-blue-500/20',
  'bg-purple-500': 'bg-purple-500/10 text-purple-700 border-purple-500/20',
  'bg-emerald-500': 'bg-emerald-500/10 text-emerald-700 border-emerald-500/20',
  'bg-amber-500': 'bg-amber-500/10 text-amber-700 border-amber-500/20',
  'bg-orange-500': 'bg-orange-500/10 text-orange-700 border-orange-500/20',
  'bg-teal-500': 'bg-teal-500/10 text-teal-700 border-teal-500/20',
  'bg-green-500': 'bg-green-500/10 text-green-700 border-green-500/20',
  'bg-slate-600': 'bg-slate-600/10 text-slate-700 border-slate-600/20',
  'bg-yellow-600': 'bg-yellow-600/10 text-yellow-700 border-yellow-600/20',
  'bg-pink-500': 'bg-pink-500/10 text-pink-700 border-pink-500/20',
  'bg-indigo-600': 'bg-indigo-600/10 text-indigo-700 border-indigo-600/20',
  'bg-sky-500': 'bg-sky-500/10 text-sky-700 border-sky-500/20',
  'bg-cyan-600': 'bg-cyan-600/10 text-cyan-700 border-cyan-600/20',
};

// ── Props ──────────────────────────────────────────────────────────────────

interface WorkflowCardGridProps {
  workflows: WorkflowDefinition[];
  /** Map of workflow_id → last WorkflowRun (for status display) */
  lastRunByWorkflow?: Map<string, WorkflowRun>;
  /** Currently selected workflow ID (shows ring highlight) */
  selectedId?: string | null;
  /** Show ownership badges on each card */
  showOwnerBadge?: boolean;
  onSelect?: (wf: WorkflowDefinition) => void;
  onEdit?: (wf: WorkflowDefinition) => void;
  onRun?: (wf: WorkflowDefinition) => void;
  onDelete?: (wf: WorkflowDefinition) => void;
  onCopyToOrg?: (wf: WorkflowDefinition) => void;
  onCopyToUser?: (wf: WorkflowDefinition) => void;
  onCreateNew?: () => void;
  emptyIcon?: typeof Hammer;
  emptyTitle?: string;
  emptyDescription?: string;
}

export function WorkflowCardGrid({
  workflows,
  lastRunByWorkflow,
  selectedId,
  showOwnerBadge = false,
  onSelect,
  onEdit,
  onRun,
  onDelete,
  onCopyToOrg,
  onCopyToUser,
  onCreateNew,
  emptyIcon = Hammer,
  emptyTitle = 'No workflows yet',
  emptyDescription = 'Create your first workflow to start processing data sources.',
}: WorkflowCardGridProps) {
  if (workflows.length === 0) {
    return (
      <EmptyState
        icon={emptyIcon}
        title={emptyTitle}
        description={emptyDescription}
        action={onCreateNew ? { label: 'Create Workflow', onClick: onCreateNew } : undefined}
      />
    );
  }

  return (
    <CardGrid columns={{ sm: 2, lg: 3 }} gap={3}>
      {workflows.map((wf) => {
        const nodeCount = wf.nodes?.length ?? 0;
        const isSelected = selectedId === wf.id;

        return (
          <Card
            key={wf.id}
            className={cn('card-interactive cursor-pointer transition-all', isSelected && 'ring-2 ring-primary border-primary')}
            onClick={() => onSelect?.(wf)}
          >
            <CardHeader className="pb-2">
              <div className="flex items-center justify-between">
                <CardTitle className="text-sm">{wf.name}</CardTitle>
                <div className="flex items-center gap-1.5">
                  {showOwnerBadge && <OwnershipBadge ownerType={wf.owner_type} />}
                  {wf.is_system && !showOwnerBadge && (
                    <Badge variant="secondary" className="text-[10px]">System</Badge>
                  )}
                  <Badge variant="outline" className="text-[10px]">
                    {nodeCount} node{nodeCount !== 1 ? 's' : ''}
                  </Badge>
                </div>
              </div>
              {wf.description && (
                <CardDescription className="text-xs line-clamp-2">{wf.description}</CardDescription>
              )}
            </CardHeader>
            <CardContent>
              <div className="flex items-center justify-between">
                {/* Node type badges */}
                <div className="flex flex-wrap gap-1">
                  {(wf.nodes ?? []).slice(0, 4).map((node) => {
                    const nDef = getNodeTypeDef(node.type);
                    const badgeColor = NODE_BADGE_COLORS[nDef?.color ?? ''] ?? 'bg-muted text-muted-foreground';
                    return (
                      <Badge key={node.id} variant="outline" className={cn('text-[9px] px-1.5', badgeColor)}>
                        {node.name}
                      </Badge>
                    );
                  })}
                  {nodeCount > 4 && (
                    <Badge variant="outline" className="text-[9px] px-1.5">+{nodeCount - 4}</Badge>
                  )}
                </div>

                {/* Action buttons */}
                <div className="flex items-center gap-1">
                  {onRun && (
                    <button
                      className="p-1 rounded hover:bg-primary/10 hover:text-primary transition-colors"
                      title="Run workflow"
                      aria-label={`Run ${wf.name}`}
                      onClick={(e) => { e.stopPropagation(); onRun(wf); }}
                    >
                      <Play className="h-3.5 w-3.5" />
                    </button>
                  )}
                  {onEdit && (
                    <button
                      className="p-1 rounded hover:bg-blue-500/10 hover:text-blue-600 transition-colors"
                      title="Edit in visual builder"
                      aria-label={`Edit ${wf.name}`}
                      onClick={(e) => { e.stopPropagation(); onEdit(wf); }}
                    >
                      <Pencil className="h-3.5 w-3.5" />
                    </button>
                  )}
                  {onDelete && !wf.is_system && (
                    <button
                      className="p-1 rounded hover:bg-destructive/10 hover:text-destructive transition-colors"
                      aria-label={`Delete ${wf.name}`}
                      onClick={(e) => {
                        e.stopPropagation();
                        if (confirm(`Delete "${wf.name}"?`)) onDelete(wf);
                      }}
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </button>
                  )}
                  {(onCopyToOrg || onCopyToUser) && (
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <button
                          className="p-1 rounded hover:bg-muted transition-colors"
                          aria-label={`More actions for ${wf.name}`}
                          onClick={(e) => e.stopPropagation()}
                        >
                          <MoreHorizontal className="h-3.5 w-3.5" />
                        </button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end" onClick={(e) => e.stopPropagation()}>
                        {onCopyToOrg && wf.owner_type !== 'organization' && (
                          <DropdownMenuItem onClick={() => onCopyToOrg(wf)}>
                            <Building2 className="h-3.5 w-3.5 mr-2" />
                            Copy to Organization...
                          </DropdownMenuItem>
                        )}
                        {onCopyToUser && wf.owner_type !== 'user' && (
                          <DropdownMenuItem onClick={() => onCopyToUser(wf)}>
                            <User className="h-3.5 w-3.5 mr-2" />
                            Copy to My Workflows
                          </DropdownMenuItem>
                        )}
                      </DropdownMenuContent>
                    </DropdownMenu>
                  )}
                </div>
              </div>

              {/* Last run status */}
              {lastRunByWorkflow && (() => {
                const lastRun = lastRunByWorkflow.get(wf.id);
                if (!lastRun) {
                  return <p className="text-[10px] text-muted-foreground/50 mt-1.5">Never run — click ▶ to execute</p>;
                }
                const runStatus = getStatusInfo(lastRun.status, 'workflow');
                return (
                  <div className="text-[10px] text-muted-foreground mt-1.5 flex items-center gap-1">
                    Last run: {formatDistanceToNow(new Date(lastRun.created_at), { addSuffix: true })}
                    <StatusBadge
                      status={runStatus.variant}
                      label={runStatus.label}
                      icon={runStatus.icon}
                      pulse={lastRun.status === 'running'}
                      size="sm"
                    />
                  </div>
                );
              })()}
            </CardContent>
          </Card>
        );
      })}
    </CardGrid>
  );
}
