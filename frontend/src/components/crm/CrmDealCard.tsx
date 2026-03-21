import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  ArrowRight,
  Calendar,
  DollarSign,
  MoreHorizontal,
  Building2,
  Trash2,
  Edit,
  CheckCircle2,
  Loader2,
  Search,
  ShieldCheck,
  CircleDot,
  RotateCcw,
  CheckSquare,
  User,
  TrendingUp,
  Clock,
  FileText,
  Presentation,
  Receipt,
  Trophy,
  Bot,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { Link } from 'react-router-dom';
import type { CrmDealWithContact } from '@/types/crm';
import { formatDistanceToNow } from 'date-fns';

interface BoardProgressInfo {
  boardName: string;
  completedAssets: number;
  totalAssets: number;
  percentage: number;
}

interface StageOption {
  id: string;
  name: string;
}

interface CrmDealCardProps {
  deal: CrmDealWithContact;
  stageName?: string;
  stageColor?: string;
  onEdit?: (deal: CrmDealWithContact) => void;
  onDelete?: (deal: CrmDealWithContact) => void;
  onMoveTo?: (deal: CrmDealWithContact, stageId: string) => void;
  stages?: StageOption[];
  boardProgress?: BoardProgressInfo;
}

function StatusChip({
  icon: Icon,
  label,
  variant,
  pulse,
}: {
  icon: React.ElementType;
  label: string;
  variant: 'amber' | 'green' | 'blue' | 'red' | 'muted';
  pulse?: boolean;
}) {
  const colors = {
    amber: 'bg-amber-100 text-amber-700 border-amber-200 dark:bg-amber-950/50 dark:text-amber-400 dark:border-amber-800',
    green: 'bg-green-100 text-green-700 border-green-200 dark:bg-green-950/50 dark:text-green-400 dark:border-green-800',
    blue:  'bg-blue-100 text-blue-700 border-blue-200 dark:bg-blue-950/50 dark:text-blue-400 dark:border-blue-800',
    red:   'bg-red-100 text-red-700 border-red-200 dark:bg-red-950/50 dark:text-red-400 dark:border-red-800',
    muted: 'bg-muted text-muted-foreground border-border',
  };
  return (
    <span className={cn(
      'inline-flex items-center gap-0.5 px-1 py-0.5 rounded text-[10px] font-medium border',
      colors[variant],
      pulse && 'animate-pulse',
    )}>
      <Icon className="h-2.5 w-2.5" />
      {label}
    </span>
  );
}

