import {
  Bot,
  Clapperboard,
  Download,
  ExternalLink,
  FileText,
  Image,
  Play,
  RotateCw,
  Upload,
  User,
  X,
  ZoomIn,
  ZoomOut,
} from 'lucide-react';
import { useMemo, useState } from 'react';
import type {
  ArtifactPhase,
  ArtifactType,
  ExecutionArtifact,
} from 'shared/types';

import { Badge } from '@/components/ui/badge';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { IconButton } from '@/components/ui/icon-button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { artifactContentApi } from '@/lib/api';
import { cn } from '@/lib/utils';

const VIDEO_EDIT_TYPES: ArtifactType[] = [
  'video_edit_session',
  'render_deliverable',
];

interface ArtifactPreviewCardProps {
  artifact: ExecutionArtifact;
  size?: 'sm' | 'md' | 'lg';
  showCreator?: boolean;
  onDownload?: () => void;
  onUploadComplete?: () => void;
  className?: string;
}

// Artifact type categories
const visualTypes: ArtifactType[] = [
  'screenshot',
  'visual_brief',
  'platform_screenshot',
];
const documentTypes: ArtifactType[] = [
  'research_report',
  'strategy_document',
  'content_draft',
  'content_calendar',
  'competitor_analysis',
  'plan',
  'media_ingest_manifest',
  'media_analysis_report',
];
const mediaTypes: ArtifactType[] = [
  'walkthrough',
  'browser_recording',
  'video_edit_session',
  'render_deliverable',
];

// Phase colors
const phaseColors: Record<ArtifactPhase, string> = {
  planning:
    'bg-purple-100 text-purple-700 dark:bg-purple-900 dark:text-purple-300',
  execution:
    'bg-yellow-100 text-yellow-700 dark:bg-yellow-900 dark:text-yellow-300',
  verification:
    'bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300',
};

// Size configurations
const sizeConfig = {
  sm: { height: 'h-16', imageHeight: 'h-12', textLines: 2 },
  md: { height: 'h-24', imageHeight: 'h-20', textLines: 3 },
  lg: { height: 'h-32', imageHeight: 'h-28', textLines: 5 },
};

// Image lightbox component
function ImageLightbox({
  src,
  alt,
  open,
  onClose,
}: {
  src: string;
  alt: string;
  open: boolean;
  onClose: () => void;
}) {
  const [zoom, setZoom] = useState(1);
  const [rotation, setRotation] = useState(0);

  const handleZoomIn = () => setZoom((z) => Math.min(z + 0.25, 3));
  const handleZoomOut = () => setZoom((z) => Math.max(z - 0.25, 0.5));
  const handleRotate = () => setRotation((r) => (r + 90) % 360);

  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="max-w-[90vw] max-h-[90vh] p-0 overflow-hidden">
        <DialogHeader className="p-4 border-b flex flex-row items-center justify-between">
          <DialogTitle className="text-sm font-medium truncate flex-1">
            {alt}
          </DialogTitle>
          <div className="flex items-center gap-1">
            <IconButton
              variant="ghost"
              onClick={handleZoomOut}
              className="h-8 w-8"
              icon={ZoomOut}
              label="Zoom out"
            />
            <span className="text-xs text-muted-foreground w-12 text-center">
              {Math.round(zoom * 100)}%
            </span>
            <IconButton
              variant="ghost"
              onClick={handleZoomIn}
              className="h-8 w-8"
              icon={ZoomIn}
              label="Zoom in"
            />
            <IconButton
              variant="ghost"
              onClick={handleRotate}
              className="h-8 w-8"
              icon={RotateCw}
              label="Rotate"
            />
            <IconButton
              variant="ghost"
              onClick={onClose}
              className="h-8 w-8"
              icon={X}
              label="Close"
            />
          </div>
        </DialogHeader>
        <div className="flex-1 overflow-auto p-4 flex items-center justify-center bg-black/5 dark:bg-black/20 min-h-[60vh]">
          <img
            src={src}
            alt={alt}
            className="max-w-full max-h-full object-contain transition-transform duration-200"
            style={{
              transform: `scale(${zoom}) rotate(${rotation}deg)`,
            }}
          />
        </div>
      </DialogContent>
    </Dialog>
  );
}

