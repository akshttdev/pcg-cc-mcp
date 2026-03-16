import { Globe2, Settings2, ChevronRight } from 'lucide-react';
import { ImageUploadSection } from '@/components/ui/ImageUploadSection';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { ExecutorProfileSelector } from '@/components/settings';
import BranchSelector from '@/components/tasks/BranchSelector';
import { imagesApi } from '@/lib/api';
import { useUserSystem } from '@/components/config-provider';
import type {
  TaskStatus,
  TaskTemplate,
  ImageResponse,
  GitBranch,
  ExecutorProfileId,
} from 'shared/types';

interface QuickstartSectionProps {
  images: ImageResponse[];
  handleImagesChange: (images: ImageResponse[]) => void;
  handleImageUploaded: (imageId: string) => void;
  isSubmitting: boolean;
  isSubmittingAndStart: boolean;
  isEditMode: boolean;
  templates: TaskTemplate[];
  selectedTemplate: string;
  handleTemplateChange: (templateId: string) => void;
  status: TaskStatus;
  setStatus: (v: TaskStatus) => void;
  simpleMode: boolean;
  quickstartExpanded: boolean;
  setQuickstartExpanded: (v: boolean) => void;
  branches: GitBranch[];
  selectedBranch: string;
  setSelectedBranch: (v: string) => void;
  selectedExecutorProfile: ExecutorProfileId | null;
  setSelectedExecutorProfile: (v: ExecutorProfileId | null) => void;
}

export function QuickstartSection({
  images,
  handleImagesChange,
  handleImageUploaded,
  isSubmitting,
  isSubmittingAndStart,
  isEditMode,
  templates,
  selectedTemplate,
  handleTemplateChange,
  status,
  setStatus,
  simpleMode,
  quickstartExpanded,
  setQuickstartExpanded,
  branches,
  selectedBranch,
  setSelectedBranch,
  selectedExecutorProfile,
  setSelectedExecutorProfile,
}: QuickstartSectionProps) {
  const { profiles } = useUserSystem();

  // Wrap handleImageUploaded to match ImageUploadSection's expected signature
  const onImageUploaded = (image: ImageResponse) => {
    handleImageUploaded(image.id);
  };

  return (
    <>
      <ImageUploadSection
        images={images}
        onImagesChange={handleImagesChange}
        onUpload={imagesApi.upload}
        onDelete={imagesApi.delete}
        onImageUploaded={onImageUploaded}
        disabled={isSubmitting || isSubmittingAndStart}
        readOnly={isEditMode}
        collapsible={true}
        defaultExpanded={false}
      />

      {!isEditMode && templates.length > 0 && (
        <div className="pt-2">
          <details className="group">
            <summary className="cursor-pointer text-sm text-muted-foreground hover:text-foreground transition-colors list-none flex items-center gap-2">
              <svg
                className="h-3 w-3 transition-transform group-open:rotate-90"
                viewBox="0 0 20 20"
                fill="currentColor"
              >
                <path
                  fillRule="evenodd"
                  d="M7.293 14.707a1 1 0 010-1.414L10.586 10 7.293 6.707a1 1 0 011.414-1.414l4 4a1 1 0 010 1.414l-4 4a1 1 0 01-1.414 0z"
                  clipRule="evenodd"
                />
              </svg>
              Use a template
            </summary>
            <div className="mt-3 space-y-2">
              <p className="text-xs text-muted-foreground">
                Templates help you quickly create tasks with predefined
                content.
              </p>
              <Select
                value={selectedTemplate}
                onValueChange={handleTemplateChange}
              >
                <SelectTrigger id="task-template" className="w-full">
                  <SelectValue placeholder="Choose a template to prefill this form" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="none">No template</SelectItem>
                  {templates.map((template) => (
                    <SelectItem key={template.id} value={template.id}>
                      <div className="flex items-center gap-2">
                        {template.project_id === null && (
                          <Globe2 className="h-3 w-3 text-muted-foreground" />
                        )}
                        <span>{template.template_name}</span>
                      </div>
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </details>
        </div>
      )}

      {isEditMode && (
        <div className="pt-2">
          <Label htmlFor="task-status" className="text-sm font-medium">
            Status
          </Label>
          <Select
            value={status}
            onValueChange={(value) => setStatus(value as TaskStatus)}
            disabled={isSubmitting || isSubmittingAndStart}
          >
            <SelectTrigger className="mt-1.5">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="todo">To Do</SelectItem>
              <SelectItem value="inprogress">In Progress</SelectItem>
              <SelectItem value="inreview">In Review</SelectItem>
              <SelectItem value="done">Done</SelectItem>
              <SelectItem value="cancelled">Cancelled</SelectItem>
            </SelectContent>
          </Select>
        </div>
      )}

      {!isEditMode && !simpleMode &&
        (() => {
          const quickstartSection = (
            <div className="pt-2">
              <details
                className="group"
                open={quickstartExpanded}
                onToggle={(e) =>
                  setQuickstartExpanded(
                    (e.target as HTMLDetailsElement).open
                  )
                }
              >
                <summary className="cursor-pointer text-sm text-muted-foreground hover:text-foreground transition-colors list-none flex items-center gap-2">
                  <ChevronRight className="h-3 w-3 transition-transform group-open:rotate-90" />
                  <Settings2 className="h-3 w-3" />
                  Quickstart
                </summary>
                <div className="mt-3 space-y-3">
                  <p className="text-xs text-muted-foreground">
                    Configuration for "Create & Start" workflow
                  </p>

                  {/* Executor Profile Selector */}
                  {profiles && selectedExecutorProfile && (
                    <ExecutorProfileSelector
                      profiles={profiles}
                      selectedProfile={selectedExecutorProfile}
                      onProfileSelect={setSelectedExecutorProfile}
                      disabled={isSubmitting || isSubmittingAndStart}
                    />
                  )}

                  {/* Branch Selector */}
                  {branches.length > 0 && (
                    <div>
                      <Label
                        htmlFor="base-branch"
                        className="text-sm font-medium"
                      >
                        Branch
                      </Label>
                      <div className="mt-1.5">
                        <BranchSelector
                          branches={branches}
                          selectedBranch={selectedBranch}
                          onBranchSelect={setSelectedBranch}
                          placeholder="Select branch"
                          className={
                            isSubmitting || isSubmittingAndStart
                              ? 'opacity-50 cursor-not-allowed'
                              : ''
                          }
                        />
                      </div>
                    </div>
                  )}
                </div>
              </details>
            </div>
          );
          return quickstartSection;
        })()}
    </>
  );
}
