import { useQueryClient } from '@tanstack/react-query';
import { FileText, Phone } from 'lucide-react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { CardGrid } from '@/components/ui/card-grid';
import { crmKeys } from '@/lib/query-keys';
import type { CrmDealWithContact } from '@/types/crm';
import { discovery as tid } from 'shared/testids';

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
  onSwitchTab?: (tab: string) => void;
}

export function OverviewTab({
  deal,
  stageColor,
  onConvert,
  orgId,
  onSwitchTab,
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

  const discoveryCallStatus = customFields.discovery_call_status as string | undefined;

  return (
    <div className="p-5">
      <CardGrid columns={{ md: 2 }} gap={6}>
        {/* Column 1: Context, Expedite, Metrics, Call Scheduling */}
        <div className="space-y-5">
          {/* Discovery Stage Hero Card */}
          {currentStage === 'discovery' && (
            <Card data-testid={tid.heroCard} className="border-purple-500/30 bg-purple-50/50 dark:bg-purple-950/20">
              <CardHeader className="pb-3">
                <CardTitle className="flex items-center gap-2 text-base">
                  <Phone className="h-4 w-4 text-purple-600" />
                  Discovery Call
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-3">
                <div className="flex items-center gap-2">
                  <span className="text-sm text-muted-foreground">Status:</span>
                  <Badge
                    data-testid={tid.callStatus}
                    variant={discoveryCallStatus === 'completed' ? 'default' : 'secondary'}
                    className={discoveryCallStatus === 'completed' ? 'bg-green-600' : ''}
                  >
                    {discoveryCallStatus === 'completed' ? 'Completed' : discoveryCallStatus === 'scheduled' ? 'Scheduled' : 'Not scheduled'}
                  </Badge>
                </div>
                <div className="flex gap-2">
                  <Button
                    data-testid={tid.scheduleCall}
                    variant="outline"
                    size="sm"
                    className="gap-1.5"
                    onClick={() => {
                      document.querySelector('[data-testid="call-discovery-row"]')?.scrollIntoView({ behavior: 'smooth' });
                    }}
                  >
                    <Phone className="h-3.5 w-3.5" />
                    {discoveryCallStatus ? 'Edit Call' : 'Schedule Call'}
                  </Button>
                  {discoveryCallStatus === 'completed' && onSwitchTab && (
                    <Button
                      data-testid={tid.linkTranscript}
                      variant="outline"
                      size="sm"
                      className="gap-1.5"
                      onClick={() => onSwitchTab('transcripts')}
                    >
                      <FileText className="h-3.5 w-3.5" />
                      Link Transcript
                    </Button>
                  )}
                </div>
              </CardContent>
            </Card>
          )}

          <OverviewContextSection deal={deal} invalidateKanban={invalidateKanban} />

          {/* Call Scheduling (Discovery / Proposal / Present stages) */}
          {(currentStage === 'discovery' ||
            currentStage === 'proposal' ||
            currentStage === 'present' ||
            currentStage === 'present_&_invoice') && (
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
