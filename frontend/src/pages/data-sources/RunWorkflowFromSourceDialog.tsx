import { useState, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import { useQuery, useMutation } from '@tanstack/react-query';
import { Button } from '@/components/ui/button';
import { FileText, Play, Loader2 } from 'lucide-react';
import { workflowsApi, dataSourcesApi, type DataSourceRecord } from '@/lib/api';
import { workflowKeys } from '@/lib/query-keys';
import { toast } from 'sonner';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { EmptyState } from '@/components/ui/empty-state';

export function RunWorkflowFromSourceDialog({ source, onClose }: {
  source: DataSourceRecord | null;
  onClose: () => void;
}) {
  const navigate = useNavigate();
  const [selectedWorkflowId, setSelectedWorkflowId] = useState('');

  const { data: allWorkflows = [] } = useQuery({
    queryKey: workflowKeys.definitions(),
    queryFn: () => workflowsApi.listDefinitions(),
    enabled: !!source,
  });

  // Show workflows that have a data_source node (purpose-built for processing sources),
  // plus any workflow as a fallback (all workflows can accept data source content)
  const workflows = useMemo(() => {
    const dataSourceWorkflows = allWorkflows.filter(wf =>
      wf.nodes?.some(n => n.type === 'data_source')
    );
    const otherWorkflows = allWorkflows.filter(wf =>
      !wf.nodes?.some(n => n.type === 'data_source')
    );
    return [...dataSourceWorkflows, ...otherWorkflows];
  }, [allWorkflows]);

  const runMutation = useMutation({
    mutationFn: () => dataSourcesApi.runWorkflow(source!.id, selectedWorkflowId),
    onSuccess: (data) => {
      if (data.workflow_run_id && data.staged_records > 0) {
        toast.success(`Workflow complete — ${data.staged_records} records staged`);
        handleClose();
        navigate('/workflows?tab=staging&run=' + data.workflow_run_id);
      } else {
        toast.info('Workflow completed with no new records');
        handleClose();
      }
    },
    onError: () => toast.error('Failed to run workflow'),
  });

  const handleClose = () => {
    setSelectedWorkflowId('');
    runMutation.reset();
    onClose();
  };

  return (
    <Dialog open={!!source} onOpenChange={(open) => { if (!open) handleClose(); }}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Play className="h-4 w-4" />
            Run Workflow
          </DialogTitle>
        </DialogHeader>

        <div className="space-y-4">
          <div>
            <p className="text-sm text-muted-foreground mb-1">Data Source</p>
            <div className="flex items-center gap-2 p-2 rounded-md bg-muted/50 text-sm">
              <FileText className="h-4 w-4 text-muted-foreground shrink-0" />
              <span className="truncate font-medium">{source?.title}</span>
            </div>
          </div>

          <div>
            <p className="text-sm text-muted-foreground mb-1">Workflow</p>
            {workflows.length === 0 ? (
              <EmptyState
                title="No workflows available yet"
                action={{ label: "Create a Workflow", onClick: () => { handleClose(); navigate('/workflows'); } }}
                className="py-4 border rounded-md bg-muted/30"
              />
            ) : (
              <Select value={selectedWorkflowId} onValueChange={setSelectedWorkflowId}>
                <SelectTrigger>
                  <SelectValue placeholder="Select a workflow..." />
                </SelectTrigger>
                <SelectContent>
                  {workflows.map((wf) => (
                    <SelectItem key={wf.id} value={wf.id}>
                      <span>{wf.name}</span>
                      {wf.is_system && <span className="ml-2 text-muted-foreground text-xs">(System)</span>}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )}
          </div>

          <div className="flex gap-2 justify-end">
            <Button variant="ghost" onClick={handleClose}>Cancel</Button>
            <Button
              onClick={() => runMutation.mutate()}
              disabled={!selectedWorkflowId || runMutation.isPending}
            >
              {runMutation.isPending
                ? <Loader2 className="h-3.5 w-3.5 mr-2 animate-spin" />
                : <Play className="h-3.5 w-3.5 mr-2" />}
              {runMutation.isPending ? 'Running...' : 'Run Workflow'}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
