import { useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Textarea } from '@/components/ui/textarea';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  AlertTriangle,
  CheckCircle,
  XCircle,
  Clock,
  FileCode,
  Globe,
  DollarSign,
  Timer,
  Settings,
  SkipForward,
} from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import {
  usePendingCheckpoints,
  useReviewCheckpoint,
  useSkipCheckpoint,
  type ExecutionCheckpoint,
  type CheckpointType,
  type CheckpointStatus,
} from '@/hooks/useAutonomy';

interface CheckpointReviewPanelProps {
  executionId: string;
  currentUserId: string;
  currentUserName?: string;
  className?: string;
}

function getCheckpointTypeIcon(type: CheckpointType | undefined) {
  switch (type) {
    case 'file_change':
      return <FileCode className="h-4 w-4" />;
    case 'external_call':
      return <Globe className="h-4 w-4" />;
    case 'cost_threshold':
      return <DollarSign className="h-4 w-4" />;
    case 'time_threshold':
      return <Timer className="h-4 w-4" />;
    case 'custom':
    default:
      return <Settings className="h-4 w-4" />;
  }
}

function getStatusBadge(status: CheckpointStatus) {
  switch (status) {
    case 'pending':
      return <Badge variant="secondary"><Clock className="h-3 w-3 mr-1" />Pending</Badge>;
    case 'approved':
      return <Badge variant="default" className="bg-green-600"><CheckCircle className="h-3 w-3 mr-1" />Approved</Badge>;
    case 'rejected':
      return <Badge variant="destructive"><XCircle className="h-3 w-3 mr-1" />Rejected</Badge>;
    case 'auto_approved':
      return <Badge variant="outline" className="text-green-600"><CheckCircle className="h-3 w-3 mr-1" />Auto-Approved</Badge>;
    case 'skipped':
      return <Badge variant="outline"><SkipForward className="h-3 w-3 mr-1" />Skipped</Badge>;
    case 'expired':
      return <Badge variant="outline" className="text-muted-foreground"><Clock className="h-3 w-3 mr-1" />Expired</Badge>;
    default:
      return <Badge variant="secondary">{status}</Badge>;
  }
}

interface CheckpointItemProps {
  checkpoint: ExecutionCheckpoint;
  onApprove: (id: string, note?: string) => void;
  onReject: (id: string, note?: string) => void;
  onSkip: (id: string) => void;
  isReviewing: boolean;
}

