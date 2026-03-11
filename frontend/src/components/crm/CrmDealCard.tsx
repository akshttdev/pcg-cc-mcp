import { useDraggable } from '@dnd-kit/core';
import { Card, CardContent } from '@/components/ui/card';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  Calendar,
  DollarSign,
  MoreVertical,
  Building2,
  Trash2,
  Edit,
  Tag,
  Brain,
  CheckCircle2,
  Clock,
  AlertCircle,
  Loader2,
  FileText,
  RotateCcw,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { CrmDealWithContact } from '@/types/crm';
import { formatDistanceToNow } from 'date-fns';

interface BoardProgressInfo {
  boardName: string;
  completedAssets: number;
  totalAssets: number;
  percentage: number;
}

interface CrmDealCardProps {
  deal: CrmDealWithContact;
  onClick?: (deal: CrmDealWithContact) => void;
  onEdit?: (deal: CrmDealWithContact) => void;
  onDelete?: (deal: CrmDealWithContact) => void;
  isDragging?: boolean;
  boardProgress?: BoardProgressInfo;
}

function IntelStatusIcon({ status }: { status?: string }) {
  if (!status || status === 'idle') {
    return <Brain className="h-3.5 w-3.5 text-muted-foreground/50" />;
  }
  if (status === 'running' || status === 'queued') {
    return <Loader2 className="h-3.5 w-3.5 text-blue-500 animate-spin" />;
  }
  if (status === 'done') {
    return <CheckCircle2 className="h-3.5 w-3.5 text-green-500" />;
  }
  if (status === 'failed') {
    return <AlertCircle className="h-3.5 w-3.5 text-red-400" />;
  }
  return <Brain className="h-3.5 w-3.5 text-muted-foreground/50" />;
}

function ReportStatusChip({ reviewStatus }: { reviewStatus?: string }) {
  if (!reviewStatus) return null;
  if (reviewStatus === 'pending_review') {
    return (
      <Badge className="text-[9px] px-1.5 py-0 bg-amber-100 text-amber-700 border-amber-200 hover:bg-amber-100">
        Needs Review
      </Badge>
    );
  }
  if (reviewStatus === 'approved') {
    return (
      <Badge className="text-[9px] px-1.5 py-0 bg-green-100 text-green-700 border-green-200 hover:bg-green-100">
        Approved
      </Badge>
    );
  }
  if (reviewStatus === 'rejected') {
    return (
      <Badge className="text-[9px] px-1.5 py-0 bg-red-100 text-red-700 border-red-200 hover:bg-red-100">
        <RotateCcw className="h-2.5 w-2.5 mr-0.5" />
        Revision
      </Badge>
    );
  }
  return null;
}

function TaskStatusBadge({ status, assignee }: { status?: string; assignee?: string }) {
  const label = status === 'inprogress' ? 'In Progress'
    : status === 'done' ? 'Done'
    : status === 'cancelled' ? 'Cancelled'
    : 'Review Pending';

  const colorClass = status === 'inprogress'
    ? 'text-blue-600 bg-blue-50'
    : status === 'done'
    ? 'text-green-600 bg-green-50'
    : 'text-amber-600 bg-amber-50';

  return (
    <div className={cn('flex items-center gap-1 rounded px-1.5 py-0.5 text-[10px] font-medium', colorClass)}>
      <Clock className="h-2.5 w-2.5 shrink-0" />
      <span>{label}</span>
      {assignee && <span className="text-muted-foreground font-normal">· {assignee.split(' ')[0]}</span>}
    </div>
  );
}

