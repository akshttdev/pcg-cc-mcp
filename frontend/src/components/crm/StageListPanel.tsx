import { ArrowDown, ArrowUp, Pencil, Plus, Trash2 } from 'lucide-react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import { Skeleton } from '@/components/ui/skeleton';
import type { CrmPipelineStage } from '@/types/crm';

interface StageListPanelProps {
  pipelineName?: string;
  stageCountText: string;
  stages?: CrmPipelineStage[];
  isLoading: boolean;
  hasPipeline: boolean;
  reorderPending: boolean;
  onAddStage: () => void;
  onEditStage: (stage: CrmPipelineStage) => void;
  onDeleteStage: (stage: CrmPipelineStage) => void;
  onMoveStage: (stageId: string, direction: 'up' | 'down') => void;
}

export function StageListPanel({
  pipelineName,
  stageCountText,
  stages,
  isLoading,
  hasPipeline,
  reorderPending,
  onAddStage,
  onEditStage,
  onDeleteStage,
  onMoveStage,
}: StageListPanelProps) {
  return (
    <Card className="min-h-[420px]">
      <CardHeader className="flex flex-row items-center justify-between gap-4">
        <div>
          <CardTitle className="text-base">
            {pipelineName ?? 'Select a pipeline'}
          </CardTitle>
          <CardDescription>
            {hasPipeline ? stageCountText : 'Choose a pipeline to manage stages.'}
          </CardDescription>
        </div>
        {hasPipeline && (
          <Button size="sm" onClick={onAddStage}>
            <Plus className="h-3.5 w-3.5 mr-1" />
            Add Stage
          </Button>
        )}
      </CardHeader>
      <CardContent className="p-0">
        {isLoading ? (
          <div className="space-y-3 p-4">
            {[1, 2, 3].map((item) => (
              <Skeleton key={item} className="h-16 w-full" />
            ))}
          </div>
        ) : !hasPipeline ? (
          <div className="p-6 text-sm text-muted-foreground">
            Select a pipeline to edit its stages.
          </div>
        ) : (
          <ScrollArea className="h-[520px]">
            <div className="space-y-3 p-4">
              {stages?.map((stage, index) => (
                <div
                  key={stage.id}
                  className="rounded-lg border bg-card px-4 py-3"
                >
                  <div className="flex items-center justify-between gap-4">
                    <div>
                      <div className="flex items-center gap-2">
                        <span
                          className="inline-flex h-3 w-3 rounded-full"
                          style={{ backgroundColor: stage.color }}
                        />
                        <p className="font-medium text-sm">{stage.name}</p>
                        <Badge variant="secondary" className="text-[11px]">
                          {stage.probability}%
                        </Badge>
                      </div>
                      <div className="mt-1 flex gap-2 text-xs text-muted-foreground">
                        {stage.is_closed ? <Badge variant="outline">Closed</Badge> : <Badge variant="outline">Open</Badge>}
                        {stage.is_won ? <Badge variant="outline">Won</Badge> : <Badge variant="outline">Standard</Badge>}
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <Button
                        size="icon"
                        variant="ghost"
                        className="h-8 w-8"
                        onClick={() => onMoveStage(stage.id, 'up')}
                        disabled={index === 0 || reorderPending}
                        title="Move up"
                      >
                        <ArrowUp className="h-4 w-4" />
                      </Button>
                      <Button
                        size="icon"
                        variant="ghost"
                        className="h-8 w-8"
                        onClick={() => onMoveStage(stage.id, 'down')}
                        disabled={index === (stages?.length ?? 0) - 1 || reorderPending}
                        title="Move down"
                      >
                        <ArrowDown className="h-4 w-4" />
                      </Button>
                      <Button
                        size="icon"
                        variant="ghost"
                        className="h-8 w-8"
                        onClick={() => onEditStage(stage)}
                        title="Edit stage"
                      >
                        <Pencil className="h-4 w-4" />
                      </Button>
                      <Button
                        size="icon"
                        variant="ghost"
                        className="h-8 w-8 text-destructive"
                        onClick={() => onDeleteStage(stage)}
                        title="Delete stage"
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </div>
                  </div>
                  {stage.description && (
                    <p className="mt-2 text-xs text-muted-foreground">{stage.description}</p>
                  )}
                  {index < (stages?.length ?? 0) - 1 && <Separator className="mt-3" />}
                </div>
              ))}
              {(!stages || stages.length === 0) && (
                <div className="text-sm text-muted-foreground">
                  No stages configured. Add the first stage to define your pipeline.
                </div>
              )}
            </div>
          </ScrollArea>
        )}
      </CardContent>
    </Card>
  );
}