function CheckpointItem({ checkpoint, onApprove, onReject, onSkip, isReviewing }: CheckpointItemProps) {
  const [showDialog, setShowDialog] = useState(false);
  const [action, setAction] = useState<'approve' | 'reject' | null>(null);
  const [note, setNote] = useState('');

  const checkpointData = checkpoint.checkpoint_data;
  const checkpointType = (checkpointData as { type?: CheckpointType })?.type;

  const handleSubmit = () => {
    if (action === 'approve') {
      onApprove(checkpoint.id, note || undefined);
    } else if (action === 'reject') {
      onReject(checkpoint.id, note || undefined);
    }
    setShowDialog(false);
    setNote('');
    setAction(null);
  };

  return (
    <>
      <div className="p-3 rounded-lg border border-yellow-500 bg-yellow-500/10">
        <div className="flex items-start justify-between gap-2">
          <div className="flex items-center gap-2">
            <AlertTriangle className="h-4 w-4 text-yellow-600" />
            {getCheckpointTypeIcon(checkpointType)}
            {getStatusBadge(checkpoint.status)}
          </div>
          <span className="text-xs text-muted-foreground">
            {formatDistanceToNow(new Date(checkpoint.created_at), { addSuffix: true })}
          </span>
        </div>

        {checkpoint.trigger_reason && (
          <p className="mt-2 text-sm font-medium">{checkpoint.trigger_reason}</p>
        )}

        {checkpointData && Object.keys(checkpointData).length > 0 && (
          <div className="mt-2 p-2 bg-muted rounded text-xs font-mono overflow-x-auto">
            <pre>{JSON.stringify(checkpointData, null, 2)}</pre>
          </div>
        )}

        {checkpoint.expires_at && (
          <p className="mt-2 text-xs text-muted-foreground">
            Expires {formatDistanceToNow(new Date(checkpoint.expires_at), { addSuffix: true })}
          </p>
        )}

        <div className="mt-3 flex items-center gap-2">
          <Button
            size="sm"
            variant="default"
            onClick={() => {
              setAction('approve');
              setShowDialog(true);
            }}
            disabled={isReviewing || checkpoint.status !== 'pending'}
          >
            <CheckCircle className="h-3 w-3 mr-1" />
            Approve
          </Button>
          <Button
            size="sm"
            variant="destructive"
            onClick={() => {
              setAction('reject');
              setShowDialog(true);
            }}
            disabled={isReviewing || checkpoint.status !== 'pending'}
          >
            <XCircle className="h-3 w-3 mr-1" />
            Reject
          </Button>
          <Button
            size="sm"
            variant="outline"
            onClick={() => onSkip(checkpoint.id)}
            disabled={isReviewing || checkpoint.status !== 'pending'}
          >
            <SkipForward className="h-3 w-3 mr-1" />
            Skip
          </Button>
        </div>
      </div>

      <Dialog open={showDialog} onOpenChange={setShowDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {action === 'approve' ? 'Approve Checkpoint' : 'Reject Checkpoint'}
            </DialogTitle>
            <DialogDescription>
              {action === 'approve'
                ? 'Approve this checkpoint to allow execution to continue.'
                : 'Reject this checkpoint to stop execution and require changes.'}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4">
            <div>
              <label className="text-sm font-medium">Note (optional)</label>
              <Textarea
                value={note}
                onChange={(e) => setNote(e.target.value)}
                placeholder="Add a note about your decision..."
                className="mt-1"
              />
            </div>
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={() => setShowDialog(false)}>
              Cancel
            </Button>
            <Button
              variant={action === 'approve' ? 'default' : 'destructive'}
              onClick={handleSubmit}
              disabled={isReviewing}
            >
              {action === 'approve' ? 'Approve' : 'Reject'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

export function CheckpointReviewPanel({
  executionId,
  currentUserId,
  currentUserName,
  className,
}: CheckpointReviewPanelProps) {
  const { data: pendingCheckpoints = [], isLoading } = usePendingCheckpoints(executionId);
  const { mutate: review, isPending: isReviewing } = useReviewCheckpoint();
  const { mutate: skip, isPending: isSkipping } = useSkipCheckpoint();

  const handleApprove = (checkpointId: string, note?: string) => {
    review({
      checkpointId,
      reviewer_id: currentUserId,
      reviewer_name: currentUserName,
      decision: 'approved',
      review_note: note,
    });
  };

  const handleReject = (checkpointId: string, note?: string) => {
    review({
      checkpointId,
      reviewer_id: currentUserId,
      reviewer_name: currentUserName,
      decision: 'rejected',
      review_note: note,
    });
  };

  const handleSkip = (checkpointId: string) => {
    skip(checkpointId);
  };

  return (
    <Card className={className}>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <CardTitle className="text-base flex items-center gap-2">
            <AlertTriangle className="h-4 w-4" />
            Pending Checkpoints
            {pendingCheckpoints.length > 0 && (
              <Badge variant="destructive">{pendingCheckpoints.length}</Badge>
            )}
          </CardTitle>
        </div>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="text-center py-4 text-muted-foreground">Loading...</div>
        ) : pendingCheckpoints.length === 0 ? (
          <div className="text-center py-4 text-muted-foreground">
            No pending checkpoints
          </div>
        ) : (
          <ScrollArea className="h-[300px]">
            <div className="space-y-3">
              {pendingCheckpoints.map((checkpoint) => (
                <CheckpointItem
                  key={checkpoint.id}
                  checkpoint={checkpoint}
                  onApprove={handleApprove}
                  onReject={handleReject}
                  onSkip={handleSkip}
                  isReviewing={isReviewing || isSkipping}
                />
              ))}
            </div>
          </ScrollArea>
        )}
      </CardContent>
    </Card>
  );
}
