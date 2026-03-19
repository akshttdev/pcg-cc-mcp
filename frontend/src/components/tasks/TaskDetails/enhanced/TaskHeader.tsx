import { useMemo } from 'react';
import type { TaskWithAttemptStatus } from 'shared/types';
import type { TaskCardMode } from '../../EnhancedTaskCard';
import { EnhancedTaskHeader } from '../EnhancedTaskHeader';
import { BreadcrumbNav } from '@/components/breadcrumb/BreadcrumbNav';

interface TaskHeaderProps {
  task: TaskWithAttemptStatus;
  mode: TaskCardMode;
  onEdit?: () => void;
  onDelete?: () => void;
  onDuplicate?: () => void;
  onClose: () => void;
  onToggleFullscreen?: () => void;
  isFullscreen?: boolean;
  onToggleExpand?: () => void;
  isExpanded?: boolean;
  hideClose?: boolean;
}

export function TaskHeader({
  task,
  mode,
  onEdit,
  onDelete,
  onDuplicate,
  onClose,
  onToggleFullscreen,
  isFullscreen,
  onToggleExpand,
  isExpanded,
  hideClose,
}: TaskHeaderProps) {
  // Stable callback wrapper for BreadcrumbNav (expects (fs: boolean) => void)
  const handleToggleFullscreen = useMemo(
    () => onToggleFullscreen ? () => onToggleFullscreen() : undefined,
    [onToggleFullscreen]
  );

  return (
    <>
      {/* Breadcrumb nav in fullscreen mode (panel covers AppShell) */}
      {isFullscreen && (
        <BreadcrumbNav onToggleFullscreen={handleToggleFullscreen} isFullscreen={isFullscreen} />
      )}

      {/* Header */}
      <EnhancedTaskHeader
        task={task}
        mode={mode}
        onEdit={onEdit}
        onDelete={onDelete}
        onDuplicate={onDuplicate}
        onClose={onClose}
        onToggleExpand={onToggleExpand}
        isExpanded={isExpanded}
        hideClose={hideClose}
      />
    </>
  );
}
