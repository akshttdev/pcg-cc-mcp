import { useState } from 'react';
import { Link2, X, AlertCircle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { IconButton } from '@/components/ui/icon-button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Badge } from '@/components/ui/badge';
import { useDependencyStore } from '@/stores/useDependencyStore';
import type { DependencyType } from '@/types/dependencies';
import type { TaskWithAttemptStatus } from 'shared/types';
import { toast } from 'sonner';

interface DependencyManagerProps {
  taskId: string;
  projectTasks: TaskWithAttemptStatus[];
  onNavigateToTask?: (taskId: string) => void;
}

export function DependencyManager({
  taskId,
  projectTasks,
  onNavigateToTask,
}: DependencyManagerProps) {
  const {
    getDependenciesForTask,
    addDependency,
    removeDependency,
    isBlocked,
  } = useDependencyStore();

  const [selectedTaskId, setSelectedTaskId] = useState<string>('');
  const [dependencyType, setDependencyType] = useState<DependencyType>('blocks');

  const dependencies = getDependenciesForTask(taskId);
  const blocked = isBlocked(taskId);

  // Filter out current task and tasks that already have dependencies
  const availableTasks = projectTasks.filter((task) => {
    if (task.id === taskId) return false;

    const existingDeps = [
      ...dependencies.blocks.map((d) => d.targetTaskId),
      ...dependencies.blockedBy.map((d) => d.sourceTaskId),
      ...dependencies.relatesTo.map((d) =>
        d.sourceTaskId === taskId ? d.targetTaskId : d.sourceTaskId
      ),
    ];

    return !existingDeps.includes(task.id);
  });

  const handleAddDependency = () => {
    if (!selectedTaskId) {
      toast.error('Please select a task');
      return;
    }

    addDependency(taskId, selectedTaskId, dependencyType);
    toast.success('Dependency added');
    setSelectedTaskId('');
  };

  const handleRemove = (depId: string) => {
    removeDependency(depId);
    toast.success('Dependency removed');
  };

  const getTaskTitle = (taskId: string): string => {
    const task = projectTasks.find((t) => t.id === taskId);
    return task?.title || 'Unknown Task';
  };

  const totalDeps =
    dependencies.blocks.length +
    dependencies.blockedBy.length +
    dependencies.relatesTo.length;

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-base flex items-center gap-2">
          <Link2 className="h-4 w-4" />
          Task Dependencies
          {blocked && (
            <Badge variant="destructive" className="ml-auto">
              <AlertCircle className="h-3 w-3 mr-1" />
              Blocked
            </Badge>
          )}
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Add Dependency */}
        <div className="space-y-2">
          <div className="flex gap-2">
            <Select value={dependencyType} onValueChange={(value) => setDependencyType(value as DependencyType)}>
              <SelectTrigger className="w-[140px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="blocks">Blocks</SelectItem>
                <SelectItem value="relates_to">Relates to</SelectItem>
              </SelectContent>
            </Select>
            <Select value={selectedTaskId} onValueChange={setSelectedTaskId}>
              <SelectTrigger className="flex-1">
                <SelectValue placeholder="Select task..." />
              </SelectTrigger>
              <SelectContent>
                {availableTasks.length === 0 ? (
                  <div className="p-2 text-sm text-muted-foreground">
                    No available tasks
                  </div>
                ) : (
                  availableTasks.map((task) => (
                    <SelectItem key={task.id} value={task.id}>
                      {task.title}
                    </SelectItem>
                  ))
                )}
              </SelectContent>
            </Select>
          </div>
          <Button
            onClick={handleAddDependency}
            size="sm"
            className="w-full"
            disabled={!selectedTaskId}
          >
            Add Dependency
          </Button>
        </div>

        {/* Dependency List */}
        {totalDeps === 0 ? (
          <p className="text-sm text-muted-foreground text-center py-4">
            No dependencies yet
          </p>
        ) : (
          <div className="space-y-3">
            {/* Blocks */}
            {dependencies.blocks.length > 0 && (
              <div className="space-y-1">
                <p className="text-xs font-medium text-muted-foreground">BLOCKS</p>
                {dependencies.blocks.map((dep) => (
                  <div
                    key={dep.id}
                    className="flex items-center justify-between p-2 rounded border hover:bg-muted/50 transition-colors"
                  >
                    <button
                      onClick={() => onNavigateToTask?.(dep.targetTaskId)}
                      className="flex-1 text-left text-sm hover:underline"
                    >
                      {getTaskTitle(dep.targetTaskId)}
                    </button>
                    <IconButton
                      variant="ghost" className="h-6 w-6"
                      onClick={() => handleRemove(dep.id)}
                      icon={X}
                      label="Remove dependency"
                      iconClassName="h-3 w-3"
                    />
                  </div>
                ))}
              </div>
            )}

            {/* Blocked By */}
            {dependencies.blockedBy.length > 0 && (
              <div className="space-y-1">
                <p className="text-xs font-medium text-muted-foreground">BLOCKED BY</p>
                {dependencies.blockedBy.map((dep) => (
                  <div
                    key={dep.id}
                    className="flex items-center justify-between p-2 rounded border border-destructive/50 hover:bg-muted/50 transition-colors"
                  >
                    <button
                      onClick={() => onNavigateToTask?.(dep.sourceTaskId)}
                      className="flex-1 text-left text-sm hover:underline"
                    >
                      {getTaskTitle(dep.sourceTaskId)}
                    </button>
                    <IconButton
                      variant="ghost" className="h-6 w-6"
                      onClick={() => handleRemove(dep.id)}
                      icon={X}
                      label="Remove dependency"
                      iconClassName="h-3 w-3"
                    />
                  </div>
                ))}
              </div>
            )}

            {/* Related Tasks */}
            {dependencies.relatesTo.length > 0 && (
              <div className="space-y-1">
                <p className="text-xs font-medium text-muted-foreground">RELATED</p>
                {dependencies.relatesTo.map((dep) => {
                  const relatedTaskId =
                    dep.sourceTaskId === taskId ? dep.targetTaskId : dep.sourceTaskId;
                  return (
                    <div
                      key={dep.id}
                      className="flex items-center justify-between p-2 rounded border hover:bg-muted/50 transition-colors"
                    >
                      <button
                        onClick={() => onNavigateToTask?.(relatedTaskId)}
                        className="flex-1 text-left text-sm hover:underline"
                      >
                        {getTaskTitle(relatedTaskId)}
                      </button>
                      <IconButton
                        variant="ghost" className="h-6 w-6"
                        onClick={() => handleRemove(dep.id)}
                        icon={X}
                        label="Remove dependency"
                        iconClassName="h-3 w-3"
                      />
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
