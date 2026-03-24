import { useState } from 'react';
import { dealDetail as tid } from 'shared/testids';

import { Dialog, DialogContent, DialogDescription,DialogTitle } from '@/components/ui/dialog';
import { ResizableDrawer } from '@/components/ui/resizable-drawer';
import { ScrollArea } from '@/components/ui/scroll-area';
import type { TabDefinition } from '@/components/ui/tabs';
import { TabPanel, TabsContent } from '@/components/ui/tabs';
import type { CrmDealWithContact, CrmPipelineStage } from '@/types/crm';

import { DealConvertDialog } from '../DealConvertDialog';
import { DealHeader, PipelineStepper } from './DealHeader';
import { ActivityTab } from './tabs/ActivityTab';
import { AgentHistoryTab } from './tabs/AgentHistoryTab';
import { DeckTab } from './tabs/DeckTab';
import { IntelTab } from './tabs/IntelTab';
import { OverviewTab } from './tabs/OverviewTab';
import { ProjectsTab } from './tabs/ProjectsTab';
import { ProposalTab } from './tabs/ProposalTab';
import { ReviewTab } from './tabs/ReviewTab';
import { TranscriptsTab } from './tabs/TranscriptsTab';

// ── Props ────────────────────────────────────────────────────────────────────

interface CrmDealDetailPanelProps {
  deal: CrmDealWithContact | null;
  isOpen: boolean;
  onClose: () => void;
  onEdit: (deal: CrmDealWithContact) => void;
  onDelete: (deal: CrmDealWithContact) => void;
  onMoveTo?: (deal: CrmDealWithContact, stageId: string) => void;
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
  onMoveTo,
  orgId,
  projectId,
  stageName,
  allStages,
}: CrmDealDetailPanelProps) {
  const [activeTab, setActiveTab] = useState('overview');
  const [convertOpen, setConvertOpen] = useState(false);
  const [isFullscreen, setIsFullscreen] = useState(false);

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
  const proposalDot = deal.won_at ? undefined : deal.proposal_status === 'approved' ? 'green' : deal.proposal_text ? 'amber' : undefined;
  const deckDot = deal.won_at ? 'green' : deal.deck_url ? 'amber' : undefined;

  const panelContent = (isExpanded: boolean, toggleExpand: () => void) => (
    <div className="flex flex-col h-full overflow-hidden bg-background" data-testid="deal-detail-panel">
      {/* Stage color top bar */}
      <div className="h-1 w-full shrink-0" style={{ backgroundColor: stageColor }} />

      {/* Header */}
      <DealHeader
        deal={deal}
        stageColor={stageColor}
        onEdit={onEdit}
        onDelete={onDelete}
        onClose={onClose}
        isExpanded={isExpanded}
        onToggleExpand={toggleExpand}
      />

          {/* Pipeline Stage Stepper */}
          <PipelineStepper
            currentStage={effectiveStageName}
            allStages={allStages}
            onStageClick={deal && onMoveTo ? (_name, stageId) => onMoveTo(deal, stageId) : undefined}
          />

          {/* Tabs */}
          <TabPanel
            tabs={[
              { value: 'overview', label: 'Overview' },
              { value: 'intel', label: 'Intel', indicator: intelDone ? 'green' : null },
              { value: 'review', label: 'Review', indicator: hasActiveReview ? 'amber' : null },
              { value: 'transcripts', label: 'Transcripts' },
              { value: 'proposal', label: 'Proposal', indicator: proposalDot as TabDefinition['indicator'] },
              { value: 'deck', label: 'Deck & Close', indicator: deckDot as TabDefinition['indicator'] },
              { value: 'projects', label: 'Projects' },
              { value: 'activity', label: 'Activity' },
              { value: 'agents', label: 'Agent History', indicator: deal.active_agent_flow_status === 'executing' ? 'amber' : null },
            ] satisfies TabDefinition[]}
            value={activeTab}
            onValueChange={setActiveTab}
            className="flex-1 flex flex-col min-h-0"
            listClassName="mx-5 mt-3 mb-0 h-9 bg-transparent p-0 border-b rounded-none justify-start gap-0 w-auto shrink-0 overflow-x-auto"
            triggerClassName="h-9 rounded-none px-3 text-xs font-medium border-b-2 border-transparent data-[state=active]:border-primary data-[state=active]:text-foreground data-[state=active]:shadow-none bg-transparent"
            testId="deal-detail-tabs"
          >

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

              <TabsContent value="transcripts" className="h-full m-0">
                <ScrollArea className="h-full">
                  <TranscriptsTab deal={deal} />
                </ScrollArea>
              </TabsContent>

              <TabsContent value="proposal" className="h-full m-0">
                <ScrollArea className="h-full">
                  <ProposalTab deal={deal} />
                </ScrollArea>
              </TabsContent>

              <TabsContent value="deck" className="h-full m-0">
                <ScrollArea className="h-full">
                  <DeckTab deal={deal} onMarkWon={onClose} />
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

              <TabsContent value="agents" className="h-full m-0">
                <ScrollArea className="h-full">
                  <AgentHistoryTab dealId={deal.id} />
                </ScrollArea>
              </TabsContent>
            </div>
          </TabPanel>
    </div>
  );

  // Fullscreen dialog mode
  if (isFullscreen) {
    return (
      <>
        <Dialog open={isOpen} onOpenChange={(open) => { if (!open) { setIsFullscreen(false); onClose(); } }}>
          <DialogContent className="max-w-6xl h-[90vh] p-0 overflow-hidden flex flex-col" data-testid={tid.expanded}>
            <DialogTitle className="sr-only">{deal.name}</DialogTitle>
            <DialogDescription className="sr-only">Deal detail panel</DialogDescription>
            {panelContent(true, () => setIsFullscreen(false))}
          </DialogContent>
        </Dialog>
        <DealConvertDialog deal={deal} open={convertOpen} onOpenChange={setConvertOpen} orgId={orgId} />
      </>
    );
  }

  // Resizable drawer mode (default) — drag left edge to resize
  return (
    <>
      <ResizableDrawer
        open={isOpen}
        onClose={onClose}
        defaultWidth={560}
        minWidth={400}
        storageKey="orcha:deal-drawer-width"
        className="border-l border-border bg-background"
        data-testid={tid.drawer}
        onExpand={() => setIsFullscreen(true)}
      >
        {() => panelContent(false, () => setIsFullscreen(true))}
      </ResizableDrawer>
      <DealConvertDialog deal={deal} open={convertOpen} onOpenChange={setConvertOpen} orgId={orgId} />
    </>
  );
}
