// Use plain elements instead of Sheet primitives — DealHeader renders inside
// both Sheet and Dialog contexts (expand mode), and Radix Sheet/Dialog
// primitives require their specific parent context or throw errors.
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { Button } from '@/components/ui/button';
import { IconButton } from '@/components/ui/icon-button';
import {
  Calendar,
  Building2,
  Edit,
  Trash2,

  TrendingUp,
  CheckSquare,
  ChevronRight,
  Maximize2,
  Minimize2,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { CrmDealWithContact, CrmPipelineStage } from '@/types/crm';

// ── Helpers ──────────────────────────────────────────────────────────────────

function formatAmount(amount: number | undefined | null, currency: string) {
  if (!amount) return null;
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: currency || 'USD',
    minimumFractionDigits: 0,
    maximumFractionDigits: 0,
  }).format(amount);
}

function getInitials(deal: CrmDealWithContact) {
  return deal.contact_name
    ? deal.contact_name
        .split(' ')
        .map((n) => n[0])
        .join('')
        .toUpperCase()
        .slice(0, 2)
    : deal.name.slice(0, 2).toUpperCase();
}

// ── DealHeader ───────────────────────────────────────────────────────────────

interface DealHeaderProps {
  deal: CrmDealWithContact;
  stageColor: string;
  onEdit: (deal: CrmDealWithContact) => void;
  onDelete: (deal: CrmDealWithContact) => void;
  isExpanded?: boolean;
  onToggleExpand?: () => void;
}

export function DealHeader({ deal, stageColor, onEdit, onDelete, isExpanded, onToggleExpand }: DealHeaderProps) {
  const initials = getInitials(deal);
  const formattedAmount = formatAmount(deal.amount, deal.currency);
  const taskTotal = deal.task_total ?? 0;
  const taskDone = deal.task_done ?? 0;

  return (
    <div className="flex flex-col space-y-2 text-center sm:text-left px-5 pt-4 pb-3 border-b shrink-0">
      <div className="flex items-start gap-3">
        <Avatar className="h-11 w-11 shrink-0 mt-0.5">
          {deal.contact_avatar_url && (
            <AvatarImage src={deal.contact_avatar_url} alt={deal.contact_name || deal.name} />
          )}
          <AvatarFallback
            className="text-sm font-semibold"
            style={{
              backgroundColor: `${stageColor}22`,
              color: stageColor,
            }}
          >
            {initials}
          </AvatarFallback>
        </Avatar>

        <div className="min-w-0 flex-1">
          <h2 className="text-base font-semibold text-foreground leading-tight">{deal.name}</h2>
          <p className="text-xs text-muted-foreground mt-0.5 flex items-center gap-1.5 flex-wrap">
            {deal.contact_name && <span>{deal.contact_name}</span>}
            {deal.contact_company && (
              <>
                {deal.contact_name && <span className="text-muted-foreground/40">·</span>}
                <span className="flex items-center gap-0.5">
                  <Building2 className="h-3 w-3" />
                  {deal.contact_company}
                </span>
              </>
            )}
          </p>
        </div>

        <div className="flex items-center gap-1 shrink-0">
          {onToggleExpand && (
            <Button variant="ghost" size="icon" className="h-7 w-7" onClick={onToggleExpand} title={isExpanded ? 'Minimize' : 'Expand'}>
              {isExpanded ? <Minimize2 className="h-3.5 w-3.5" /> : <Maximize2 className="h-3.5 w-3.5" />}
            </Button>
          )}
          <IconButton
            variant="ghost" className="h-7 w-7" onClick={() => onEdit(deal)}
            icon={Edit}
            label="Edit deal"
            iconClassName="h-3.5 w-3.5"
          />
          <IconButton
            variant="ghost" className="h-7 w-7 text-destructive hover:text-destructive"
            onClick={() => onDelete(deal)}
            icon={Trash2}
            label="Delete deal"
            iconClassName="h-3.5 w-3.5"
          />
        </div>
      </div>

      {/* Metric pills row */}
      <div className="flex items-center gap-2 mt-2 flex-wrap">
        {formattedAmount && (
          <span className="inline-flex items-center text-xs font-semibold text-green-600">
            {formattedAmount}
          </span>
        )}
        {deal.probability > 0 && (
          <span className="inline-flex items-center gap-0.5 text-[11px] text-muted-foreground">
            <TrendingUp className="h-3 w-3" />
            {deal.probability}% probability
          </span>
        )}
        {taskTotal > 0 && (
          <span
            className={cn(
              'inline-flex items-center gap-0.5 text-[11px]',
              taskDone === taskTotal ? 'text-green-600 font-medium' : 'text-muted-foreground'
            )}
          >
            <CheckSquare className="h-3 w-3" />
            {taskDone}/{taskTotal} tasks
          </span>
        )}
        {deal.expected_close_date && (
          <span className="inline-flex items-center gap-0.5 text-[11px] text-muted-foreground">
            <Calendar className="h-3 w-3" />
            {new Date(deal.expected_close_date).toLocaleDateString('en-US', {
              month: 'short',
              day: 'numeric',
              year: 'numeric',
            })}
          </span>
        )}
      </div>
    </div>
  );
}

