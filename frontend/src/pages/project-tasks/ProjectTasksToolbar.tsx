import { Button } from '@/components/ui/button';
import { Archive, CheckSquare, Download, EyeOff, Sparkles, Upload } from 'lucide-react';
import { ViewSwitcher } from '@/components/views/ViewSwitcher';
import { AskTopsiButton } from '@/components/topsi/AskTopsiButton';
import { TagManager } from '@/components/tags/TagManager';
import { FilterButton } from '@/components/filters/FilterButton';
import { SortMenu } from '@/components/tasks/SortMenu';
import { SavedFiltersMenu } from '@/components/filters/SavedFiltersMenu';
import { useViewStore } from '@/stores/useViewStore';
import type { Project } from 'shared/types';

interface ProjectTasksToolbarProps {
  projectId: string;
  project: Project | null;
  selectionMode: boolean;
  onToggleSelectionMode: () => void;
  showArchived: boolean;
  archivedCount: number;
  onToggleArchived: () => void;
  hideTestTasks: boolean;
  onToggleHideTests: () => void;
  onFilterClick: () => void;
  onImportClick: () => void;
  onExportClick: () => void;
}

export function ProjectTasksToolbar({
  projectId,
  project,
  selectionMode,
  onToggleSelectionMode,
  showArchived,
  archivedCount,
  onToggleArchived,
  hideTestTasks,
  onToggleHideTests,
  onFilterClick,
  onImportClick,
  onExportClick,
}: ProjectTasksToolbarProps) {
  const { useEnhancedCards, setUseEnhancedCards } = useViewStore();

  return (
    <div className="px-6 py-4 border-b bg-background/95 backdrop-blur sticky top-0 z-10 flex flex-wrap items-center gap-2">
      <h2 className="text-lg font-semibold mr-auto">{project?.name || 'Tasks'}</h2>
      <div className="flex items-center gap-2 flex-wrap">
        <FilterButton
          projectId={projectId}
          onClick={onFilterClick}
        />
        <SortMenu />
        <SavedFiltersMenu projectId={projectId} />
        <Button
          variant="outline"
          size="sm"
          onClick={onImportClick}
          className="gap-2"
        >
          <Upload className="h-4 w-4" />
          Import
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={onExportClick}
          className="gap-2"
        >
          <Download className="h-4 w-4" />
          Export
        </Button>
        <Button
          variant={selectionMode ? 'default' : 'outline'}
          size="sm"
          onClick={onToggleSelectionMode}
          className="gap-2"
        >
          <CheckSquare className="h-4 w-4" />
          {selectionMode ? 'Done' : 'Select'}
        </Button>
        <Button
          variant={showArchived ? 'default' : 'outline'}
          size="sm"
          onClick={onToggleArchived}
          className="gap-2"
          title={showArchived ? 'Hide archived tasks' : 'Show archived tasks'}
        >
          <Archive className="h-4 w-4" />
          {showArchived ? 'Archived' : 'Archived'}
          {archivedCount > 0 && (
            <span className="ml-1 rounded-full bg-muted-foreground/20 px-1.5 py-0.5 text-xs font-medium">
              {archivedCount}
            </span>
          )}
        </Button>
        <Button
          variant={hideTestTasks ? 'default' : 'outline'}
          size="sm"
          onClick={onToggleHideTests}
          className="gap-2"
          title={hideTestTasks ? 'Show test tasks' : 'Hide [E2E]/[Test] tasks'}
        >
          <EyeOff className="h-4 w-4" />
          {hideTestTasks ? 'Tests Hidden' : 'Hide Tests'}
        </Button>
        <Button
          variant={useEnhancedCards ? 'default' : 'outline'}
          size="sm"
          onClick={() => setUseEnhancedCards(!useEnhancedCards)}
          className="gap-2"
          title={useEnhancedCards ? 'Switch to classic cards' : 'Switch to enhanced cards'}
        >
          <Sparkles className="h-4 w-4" />
          {useEnhancedCards ? 'Enhanced Cards' : 'Classic Cards'}
        </Button>
        <TagManager projectId={projectId} />
        <ViewSwitcher />
        {project && (
          <AskTopsiButton
            entityType="project"
            entityId={projectId}
            entityName={project.name}
          />
        )}
      </div>
    </div>
  );
}
