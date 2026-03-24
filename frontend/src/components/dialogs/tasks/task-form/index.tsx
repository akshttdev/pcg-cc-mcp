import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Switch } from '@/components/ui/switch';
import { Label } from '@/components/ui/label';
import { FormDialogBody } from '@/components/ui/form-dialog-body';
import NiceModal, { useModal } from '@ebay/nice-modal-react';
import { useTaskFormState } from './useTaskFormState';
import { BasicInfoSection } from './BasicInfoSection';
import { FieldsSection } from './FieldsSection';
import { QuickstartSection } from './QuickstartSection';
import type { TaskFormDialogProps } from './types';

export type { TaskFormDialogProps } from './types';

export const TaskFormDialog = NiceModal.create<TaskFormDialogProps>(
  (props) => {
    const modal = useModal();
    const state = useTaskFormState({ props, modal });

    return (
      <>
        <Dialog open={modal.visible} onOpenChange={state.handleDialogOpenChange}>
          <DialogContent className="sm:max-w-[650px] max-h-[90vh] flex flex-col overflow-hidden">
            <DialogHeader>
              <div className="flex items-center justify-between">
                <DialogTitle>
                  {state.isEditMode ? 'Edit Task' : 'Create New Task'}
                </DialogTitle>
                <div className="flex items-center gap-2 mr-6">
                  <Switch
                    id="simple-mode"
                    checked={state.simpleMode}
                    onCheckedChange={(checked) => {
                      state.setSimpleMode(checked);
                      localStorage.setItem('pcg-task-simple-mode', String(checked));
                    }}
                  />
                  <Label htmlFor="simple-mode" className="text-xs text-muted-foreground cursor-pointer">
                    Simple
                  </Label>
                </div>
              </div>
            </DialogHeader>
            <FormDialogBody
              footer={
                <>
                  <Button
                    variant="outline"
                    onClick={state.handleCancel}
                    disabled={state.isSubmitting || state.isSubmittingAndStart}
                  >
                    Cancel
                  </Button>
                  {state.isEditMode ? (
                    <Button
                      onClick={state.handleSubmit}
                      disabled={state.isSubmitting || !state.title.trim()}
                    >
                      {state.isSubmitting ? 'Updating...' : 'Update Task'}
                    </Button>
                  ) : (
                    <>
                      <Button
                        variant="outline"
                        onClick={state.handleSubmit}
                        disabled={
                          state.isSubmitting || state.isSubmittingAndStart || !state.title.trim()
                        }
                      >
                        {state.isSubmitting ? 'Creating...' : 'Create Task'}
                      </Button>
                      <Button
                        onClick={state.handleCreateAndStart}
                        disabled={
                          state.isSubmitting || state.isSubmittingAndStart || !state.title.trim()
                        }
                        className={'font-medium'}
                      >
                        {state.isSubmittingAndStart
                          ? 'Creating & Starting...'
                          : 'Create & Start'}
                      </Button>
                    </>
                  )}
                </>
              }
            >
            <div className="space-y-4">
              <BasicInfoSection
                title={state.title}
                setTitle={state.setTitle}
                description={state.description}
                setDescription={state.setDescription}
                isSubmitting={state.isSubmitting}
                isSubmittingAndStart={state.isSubmittingAndStart}
                isEditMode={state.isEditMode}
                handleSubmit={state.handleSubmit}
                handleCreateAndStart={state.handleCreateAndStart}
              />

              <FieldsSection
                priority={state.priority}
                setPriority={state.setPriority}
                selectedBoardId={state.selectedBoardId}
                setSelectedBoardId={state.setSelectedBoardId}
                boards={state.boards}
                boardsLoading={state.boardsLoading}
                boardsError={state.boardsError}
                dueDate={state.dueDate}
                setDueDate={state.setDueDate}
                assigneeId={state.assigneeId}
                setAssigneeId={state.setAssigneeId}
                assignedAgent={state.assignedAgent}
                setAssignedAgent={state.setAssignedAgent}
                assignedMcpsInput={state.assignedMcpsInput}
                setAssignedMcpsInput={state.setAssignedMcpsInput}
                tagsInput={state.tagsInput}
                setTagsInput={state.setTagsInput}
                completionCriteria={state.completionCriteria}
                setCompletionCriteria={state.setCompletionCriteria}
                outputFormat={state.outputFormat}
                setOutputFormat={state.setOutputFormat}
                requiresApproval={state.requiresApproval}
                setRequiresApproval={state.setRequiresApproval}
                isSubmitting={state.isSubmitting}
                isSubmittingAndStart={state.isSubmittingAndStart}
                simpleMode={state.simpleMode}
              />

              <QuickstartSection
                images={state.images}
                handleImagesChange={state.handleImagesChange}
                handleImageUploaded={state.handleImageUploaded}
                isSubmitting={state.isSubmitting}
                isSubmittingAndStart={state.isSubmittingAndStart}
                isEditMode={state.isEditMode}
                templates={state.templates}
                selectedTemplate={state.selectedTemplate}
                handleTemplateChange={state.handleTemplateChange}
                status={state.status}
                setStatus={state.setStatus}
                simpleMode={state.simpleMode}
                quickstartExpanded={state.quickstartExpanded}
                setQuickstartExpanded={state.setQuickstartExpanded}
                branches={state.branches}
                selectedBranch={state.selectedBranch}
                setSelectedBranch={state.setSelectedBranch}
                selectedExecutorProfile={state.selectedExecutorProfile}
                setSelectedExecutorProfile={state.setSelectedExecutorProfile}
              />

            </div>
            </FormDialogBody>
          </DialogContent>
        </Dialog>

        {/* Discard Warning Dialog */}
        <Dialog open={state.showDiscardWarning} onOpenChange={state.setShowDiscardWarning}>
          <DialogContent className="sm:max-w-[425px]">
            <DialogHeader>
              <DialogTitle>Discard unsaved changes?</DialogTitle>
            </DialogHeader>
            <div className="py-4">
              <p className="text-sm text-muted-foreground">
                You have unsaved changes. Are you sure you want to discard them?
              </p>
            </div>
            <DialogFooter>
              <Button
                variant="outline"
                onClick={() => state.setShowDiscardWarning(false)}
              >
                Continue Editing
              </Button>
              <Button variant="destructive" onClick={state.handleDiscardChanges}>
                Discard Changes
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </>
    );
  }
);
