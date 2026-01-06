import { useState } from 'react';
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Download,
  ExternalLink,
  Copy,
  CheckCircle2,
  Clock,
  Bot,
  User,
  FileText,
  Image as ImageIcon,
  Video,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { ExecutionArtifact, ArtifactReview } from 'shared/types';
import { parseArtifactMetadata, formatArtifactPhase } from './utils';

interface ArtifactPreviewPanelProps {
  artifact: ExecutionArtifact | null;
  reviews?: ArtifactReview[];
  isOpen: boolean;
  onClose: () => void;
  onDownload?: () => void;
  onRequestReview?: () => void;
}

export function ArtifactPreviewPanel({
  artifact,
  reviews = [],
  isOpen,
  onClose,
  onDownload,
  onRequestReview,
}: ArtifactPreviewPanelProps) {
  const [copied, setCopied] = useState(false);

  if (!artifact) return null;

  const metadata = parseArtifactMetadata(artifact.metadata);
  const phaseLabel = formatArtifactPhase(metadata.phase);
  const reviewStatus = metadata.review_status ?? 'none';
  const createdByAgentId = metadata.created_by_agent_id;
  const createdByName = metadata.created_by_agent_name || metadata.created_by;

  const imageTypes = new Set([
    'screenshot',
    'platform_screenshot',
    'visual_brief',
  ]);
  const videoTypes = new Set(['walkthrough', 'browser_recording']);
  const documentTypes = new Set([
    'plan',
    'diff_summary',
    'test_result',
    'content_draft',
    'verification_report',
    'research_report',
    'strategy_document',
    'content_calendar',
    'competitor_analysis',
    'schedule_manifest',
    'engagement_log',
    'aggregated_research',
    'error_report',
  ]);

  const isImage = imageTypes.has(artifact.artifact_type);
  const isVideo = videoTypes.has(artifact.artifact_type);
  const isDocument = documentTypes.has(artifact.artifact_type);

  const handleCopyContent = async () => {
    if (artifact.content) {
      await navigator.clipboard.writeText(artifact.content);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleString();
  };

  return (
    <Sheet open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <SheetContent className="w-full sm:max-w-2xl overflow-hidden flex flex-col">
        <SheetHeader className="pb-4 border-b">
          <SheetTitle className="flex items-center gap-2">
            {isImage && <ImageIcon className="h-5 w-5" />}
            {isVideo && <Video className="h-5 w-5" />}
            {isDocument && <FileText className="h-5 w-5" />}
            {artifact.title || artifact.artifact_type}
          </SheetTitle>
        </SheetHeader>

        <Tabs defaultValue="preview" className="flex-1 flex flex-col mt-4">
          <TabsList>
            <TabsTrigger value="preview">Preview</TabsTrigger>
            <TabsTrigger value="details">Details</TabsTrigger>
            <TabsTrigger value="reviews">
              Reviews ({reviews.length})
            </TabsTrigger>
          </TabsList>

          <TabsContent value="preview" className="flex-1 mt-4">
            <ScrollArea className="h-[calc(100vh-250px)]">
              {isImage && artifact.file_path && (
                <div className="flex items-center justify-center p-4 bg-muted rounded-lg">
                  <img
                    src={`/api/files/${artifact.file_path}`}
                    alt={artifact.title || 'Artifact'}
                    className="max-w-full max-h-[60vh] object-contain rounded"
                  />
                </div>
              )}

              {isVideo && artifact.file_path && (
                <div className="flex items-center justify-center p-4 bg-muted rounded-lg">
                  <video
                    src={`/api/files/${artifact.file_path}`}
                    controls
                    className="max-w-full max-h-[60vh] rounded"
                  />
                </div>
              )}

              {isDocument && artifact.content && (
                <div className="prose prose-sm max-w-none p-4 bg-muted rounded-lg">
                  <pre className="whitespace-pre-wrap text-sm">
                    {artifact.content}
                  </pre>
                </div>
              )}

              {!isImage && !isVideo && !isDocument && (
                <div className="flex flex-col items-center justify-center h-64 text-muted-foreground">
                  <FileText className="h-12 w-12 mb-4" />
                  <p>No preview available for this artifact type</p>
                </div>
              )}
            </ScrollArea>
          </TabsContent>

          <TabsContent value="details" className="flex-1 mt-4">
            <ScrollArea className="h-[calc(100vh-250px)]">
              <div className="space-y-4">
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <label className="text-sm font-medium text-muted-foreground">
                      Type
                    </label>
                    <div>
                      <Badge variant="outline">{artifact.artifact_type}</Badge>
                    </div>
                  </div>

                  <div>
                    <label className="text-sm font-medium text-muted-foreground">
                      Phase
                    </label>
                    <div>
                      <Badge>{phaseLabel}</Badge>
                    </div>
                  </div>

                  <div>
                    <label className="text-sm font-medium text-muted-foreground">
                      Review Status
                    </label>
                    <div>
                      <Badge
                        variant={
                          reviewStatus === 'approved'
                            ? 'default'
                            : reviewStatus === 'rejected'
                              ? 'destructive'
                              : 'secondary'
                        }
                      >
                        {reviewStatus ?? 'none'}
                      </Badge>
                    </div>
                  </div>

                  <div>
                    <label className="text-sm font-medium text-muted-foreground">
                      Created
                    </label>
                    <div className="flex items-center gap-1 text-sm">
                      <Clock className="h-3 w-3" />
                      {formatDate(artifact.created_at)}
                    </div>
                  </div>
                </div>

                <div>
                  <label className="text-sm font-medium text-muted-foreground">
                    Created By
                  </label>
                  <div className="flex items-center gap-2 mt-1">
                    {createdByAgentId ? (
                      <>
                        <Bot className="h-4 w-4" />
                        <span className="text-sm">
                          Agent: {createdByName || createdByAgentId}
                        </span>
                      </>
                    ) : (
                      <>
                        <User className="h-4 w-4" />
                        <span className="text-sm">Manual upload</span>
                      </>
                    )}
                  </div>
                </div>

                {artifact.file_path && (
                  <div>
                    <label className="text-sm font-medium text-muted-foreground">
                      File Path
                    </label>
                    <div className="text-sm font-mono bg-muted p-2 rounded mt-1 truncate">
                      {artifact.file_path}
                    </div>
                  </div>
                )}

                {artifact.metadata && (
                  <div>
                    <label className="text-sm font-medium text-muted-foreground">
                      Metadata
                    </label>
                    <pre className="text-sm bg-muted p-2 rounded mt-1 overflow-auto max-h-40">
                      {JSON.stringify(metadata, null, 2)}
                    </pre>
                  </div>
                )}
              </div>
            </ScrollArea>
          </TabsContent>

          <TabsContent value="reviews" className="flex-1 mt-4">
            <ScrollArea className="h-[calc(100vh-250px)]">
              {reviews.length === 0 ? (
                <div className="flex flex-col items-center justify-center h-40 text-muted-foreground">
                  <p>No reviews yet</p>
                  {onRequestReview && (
                    <Button
                      variant="outline"
                      className="mt-4"
                      onClick={onRequestReview}
                    >
                      Request Review
                    </Button>
                  )}
                </div>
              ) : (
                <div className="space-y-4">
                  {reviews.map((review) => (
                    <div
                      key={review.id}
                      className="p-4 border rounded-lg space-y-2"
                    >
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-2">
                          {review.reviewer_agent_id ? (
                            <Bot className="h-4 w-4" />
                          ) : (
                            <User className="h-4 w-4" />
                          )}
                          <span className="font-medium">
                            {review.reviewer_name || 'Anonymous'}
                          </span>
                        </div>
                        <Badge
                          variant={
                            review.status === 'approved'
                              ? 'default'
                              : review.status === 'rejected'
                                ? 'destructive'
                                : 'secondary'
                          }
                        >
                          {review.status}
                        </Badge>
                      </div>

                      {review.feedback_text && (
                        <p className="text-sm text-muted-foreground">
                          {review.feedback_text}
                        </p>
                      )}

                      {review.rating && (
                        <div className="flex items-center gap-1">
                          {Array.from({ length: 5 }).map((_, i) => (
                            <CheckCircle2
                              key={i}
                              className={cn(
                                'h-4 w-4',
                                i < review.rating!
                                  ? 'text-yellow-500'
                                  : 'text-gray-300'
                              )}
                            />
                          ))}
                        </div>
                      )}

                      <div className="text-xs text-muted-foreground">
                        {formatDate(review.created_at)}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </ScrollArea>
          </TabsContent>
        </Tabs>

        {/* Actions */}
        <div className="flex gap-2 pt-4 border-t mt-4">
          {artifact.content && (
            <Button variant="outline" onClick={handleCopyContent}>
              {copied ? (
                <CheckCircle2 className="h-4 w-4 mr-2" />
              ) : (
                <Copy className="h-4 w-4 mr-2" />
              )}
              {copied ? 'Copied!' : 'Copy Content'}
            </Button>
          )}
          {artifact.file_path && onDownload && (
            <Button variant="outline" onClick={onDownload}>
              <Download className="h-4 w-4 mr-2" />
              Download
            </Button>
          )}
          {artifact.file_path && (
            <Button
              variant="outline"
              onClick={() =>
                window.open(`/api/files/${artifact.file_path}`, '_blank')
              }
            >
              <ExternalLink className="h-4 w-4 mr-2" />
              Open
            </Button>
          )}
        </div>
      </SheetContent>
    </Sheet>
  );
}
