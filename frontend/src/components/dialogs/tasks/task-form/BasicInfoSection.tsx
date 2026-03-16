import { Input } from '@/components/ui/input';
import { RichTextEditor } from '@/components/editor/RichTextEditor';
import { Label } from '@/components/ui/label';

interface BasicInfoSectionProps {
  title: string;
  setTitle: (v: string) => void;
  description: string;
  setDescription: (v: string) => void;
  isSubmitting: boolean;
  isSubmittingAndStart: boolean;
  isEditMode: boolean;
  handleSubmit: () => void;
  handleCreateAndStart: () => void;
}

export function BasicInfoSection({
  title,
  setTitle,
  description,
  setDescription,
  isSubmitting,
  isSubmittingAndStart,
  isEditMode,
  handleSubmit,
  handleCreateAndStart,
}: BasicInfoSectionProps) {
  return (
    <>
      <div>
        <Label htmlFor="task-title" className="text-sm font-medium">
          Title
        </Label>
        <Input
          id="task-title"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          placeholder="What needs to be done?"
          className="mt-1.5"
          disabled={isSubmitting || isSubmittingAndStart}
          autoFocus
          onCommandEnter={
            isEditMode ? handleSubmit : handleCreateAndStart
          }
          onCommandShiftEnter={handleSubmit}
        />
      </div>

      <div>
        <Label
          htmlFor="task-description"
          className="text-sm font-medium"
        >
          Description
        </Label>
        <RichTextEditor
          value={description}
          onChange={(value) => setDescription(value)}
          placeholder="Add more details (optional). Supports Markdown formatting."
          height={250}
          enableToolbar={true}
          className="mt-1.5"
          readOnly={isSubmitting || isSubmittingAndStart}
        />
      </div>
    </>
  );
}
