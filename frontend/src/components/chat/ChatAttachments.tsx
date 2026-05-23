import {
  FileText,
  Image as ImageIcon,
  Loader2,
  Paperclip,
  X,
} from 'lucide-react';
import { useCallback, useRef, useState } from 'react';
import { toast } from 'sonner';

import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

// ── Types ───────────────────────────────────────────────────────────────────

export interface ChatAttachment {
  id: string;
  filename: string;
  mimeType: string;
  previewUrl?: string;
  isUploading?: boolean;
}

interface ChatAttachmentButtonProps {
  onFilesSelected: (files: File[]) => void;
  disabled?: boolean;
  accept?: string;
  className?: string;
}

interface ChatAttachmentPreviewProps {
  attachment: ChatAttachment;
  onRemove: (id: string) => void;
  disabled?: boolean;
}

interface ChatAttachmentListProps {
  attachments: ChatAttachment[];
  onRemove: (id: string) => void;
  disabled?: boolean;
  className?: string;
}

interface UseChatAttachmentsOptions {
  projectId: string;
  maxFiles?: number;
  maxSizeMB?: number;
}

interface UseChatAttachmentsReturn {
  attachments: ChatAttachment[];
  attachmentIds: string[];
  isUploading: boolean;
  addFiles: (files: File[]) => Promise<void>;
  removeAttachment: (id: string) => void;
  clearAttachments: () => void;
}

// ── Constants ───────────────────────────────────────────────────────────────

const ACCEPTED_FILE_TYPES = 'image/*,.pdf,.txt,.md,.doc,.docx';
const MAX_FILES = 5;
const MAX_SIZE_MB = 20;

// ── Helper Functions ────────────────────────────────────────────────────────

const isImageMimeType = (mimeType: string): boolean => {
  return mimeType.startsWith('image/');
};

const getFileIcon = (mimeType: string) => {
  if (isImageMimeType(mimeType)) {
    return ImageIcon;
  }
  return FileText;
};

// ── Upload Function ─────────────────────────────────────────────────────────

interface UploadedAsset {
  data: {
    id: string;
    filename: string;
    mimeType: string;
    filePath: string;
  };
}

async function uploadToMediaLibrary(
  projectId: string,
  file: File
): Promise<{ id: string; filename: string; mimeType: string }> {
  const formData = new FormData();
  formData.append('file', file);

  const response = await fetch(`/api/projects/${projectId}/media`, {
    method: 'POST',
    body: formData,
    credentials: 'include',
  });

  if (!response.ok) {
    throw new Error(`Upload failed: ${response.statusText}`);
  }

  const result: UploadedAsset = await response.json();
  return {
    id: result.data.id,
    filename: result.data.filename,
    mimeType: result.data.mimeType || file.type,
  };
}

// ── Components ──────────────────────────────────────────────────────────────

/**
 * Attachment button with file input trigger
 */
export function ChatAttachmentButton({
  onFilesSelected,
  disabled = false,
  accept = ACCEPTED_FILE_TYPES,
  className,
}: ChatAttachmentButtonProps) {
  const inputRef = useRef<HTMLInputElement>(null);

  const handleClick = () => {
    inputRef.current?.click();
  };

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = Array.from(e.target.files || []);
    if (files.length > 0) {
      onFilesSelected(files);
    }
    // Reset input so same file can be selected again
    e.target.value = '';
  };

  return (
    <>
      <input
        ref={inputRef}
        type="file"
        accept={accept}
        multiple
        onChange={handleChange}
        className="hidden"
        disabled={disabled}
      />
      <Button
        type="button"
        variant="ghost"
        size="icon"
        onClick={handleClick}
        disabled={disabled}
        className={cn(
          'h-8 w-8 text-muted-foreground hover:text-foreground',
          className
        )}
        title="Attach files (images, PDFs, documents)"
      >
        <Paperclip className="h-4 w-4" />
      </Button>
    </>
  );
}

/**
 * Single attachment preview with thumbnail and remove button
 */
