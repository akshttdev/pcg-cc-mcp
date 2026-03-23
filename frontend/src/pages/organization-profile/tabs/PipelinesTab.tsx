import { useState } from 'react';
import { Target, TrendingUp } from 'lucide-react';
import { CrmPipelineBoard } from '@/components/crm/CrmPipelineBoard';
import { CrmPipelineSettings } from '@/components/crm/CrmPipelineSettings';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import type { PipelineType } from '@/types/crm';

export function PipelinesTab({ orgId, defaultPipeline }: { orgId: string; defaultPipeline?: string }) {
  const [pipelineType, setPipelineType] = useState<'sales' | 'delivery'>(
    defaultPipeline === 'lifecycle' ? 'delivery' : 'sales'
  );
  const [settingsOpen, setSettingsOpen] = useState(false);

  return (
    <div className="space-y-4">
      <div className="flex gap-1 p-1 bg-muted/50 rounded-lg w-fit">
        <button
          onClick={() => setPipelineType('sales')}
          className={`px-4 py-1.5 text-sm font-medium rounded-md transition-all ${
            pipelineType === 'sales'
              ? 'bg-background text-foreground shadow-sm'
              : 'text-muted-foreground hover:text-foreground'
          }`}
        >
          <Target className="h-3.5 w-3.5 inline mr-1.5" />
          Acquisition
        </button>
        <button
          onClick={() => setPipelineType('delivery')}
          className={`px-4 py-1.5 text-sm font-medium rounded-md transition-all ${
            pipelineType === 'delivery'
              ? 'bg-background text-foreground shadow-sm'
              : 'text-muted-foreground hover:text-foreground'
          }`}
        >
          <TrendingUp className="h-3.5 w-3.5 inline mr-1.5" />
          Lifecycle
        </button>
      </div>
      <CrmPipelineBoard
        orgId={orgId}
        pipelineType={pipelineType as PipelineType}
        title={pipelineType === 'sales' ? 'Acquisition Pipeline' : 'Client Lifecycle'}
        onSettingsClick={() => setSettingsOpen(true)}
      />
      <Dialog open={settingsOpen} onOpenChange={setSettingsOpen}>
        <DialogContent className="max-w-4xl max-h-[85vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Pipeline Settings</DialogTitle>
          </DialogHeader>
          <CrmPipelineSettings organizationId={orgId} />
        </DialogContent>
      </Dialog>
    </div>
  );
}