// ── Pipeline Stage Stepper ────────────────────────────────────────────────────

const FALLBACK_STAGES = [
  { name: 'Lead', color: '#6B7280' },
  { name: 'Business Analysis', color: '#3B82F6' },
  { name: 'Discovery', color: '#8B5CF6' },
  { name: 'Build Proposal', color: '#F59E0B' },
  { name: 'Polish', color: '#EC4899' },
  { name: 'Proposal Meeting', color: '#EF4444' },
  { name: 'Closed Won', color: '#22C55E' },
  { name: 'Closed Lost', color: '#9CA3AF' },
];

interface PipelineStepperProps {
  currentStage: string;
  allStages?: CrmPipelineStage[];
  onStageClick?: (stageName: string, stageId: string) => void;
}

export function PipelineStepper({ currentStage, allStages, onStageClick }: PipelineStepperProps) {
  const stages =
    allStages && allStages.length > 0
      ? allStages.map((s) => ({ id: s.id, name: s.name, color: s.color }))
      : FALLBACK_STAGES.map((s) => ({ id: '', ...s }));

  const activeStages = stages.filter((s) => s.name.toLowerCase() !== 'closed lost');
  const isClosedLost = currentStage.toLowerCase() === 'closed lost';
  const currentIndex = stages.findIndex(
    (s) => s.name.toLowerCase() === currentStage.toLowerCase()
  );

  if (currentIndex === -1 && !isClosedLost) return null;

  return (
    <div className="px-5 py-2.5 border-b bg-muted/20 shrink-0">
      <div className="flex items-center gap-0">
        {activeStages.map((stage, i) => {
          const isCompleted = i < currentIndex;
          const isCurrent = i === currentIndex;

          const isClickable = onStageClick && !isCurrent && stage.id;
          return (
            <div key={stage.name} className="flex items-center flex-1 min-w-0">
              <div
                className={cn("flex flex-col items-center flex-1 min-w-0", isClickable && "cursor-pointer group/stage")}
                onClick={isClickable ? () => onStageClick(stage.name, stage.id) : undefined}
                title={isClickable ? `Move to ${stage.name}` : stage.name}
              >
                <div
                  className={cn(
                    'w-full h-1 rounded-sm transition-all',
                    isClickable && 'group-hover/stage:h-2 group-hover/stage:opacity-80',
                    !isCurrent &&
                      (isClosedLost
                        ? 'bg-muted'
                        : isCompleted
                          ? 'bg-green-500'
                          : 'bg-muted')
                  )}
                  style={isCurrent && !isClosedLost ? { backgroundColor: stage.color } : undefined}
                />
                {isCurrent && (
                  <span
                    className="text-[9px] font-medium mt-0.5 text-center truncate max-w-full leading-none"
                    style={{ color: stage.color }}
                  >
                    {stage.name}
                  </span>
                )}
              </div>
              {i < activeStages.length - 1 && (
                <ChevronRight className="h-2.5 w-2.5 text-muted-foreground/20 shrink-0" />
              )}
            </div>
          );
        })}
      </div>
      {isClosedLost && (
        <p className="text-[10px] text-muted-foreground text-center mt-1 font-medium">
          Closed Lost
        </p>
      )}
    </div>
  );
}
