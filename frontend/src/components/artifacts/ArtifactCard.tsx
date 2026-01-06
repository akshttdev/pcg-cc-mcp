import { Card, CardContent } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  FileText,
  Image,
  Video,
  File,
  Clock,
  CheckCircle2,
  XCircle,
  AlertTriangle,
  Eye,
  Download,
  Pin,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type {
  ExecutionArtifact,
  ArtifactType,
  ArtifactPhase,
  ArtifactReviewStatus,
} from 'shared/types';
import { parseArtifactMetadata, formatArtifactPhase } from './utils';

interface ArtifactCardProps {
  artifact: ExecutionArtifact;
  isPinned?: boolean;
  onClick?: () => void;
  onPin?: () => void;
  onDownload?: () => void;
  onPreview?: () => void;
  className?: string;
}

const artifactTypeIcons: Partial<Record<ArtifactType, React.ReactNode>> = {
  plan: <FileText className="h-5 w-5" />,
  screenshot: <Image className="h-5 w-5" />,
  walkthrough: <Video className="h-5 w-5" />,
  diff_summary: <FileText className="h-5 w-5" />,
  test_result: <FileText className="h-5 w-5" />,
  checkpoint: <File className="h-5 w-5" />,
  error_report: <FileText className="h-5 w-5 text-red-500" />,
  research_report: <FileText className="h-5 w-5 text-blue-500" />,
  strategy_document: <FileText className="h-5 w-5 text-purple-500" />,
  content_calendar: <FileText className="h-5 w-5 text-green-500" />,
  competitor_analysis: <FileText className="h-5 w-5 text-orange-500" />,
  content_draft: <FileText className="h-5 w-5 text-yellow-500" />,
  visual_brief: <Image className="h-5 w-5 text-pink-500" />,
  schedule_manifest: <FileText className="h-5 w-5 text-cyan-500" />,
  engagement_log: <FileText className="h-5 w-5 text-indigo-500" />,
  verification_report: <CheckCircle2 className="h-5 w-5 text-green-500" />,
  browser_recording: <Video className="h-5 w-5 text-red-500" />,
  compliance_score: <FileText className="h-5 w-5 text-teal-500" />,
  platform_screenshot: <Image className="h-5 w-5 text-violet-500" />,
  subagent_result: <File className="h-5 w-5 text-amber-500" />,
  aggregated_research: <FileText className="h-5 w-5 text-emerald-500" />,
};

const phaseColors: Record<ArtifactPhase, string> = {
  planning: 'bg-blue-100 text-blue-800',
  execution: 'bg-yellow-100 text-yellow-800',
  verification: 'bg-green-100 text-green-800',
};

const reviewStatusIcons: Record<ArtifactReviewStatus, React.ReactNode> = {
  none: null,
  pending: <Clock className="h-3 w-3 text-yellow-500" />,
  approved: <CheckCircle2 className="h-3 w-3 text-green-500" />,
  rejected: <XCircle className="h-3 w-3 text-red-500" />,
  revision_requested: <AlertTriangle className="h-3 w-3 text-orange-500" />,
};

export function ArtifactCard({
  artifact,
  isPinned,
  onClick,
  onPin,
  onDownload,
  onPreview,
  className,
}: ArtifactCardProps) {
  const metadata = parseArtifactMetadata(artifact.metadata);
  const artifactPhase = metadata.phase;
  const reviewStatus = (metadata.review_status ?? 'none') as ArtifactReviewStatus;

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  const hasPreview =
    artifact.artifact_type === 'screenshot' ||
    artifact.artifact_type === 'platform_screenshot' ||
    artifact.artifact_type === 'walkthrough' ||
    artifact.artifact_type === 'browser_recording';

  return (
    <Card
      className={cn(
        'cursor-pointer transition-all hover:shadow-md overflow-hidden',
        isPinned && 'ring-2 ring-primary',
        className
      )}
      onClick={onClick}
    >
      {/* Thumbnail/Preview Area */}
      <div className="relative h-32 bg-muted flex items-center justify-center">
        {hasPreview && artifact.file_path ? (
          <img
            src={`/api/files/${artifact.file_path}`}
            alt={artifact.title || 'Artifact preview'}
            className="w-full h-full object-cover"
            onError={(e) => {
              // Fallback to icon if image fails to load
              (e.target as HTMLImageElement).style.display = 'none';
            }}
          />
        ) : (
          <div className="text-muted-foreground">
            {artifactTypeIcons[artifact.artifact_type] || (
              <File className="h-12 w-12" />
            )}
          </div>
        )}

        {/* Overlay badges */}
        {artifactPhase && (
          <div className="absolute top-2 left-2">
            <Badge
              className={cn(
                'text-[10px] uppercase tracking-wide',
                phaseColors[artifactPhase]
              )}
            >
              {formatArtifactPhase(artifactPhase)}
            </Badge>
          </div>
        )}

        <div className="absolute top-2 right-2 flex gap-1">
          {reviewStatus !== 'none' && reviewStatusIcons[reviewStatus] && (
            <div className="bg-white rounded-full p-1 shadow">
              {reviewStatusIcons[reviewStatus]}
            </div>
          )}
          {isPinned && (
            <div className="bg-primary text-primary-foreground rounded-full p-1 shadow">
              <Pin className="h-3 w-3" />
            </div>
          )}
        </div>

        {/* Action buttons on hover */}
        <div className="absolute inset-0 bg-black/50 opacity-0 hover:opacity-100 transition-opacity flex items-center justify-center gap-2">
          {onPreview && (
            <Button
              size="icon"
              variant="secondary"
              onClick={(e) => {
                e.stopPropagation();
                onPreview();
              }}
            >
              <Eye className="h-4 w-4" />
            </Button>
          )}
          {onDownload && artifact.file_path && (
            <Button
              size="icon"
              variant="secondary"
              onClick={(e) => {
                e.stopPropagation();
                onDownload();
              }}
            >
              <Download className="h-4 w-4" />
            </Button>
          )}
          {onPin && (
            <Button
              size="icon"
              variant={isPinned ? 'default' : 'secondary'}
              onClick={(e) => {
                e.stopPropagation();
                onPin();
              }}
            >
              <Pin className="h-4 w-4" />
            </Button>
          )}
        </div>
      </div>

      <CardContent className="p-3">
        {/* Title */}
        <div className="font-medium text-sm truncate mb-1">
          {artifact.title || artifact.artifact_type}
        </div>

        {/* Type badge */}
        <Badge variant="outline" className="text-xs mb-2">
          {artifact.artifact_type}
        </Badge>

        {/* Metadata */}
        <div className="flex items-center justify-between text-xs text-muted-foreground">
          <div className="flex items-center gap-1">
            <Clock className="h-3 w-3" />
            {formatDate(artifact.created_at)}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
