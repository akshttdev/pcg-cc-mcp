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
  AlertTriangle,
  ArrowRight,
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
import { StatusBadge } from '@/components/ui/status-badge';
import { Link } from 'react-router-dom';
import type { CrmDealWithContact } from '@/types/crm';
import { dealCard as tid } from 'shared/testids';
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
  const agentFailed = deal.active_agent_flow_status === 'failed';
  const agentCancelled = deal.active_agent_flow_status === 'cancelled';
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
            <AvatarFallback className="text-xs font-semibold" style={{
              backgroundColor: `${accentColor}22`,
              color: accentColor,
            }}>
              {initials}
            </AvatarFallback>
          </Avatar>

          <div className="min-w-0 flex-1">
            <p className="font-medium text-sm leading-tight line-clamp-2">{deal.name}</p>
            {deal.contact_company && (
              <p className="text-xs text-muted-foreground truncate flex items-center gap-1 mt-0.5">
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
                  data-testid={tid.menu(deal.id)}
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
        {(agentRunning || agentPending || agentFailed || agentCancelled || researchNeeded || researchReady || intelRunning || hasActiveReviewTask || reviewTaskDone ||
          deal.report_review_status === 'rejected' || hasProposal || hasDeck || hasInvoice || isWon) && (
          <div className="flex flex-wrap gap-1">
            {agentRunning && <StatusBadge status="info" icon={Bot} label={`${agentName ?? 'Agent'} running…`} pulse size="sm" />}
            {agentPending && <StatusBadge status="warning" icon={Clock} label={`${agentName ?? 'Agent'} pending`} pulse size="sm" />}
            {agentFailed && <StatusBadge status="error" icon={AlertTriangle} label={`${agentName ?? 'Agent'} failed`} size="sm" />}
            {agentCancelled && <StatusBadge status="warning" icon={RotateCcw} label={`${agentName ?? 'Agent'} cancelled`} size="sm" />}
            {researchNeeded && <StatusBadge status="warning" icon={Search} label="Research needed" size="sm" />}
            {intelRunning && <StatusBadge status="info" icon={Loader2} label="Researching…" pulse size="sm" />}
            {researchReady && !hasActiveReviewTask && <StatusBadge status="success" icon={ShieldCheck} label="Ready for review" size="sm" />}
            {hasActiveReviewTask && <StatusBadge status="info" icon={CircleDot} label="Needs review" pulse size="sm" />}
            {reviewTaskDone && <StatusBadge status="success" icon={CheckCircle2} label="Review done" size="sm" />}
            {deal.report_review_status === 'rejected' && <StatusBadge status="error" icon={RotateCcw} label="Revision needed" size="sm" />}
            {hasProposal && !proposalApproved && <StatusBadge status="warning" icon={FileText} label="Proposal draft" size="sm" />}
            {proposalApproved && <StatusBadge status="success" icon={FileText} label="Proposal ✓" size="sm" />}
            {hasDeck && <StatusBadge status="success" icon={Presentation} label="Deck ready" size="sm" />}
            {hasInvoice && <StatusBadge status="info" icon={Receipt} label="Invoice sent" size="sm" />}
            {isWon && <StatusBadge status="success" icon={Trophy} label="Won!" size="sm" />}
          </div>
        )}

        {/* Row 3: Intel summary snippet */}
        {intelDone && deal.intelligence_summary && (
          <p className="text-xs text-muted-foreground leading-relaxed italic border-l-2 pl-2 line-clamp-2"
            style={{ borderColor: `${accentColor}60` }}>
            {deal.intelligence_summary}
          </p>
        )}

        {/* Row 4: Review task assignee */}
        {deal.review_task_id && deal.review_task_assignee && (
          <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
            <User className="h-3 w-3 shrink-0" />
            <span className="truncate">{deal.review_task_assignee}</span>
            {deal.review_task_status && (
              <span className={cn(
                'text-xs font-medium',
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
            <div className="flex items-center justify-between text-xs text-muted-foreground">
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
            <div className="flex items-center justify-between text-xs text-muted-foreground">
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
              <span key={tag} className="inline-flex items-center px-1.5 py-0.5 rounded text-xs bg-muted text-muted-foreground border border-border/50">
                {tag}
              </span>
            ))}
            {tags.length > 2 && (
              <span className="inline-flex items-center px-1.5 py-0.5 rounded text-xs bg-muted text-muted-foreground border border-border/50">
                +{tags.length - 2}
              </span>
            )}
          </div>
        )}

        {/* Row 8: Footer — amount, close date, probability */}
        <div className="flex items-center gap-2 pt-1.5 border-t border-border/40">
          {formattedAmount && (
            <span className="inline-flex items-center text-xs font-semibold text-green-600">
              {formattedAmount}
            </span>
          )}
          {deal.probability > 0 && (
            <span className="inline-flex items-center gap-0.5 text-xs text-muted-foreground">
              <TrendingUp className="h-2.5 w-2.5" />
              {deal.probability}%
            </span>
          )}
        </div>
        {/* Row 9: Timestamp */}
        {lastActivity && (
          <span className="inline-flex items-center gap-1 text-xs text-muted-foreground/60">
            <Clock className="h-2.5 w-2.5 shrink-0" />
            {lastActivity}
          </span>
        )}

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
