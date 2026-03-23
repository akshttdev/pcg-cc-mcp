import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { DialogTitle } from '@/components/ui/dialog';
import {
  Save,
  Eye,
  Network,
  Bell,
  List,
} from 'lucide-react';
import type { WorkflowDefinition } from '@/lib/api';

interface EditorHeaderProps {
  isNew: boolean;
  workflow?: WorkflowDefinition | null;
  showGraph: boolean;
  setShowGraph: (show: boolean) => void;
  nodesCount: number;
  isPreviewing: boolean;
  handlePreview: () => void;
  setTriggersOpen: (open: boolean) => void;
  onOpenChange: (open: boolean) => void;
  handleSave: () => void;
  canSave: boolean;
  isSaving?: boolean;
}

export function EditorHeader({
  isNew,
  workflow,
  showGraph,
  setShowGraph,
  nodesCount,
  isPreviewing,
  handlePreview,
  setTriggersOpen,
  onOpenChange,
  handleSave,
  canSave,
  isSaving,
}: EditorHeaderProps) {
  return (
    <div className="flex items-center justify-between px-6 py-4 border-b">
      <div className="flex items-center gap-3">
        <DialogTitle className="text-lg">
          {isNew ? 'New Workflow' : `Edit: ${workflow?.name}`}
        </DialogTitle>
        {workflow?.is_system && (
          <Badge variant="secondary" className="text-xs">
            System
          </Badge>
        )}
      </div>
      <div className="flex items-center gap-2">
        <Button
          size="sm"
          variant={showGraph ? 'secondary' : 'ghost'}
          onClick={() => setShowGraph(!showGraph)}
          disabled={nodesCount === 0}
          className="gap-1.5"
        >
          {showGraph ? (
            <><List className="h-3.5 w-3.5" />List</>
          ) : (
            <><Network className="h-3.5 w-3.5" />Graph</>
          )}
        </Button>
        <Button
          size="sm"
          variant="ghost"
          onClick={handlePreview}
          disabled={nodesCount === 0 || isPreviewing}
          className="gap-1.5"
        >
          <Eye className="h-3.5 w-3.5" />
          {isPreviewing ? 'Running...' : 'Preview'}
        </Button>
        <Button
          size="sm"
          variant="ghost"
          onClick={() => setTriggersOpen(true)}
          disabled={isNew}
          className="gap-1.5"
          title={isNew ? 'Save workflow first to manage triggers' : 'Manage triggers'}
        >
          <Bell className="h-3.5 w-3.5" />
          Triggers
        </Button>
        <Button
          size="sm"
          variant="outline"
          onClick={() => onOpenChange(false)}
        >
          Cancel
        </Button>
        <Button
          size="sm"
          onClick={handleSave}
          disabled={!canSave || isSaving}
          className="gap-1.5"
        >
          <Save className="h-3.5 w-3.5" />
          {isSaving ? 'Saving...' : 'Save Workflow'}
        </Button>
      </div>
    </div>
  );
}
