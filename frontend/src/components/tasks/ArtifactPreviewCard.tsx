import { useState, useMemo } from 'react';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Badge } from '@/components/ui/badge';
import {
  FileText,
  Image,
  Video,
  Play,
  X,
  Download,
  ZoomIn,
  ZoomOut,
  RotateCw,
  Bot,
  User,
} from 'lucide-react';
import type { ExecutionArtifact, ArtifactType, ArtifactPhase } from 'shared/types';

interface ArtifactPreviewCardProps {
  artifact: ExecutionArtifact;
  size?: 'sm' | 'md' | 'lg';
  showCreator?: boolean;
  onDownload?: () => void;
  className?: string;
}

// Artifact type categories
const visualTypes: ArtifactType[] = ['screenshot', 'visual_brief', 'platform_screenshot'];
const documentTypes: ArtifactType[] = ['research_report', 'strategy_document', 'content_draft', 'content_calendar', 'competitor_analysis', 'plan'];
const mediaTypes: ArtifactType[] = ['walkthrough', 'browser_recording'];

// Phase colors
const phaseColors: Record<ArtifactPhase, string> = {
  planning: 'bg-purple-100 text-purple-700 dark:bg-purple-900 dark:text-purple-300',
  execution: 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900 dark:text-yellow-300',
  verification: 'bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300',
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
            <Button variant="ghost" size="icon" onClick={handleZoomOut} className="h-8 w-8">
              <ZoomOut className="h-4 w-4" />
            </Button>
            <span className="text-xs text-muted-foreground w-12 text-center">
              {Math.round(zoom * 100)}%
            </span>
            <Button variant="ghost" size="icon" onClick={handleZoomIn} className="h-8 w-8">
              <ZoomIn className="h-4 w-4" />
            </Button>
            <Button variant="ghost" size="icon" onClick={handleRotate} className="h-8 w-8">
              <RotateCw className="h-4 w-4" />
            </Button>
            <Button variant="ghost" size="icon" onClick={onClose} className="h-8 w-8">
              <X className="h-4 w-4" />
            </Button>
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
  open,
  onClose,
}: {
  content: string;
  title: string;
  open: boolean;
  onClose: () => void;
}) {
  // Simple markdown-like rendering (can be enhanced with a proper markdown library)
  const renderedContent = useMemo(() => {
    if (!content) return '';

    // Convert markdown headers
    let html = content
      .replace(/^### (.*$)/gim, '<h3 class="text-lg font-semibold mt-4 mb-2">$1</h3>')
      .replace(/^## (.*$)/gim, '<h2 class="text-xl font-semibold mt-6 mb-3">$1</h2>')
      .replace(/^# (.*$)/gim, '<h1 class="text-2xl font-bold mt-6 mb-4">$1</h1>')
      // Bold and italic
      .replace(/\*\*\*(.*?)\*\*\*/g, '<strong><em>$1</em></strong>')
      .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
      .replace(/\*(.*?)\*/g, '<em>$1</em>')
      // Code blocks
      .replace(/```(\w*)\n([\s\S]*?)```/g, '<pre class="bg-muted p-3 rounded-md overflow-x-auto my-3 text-sm"><code>$2</code></pre>')
      .replace(/`([^`]+)`/g, '<code class="bg-muted px-1 py-0.5 rounded text-sm">$1</code>')
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
      <DialogContent className="max-w-3xl max-h-[80vh] overflow-hidden">
        <DialogHeader className="border-b pb-3">
          <DialogTitle>{title}</DialogTitle>
        </DialogHeader>
        <ScrollArea className="flex-1 max-h-[60vh]">
          <div
            className="prose prose-sm dark:prose-invert max-w-none p-4"
            dangerouslySetInnerHTML={{ __html: renderedContent }}
          />
        </ScrollArea>
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
}: {
  src: string;
  title: string;
  open: boolean;
  onClose: () => void;
}) {
  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="max-w-4xl max-h-[80vh] p-0 overflow-hidden">
        <DialogHeader className="p-4 border-b">
          <DialogTitle>{title}</DialogTitle>
        </DialogHeader>
        <div className="aspect-video bg-black">
          <video
            src={src}
            controls
            autoPlay
            className="w-full h-full"
          />
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
  className,
}: ArtifactPreviewCardProps) {
  const [lightboxOpen, setLightboxOpen] = useState(false);

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
    setLightboxOpen(true);
  };

  // Get the artifact URL
  const artifactUrl = artifact.file_path
    ? `/api/files/${encodeURIComponent(artifact.file_path)}`
    : undefined;

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
            <FileText className="h-4 w-4 text-muted-foreground mb-1" />
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
            {artifactUrl ? (
              <>
                <div className="absolute inset-0 bg-black/40" />
                <Play className="h-10 w-10 text-white/90" />
              </>
            ) : (
              <Video className="h-8 w-8 text-gray-400" />
            )}
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
              <Badge variant="outline" className={cn('text-[9px] h-4', phaseColors[phase])}>
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

        {/* Download button on hover */}
        {onDownload && artifactUrl && (
          <Button
            variant="secondary"
            size="icon"
            className="absolute top-2 right-2 h-6 w-6 opacity-0 group-hover:opacity-100 transition-opacity"
            onClick={(e) => {
              e.stopPropagation();
              onDownload();
            }}
          >
            <Download className="h-3 w-3" />
          </Button>
        )}
      </div>

      {/* Lightbox dialogs */}
      {isVisual && artifactUrl && (
        <ImageLightbox
          src={artifactUrl}
          alt={artifact.title}
          open={lightboxOpen}
          onClose={() => setLightboxOpen(false)}
        />
      )}

      {isDocument && artifact.content && (
        <DocumentPreview
          content={artifact.content}
          title={artifact.title}
          open={lightboxOpen}
          onClose={() => setLightboxOpen(false)}
        />
      )}

      {isMedia && artifactUrl && (
        <VideoPreview
          src={artifactUrl}
          title={artifact.title}
          open={lightboxOpen}
          onClose={() => setLightboxOpen(false)}
        />
      )}
    </>
  );
}

export { ImageLightbox, DocumentPreview, VideoPreview };
