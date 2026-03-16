import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { UserCombobox, AgentCombobox } from '@/components/ui/assignee-combobox';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import type { Priority, ProjectBoard } from 'shared/types';

interface FieldsSectionProps {
  priority: Priority;
  setPriority: (v: Priority) => void;
  selectedBoardId: string | null;
  setSelectedBoardId: (v: string | null) => void;
  boards: ProjectBoard[];
  boardsLoading: boolean;
  boardsError: string | null;
  dueDate: string;
  setDueDate: (v: string) => void;
  assigneeId: string;
  setAssigneeId: (v: string) => void;
  assignedAgent: string;
  setAssignedAgent: (v: string) => void;
  assignedMcpsInput: string;
  setAssignedMcpsInput: (v: string) => void;
  tagsInput: string;
  setTagsInput: (v: string) => void;
  completionCriteria: string;
  setCompletionCriteria: (v: string) => void;
  outputFormat: string;
  setOutputFormat: (v: string) => void;
  requiresApproval: boolean;
  setRequiresApproval: (v: boolean) => void;
  isSubmitting: boolean;
  isSubmittingAndStart: boolean;
  simpleMode: boolean;
}

export function FieldsSection({
  priority,
  setPriority,
  selectedBoardId,
  setSelectedBoardId,
  boards,
  boardsLoading,
  boardsError,
  dueDate,
  setDueDate,
  assigneeId,
  setAssigneeId,
  assignedAgent,
  setAssignedAgent,
  assignedMcpsInput,
  setAssignedMcpsInput,
  tagsInput,
  setTagsInput,
  completionCriteria,
  setCompletionCriteria,
  outputFormat,
  setOutputFormat,
  requiresApproval,
  setRequiresApproval,
  isSubmitting,
  isSubmittingAndStart,
  simpleMode,
}: FieldsSectionProps) {
  return (
    <>
      <div className="grid gap-4 md:grid-cols-4">
        <div className="space-y-2">
          <Label className="text-sm font-medium">Priority</Label>
          <Select
            value={priority}
            onValueChange={(value) => setPriority(value as Priority)}
            disabled={isSubmitting || isSubmittingAndStart}
          >
            <SelectTrigger>
              <SelectValue placeholder="Select priority" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="critical">Critical</SelectItem>
              <SelectItem value="high">High</SelectItem>
              <SelectItem value="medium">Medium</SelectItem>
              <SelectItem value="low">Low</SelectItem>
            </SelectContent>
          </Select>
        </div>

        <div className="space-y-2">
          <Label className="text-sm font-medium">Board</Label>
          <Select
            value={selectedBoardId ?? 'none'}
            onValueChange={(value) =>
              setSelectedBoardId(value === 'none' ? null : value)
            }
            disabled={
              isSubmitting || isSubmittingAndStart || boardsLoading
            }
          >
            <SelectTrigger>
              <SelectValue
                placeholder={
                  boardsLoading
                    ? 'Loading boards\u2026'
                    : boards.length
                    ? 'Select board'
                    : 'No boards yet'
                }
              />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="none">No board</SelectItem>
              {boards.map((board) => (
                <SelectItem key={board.id} value={board.id}>
                  {board.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          {boardsError && (
            <p className="text-xs text-destructive">{boardsError}</p>
          )}
          {!boardsLoading && boards.length === 0 && !boardsError && (
            <p className="text-xs text-muted-foreground">
              Boards live in the project overview. New projects get Brand, Dev, and
              Social buckets automatically.
            </p>
          )}
        </div>

        <div className="space-y-2">
          <Label className="text-sm font-medium">Due Date</Label>
          <Input
            type="date"
            value={dueDate}
            onChange={(e) => setDueDate(e.target.value)}
            disabled={isSubmitting || isSubmittingAndStart}
          />
        </div>
      </div>

      <div className={simpleMode ? '' : 'grid gap-4 md:grid-cols-2'}>
        <div className="space-y-2">
          <Label className="text-sm font-medium">Assignee</Label>
          <UserCombobox
            value={assigneeId}
            onChange={setAssigneeId}
            placeholder="Select user..."
            disabled={isSubmitting || isSubmittingAndStart}
          />
        </div>

        {!simpleMode && (
          <div className="space-y-2">
            <Label className="text-sm font-medium">Assigned Agent</Label>
            <AgentCombobox
              value={assignedAgent}
              onChange={setAssignedAgent}
              placeholder="Select agent..."
              disabled={isSubmitting || isSubmittingAndStart}
            />
          </div>
        )}
      </div>

      {!simpleMode && (
        <div className="grid gap-4 md:grid-cols-2">
          <div className="space-y-2">
            <Label className="text-sm font-medium">Assigned MCPs (comma-separated)</Label>
            <Input
              value={assignedMcpsInput}
              onChange={(e) => setAssignedMcpsInput(e.target.value)}
              placeholder="e.g. orcha-tasks, playwright"
              disabled={isSubmitting || isSubmittingAndStart}
            />
          </div>

          <div className="space-y-2">
            <Label className="text-sm font-medium">Tags (comma-separated)</Label>
            <Input
              value={tagsInput}
              onChange={(e) => setTagsInput(e.target.value)}
              placeholder="e.g. frontend, bug, urgent"
              disabled={isSubmitting || isSubmittingAndStart}
            />
          </div>
        </div>
      )}

      {!simpleMode && (
        <div className="grid gap-4 md:grid-cols-2">
          <div className="space-y-2">
            <Label className="text-sm font-medium">Completion Criteria</Label>
            <Textarea
              value={completionCriteria}
              onChange={(e) => setCompletionCriteria(e.target.value)}
              placeholder="What must be true for this task to be considered done? e.g., All tests pass, PR approved, deployed to staging"
              rows={3}
              disabled={isSubmitting || isSubmittingAndStart}
            />
            <p className="text-xs text-muted-foreground">
              Structured success criteria for agents to self-evaluate completion.
            </p>
          </div>

          <div className="space-y-2">
            <Label className="text-sm font-medium">Output Format</Label>
            <Textarea
              value={outputFormat}
              onChange={(e) => setOutputFormat(e.target.value)}
              placeholder="Expected deliverable format. e.g., Pull request with tests, JSON report, Markdown document"
              rows={3}
              disabled={isSubmitting || isSubmittingAndStart}
            />
            <p className="text-xs text-muted-foreground">
              Describes the expected deliverable format for agent output.
            </p>
          </div>
        </div>
      )}

      {!simpleMode && (
        <div className="flex items-center gap-3 pt-2">
          <Switch
            id="requires-approval"
            checked={requiresApproval}
            onCheckedChange={(checked) => setRequiresApproval(checked)}
            disabled={isSubmitting || isSubmittingAndStart}
          />
          <Label htmlFor="requires-approval" className="text-sm">
            Requires approval before completion
          </Label>
        </div>
      )}
    </>
  );
}
