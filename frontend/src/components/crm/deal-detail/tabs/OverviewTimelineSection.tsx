import { formatDistanceToNow } from 'date-fns';
import {
  BarChart3,
  Calendar,
  CheckCircle2,
  Clock,
  FolderKanban,
  Rocket,
  Tag,
  Workflow,
} from 'lucide-react';
import { useNavigate } from 'react-router-dom';

import { AskTopsiButton } from '@/components/topsi/AskTopsiButton';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import type { CrmDealWithContact } from '@/types/crm';

interface OverviewTimelineSectionProps {
  deal: CrmDealWithContact;
  onConvert: () => void;
}

export function OverviewTimelineSection({ deal, onConvert }: OverviewTimelineSectionProps) {
  const navigate = useNavigate();
  const currentStage = (deal.stage ?? '').toLowerCase().replace(/\s+/g, '_');

  let tags: string[] = [];
  if (deal.tags) {
    try {
      tags = JSON.parse(deal.tags);
    } catch {
      tags = deal.tags
        .split(',')
        .map((t) => t.trim())
        .filter(Boolean);
    }
  }

  let sourceInfo: { dataSourceId?: string; workflowRunId?: string } | null = null;
  if (deal.custom_fields) {
    try {
      const cf =
        typeof deal.custom_fields === 'string'
          ? JSON.parse(deal.custom_fields)
          : deal.custom_fields;
      if (cf.source_data_source_id || cf.source_workflow_run_id) {
        sourceInfo = {
          dataSourceId: cf.source_data_source_id,
          workflowRunId: cf.source_workflow_run_id,
        };
      }
    } catch {
      /* ignore */
    }
  }

  return (
    <>
      {/* Timeline */}
      <div>
        <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">
          Timeline
        </h4>
        <div className="space-y-1.5 text-sm">
          <div className="flex items-center gap-2 text-muted-foreground">
            <Calendar className="h-3.5 w-3.5 shrink-0" />
            <span className="text-muted-foreground/70">Created</span>
            <span className="ml-auto text-foreground">
              {new Date(deal.created_at).toLocaleDateString()}
            </span>
          </div>
          {deal.last_activity_at && (
            <div className="flex items-center gap-2 text-muted-foreground">
              <BarChart3 className="h-3.5 w-3.5 shrink-0" />
              <span className="text-muted-foreground/70">Last activity</span>
              <span className="ml-auto text-foreground">
                {formatDistanceToNow(new Date(deal.last_activity_at), {
                  addSuffix: true,
                })}
              </span>
            </div>
          )}
          {deal.expected_close_date && (
            <div className="flex items-center gap-2 text-muted-foreground">
              <Clock className="h-3.5 w-3.5 shrink-0" />
              <span className="text-muted-foreground/70">Expected close</span>
              <span className="ml-auto text-foreground">
                {new Date(deal.expected_close_date).toLocaleDateString(
                  'en-US',
                  {
                    month: 'short',
                    day: 'numeric',
                    year: 'numeric',
                  }
                )}
              </span>
            </div>
          )}
          {deal.actual_close_date && (deal.won_at || deal.lost_reason || ['closed won', 'closed lost', 'closed_won', 'closed_lost'].includes(currentStage)) && (
            <div className="flex items-center gap-2 text-muted-foreground">
              <CheckCircle2 className="h-3.5 w-3.5 shrink-0 text-green-500" />
              <span className="text-muted-foreground/70">Closed</span>
              <span className="ml-auto text-foreground">
                {new Date(deal.actual_close_date).toLocaleDateString()}
              </span>
            </div>
          )}
        </div>
      </div>

      {/* Tags */}
      {tags.length > 0 && (
        <div>
          <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">
            Tags
          </h4>
          <div className="flex flex-wrap gap-1.5">
            {tags.map((tag) => (
              <Badge key={tag} variant="secondary" className="text-xs gap-1">
                <Tag className="h-3 w-3" />
                {tag}
              </Badge>
            ))}
          </div>
        </div>
      )}

      {/* Win/Lost reason */}
      {deal.win_reason && (
        <div>
          <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">
            Win Reason
          </h4>
          <p className="text-sm">{deal.win_reason}</p>
        </div>
      )}
      {deal.lost_reason && (
        <div>
          <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">
            Lost Reason
          </h4>
          <p className="text-sm">{deal.lost_reason}</p>
        </div>
      )}

      {/* Source provenance */}
      {sourceInfo && (
        <div>
          <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-1.5">
            Source
          </h4>
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <Workflow className="h-3.5 w-3.5 shrink-0" />
            <span>Imported via workflow</span>
          </div>
        </div>
      )}

      {/* Actions */}
      <div className="flex gap-2 pt-2 pb-4 border-t">
        {deal.project_id ? (
          <Button
            size="sm"
            variant="outline"
            className="flex-1 gap-1.5"
            onClick={() => navigate(`/projects/${deal.project_id}/tasks`)}
          >
            <FolderKanban className="h-3.5 w-3.5" />
            View Project
          </Button>
        ) : (
          <Button size="sm" className="flex-1 gap-1.5" onClick={onConvert}>
            <Rocket className="h-3.5 w-3.5" />
            Convert to Project
          </Button>
        )}
        <AskTopsiButton
          entityType="crm_deal"
          entityId={deal.id}
          entityName={deal.name}
          className="flex-1"
        />
      </div>
    </>
  );
}