export function CrmDealCard({ deal, onClick, onEdit, onDelete, isDragging, boardProgress }: CrmDealCardProps) {
  const { attributes, listeners, setNodeRef, transform } = useDraggable({
    id: deal.id,
    data: {
      type: 'deal',
      deal,
    },
  });

  const style = transform
    ? {
        transform: `translate3d(${transform.x}px, ${transform.y}px, 0)`,
      }
    : undefined;

  const initials = deal.contact_name
    ? deal.contact_name
        .split(' ')
        .map((n) => n[0])
        .join('')
        .toUpperCase()
        .slice(0, 2)
    : deal.name.slice(0, 2).toUpperCase();

  const formatAmount = (amount: number | undefined | null) => {
    if (!amount) return null;
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: deal.currency || 'USD',
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    }).format(amount);
  };

  const formattedAmount = formatAmount(deal.amount);
  const lastActivity = deal.last_activity_at
    ? formatDistanceToNow(new Date(deal.last_activity_at), { addSuffix: true })
    : null;

  const hasIntel = deal.intelligence_status && deal.intelligence_status !== 'idle';
  const intelDone = deal.intelligence_status === 'done';
  const confidencePct = intelDone && deal.intelligence_confidence != null
    ? Math.round(deal.intelligence_confidence * 100)
    : null;

  const summaryPreview = deal.intelligence_summary
    ? deal.intelligence_summary.slice(0, 100) + (deal.intelligence_summary.length > 100 ? '…' : '')
    : null;

  const handleClick = (e: React.MouseEvent) => {
    if ((e.target as HTMLElement).closest('[data-deal-menu]')) return;
    onClick?.(deal);
  };

  let tags: string[] = [];
  if (deal.tags) {
    try { tags = JSON.parse(deal.tags); } catch { tags = deal.tags.split(',').map(t => t.trim()).filter(Boolean); }
  }

  return (
    <Card
      ref={setNodeRef}
      style={style}
      className={cn(
        'cursor-grab active:cursor-grabbing transition-shadow hover:shadow-md group',
        isDragging && 'opacity-50 shadow-lg ring-2 ring-primary',
        onClick && !isDragging && 'cursor-pointer'
      )}
      onClick={handleClick}
      {...listeners}
      {...attributes}
    >
      <CardContent className="p-3 space-y-2">
        {/* Header: Avatar + Name + Company + Context menu */}
        <div className="flex items-start justify-between gap-2">
          <div className="flex items-center gap-2 min-w-0 flex-1">
            <Avatar className="h-8 w-8 shrink-0">
              {deal.contact_avatar_url && (
                <AvatarImage src={deal.contact_avatar_url} alt={deal.contact_name || deal.name} />
              )}
              <AvatarFallback className="text-xs bg-primary/10 text-primary">
                {initials}
              </AvatarFallback>
            </Avatar>
            <div className="min-w-0 flex-1">
              <p className="font-medium text-sm truncate">{deal.name}</p>
              {deal.contact_company && (
                <p className="text-xs text-muted-foreground truncate flex items-center gap-1">
                  <Building2 className="h-2.5 w-2.5 shrink-0" />
                  {deal.contact_company}
                </p>
              )}
            </div>
          </div>

          {/* Context menu */}
          <div className="flex items-center gap-0.5 shrink-0" data-deal-menu>
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-6 w-6 p-0 opacity-0 group-hover:opacity-100 transition-opacity"
                  data-deal-menu
                  onClick={(e) => e.stopPropagation()}
                >
                  <MoreVertical className="h-3 w-3" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-36">
                <DropdownMenuItem onClick={() => onEdit?.(deal)}>
                  <Edit className="h-3 w-3 mr-2" />
                  Edit
                </DropdownMenuItem>
                <DropdownMenuItem
                  onClick={() => onDelete?.(deal)}
                  className="text-destructive focus:text-destructive"
                >
                  <Trash2 className="h-3 w-3 mr-2" />
                  Delete
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>

        {/* Intel badge row */}
        {(hasIntel || deal.report_review_status) && (
          <div className="flex items-center flex-wrap gap-1.5">
            <div className="flex items-center gap-1 text-xs">
              <IntelStatusIcon status={deal.intelligence_status} />
              {confidencePct != null && (
                <span className="text-green-600 font-medium">{confidencePct}%</span>
              )}
              {(deal.research_pass_count ?? 0) > 0 && (
                <span className="text-muted-foreground">{deal.research_pass_count} pass{(deal.research_pass_count ?? 0) !== 1 ? 'es' : ''}</span>
              )}
            </div>
            {deal.report_review_status && (
              <ReportStatusChip reviewStatus={deal.report_review_status} />
            )}
          </div>
        )}

        {/* Intelligence summary preview */}
        {summaryPreview && (
          <p className="text-[11px] text-muted-foreground leading-relaxed italic border-l-2 border-muted pl-2">
            {summaryPreview}
          </p>
        )}

        {/* Review task row */}
        {deal.review_task_id && (
          <div className="flex items-center gap-1.5">
            <FileText className="h-3 w-3 text-muted-foreground shrink-0" />
            <TaskStatusBadge status={deal.review_task_status} assignee={deal.review_task_assignee ?? undefined} />
          </div>
        )}

        {/* Deal Amount */}
        {formattedAmount && (
          <div className="flex items-center gap-1.5 text-sm">
            <DollarSign className="h-3.5 w-3.5 text-green-600" />
            <span className="font-semibold text-green-600">{formattedAmount}</span>
          </div>
        )}

        {/* Tags (up to 2) */}
        {tags.length > 0 && (
          <div className="flex flex-wrap gap-1">
            {tags.slice(0, 2).map((tag) => (
              <Badge key={tag} variant="secondary" className="text-[10px] px-1.5 py-0 gap-0.5">
                <Tag className="h-2.5 w-2.5" />
                {tag}
              </Badge>
            ))}
            {tags.length > 2 && (
              <Badge variant="outline" className="text-[10px] px-1.5 py-0">+{tags.length - 2}</Badge>
            )}
          </div>
        )}

        {/* Footer: Date + last activity */}
        <div className="flex items-center justify-between pt-1 border-t">
          {deal.expected_close_date ? (
            <div className="flex items-center gap-1 text-xs text-muted-foreground">
              <Calendar className="h-3 w-3" />
              <span>
                {new Date(deal.expected_close_date).toLocaleDateString('en-US', {
                  month: 'short',
                  day: 'numeric',
                })}
              </span>
            </div>
          ) : <span />}
          {lastActivity && (
            <Badge variant="outline" className="text-[10px] px-1.5 py-0">
              {lastActivity}
            </Badge>
          )}
        </div>

        {/* Board asset progress (delivery pipeline deals) */}
        {boardProgress && boardProgress.totalAssets > 0 && (
          <div className="space-y-1 pt-1 border-t">
            <div className="flex items-center justify-between text-xs">
              <span className="text-muted-foreground flex items-center gap-1">
                <CheckCircle2 className="h-3 w-3" />
                {boardProgress.boardName}
              </span>
              <span className="font-medium">
                {boardProgress.completedAssets}/{boardProgress.totalAssets}
              </span>
            </div>
            <div className="h-1.5 bg-muted rounded-full overflow-hidden">
              <div
                className={cn(
                  'h-full rounded-full transition-all',
                  boardProgress.percentage === 100 ? 'bg-green-500' : 'bg-blue-500'
                )}
                style={{ width: `${boardProgress.percentage}%` }}
              />
            </div>
          </div>
        )}

        {/* Probability indicator */}
        {deal.probability > 0 && (
          <div className="h-1 bg-muted rounded-full overflow-hidden">
            <div
              className="h-full bg-primary transition-all"
              style={{ width: `${deal.probability}%` }}
            />
          </div>
        )}
      </CardContent>
    </Card>
  );
}
