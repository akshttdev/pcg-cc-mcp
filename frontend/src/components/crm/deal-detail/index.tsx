import { useState } from 'react';
import { Sheet, SheetContent } from '@/components/ui/sheet';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { ScrollArea } from '@/components/ui/scroll-area';
import { cn } from '@/lib/utils';
import { DealHeader, PipelineStepper } from './DealHeader';
import { OverviewTab } from './tabs/OverviewTab';
import { IntelTab } from './tabs/IntelTab';
import { ReviewTab } from './tabs/ReviewTab';
import { ProjectsTab } from './tabs/ProjectsTab';
import { ActivityTab } from './tabs/ActivityTab';
import { DealConvertDialog } from '../DealConvertDialog';
import type { CrmDealWithContact, CrmPipelineStage } from '@/types/crm';

// ── Props ────────────────────────────────────────────────────────────────────

interface CrmDealDetailPanelProps {
  deal: CrmDealWithContact | null;
  isOpen: boolean;
  onClose: () => void;
  onEdit: (deal: CrmDealWithContact) => void;
  onDelete: (deal: CrmDealWithContact) => void;
  orgId?: string;
  projectId?: string;
  stageName?: string;
  allStages?: CrmPipelineStage[];
}

// ── CrmDealDetailPanel ──────────────────────────────────────────────────────

export function CrmDealDetailPanel({
  deal,
  isOpen,
  onClose,
  onEdit,
  onDelete,
  orgId,
  projectId,
  stageName,
  allStages,
}: CrmDealDetailPanelProps) {
  const [activeTab, setActiveTab] = useState('overview');
  const [convertOpen, setConvertOpen] = useState(false);

  if (!deal) return null;

  const effectiveStageName = stageName || deal.stage || '';

  const stageColor =
    allStages?.find((s) => s.name.toLowerCase() === effectiveStageName.toLowerCase())?.color ||
    '#6B7280';

  const intelDone = deal.intelligence_status === 'done';
  const hasActiveReview =
    deal.review_task_id &&
    deal.review_task_status !== 'done' &&
    deal.review_task_status !== 'cancelled';

  return (
    <>
      <Sheet open={isOpen} onOpenChange={(open) => !open && onClose()}>
        <SheetContent className="w-full sm:max-w-xl overflow-hidden flex flex-col p-0">
          {/* Stage color top bar */}
          <div className="h-1 w-full shrink-0" style={{ backgroundColor: stageColor }} />

          {/* Header */}
          <DealHeader deal={deal} stageColor={stageColor} onEdit={onEdit} onDelete={onDelete} />

          {/* Pipeline Stage Stepper */}
          <PipelineStepper currentStage={effectiveStageName} allStages={allStages} />

          {/* Tabs */}
          <Tabs
            value={activeTab}
            onValueChange={setActiveTab}
            className="flex-1 flex flex-col min-h-0"
          >
            <TabsList className="mx-5 mt-3 mb-0 h-9 bg-transparent p-0 border-b rounded-none justify-start gap-0 w-auto shrink-0">
              {[
                { value: 'overview', label: 'Overview' },
                { value: 'intel', label: 'Intel', dot: intelDone ? 'green' : undefined },
                {
                  value: 'review',
                  label: 'Review',
                  dot: hasActiveReview ? 'amber' : undefined,
                },
                { value: 'projects', label: 'Projects' },
                { value: 'activity', label: 'Activity' },
              ].map(({ value, label, dot }) => (
                <TabsTrigger
                  key={value}
                  value={value}
                  className="relative h-9 rounded-none px-3 text-xs font-medium border-b-2 border-transparent data-[state=active]:border-primary data-[state=active]:text-foreground data-[state=active]:shadow-none bg-transparent"
                >
                  {label}
                  {dot && (
                    <span
                      className={cn(
                        'absolute top-1.5 right-1 w-1.5 h-1.5 rounded-full',
                        dot === 'green' ? 'bg-green-500' : 'bg-amber-500 animate-pulse'
                      )}
                    />
                  )}
                </TabsTrigger>
              ))}
            </TabsList>

            <div className="flex-1 min-h-0 overflow-hidden">
              <TabsContent value="overview" className="h-full m-0">
                <ScrollArea className="h-full">
                  <OverviewTab
                    deal={deal}
                    stageColor={stageColor}
                    onConvert={() => setConvertOpen(true)}
                    orgId={orgId}
                  />
                </ScrollArea>
              </TabsContent>

              <TabsContent value="intel" className="h-full m-0">
                <ScrollArea className="h-full">
                  <IntelTab deal={deal} />
                </ScrollArea>
              </TabsContent>

              <TabsContent value="review" className="h-full m-0">
                <ScrollArea className="h-full">
                  <ReviewTab deal={deal} stageName={effectiveStageName} />
                </ScrollArea>
              </TabsContent>

              <TabsContent value="projects" className="h-full m-0">
                <ScrollArea className="h-full">
                  <ProjectsTab deal={deal} orgId={orgId} />
                </ScrollArea>
              </TabsContent>

              <TabsContent value="activity" className="h-full m-0">
                <ScrollArea className="h-full">
                  <ActivityTab deal={deal} projectId={projectId} />
                </ScrollArea>
              </TabsContent>
            </div>
          </Tabs>
        </SheetContent>
      </Sheet>

      <DealConvertDialog deal={deal} open={convertOpen} onOpenChange={setConvertOpen} orgId={orgId} />
    </>
  );
}