// Document preview component with markdown rendering
function DocumentPreview({
  content,
  title,
  filePath,
  open,
  onClose,
}: {
  content: string;
  title: string;
  filePath?: string;
  open: boolean;
  onClose: () => void;
}) {
  const isPdf = filePath?.toLowerCase().includes('.pdf');
  const isDropbox = filePath?.includes('dropbox.com');
  // Dropbox direct-download link for iframe embedding
  const embedUrl =
    isDropbox && isPdf
      ? filePath!
          .replace('www.dropbox.com', 'dl.dropboxusercontent.com')
          .replace('?dl=0', '')
          .replace('&dl=0', '')
      : filePath;
  // Simple markdown-like rendering (can be enhanced with a proper markdown library)
  const renderedContent = useMemo(() => {
    if (!content) return '';

    // Convert markdown headers
    const html = content
      .replace(
        /^### (.*$)/gim,
        '<h3 class="text-lg font-semibold mt-4 mb-2">$1</h3>'
      )
      .replace(
        /^## (.*$)/gim,
        '<h2 class="text-xl font-semibold mt-6 mb-3">$1</h2>'
      )
      .replace(
        /^# (.*$)/gim,
        '<h1 class="text-2xl font-bold mt-6 mb-4">$1</h1>'
      )
      // Bold and italic
      .replace(/\*\*\*(.*?)\*\*\*/g, '<strong><em>$1</em></strong>')
      .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
      .replace(/\*(.*?)\*/g, '<em>$1</em>')
      // Code blocks
      .replace(
        /```(\w*)\n([\s\S]*?)```/g,
        '<pre class="bg-muted p-3 rounded-md overflow-x-auto my-3 text-sm"><code>$2</code></pre>'
      )
      .replace(
        /`([^`]+)`/g,
        '<code class="bg-muted px-1 py-0.5 rounded text-sm">$1</code>'
      )
      // Lists
      .replace(/^\s*[-*]\s(.*)$/gim, '<li class="ml-4">$1</li>')
      .replace(/(<li.*<\/li>\n?)+/g, '<ul class="list-disc my-2">$&</ul>')
      // Numbered lists
      .replace(/^\s*\d+\.\s(.*)$/gim, '<li class="ml-4">$1</li>')
      // Line breaks
      .replace(/\n\n/g, '</p><p class="my-3">')
      .replace(/\n/g, '<br/>');

    return `<p class="my-3">${html}</p>`;
  }, [content]);

  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="max-w-4xl max-h-[90vh] overflow-hidden flex flex-col">
        <DialogHeader className="border-b pb-3 shrink-0">
          <div className="flex items-start justify-between gap-3">
            <DialogTitle className="flex-1">{title}</DialogTitle>
            {filePath && (
              <a
                href={filePath}
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center gap-1.5 shrink-0 text-xs bg-primary/10 hover:bg-primary/20 text-primary border border-primary/30 rounded px-2.5 py-1.5 transition-colors"
              >
                <ExternalLink className="h-3.5 w-3.5" />
                Open {isPdf ? 'PDF' : 'File'}
              </a>
            )}
          </div>
        </DialogHeader>

        {/* PDF embed */}
        {isPdf && embedUrl && (
          <div
            className="flex-1 min-h-0 rounded overflow-hidden border border-border/40"
            style={{ height: '65vh' }}
          >
            <iframe
              src={embedUrl}
              title={title}
              className="w-full h-full"
              style={{ border: 'none' }}
            />
          </div>
        )}

        {/* Text content (shown when no PDF, or as supplement) */}
        {(!isPdf || content) && (
          <ScrollArea className="flex-1 max-h-[50vh]">
            <div
              className="prose prose-sm dark:prose-invert max-w-none p-4"
              dangerouslySetInnerHTML={{ __html: renderedContent }}
            />
          </ScrollArea>
        )}
      </DialogContent>
    </Dialog>
  );
}

// Video preview component
function VideoPreview({
  src,
  title,
  open,
  onClose,
  artifactId,
}: {
  src: string;
  title: string;
  open: boolean;
  onClose: () => void;
  artifactId?: string;
}) {
  const openReview = async () => {
    if (!artifactId) return;
    try {
      const res = await fetch(`/api/artifacts/${artifactId}/review-link`, {
        method: 'POST',
        credentials: 'include',
      });
      const data = await res.json();
      if (data?.data?.token) {
        window.open(
          `${window.location.origin}/review/${data.data.token}`,
          '_blank'
        );
      }
    } catch {
      /* ignore */
    }
  };

  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="max-w-4xl max-h-[80vh] p-0 overflow-hidden">
        <DialogHeader className="p-4 border-b flex flex-row items-center justify-between">
          <DialogTitle className="truncate flex-1">{title}</DialogTitle>
          {artifactId && (
            <button
              onClick={openReview}
              className="flex items-center gap-1.5 bg-amber-500/90 hover:bg-amber-400 text-white rounded px-2.5 py-1 text-xs font-medium transition-colors ml-3 shrink-0"
            >
              <Clapperboard className="h-3.5 w-3.5" />
              Review
            </button>
          )}
        </DialogHeader>
        <div className="aspect-video bg-black">
          <video src={src} controls autoPlay className="w-full h-full" />
        </div>
      </DialogContent>
    </Dialog>
  );
}

export function ArtifactPreviewCard({
  artifact,
  size = 'md',
  showCreator = false,
  onDownload,
  onUploadComplete,
  className,
}: ArtifactPreviewCardProps) {
  const [lightboxOpen, setLightboxOpen] = useState(false);
  const [uploading, setUploading] = useState(false);

  const metadata = useMemo(() => {
    try {
      return artifact.metadata ? JSON.parse(artifact.metadata) : {};
    } catch {
      return {};
    }
  }, [artifact.metadata]);

  const phase = metadata.phase as ArtifactPhase | undefined;
  const createdBy = metadata.created_by as 'agent' | 'human' | undefined;

  const isVisual = visualTypes.includes(artifact.artifact_type);
  const isDocument = documentTypes.includes(artifact.artifact_type);
  const isMedia = mediaTypes.includes(artifact.artifact_type);

  const config = sizeConfig[size];

  const handleClick = () => {
    // Open PDFs in the dialog (iframe embed); only bypass for non-PDF external links
    const isPdfFile = artifact.file_path?.toLowerCase().includes('.pdf');
    if (isDocument && artifact.file_path && !artifact.content && !isPdfFile) {
      window.open(artifact.file_path, '_blank', 'noopener,noreferrer');
      return;
    }
    setLightboxOpen(true);
  };

  const handleUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    setUploading(true);
    const formData = new FormData();
    formData.append('file', file, file.name);
    try {
      const res = await fetch(`/api/artifacts/${artifact.id}/upload`, {
        method: 'POST',
        body: formData,
        credentials: 'include',
      });
      if (res.ok) onUploadComplete?.();
    } catch {
      /* ignore */
    } finally {
      setUploading(false);
      e.target.value = '';
    }
  };

  // Get the artifact URL (content endpoint for JSON, file endpoint for media)
  const artifactUrl = artifactContentApi.getContentUrl(artifact.id);

  // For video types, get the first video file URL from content or file_path
  const videoFileUrl = useMemo(() => {
    if (!isMedia) return undefined;
    if (artifact.content) {
      try {
        const data = JSON.parse(artifact.content);
        const items = data.deliverables || data.edits || [];
        if (items.length > 0) {
          const item = items[0];
          const file = item.file || item.path?.split('/').pop();
          if (file) return artifactContentApi.getFileUrl(artifact.id, file);
          // Support absolute url field (e.g. /api/artifacts/.../files/...)
          if (item.url) return item.url.startsWith('/') ? item.url : item.url;
        }
      } catch {
        /* ignore */
      }
    }
    // Fall back to file_path stored directly on the artifact
    if (artifact.file_path) return artifact.file_path;
    return undefined;
  }, [artifact, isMedia]);

  return (
    <>
      <div
        className={cn(
          'relative group rounded-lg border overflow-hidden cursor-pointer transition-all hover:shadow-md hover:border-primary/50',
          config.height,
          className
        )}
        onClick={handleClick}
      >
        {/* Visual artifact preview */}
        {isVisual && (
          <div className="absolute inset-0 bg-gray-100 dark:bg-gray-800">
            {artifactUrl ? (
              <img
                src={artifactUrl}
                alt={artifact.title}
                className="w-full h-full object-cover"
              />
            ) : (
              <div className="flex items-center justify-center h-full">
                <Image className="h-8 w-8 text-gray-400" />
              </div>
            )}
            {/* Overlay gradient */}
            <div className="absolute inset-0 bg-gradient-to-t from-black/70 via-transparent to-transparent" />
            {/* Zoom icon on hover */}
            <div className="absolute inset-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity bg-black/20">
              <ZoomIn className="h-8 w-8 text-white" />
            </div>
          </div>
        )}

        {/* Document artifact preview */}
        {isDocument && (
          <div className="absolute inset-0 p-3 bg-gray-50 dark:bg-gray-900">
            <div className="flex items-center gap-1.5 mb-1">
              <FileText className="h-4 w-4 text-muted-foreground shrink-0" />
              {artifact.file_path && (
                <span className="text-[10px] font-medium text-primary/80 bg-primary/10 rounded px-1 py-0.5 uppercase tracking-wide">
                  {artifact.file_path.toLowerCase().includes('.pdf')
                    ? 'PDF'
                    : artifact.file_path.toLowerCase().includes('.docx')
                      ? 'DOCX'
                      : artifact.file_path.includes('docs.google.com')
                        ? 'Doc'
                        : artifact.file_path.includes('dropbox.com')
                          ? 'File'
                          : 'Link'}
                </span>
              )}
            </div>
            {artifact.content && (
              <div
                className={cn(
                  'text-xs text-muted-foreground overflow-hidden',
                  size === 'sm' && 'line-clamp-2',
                  size === 'md' && 'line-clamp-3',
                  size === 'lg' && 'line-clamp-5'
                )}
              >
                {artifact.content.substring(0, 300)}
              </div>
            )}
          </div>
        )}

        {/* Media artifact preview */}
        {isMedia && (
          <div className="absolute inset-0 bg-gray-900 flex items-center justify-center">
            <div className="absolute inset-0 bg-black/40" />
            <Play className="h-10 w-10 text-white/90" />
            {metadata.duration_seconds && (
              <div className="absolute bottom-2 right-2 bg-black/70 text-white text-xs px-1.5 py-0.5 rounded">
                {Math.floor(metadata.duration_seconds / 60)}:
                {String(metadata.duration_seconds % 60).padStart(2, '0')}
              </div>
            )}
          </div>
        )}

        {/* Title and badges overlay */}
        <div className="absolute bottom-0 left-0 right-0 p-2 bg-gradient-to-t from-black/80 to-transparent">
          <div className="flex items-center gap-1 mb-1">
            {phase && (
              <Badge
                variant="outline"
                className={cn('text-[9px] h-4', phaseColors[phase])}
              >
                {phase}
              </Badge>
            )}
            {showCreator && createdBy && (
              <div
                className={cn(
                  'h-4 w-4 rounded-full flex items-center justify-center',
                  createdBy === 'agent'
                    ? 'bg-blue-500 text-white'
                    : 'bg-green-500 text-white'
                )}
              >
                {createdBy === 'agent' ? (
                  <Bot className="h-2.5 w-2.5" />
                ) : (
                  <User className="h-2.5 w-2.5" />
                )}
              </div>
            )}
          </div>
          <span className="text-xs text-white font-medium truncate block">
            {artifact.title}
          </span>
        </div>

        {/* Review button for video edit types */}
        {VIDEO_EDIT_TYPES.includes(artifact.artifact_type) && (
          <button
            title="Open Review"
            onClick={async (e) => {
              e.stopPropagation();
              try {
                const res = await fetch(
                  `/api/artifacts/${artifact.id}/review-link`,
                  {
                    method: 'POST',
                    credentials: 'include',
                  }
                );
                const data = await res.json();
                if (data?.data?.token) {
                  window.open(
                    `${window.location.origin}/review/${data.data.token}`,
                    '_blank'
                  );
                }
              } catch {
                /* ignore */
              }
            }}
            className="absolute top-2 left-2 flex items-center gap-1 bg-amber-500/90 hover:bg-amber-400 text-white rounded px-1.5 py-0.5 text-xs font-medium transition-colors opacity-0 group-hover:opacity-100"
          >
            <Clapperboard className="h-2.5 w-2.5" />
            Review
          </button>
        )}

        {/* Upload button on hover */}
        {onUploadComplete && (
          <label
            className="absolute top-2 left-2 h-6 w-6 flex items-center justify-center rounded bg-secondary text-secondary-foreground opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer hover:bg-secondary/80"
            title="Upload file for this artifact"
            onClick={(e) => e.stopPropagation()}
          >
            {uploading ? (
              <span className="h-3 w-3 border border-current border-t-transparent rounded-full animate-spin" />
            ) : (
              <Upload className="h-3 w-3" />
            )}
            <input
              type="file"
              className="hidden"
              onChange={handleUpload}
              disabled={uploading}
            />
          </label>
        )}

        {/* Download button on hover */}
        {onDownload && (artifact.file_path || artifact.content) && (
          <IconButton
            variant="secondary"
            className="absolute top-2 right-2 h-6 w-6 opacity-0 group-hover:opacity-100 transition-opacity"
            onClick={(e) => {
              e.stopPropagation();
              onDownload();
            }}
            icon={Download}
            label="Download"
            iconClassName="h-3 w-3"
          />
        )}
      </div>

      {/* Lightbox dialogs */}
      {isVisual && artifact.file_path && (
        <ImageLightbox
          src={artifactUrl}
          alt={artifact.title}
          open={lightboxOpen}
          onClose={() => setLightboxOpen(false)}
        />
      )}

      {isDocument && (artifact.content || artifact.file_path) && (
        <DocumentPreview
          content={artifact.content ?? ''}
          title={artifact.title}
          filePath={artifact.file_path ?? undefined}
          open={lightboxOpen}
          onClose={() => setLightboxOpen(false)}
        />
      )}

      {isMedia && videoFileUrl && (
        <VideoPreview
          src={videoFileUrl}
          title={artifact.title}
          open={lightboxOpen}
          onClose={() => setLightboxOpen(false)}
          artifactId={
            VIDEO_EDIT_TYPES.includes(artifact.artifact_type)
              ? artifact.id
              : undefined
          }
        />
      )}
    </>
  );
}

export { DocumentPreview, ImageLightbox, VideoPreview };
