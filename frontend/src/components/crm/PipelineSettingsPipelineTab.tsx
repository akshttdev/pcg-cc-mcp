import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import { Pencil, Settings2, Trash2 } from 'lucide-react';
import type { CrmPipeline, CrmPipelineWithStages } from '@/types/crm';

interface PipelineSettingsPipelineTabProps {
  pipelineData: CrmPipelineWithStages;
  selectedPipeline: CrmPipeline | null;
  onEditPipeline: (pipeline: CrmPipeline) => void;
  onDeletePipeline: (pipeline: CrmPipeline) => void;
  canDelete: boolean;
}

export function PipelineSettingsPipelineTab({
  pipelineData,
  selectedPipeline,
  onEditPipeline,
  onDeletePipeline,
  canDelete,
}: PipelineSettingsPipelineTabProps) {
  return (
    <div className="p-4 space-y-4">
      <div className="space-y-3">
        <div className="flex items-center gap-2">
          <Settings2 className="h-4 w-4 text-muted-foreground" />
          <p className="text-sm font-medium">Pipeline Details</p>
        </div>
        <div className="grid gap-3 text-sm">
          <div className="flex items-center justify-between">
            <span className="text-muted-foreground">Name</span>
            <span className="font-medium">{pipelineData.name}</span>
          </div>
          <Separator />
          <div className="flex items-center justify-between">
            <span className="text-muted-foreground">Type</span>
            <Badge variant="outline">{pipelineData.pipeline_type}</Badge>
          </div>
          <Separator />
          <div className="flex items-center justify-between">
            <span className="text-muted-foreground">Description</span>
            <span className="text-right max-w-[250px] truncate">{pipelineData.description || '\u2014'}</span>
          </div>
          <Separator />
          <div className="flex items-center justify-between">
            <span className="text-muted-foreground">Color</span>
            <div className="flex items-center gap-2">
              <span className="inline-flex h-4 w-4 rounded-full border" style={{ backgroundColor: selectedPipeline?.color ?? '#3B82F6' }} />
              <span className="text-xs font-mono">{selectedPipeline?.color ?? '#3B82F6'}</span>
            </div>
          </div>
          <Separator />
          <div className="flex items-center justify-between">
            <span className="text-muted-foreground">Stages</span>
            <span>{pipelineData.stages.length}</span>
          </div>
          <Separator />
          <div className="flex items-center justify-between">
            <span className="text-muted-foreground">Status</span>
            <Badge variant={selectedPipeline?.is_active ? 'default' : 'secondary'}>{selectedPipeline?.is_active ? 'Active' : 'Inactive'}</Badge>
          </div>
        </div>
      </div>
      <div className="pt-2 flex gap-2">
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            if (selectedPipeline) onEditPipeline(selectedPipeline);
          }}
        >
          <Pencil className="h-3.5 w-3.5 mr-1.5" />
          Edit Pipeline
        </Button>
        <Button
          variant="ghost"
          size="sm"
          className="text-destructive"
          onClick={() => selectedPipeline && onDeletePipeline(selectedPipeline)}
          disabled={!canDelete}
        >
          <Trash2 className="h-3.5 w-3.5 mr-1.5" />
          Delete
        </Button>
      </div>
    </div>
  );
}
