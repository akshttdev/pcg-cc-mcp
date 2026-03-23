import { IconButton } from '@/components/ui/icon-button';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import { ArrowDown, ArrowUp, Pencil, Trash2 } from 'lucide-react';
import type { CrmPipelineStage } from '@/types/crm';

interface PipelineSettingsStagesTabProps {
  stages: CrmPipelineStage[];
  onMoveStage: (stageId: string, direction: 'up' | 'down') => void;
  onEditStage: (stage: CrmPipelineStage) => void;
  onDeleteStage: (stage: CrmPipelineStage) => void;
  isReordering: boolean;
}

export function PipelineSettingsStagesTab({
  stages,
  onMoveStage,
  onEditStage,
  onDeleteStage,
  isReordering,
}: PipelineSettingsStagesTabProps) {
  return (
    <ScrollArea className="h-[480px]">
      <div className="space-y-3 p-4">
        {stages.map((stage, index) => (
          <div key={stage.id} className="rounded-lg border bg-card px-4 py-3">
            <div className="flex items-center justify-between gap-4">
              <div>
                <div className="flex items-center gap-2">
                  <span className="inline-flex h-3 w-3 rounded-full" style={{ backgroundColor: stage.color }} />
                  <p className="font-medium text-sm">{stage.name}</p>
                  <Badge variant="secondary" className="text-xs">{stage.probability}%</Badge>
                </div>
                <div className="mt-1 flex gap-2 text-xs text-muted-foreground">
                  {stage.is_closed ? <Badge variant="outline">Closed</Badge> : <Badge variant="outline">Open</Badge>}
                  {stage.is_won ? <Badge variant="outline">Won</Badge> : <Badge variant="outline">Standard</Badge>}
                </div>
              </div>
              <div className="flex items-center gap-2">
                <IconButton icon={ArrowUp} label="Move up" variant="ghost" className="h-8 w-8" onClick={() => onMoveStage(stage.id, 'up')} disabled={index === 0 || isReordering} />
                <IconButton icon={ArrowDown} label="Move down" variant="ghost" className="h-8 w-8" onClick={() => onMoveStage(stage.id, 'down')} disabled={index === stages.length - 1 || isReordering} />
                <IconButton icon={Pencil} label="Edit stage" variant="ghost" className="h-8 w-8" onClick={() => onEditStage(stage)} />
                <IconButton icon={Trash2} label="Delete stage" variant="ghost" className="h-8 w-8 text-destructive" onClick={() => onDeleteStage(stage)} />
              </div>
            </div>
            {stage.description && (
              <p className="mt-2 text-xs text-muted-foreground">{stage.description}</p>
            )}
            {index < stages.length - 1 && <Separator className="mt-3" />}
          </div>
        ))}
        {stages.length === 0 && (
          <div className="text-sm text-muted-foreground">
            No stages configured. Add the first stage to define your pipeline.
          </div>
        )}
      </div>
    </ScrollArea>
  );
}
