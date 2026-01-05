import { useState } from 'react';
import { cn } from '@/lib/utils';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
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
  Pause,
  Play,
  UserCheck,
  Send,
  Loader2,
  User,
  Bot,
  MessageSquare,
} from 'lucide-react';
import type { ControlState } from '@/hooks/useCollaboration';
import {
  useCollaborationState,
  usePauseExecution,
  useResumeExecution,
  useTakeoverExecution,
  useReturnControl,
  useInjectContext,
} from '@/hooks/useCollaboration';

interface ExecutionControlPanelProps {
  executionId: string;
  currentUserId: string;
  currentUserName?: string;
  className?: string;
}

function getControlStateBadge(state: ControlState) {
  switch (state) {
    case 'running':
      return <Badge className="bg-green-500">Running</Badge>;
    case 'paused':
      return <Badge variant="secondary">Paused</Badge>;
    case 'human_takeover':
      return <Badge className="bg-blue-500">Human Control</Badge>;
    case 'awaiting_input':
      return <Badge className="bg-yellow-500">Awaiting Input</Badge>;
    default:
      return <Badge variant="outline">{state}</Badge>;
  }
}

export function ExecutionControlPanel({
  executionId,
  currentUserId,
  currentUserName,
  className,
}: ExecutionControlPanelProps) {
  const { data: state, isLoading } = useCollaborationState(executionId);
  const { mutate: pauseExecution, isPending: isPausing } = usePauseExecution(executionId);
  const { mutate: resumeExecution, isPending: isResuming } = useResumeExecution(executionId);
  const { mutate: takeoverExecution, isPending: isTakingOver } = useTakeoverExecution(executionId);
  const { mutate: returnControlMutation, isPending: isReturning } = useReturnControl(executionId);
  const { mutate: injectContextMutation, isPending: isInjecting } = useInjectContext(executionId);

  const [showPauseDialog, setShowPauseDialog] = useState(false);
  const [showTakeoverDialog, setShowTakeoverDialog] = useState(false);
  const [showReturnDialog, setShowReturnDialog] = useState(false);
  const [showInjectDialog, setShowInjectDialog] = useState(false);

  const [pauseReason, setPauseReason] = useState('');
  const [takeoverReason, setTakeoverReason] = useState('');
  const [returnAgentId, setReturnAgentId] = useState('');
  const [returnNotes, setReturnNotes] = useState('');
  const [injectContent, setInjectContent] = useState('');

  if (isLoading || !state) {
    return (
      <Card className={cn('', className)}>
        <CardContent className="py-6 flex items-center justify-center">
          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
        </CardContent>
      </Card>
    );
  }

  const isHumanInControl = state.control_state === 'human_takeover';
  const isCurrentUserInControl =
    isHumanInControl &&
    state.current_controller?.actor_type === 'human' &&
    state.current_controller?.actor_id === currentUserId;
  const isPaused = state.control_state === 'paused';
  const isRunning = state.control_state === 'running';

  const handlePause = () => {
    pauseExecution(
      {
        reason: pauseReason || undefined,
        initiated_by: currentUserId,
        initiated_by_name: currentUserName,
      },
      {
        onSuccess: () => {
          setShowPauseDialog(false);
          setPauseReason('');
        },
      }
    );
  };

  const handleResume = () => {
    resumeExecution({
      initiated_by: currentUserId,
      initiated_by_name: currentUserName,
    });
  };

  const handleTakeover = () => {
    takeoverExecution(
      {
        human_id: currentUserId,
        human_name: currentUserName,
        reason: takeoverReason || undefined,
      },
      {
        onSuccess: () => {
          setShowTakeoverDialog(false);
          setTakeoverReason('');
        },
      }
    );
  };

  const handleReturnControl = () => {
    returnControlMutation(
      {
        human_id: currentUserId,
        human_name: currentUserName,
        to_agent_id: returnAgentId || 'default-agent',
        context_notes: returnNotes || undefined,
      },
      {
        onSuccess: () => {
          setShowReturnDialog(false);
          setReturnAgentId('');
          setReturnNotes('');
        },
      }
    );
  };

  const handleInjectContext = () => {
    if (!injectContent.trim()) return;

    injectContextMutation(
      {
        injector_id: currentUserId,
        injector_name: currentUserName,
        injection_type: 'note',
        content: injectContent,
      },
      {
        onSuccess: () => {
          setShowInjectDialog(false);
          setInjectContent('');
        },
      }
    );
  };

  return (
    <Card className={cn('', className)}>
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-sm font-medium flex items-center gap-2">
            Execution Control
            {getControlStateBadge(state.control_state)}
          </CardTitle>
          {state.current_controller && (
            <div className="flex items-center gap-1 text-xs text-muted-foreground">
              {state.current_controller.actor_type === 'human' ? (
                <User className="h-3 w-3" />
              ) : (
                <Bot className="h-3 w-3" />
              )}
              <span>{state.current_controller.actor_name || state.current_controller.actor_id}</span>
            </div>
          )}
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Control buttons */}
        <div className="flex flex-wrap gap-2">
          {/* Pause/Resume */}
          {isRunning && (
            <Button variant="outline" size="sm" onClick={() => setShowPauseDialog(true)}>
              <Pause className="h-4 w-4 mr-2" />
              Pause
            </Button>
          )}
          {isPaused && (
            <Button variant="outline" size="sm" onClick={handleResume} disabled={isResuming}>
              {isResuming ? (
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
              ) : (
                <Play className="h-4 w-4 mr-2" />
              )}
              Resume
            </Button>
          )}

          {/* Takeover */}
          {!isHumanInControl && (
            <Button variant="outline" size="sm" onClick={() => setShowTakeoverDialog(true)}>
              <UserCheck className="h-4 w-4 mr-2" />
              Take Control
            </Button>
          )}

          {/* Return Control */}
          {isCurrentUserInControl && (
            <Button variant="outline" size="sm" onClick={() => setShowReturnDialog(true)}>
              <Bot className="h-4 w-4 mr-2" />
              Return to Agent
            </Button>
          )}

          {/* Inject Note */}
          <Button variant="outline" size="sm" onClick={() => setShowInjectDialog(true)}>
            <MessageSquare className="h-4 w-4 mr-2" />
            Add Note
          </Button>
        </div>

        {/* Pending injections count */}
        {state.pending_injections.length > 0 && (
          <div className="flex items-center gap-2 text-sm text-yellow-600 dark:text-yellow-400">
            <MessageSquare className="h-4 w-4" />
            {state.pending_injections.length} pending note(s)
          </div>
        )}

        {/* Pause Dialog */}
        <Dialog open={showPauseDialog} onOpenChange={setShowPauseDialog}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Pause Execution</DialogTitle>
              <DialogDescription>
                The agent will be paused until you resume it.
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-2">
              <Label htmlFor="pauseReason">Reason (optional)</Label>
              <Input
                id="pauseReason"
                value={pauseReason}
                onChange={(e) => setPauseReason(e.target.value)}
                placeholder="Why are you pausing?"
              />
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setShowPauseDialog(false)}>
                Cancel
              </Button>
              <Button onClick={handlePause} disabled={isPausing}>
                {isPausing ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : null}
                Pause
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        {/* Takeover Dialog */}
        <Dialog open={showTakeoverDialog} onOpenChange={setShowTakeoverDialog}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Take Control</DialogTitle>
              <DialogDescription>
                You will take over control from the agent. The agent will pause until you return
                control.
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-2">
              <Label htmlFor="takeoverReason">Reason (optional)</Label>
              <Input
                id="takeoverReason"
                value={takeoverReason}
                onChange={(e) => setTakeoverReason(e.target.value)}
                placeholder="Why are you taking over?"
              />
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setShowTakeoverDialog(false)}>
                Cancel
              </Button>
              <Button onClick={handleTakeover} disabled={isTakingOver}>
                {isTakingOver ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : null}
                Take Control
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        {/* Return Control Dialog */}
        <Dialog open={showReturnDialog} onOpenChange={setShowReturnDialog}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Return Control to Agent</DialogTitle>
              <DialogDescription>
                The agent will resume execution with any notes you provide.
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="returnAgentId">Agent ID (optional)</Label>
                <Input
                  id="returnAgentId"
                  value={returnAgentId}
                  onChange={(e) => setReturnAgentId(e.target.value)}
                  placeholder="Leave empty for default agent"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="returnNotes">Notes for Agent (optional)</Label>
                <Textarea
                  id="returnNotes"
                  value={returnNotes}
                  onChange={(e) => setReturnNotes(e.target.value)}
                  placeholder="Any context or instructions for the agent..."
                  rows={3}
                />
              </div>
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setShowReturnDialog(false)}>
                Cancel
              </Button>
              <Button onClick={handleReturnControl} disabled={isReturning}>
                {isReturning ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : null}
                Return Control
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        {/* Inject Context Dialog */}
        <Dialog open={showInjectDialog} onOpenChange={setShowInjectDialog}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Add Note</DialogTitle>
              <DialogDescription>
                Add a note to the execution that the agent will see.
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-2">
              <Label htmlFor="injectContent">Note</Label>
              <Textarea
                id="injectContent"
                value={injectContent}
                onChange={(e) => setInjectContent(e.target.value)}
                placeholder="Your note or instruction..."
                rows={4}
              />
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setShowInjectDialog(false)}>
                Cancel
              </Button>
              <Button
                onClick={handleInjectContext}
                disabled={isInjecting || !injectContent.trim()}
              >
                {isInjecting ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : null}
                <Send className="h-4 w-4 mr-2" />
                Send
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </CardContent>
    </Card>
  );
}