export function ChatAttachmentPreview({
  attachment,
  onRemove,
  disabled = false,
}: ChatAttachmentPreviewProps) {
  const FileIcon = getFileIcon(attachment.mimeType);
  const isImage = isImageMimeType(attachment.mimeType);

  return (
    <div
      className={cn(
        'relative flex items-center gap-2 rounded-md border bg-muted/50 p-2 pr-8',
        'transition-colors hover:bg-muted',
        attachment.isUploading && 'opacity-70'
      )}
    >
      {/* Preview thumbnail or icon */}
      <div className="flex h-8 w-8 items-center justify-center rounded bg-background">
        {attachment.isUploading ? (
          <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
        ) : isImage && attachment.previewUrl ? (
          <img
            src={attachment.previewUrl}
            alt={attachment.filename}
            className="h-8 w-8 rounded object-cover"
          />
        ) : (
          <FileIcon className="h-4 w-4 text-muted-foreground" />
        )}
      </div>

      {/* Filename */}
      <span className="max-w-[120px] truncate text-xs text-foreground">
        {attachment.filename}
      </span>

      {/* Remove button */}
      {!attachment.isUploading && (
        <button
          type="button"
          onClick={() => onRemove(attachment.id)}
          disabled={disabled}
          className={cn(
            'absolute right-1 top-1 rounded-full p-0.5',
            'text-muted-foreground hover:bg-destructive/10 hover:text-destructive',
            'focus:outline-none focus:ring-2 focus:ring-destructive/50',
            disabled && 'cursor-not-allowed opacity-50'
          )}
          title="Remove attachment"
        >
          <X className="h-3 w-3" />
        </button>
      )}
    </div>
  );
}

/**
 * Container for multiple attachment previews
 */
export function ChatAttachmentList({
  attachments,
  onRemove,
  disabled = false,
  className,
}: ChatAttachmentListProps) {
  if (attachments.length === 0) return null;

  return (
    <div className={cn('flex flex-wrap gap-2', className)}>
      {attachments.map((attachment) => (
        <ChatAttachmentPreview
          key={attachment.id}
          attachment={attachment}
          onRemove={onRemove}
          disabled={disabled}
        />
      ))}
    </div>
  );
}

// ── Hook ────────────────────────────────────────────────────────────────────

/**
 * Hook to manage chat attachments with upload functionality
 */
export function useChatAttachments({
  projectId,
  maxFiles = MAX_FILES,
  maxSizeMB = MAX_SIZE_MB,
}: UseChatAttachmentsOptions): UseChatAttachmentsReturn {
  const [attachments, setAttachments] = useState<ChatAttachment[]>([]);
  const [isUploading, setIsUploading] = useState(false);

  const attachmentIds = attachments
    .filter((a) => !a.isUploading)
    .map((a) => a.id);

  const addFiles = useCallback(
    async (files: File[]) => {
      // Validate file count
      const remainingSlots = maxFiles - attachments.length;
      if (files.length > remainingSlots) {
        toast.error(`Maximum ${maxFiles} attachments allowed`);
        files = files.slice(0, remainingSlots);
      }

      if (files.length === 0) return;

      // Validate file sizes
      const maxSizeBytes = maxSizeMB * 1024 * 1024;
      const validFiles = files.filter((file) => {
        if (file.size > maxSizeBytes) {
          toast.error(`${file.name} exceeds ${maxSizeMB}MB limit`);
          return false;
        }
        return true;
      });

      if (validFiles.length === 0) return;

      setIsUploading(true);

      // Create temporary attachments with loading state
      const tempAttachments: ChatAttachment[] = validFiles.map((file) => ({
        id: `temp-${Date.now()}-${file.name}`,
        filename: file.name,
        mimeType: file.type,
        previewUrl: isImageMimeType(file.type)
          ? URL.createObjectURL(file)
          : undefined,
        isUploading: true,
      }));

      setAttachments((prev) => [...prev, ...tempAttachments]);

      // Upload files in parallel
      const uploadPromises = validFiles.map(async (file, index) => {
        const tempId = tempAttachments[index].id;
        try {
          const uploaded = await uploadToMediaLibrary(projectId, file);

          // Replace temp attachment with uploaded one
          setAttachments((prev) =>
            prev.map((a) =>
              a.id === tempId
                ? {
                    ...a,
                    id: uploaded.id,
                    filename: uploaded.filename,
                    mimeType: uploaded.mimeType,
                    isUploading: false,
                  }
                : a
            )
          );
        } catch (error) {
          console.error('Upload failed:', error);
          toast.error(`Failed to upload ${file.name}`);
          // Remove failed upload
          setAttachments((prev) => prev.filter((a) => a.id !== tempId));
        }
      });

      await Promise.all(uploadPromises);
      setIsUploading(false);
    },
    [projectId, maxFiles, maxSizeMB, attachments.length]
  );

  const removeAttachment = useCallback((id: string) => {
    setAttachments((prev) => {
      const attachment = prev.find((a) => a.id === id);
      // Revoke object URL to prevent memory leaks
      if (attachment?.previewUrl) {
        URL.revokeObjectURL(attachment.previewUrl);
      }
      return prev.filter((a) => a.id !== id);
    });
  }, []);

  const clearAttachments = useCallback(() => {
    // Revoke all object URLs
    attachments.forEach((a) => {
      if (a.previewUrl) {
        URL.revokeObjectURL(a.previewUrl);
      }
    });
    setAttachments([]);
  }, [attachments]);

  return {
    attachments,
    attachmentIds,
    isUploading,
    addFiles,
    removeAttachment,
    clearAttachments,
  };
}