export function CrmDealCard({ deal, stageName, stageColor, onEdit, onDelete, onMoveTo, stages, boardProgress }: CrmDealCardProps) {
  const currentStage = (stageName || '').toLowerCase();

  const initials = deal.contact_name
    ? deal.contact_name.split(' ').map((n) => n[0]).join('').toUpperCase().slice(0, 2)
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

  // Agent flow status
  const agentRunning = deal.active_agent_flow_status === 'executing';
  const agentPending = deal.active_agent_flow_status === 'planning';
  const agentName = deal.active_agent_name;

  const intelStatus = deal.intelligence_status;
  const intelDone = intelStatus === 'done';
  const intelRunning = intelStatus === 'running' || intelStatus === 'queued';

  const isIntelStage = currentStage === 'intel' || currentStage === 'lead' || currentStage === 'research';
  const researchNeeded = isIntelStage && (!intelStatus || intelStatus === 'idle');
  const researchReady = isIntelStage && intelDone;

  // Proposal / deck / invoice / won chips
  const hasProposal = !!deal.proposal_text;
  const proposalApproved = deal.proposal_status === 'approved';
  const hasDeck = !!deal.deck_url;
  const hasInvoice = !!deal.invoice_id;
  const isWon = !!deal.won_at;

  const hasActiveReviewTask = deal.review_task_id && deal.review_task_status !== 'done' && deal.review_task_status !== 'cancelled';
  const reviewTaskDone = deal.review_task_id && deal.review_task_status === 'done';

  const hasTasks = (deal.task_total ?? 0) > 0;
  const taskDone = deal.task_done ?? 0;
  const taskTotal = deal.task_total ?? 1;
  const allTasksDone = hasTasks && taskDone === taskTotal;
  const taskPct = hasTasks ? Math.round((taskDone / taskTotal) * 100) : 0;

  let tags: string[] = [];
  if (deal.tags) {
    try { tags = JSON.parse(deal.tags); } catch { tags = deal.tags.split(',').map(t => t.trim()).filter(Boolean); }
  }

  const accentColor = stageColor || '#6B7280';

  return (
    <div className="flex flex-col min-w-0">
      {/* Stage color accent bar */}
      <div className="h-0.5 w-full rounded-t-lg" style={{ backgroundColor: accentColor }} />

      <div className="p-3 space-y-2.5">
        {/* Row 1: Avatar + Name + Menu */}
        <div className="flex items-start gap-2">
          <Avatar className="h-7 w-7 shrink-0 mt-0.5">
            {deal.contact_avatar_url && (
              <AvatarImage src={deal.contact_avatar_url} alt={deal.contact_name || deal.name} />
            )}
            <AvatarFallback className="text-[10px] font-semibold" style={{
              backgroundColor: `${accentColor}22`,
              color: accentColor,
            }}>
              {initials}
            </AvatarFallback>
          </Avatar>

          <div className="min-w-0 flex-1">
            <p className="font-medium text-sm leading-tight line-clamp-2">{deal.name}</p>
            {deal.contact_company && (
              <p className="text-[11px] text-muted-foreground truncate flex items-center gap-1 mt-0.5">
                <Building2 className="h-2.5 w-2.5 shrink-0" />
                {deal.company_id ? (
                  <Link
                    to={`/companies/${deal.company_id}`}
                    onClick={(e) => e.stopPropagation()}
                    className="hover:text-primary transition-colors truncate"
                  >
                    {deal.contact_company}
                  </Link>
                ) : (
                  deal.contact_company
                )}
              </p>
            )}
          </div>

          <div
            onPointerDown={(e) => e.stopPropagation()}
            onMouseDown={(e) => e.stopPropagation()}
            onClick={(e) => e.stopPropagation()}
          >
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-6 w-6 p-0 opacity-0 group-hover:opacity-100 hover:bg-muted transition-opacity shrink-0"
                >
                  <MoreHorizontal className="h-3.5 w-3.5" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-44" onClick={(e) => e.stopPropagation()}>
                {onMoveTo && stages && stages.length > 0 && (
                  <>
                    <DropdownMenuSub>
                      <DropdownMenuSubTrigger>
                        <ArrowRight className="h-3 w-3 mr-2" />Move to...
                      </DropdownMenuSubTrigger>
                      <DropdownMenuSubContent>
                        {stages
                          .filter((s) => s.id !== deal.crm_stage_id)
                          .map((stage) => (
                            <DropdownMenuItem
                              key={stage.id}
                              onClick={() => onMoveTo(deal, stage.id)}
                            >
                              {stage.name}
                            </DropdownMenuItem>
                          ))}
                      </DropdownMenuSubContent>
                    </DropdownMenuSub>
                    <DropdownMenuSeparator />
                  </>
                )}
                <DropdownMenuItem onClick={() => onEdit?.(deal)}>
                  <Edit className="h-3 w-3 mr-2" />Edit
                </DropdownMenuItem>
                <DropdownMenuItem
                  onClick={() => onDelete?.(deal)}
                  className="text-destructive focus:text-destructive"
                >
                  <Trash2 className="h-3 w-3 mr-2" />Delete
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>

        {/* Row 2: Stage-aware status chips */}
        {(agentRunning || agentPending || researchNeeded || researchReady || intelRunning || hasActiveReviewTask || reviewTaskDone ||
          deal.report_review_status === 'rejected' || hasProposal || hasDeck || hasInvoice || isWon) && (
          <div className="flex flex-wrap gap-1">
            {agentRunning && <StatusChip icon={Bot} label={`${agentName ?? 'Agent'} running…`} variant="blue" pulse />}
            {agentPending && <StatusChip icon={Clock} label={`${agentName ?? 'Agent'} pending`} variant="amber" pulse />}
            {researchNeeded && <StatusChip icon={Search} label="Research needed" variant="amber" />}
            {intelRunning && <StatusChip icon={Loader2} label="Researching…" variant="blue" pulse />}
            {researchReady && !hasActiveReviewTask && <StatusChip icon={ShieldCheck} label="Ready for review" variant="green" />}
            {hasActiveReviewTask && <StatusChip icon={CircleDot} label="Needs review" variant="blue" pulse />}
            {reviewTaskDone && <StatusChip icon={CheckCircle2} label="Review done" variant="green" />}
            {deal.report_review_status === 'rejected' && <StatusChip icon={RotateCcw} label="Revision needed" variant="red" />}
            {hasProposal && !proposalApproved && <StatusChip icon={FileText} label="Proposal draft" variant="amber" />}
            {proposalApproved && <StatusChip icon={FileText} label="Proposal ✓" variant="green" />}
            {hasDeck && <StatusChip icon={Presentation} label="Deck ready" variant="green" />}
            {hasInvoice && <StatusChip icon={Receipt} label="Invoice sent" variant="blue" />}
            {isWon && <StatusChip icon={Trophy} label="Won!" variant="green" />}
          </div>
        )}

        {/* Row 3: Intel summary snippet */}
        {intelDone && deal.intelligence_summary && (
          <p className="text-[11px] text-muted-foreground leading-relaxed italic border-l-2 pl-2 line-clamp-2"
            style={{ borderColor: `${accentColor}60` }}>
            {deal.intelligence_summary}
          </p>
        )}

        {/* Row 4: Review task assignee */}
        {deal.review_task_id && deal.review_task_assignee && (
          <div className="flex items-center gap-1.5 text-[11px] text-muted-foreground">
            <User className="h-3 w-3 shrink-0" />
            <span className="truncate">{deal.review_task_assignee}</span>
            {deal.review_task_status && (
              <span className={cn(
                'text-[10px] font-medium',
                deal.review_task_status === 'done' ? 'text-green-600' :
                deal.review_task_status === 'inprogress' ? 'text-blue-600' : 'text-muted-foreground/60'
              )}>
                · {deal.review_task_status === 'inprogress' ? 'in progress' :
                   deal.review_task_status === 'done' ? 'done' : 'pending'}
              </span>
            )}
          </div>
        )}

        {/* Row 5: Task progress */}
        {hasTasks && (
          <div className="space-y-1">
            <div className="flex items-center justify-between text-[10px] text-muted-foreground">
              <span className="flex items-center gap-1">
                <CheckSquare className="h-3 w-3" />
                <span className={cn(allTasksDone && 'text-green-600 font-medium')}>
                  {taskDone}/{taskTotal} tasks
                </span>
              </span>
              <span className={cn(allTasksDone ? 'text-green-600' : 'text-muted-foreground')}>{taskPct}%</span>
            </div>
            <div className="h-1 bg-muted rounded-full overflow-hidden">
              <div
                className={cn('h-full rounded-full transition-all', allTasksDone ? 'bg-green-500' : 'bg-blue-500')}
                style={{ width: `${taskPct}%` }}
              />
            </div>
          </div>
        )}

        {/* Row 6: Board progress (delivery pipeline) */}
        {boardProgress && boardProgress.totalAssets > 0 && (
          <div className="space-y-1">
            <div className="flex items-center justify-between text-[10px] text-muted-foreground">
              <span>{boardProgress.boardName}</span>
              <span>{boardProgress.completedAssets}/{boardProgress.totalAssets}</span>
            </div>
            <div className="h-1 bg-muted rounded-full overflow-hidden">
              <div
                className={cn('h-full rounded-full transition-all', boardProgress.percentage === 100 ? 'bg-green-500' : 'bg-blue-500')}
                style={{ width: `${boardProgress.percentage}%` }}
              />
            </div>
          </div>
        )}

        {/* Row 7: Tags */}
        {tags.length > 0 && (
          <div className="flex flex-wrap gap-1">
            {tags.slice(0, 2).map((tag) => (
              <span key={tag} className="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] bg-muted text-muted-foreground border border-border/50">
                {tag}
              </span>
            ))}
            {tags.length > 2 && (
              <span className="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] bg-muted text-muted-foreground border border-border/50">
                +{tags.length - 2}
              </span>
            )}
          </div>
        )}

        {/* Row 8: Footer — amount, close date, last activity */}
        <div className="flex items-center justify-between pt-1.5 border-t border-border/40">
          <div className="flex items-center gap-2">
            {formattedAmount && (
              <span className="inline-flex items-center gap-0.5 text-xs font-semibold text-green-600">
                <DollarSign className="h-3 w-3" />
                {formattedAmount}
              </span>
            )}
            {deal.expected_close_date && (
              <span className="inline-flex items-center gap-1 text-[10px] text-muted-foreground">
                <Calendar className="h-2.5 w-2.5" />
                {new Date(deal.expected_close_date).toLocaleDateString('en-US', { month: 'short', day: 'numeric' })}
              </span>
            )}
            {deal.probability > 0 && (
              <span className="inline-flex items-center gap-0.5 text-[10px] text-muted-foreground">
                <TrendingUp className="h-2.5 w-2.5" />
                {deal.probability}%
              </span>
            )}
          </div>
          {lastActivity && (
            <span className="inline-flex items-center gap-1 text-[10px] text-muted-foreground/60 shrink-0">
              <Clock className="h-2.5 w-2.5" />
              {lastActivity}
            </span>
          )}
        </div>

        {/* Probability bar */}
        {deal.probability > 0 && (
          <div className="h-0.5 bg-muted rounded-full overflow-hidden -mt-1.5">
            <div
              className="h-full transition-all"
              style={{
                width: `${deal.probability}%`,
                backgroundColor: accentColor,
                opacity: 0.6,
              }}
            />
          </div>
        )}
      </div>
    </div>
  );
}
