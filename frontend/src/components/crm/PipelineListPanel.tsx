import { Pencil, Plus, Trash2 } from 'lucide-react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { cn } from '@/lib/utils';
import type { CrmPipeline } from '@/types/crm';

interface PipelineListPanelProps {
  pipelines: CrmPipeline[];
  pipelinesLoading: boolean;
  selectedPipelineId: string | null;
  onSelectPipeline: (id: string) => void;
  onAddPipeline: () => void;
  onEditPipeline: (pipeline: CrmPipeline) => void;
  onDeletePipeline: (pipeline: CrmPipeline) => void;
}

export function PipelineListPanel({
  pipelines,
  pipelinesLoading,
  selectedPipelineId,
  onSelectPipeline,
  onAddPipeline,
  onEditPipeline,
  onDeletePipeline,
}: PipelineListPanelProps) {
  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between gap-2">
        <div>
          <CardTitle className="text-base">Pipelines</CardTitle>
          <CardDescription>Scope CRM boards per initiative.</CardDescription>
        </div>
        <Button size="sm" onClick={onAddPipeline}>
          <Plus className="h-3.5 w-3.5 mr-1" />
          Add
        </Button>
      </CardHeader>
      <CardContent>
        {pipelinesLoading ? (
          <div className="space-y-3">
            {[1, 2, 3].map((item) => (
              <Skeleton key={item} className="h-12 w-full" />
            ))}
          </div>
        ) : pipelines.length === 0 ? (
          <div className="text-sm text-muted-foreground">
            No pipelines yet. Create the first pipeline to get started.
          </div>
        ) : (
          <div className="space-y-2">
            {pipelines.map((pipeline) => (
              <div
                key={pipeline.id}
                role="button"
                tabIndex={0}
                className={cn(
                  'w-full rounded-md border px-3 py-2 text-left text-sm transition hover:bg-accent cursor-pointer',
                  selectedPipelineId === pipeline.id && 'border-primary bg-primary/5'
                )}
                onClick={() => onSelectPipeline(pipeline.id)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    onSelectPipeline(pipeline.id);
                  }
                }}
              >
                <div className="flex items-center justify-between">
                  <span className="font-medium">{pipeline.name}</span>
                  <Badge variant="outline" className="text-xs">
                    {pipeline.pipeline_type}
                  </Badge>
                </div>
                <p className="text-xs text-muted-foreground truncate">
                  {pipeline.description || 'No description'}
                </p>
                <div className="mt-1 flex items-center gap-2 text-xs text-muted-foreground">
                  <span
                    className="inline-flex h-2 w-2 rounded-full"
                    style={{ backgroundColor: pipeline.color ?? '#3B82F6' }}
                  />
                  {pipeline.pipeline_type === 'custom' ? 'Custom pipeline' : 'System pipeline'}
                </div>
                <div className="mt-2 flex gap-2">
                  <Button
                    type="button"
                    size="sm"
                    variant="secondary"
                    className="h-7 text-xs"
                    onClick={(event) => {
                      event.stopPropagation();
                      onEditPipeline(pipeline);
                    }}
                  >
                    <Pencil className="mr-1 h-3 w-3" />
                    Edit
                  </Button>
                  <Button
                    type="button"
                    size="sm"
                    variant="ghost"
                    className="h-7 text-xs text-destructive"
                    onClick={(event) => {
                      event.stopPropagation();
                      onDeletePipeline(pipeline);
                    }}
                    disabled={pipelines.length <= 1}
                  >
                    <Trash2 className="mr-1 h-3 w-3" />
                    Delete
                  </Button>
                </div>
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
