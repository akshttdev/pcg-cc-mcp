import { Card, CardContent, CardFooter, CardHeader } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import {
  CheckCircle2,
  XCircle,
  Clock,
  AlertTriangle,
  MessageSquare,
  Bot,
  User,
  FileText,
  Image,
  Video,
  Star,
  RotateCcw,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { useState } from 'react';
import type {
  ArtifactReview,
  ExecutionArtifact,
  ReviewStatus,
  ReviewType,
} from 'shared/types';

interface ApprovalCardProps {
  review: ArtifactReview;
  artifact?: ExecutionArtifact;
  onApprove?: (feedback?: string, rating?: number) => void;
  onReject?: (feedback: string) => void;
  onRequestRevision?: (notes: string) => void;
  onViewArtifact?: () => void;
  className?: string;
}

const reviewTypeLabels: Record<ReviewType, string> = {
  approval: 'Human Review Required',
  feedback: 'Feedback',
  revision_request: 'Revision Request',
  quality_check: 'Quality Check',
  compliance_check: 'Compliance Check',
};

const statusColors: Record<ReviewStatus, string> = {
  pending: 'bg-yellow-100 text-yellow-800',
  approved: 'bg-green-100 text-green-800',
  rejected: 'bg-red-100 text-red-800',
  revision_requested: 'bg-orange-100 text-orange-800',
  acknowledged: 'bg-blue-100 text-blue-800',
};

const statusIcons: Record<ReviewStatus, React.ReactNode> = {
  pending: <Clock className="h-4 w-4" />,
  approved: <CheckCircle2 className="h-4 w-4" />,
  rejected: <XCircle className="h-4 w-4" />,
  revision_requested: <RotateCcw className="h-4 w-4" />,
  acknowledged: <AlertTriangle className="h-4 w-4" />,
};

export function ApprovalCard({
  review,
  artifact,
  onApprove,
  onReject,
  onRequestRevision,
  onViewArtifact,
  className,
}: ApprovalCardProps) {
  const [feedback, setFeedback] = useState('');
  const [rating, setRating] = useState<number>(0);
  const [showFeedbackForm, setShowFeedbackForm] = useState(false);
  const [actionType, setActionType] = useState<'approve' | 'reject' | 'revision' | null>(null);

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  const handleAction = () => {
    if (actionType === 'approve') {
      onApprove?.(feedback || undefined, rating || undefined);
    } else if (actionType === 'reject') {
      onReject?.(feedback);
    } else if (actionType === 'revision') {
      onRequestRevision?.(feedback);
    }
    setShowFeedbackForm(false);
    setFeedback('');
    setRating(0);
    setActionType(null);
  };

  const isPending = review.status === 'pending';

  return (
    <Card className={cn('transition-all', className)}>
      <CardHeader className="p-4 pb-2">
        <div className="flex items-center justify-between">
          <Badge className={cn('text-xs', statusColors[review.status])}>
            <span className="mr-1">{statusIcons[review.status]}</span>
            {review.status}
          </Badge>
          <Badge variant="outline" className="text-xs">
            {reviewTypeLabels[review.review_type]}
          </Badge>
        </div>
      </CardHeader>

      <CardContent className="p-4 pt-2">
        {/* Artifact Preview */}
        {artifact && (
          <div
            className="flex items-center gap-3 p-3 bg-muted rounded-lg mb-3 cursor-pointer hover:bg-muted/80"
            onClick={onViewArtifact}
          >
            <div className="w-12 h-12 bg-background rounded flex items-center justify-center">
              {artifact.artifact_type.includes('Screenshot') ||
              artifact.artifact_type.includes('Image') ? (
                <Image className="h-6 w-6 text-muted-foreground" />
              ) : artifact.artifact_type.includes('Recording') ||
                artifact.artifact_type.includes('Video') ? (
                <Video className="h-6 w-6 text-muted-foreground" />
              ) : (
                <FileText className="h-6 w-6 text-muted-foreground" />
              )}
            </div>
            <div className="flex-1 min-w-0">
              <div className="font-medium text-sm truncate">
                {artifact.title || artifact.artifact_type}
              </div>
              <div className="text-xs text-muted-foreground">
                {artifact.artifact_type}
              </div>
            </div>
          </div>
        )}

        {/* Reviewer Info */}
        <div className="flex items-center gap-2 text-sm text-muted-foreground mb-2">
          {review.reviewer_agent_id ? (
            <>
              <Bot className="h-4 w-4" />
              <span>Agent Review</span>
            </>
          ) : review.reviewer_id ? (
            <>
              <User className="h-4 w-4" />
              <span>{review.reviewer_name || 'Human Review'}</span>
            </>
          ) : (
            <>
              <Clock className="h-4 w-4" />
              <span>Awaiting Reviewer</span>
            </>
          )}
        </div>

        {/* Existing Feedback */}
        {review.feedback_text && (
          <div className="p-3 bg-muted/50 rounded-lg mb-3">
            <div className="flex items-center gap-1 text-xs text-muted-foreground mb-1">
              <MessageSquare className="h-3 w-3" />
              Feedback
            </div>
            <p className="text-sm">{review.feedback_text}</p>
          </div>
        )}

        {/* Rating Display */}
        {review.rating && (
          <div className="flex items-center gap-1 mb-3">
            {Array.from({ length: 5 }).map((_, i) => (
              <Star
                key={i}
                className={cn(
                  'h-4 w-4',
                  i < review.rating! ? 'text-yellow-500 fill-yellow-500' : 'text-gray-300'
                )}
              />
            ))}
          </div>
        )}

        {/* Deadline */}
        {review.revision_deadline && (
          <div className="flex items-center gap-1 text-xs text-orange-600">
            <Clock className="h-3 w-3" />
            Deadline: {formatDate(review.revision_deadline)}
          </div>
        )}

        {/* Feedback Form */}
        {showFeedbackForm && (
          <div className="mt-4 space-y-3">
            <Textarea
              placeholder={
                actionType === 'approve'
                  ? 'Optional feedback...'
                  : actionType === 'reject'
                    ? 'Please provide a reason for rejection...'
                    : 'Describe what revisions are needed...'
              }
              value={feedback}
              onChange={(e) => setFeedback(e.target.value)}
              className="min-h-[80px]"
            />

            {actionType === 'approve' && (
              <div className="flex items-center gap-2">
                <span className="text-sm text-muted-foreground">Rating:</span>
                {Array.from({ length: 5 }).map((_, i) => (
                  <Star
                    key={i}
                    className={cn(
                      'h-5 w-5 cursor-pointer transition-colors',
                      i < rating
                        ? 'text-yellow-500 fill-yellow-500'
                        : 'text-gray-300 hover:text-yellow-400'
                    )}
                    onClick={() => setRating(i + 1)}
                  />
                ))}
              </div>
            )}

            <div className="flex gap-2">
              <Button
                size="sm"
                variant={actionType === 'approve' ? 'default' : 'destructive'}
                onClick={handleAction}
                disabled={
                  (actionType === 'reject' || actionType === 'revision') &&
                  !feedback.trim()
                }
              >
                Confirm {actionType === 'approve' ? 'Approval' : actionType === 'reject' ? 'Rejection' : 'Revision Request'}
              </Button>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => {
                  setShowFeedbackForm(false);
                  setFeedback('');
                  setRating(0);
                  setActionType(null);
                }}
              >
                Cancel
              </Button>
            </div>
          </div>
        )}
      </CardContent>

      {/* Action Buttons */}
      {isPending && !showFeedbackForm && (
        <CardFooter className="p-4 pt-0 gap-2">
          <Button
            size="sm"
            variant="default"
            className="flex-1"
            onClick={() => {
              setActionType('approve');
              setShowFeedbackForm(true);
            }}
          >
            <CheckCircle2 className="h-4 w-4 mr-1" />
            Approve
          </Button>
          <Button
            size="sm"
            variant="outline"
            className="flex-1"
            onClick={() => {
              setActionType('revision');
              setShowFeedbackForm(true);
            }}
          >
            <RotateCcw className="h-4 w-4 mr-1" />
            Revise
          </Button>
          <Button
            size="sm"
            variant="destructive"
            className="flex-1"
            onClick={() => {
              setActionType('reject');
              setShowFeedbackForm(true);
            }}
          >
            <XCircle className="h-4 w-4 mr-1" />
            Reject
          </Button>
        </CardFooter>
      )}

      {/* Timestamps */}
      <div className="px-4 pb-3 text-xs text-muted-foreground">
        Created {formatDate(review.created_at)}
        {review.resolved_at && ` • Resolved ${formatDate(review.resolved_at)}`}
      </div>
    </Card>
  );
}
