import { useQueryClient } from '@tanstack/react-query';

import { CardGrid } from '@/components/ui/card-grid';
import { crmKeys } from '@/lib/query-keys';
import type { CrmDealWithContact } from '@/types/crm';

import { CallSchedulingSection } from './OverviewCallSchedulingSection';
import { OverviewContactSection } from './OverviewContactSection';
import { OverviewContextSection } from './OverviewContextSection';
import { OverviewMetricsSection } from './OverviewMetricsSection';
import { OverviewTimelineSection } from './OverviewTimelineSection';

// ── OverviewTab ──────────────────────────────────────────────────────────────

interface OverviewTabProps {
  deal: CrmDealWithContact;
  stageColor: string;
  onConvert: () => void;
  orgId?: string;
}

export function OverviewTab({
  deal,
  stageColor,
  onConvert,
  orgId,
}: OverviewTabProps) {
  const currentStage = (deal.stage ?? '').toLowerCase().replace(/\s+/g, '_');

  const qc = useQueryClient();
  const invalidateKanban = () => {
    qc.invalidateQueries({ queryKey: crmKeys.kanbanAll() });
    qc.invalidateQueries({ queryKey: crmKeys.orgKanbanAll() });
    qc.invalidateQueries({ queryKey: crmKeys.kanbanLegacy() });
  };

  // Parse call scheduling from custom_fields
  const customFields = (() => {
    try {
      return typeof deal.custom_fields === 'string'
        ? JSON.parse(deal.custom_fields)
        : (deal.custom_fields ?? {});
    } catch {
      return {};
    }
  })();

  return (
    <div className="p-5">
      <CardGrid columns={{ md: 2 }} gap={6}>
        {/* Column 1: Context, Expedite, Metrics, Call Scheduling */}
        <div className="space-y-5">
          <OverviewContextSection deal={deal} invalidateKanban={invalidateKanban} />

          {/* Call Scheduling (Discovery / Proposal / Present stages) */}
          {(currentStage === 'discovery' ||
            currentStage === 'proposal' ||
            currentStage === 'present') && (
            <CallSchedulingSection
              deal={deal}
              customFields={customFields}
              invalidateKanban={invalidateKanban}
            />
          )}

          <OverviewMetricsSection deal={deal} stageColor={stageColor} />
        </div>

        {/* Column 2: Contact, Organization, Timeline, Actions */}
        <div className="space-y-5">
          <OverviewContactSection deal={deal} orgId={orgId} />
          <OverviewTimelineSection deal={deal} onConvert={onConvert} />
        </div>
      </CardGrid>
    </div>
  );
}
