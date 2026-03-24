import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogTitle,
} from '@/components/ui/dialog';
import { AlertTriangle } from 'lucide-react';
import type { WorkflowNode } from '@/lib/api';

interface DeleteNodeDialogProps {
  pendingDeleteNodeId: string | null;
  setPendingDeleteNodeId: (id: string | null) => void;
  nodes: WorkflowNode[];
  deleteNode: (id: string) => void;
}

export function DeleteNodeDialog({
  pendingDeleteNodeId,
  setPendingDeleteNodeId,
  nodes,
  deleteNode,
}: DeleteNodeDialogProps) {
  return (
    <Dialog open={!!pendingDeleteNodeId} onOpenChange={(open) => !open && setPendingDeleteNodeId(null)}>
      <DialogContent className="max-w-sm">
        <div className="flex flex-col items-center text-center py-2">
          <div className="w-10 h-10 rounded-full bg-destructive/10 flex items-center justify-center mb-3">
            <AlertTriangle className="h-5 w-5 text-destructive" />
          </div>
          <DialogTitle className="text-base">Delete Node</DialogTitle>
          <p className="text-sm text-muted-foreground mt-1.5">
            Delete <strong>{nodes.find(n => n.id === pendingDeleteNodeId)?.name ?? 'this node'}</strong>?
            This will also remove its connections. This cannot be undone.
          </p>
          <div className="flex gap-2 mt-4 w-full">
            <Button
              variant="outline"
              size="sm"
              className="flex-1"
              onClick={() => setPendingDeleteNodeId(null)}
            >
              Cancel
            </Button>
            <Button
              variant="destructive"
              size="sm"
              className="flex-1"
              onClick={() => {
                if (pendingDeleteNodeId) {
                  deleteNode(pendingDeleteNodeId);
                  setPendingDeleteNodeId(null);
                }
              }}
            >
              Delete
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
