// ─── CopyWorkflowDialog ────────────────────────────────────────────────────
//
// Dialog to copy/promote a workflow between ownership levels:
// - User → Organization (pick target org)
// - Organization → User (confirm copy to personal)

import { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Copy, Building2, User, Loader2 } from 'lucide-react';
import { workflowsApi } from '@/lib/api';
import type { WorkflowDefinition } from '@/lib/api';
import { workflowKeys } from '@/lib/query-keys';
import { useOrganization } from '@/contexts/organization-context';

type CopyDirection = 'to-org' | 'to-user';

interface CopyWorkflowDialogProps {
  workflow: WorkflowDefinition | null;
  direction: CopyDirection;
  onClose: () => void;
}

function generateCopyId(baseId: string): string {
  const suffix = '_copy';
  const base = baseId.replace(/_copy(_\d+)?$/, '');
  return `${base}${suffix}`;
}

export function CopyWorkflowDialog({ workflow, direction, onClose }: CopyWorkflowDialogProps) {
  const queryClient = useQueryClient();
  const { organizations = [] } = useOrganization();
  const [selectedOrgId, setSelectedOrgId] = useState<string>('');

  const copyId = workflow ? generateCopyId(workflow.id) : '';
  const copyName = workflow ? `${workflow.name} (Copy)` : '';

  const copyMutation = useMutation({
    mutationFn: async () => {
      if (!workflow) throw new Error('No workflow selected');

      return workflowsApi.createDefinition({
        id: copyId,
        name: copyName,
        description: workflow.description,
        nodes: workflow.nodes,
        connections: workflow.connections,
        owner_type: direction === 'to-org' ? 'organization' : 'user',
        owner_id: direction === 'to-org' ? selectedOrgId : undefined,
        default_model: workflow.default_model,
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: workflowKeys.definitions() });
      onClose();
    },
  });

  const isOpen = workflow !== null;
  const canSubmit = direction === 'to-user' || selectedOrgId !== '';

  return (
    <Dialog open={isOpen} onOpenChange={(open) => { if (!open) onClose(); }}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Copy className="h-4 w-4" />
            {direction === 'to-org' ? 'Copy to Organization' : 'Copy to My Workflows'}
          </DialogTitle>
          <DialogDescription>
            {direction === 'to-org'
              ? 'Create a copy of this workflow owned by an organization.'
              : 'Create a personal copy of this workflow.'}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-2">
          {/* Source info */}
          <div className="rounded-md bg-muted/50 p-3 text-sm space-y-1">
            <div className="font-medium">{workflow?.name}</div>
            <div className="text-xs text-muted-foreground">
              Will be copied as: <span className="font-mono">{copyName}</span>
            </div>
            <div className="text-xs text-muted-foreground">
              ID: <span className="font-mono">{copyId}</span>
            </div>
          </div>

          {/* Org selector (only for to-org direction) */}
          {direction === 'to-org' && (
            <div className="space-y-2">
              <Label className="flex items-center gap-1.5">
                <Building2 className="h-3.5 w-3.5" />
                Target Organization
              </Label>
              <Select value={selectedOrgId} onValueChange={setSelectedOrgId}>
                <SelectTrigger>
                  <SelectValue placeholder="Select an organization..." />
                </SelectTrigger>
                <SelectContent>
                  {organizations.map((org) => (
                    <SelectItem key={org.id} value={org.id}>
                      {org.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          )}

          {direction === 'to-user' && (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <User className="h-4 w-4" />
              This will create a personal copy in your My Workflows.
            </div>
          )}

          {copyMutation.isError && (
            <p className="text-sm text-destructive">
              Failed to copy workflow. The ID &quot;{copyId}&quot; may already exist — try renaming.
            </p>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose}>Cancel</Button>
          <Button
            onClick={() => copyMutation.mutate()}
            disabled={!canSubmit || copyMutation.isPending}
          >
            {copyMutation.isPending && <Loader2 className="h-3.5 w-3.5 mr-1.5 animate-spin" />}
            Copy Workflow
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
