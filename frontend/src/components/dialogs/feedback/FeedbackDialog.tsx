import { useState, useEffect } from 'react';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { MessageCircleQuestion, Bug, Lightbulb, AlertCircle, Send } from 'lucide-react';
import NiceModal, { useModal } from '@ebay/nice-modal-react';
import { resolveApiUrl } from '@/lib/api';

type FeedbackType = 'bug' | 'feature' | 'improvement' | 'question' | 'other';


const FEEDBACK_TYPES = [
  {
    value: 'bug' as const,
    label: 'Bug Report',
    icon: Bug,
    description: 'Report a problem or error',
  },
  {
    value: 'feature' as const,
    label: 'Feature Request',
    icon: Lightbulb,
    description: 'Suggest a new feature',
  },
  {
    value: 'improvement' as const,
    label: 'Improvement',
    icon: AlertCircle,
    description: 'Suggest an enhancement',
  },
  {
    value: 'question' as const,
    label: 'Question',
    icon: MessageCircleQuestion,
    description: 'Ask a question',
  },
  {
    value: 'other' as const,
    label: 'Other',
    icon: MessageCircleQuestion,
    description: 'General feedback',
  },
];

export const FeedbackDialog = NiceModal.create(() => {
  const modal = useModal();
  const [type, setType] = useState<FeedbackType>('bug');
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [email, setEmail] = useState('');
  const [severity, setSeverity] = useState<'low' | 'medium' | 'high' | 'critical'>('medium');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [submitted, setSubmitted] = useState(false);

  // Reset form state when modal opens
  useEffect(() => {
    if (modal.visible) {
      setType('bug');
      setTitle('');
      setDescription('');
      setEmail('');
      setSeverity('medium');
      setIsSubmitting(false);
      setSubmitted(false);
    }
  }, [modal.visible]);

  const selectedType = FEEDBACK_TYPES.find((t) => t.value === type);
  const Icon = selectedType?.icon || MessageCircleQuestion;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!title.trim() || !description.trim()) {
      return;
    }

    setIsSubmitting(true);

    try {
      const response = await fetch(resolveApiUrl('/api/feedback'), {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        credentials: 'include',
        body: JSON.stringify({
          feedback_type: type,
          title: title.trim(),
          description: description.trim(),
          email: email.trim() || undefined,
          severity: type === 'bug' ? severity : undefined,
        }),
      });

      if (!response.ok) {
        throw new Error('Failed to submit feedback');
      }

      setSubmitted(true);

      // Auto-close after showing success message
      setTimeout(() => {
        modal.hide();
      }, 2000);
    } catch (error) {
      console.error('Failed to submit feedback:', error);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleClose = () => {
    if (!isSubmitting) {
      modal.hide();
    }
  };

  if (submitted) {
    return (
      <Dialog open={modal.visible} onOpenChange={handleClose}>
        <div className="flex flex-col items-center justify-center py-12">
          <div className="w-16 h-16 rounded-full bg-green-100 dark:bg-green-900 flex items-center justify-center mb-4">
            <Send className="h-8 w-8 text-green-600 dark:text-green-400" />
          </div>
          <DialogTitle className="text-xl font-semibold mb-2">
            Thank You!
          </DialogTitle>
          <p className="text-sm text-muted-foreground text-center max-w-sm">
            Your feedback has been submitted. We appreciate you taking the time to help us improve.
          </p>
        </div>
      </Dialog>
    );
  }

  return (
    <Dialog open={modal.visible} onOpenChange={handleClose}>
      <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Icon className="h-5 w-5" />
            Submit Feedback
          </DialogTitle>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4">
          {/* Feedback Type */}
          <div>
            <Label className="text-sm font-medium">Feedback Type</Label>
            <Select value={type} onValueChange={(v) => setType(v as FeedbackType)}>
              <SelectTrigger className="mt-1.5">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {FEEDBACK_TYPES.map((feedbackType) => {
                  const TypeIcon = feedbackType.icon;
                  return (
                    <SelectItem key={feedbackType.value} value={feedbackType.value}>
                      <div className="flex items-center gap-2">
                        <TypeIcon className="h-4 w-4" />
                        <div>
                          <div className="font-medium">{feedbackType.label}</div>
                          <div className="text-xs text-muted-foreground">
                            {feedbackType.description}
                          </div>
                        </div>
                      </div>
                    </SelectItem>
                  );
                })}
              </SelectContent>
            </Select>
          </div>

          {/* Severity (only for bugs) */}
          {type === 'bug' && (
            <div>
              <Label className="text-sm font-medium">Severity</Label>
              <Select value={severity} onValueChange={(v) => setSeverity(v as any)}>
                <SelectTrigger className="mt-1.5">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="low">
                    <div className="flex items-center gap-2">
                      <div className="w-2 h-2 rounded-full bg-blue-500" />
                      <span>Low - Minor issue</span>
                    </div>
                  </SelectItem>
                  <SelectItem value="medium">
                    <div className="flex items-center gap-2">
                      <div className="w-2 h-2 rounded-full bg-yellow-500" />
                      <span>Medium - Affects some functionality</span>
                    </div>
                  </SelectItem>
                  <SelectItem value="high">
                    <div className="flex items-center gap-2">
                      <div className="w-2 h-2 rounded-full bg-orange-500" />
                      <span>High - Significant impact</span>
                    </div>
                  </SelectItem>
                  <SelectItem value="critical">
                    <div className="flex items-center gap-2">
                      <div className="w-2 h-2 rounded-full bg-red-500" />
                      <span>Critical - Blocks usage</span>
                    </div>
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>
          )}

          {/* Title */}
          <div>
            <Label htmlFor="feedback-title" className="text-sm font-medium">
              Title <span className="text-destructive">*</span>
            </Label>
            <Input
              id="feedback-title"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder={
                type === 'bug'
                  ? 'Brief description of the bug...'
                  : type === 'feature'
                  ? 'Feature you\'d like to see...'
                  : 'Brief summary...'
              }
              className="mt-1.5"
              required
              disabled={isSubmitting}
              maxLength={200}
            />
            <p className="text-xs text-muted-foreground mt-1">
              {title.length}/200 characters
            </p>
          </div>

          {/* Description */}
          <div>
            <Label htmlFor="feedback-description" className="text-sm font-medium">
              Description <span className="text-destructive">*</span>
            </Label>
            <Textarea
              id="feedback-description"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder={
                type === 'bug'
                  ? 'What happened? What were you expecting?\n\nSteps to reproduce:\n1. Go to...\n2. Click on...\n3. See error...'
                  : type === 'feature'
                  ? 'Describe the feature and why it would be useful...'
                  : 'Provide details about your feedback...'
              }
              className="mt-1.5 min-h-[150px] font-mono text-sm"
              required
              disabled={isSubmitting}
            />
          </div>

          {/* Email (optional) */}
          <div>
            <Label htmlFor="feedback-email" className="text-sm font-medium">
              Email <span className="text-muted-foreground">(optional)</span>
            </Label>
            <Input
              id="feedback-email"
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="your@email.com"
              className="mt-1.5"
              disabled={isSubmitting}
            />
            <p className="text-xs text-muted-foreground mt-1">
              Provide your email if you'd like us to follow up with you
            </p>
          </div>

          {/* Helper Info */}
          <div className="bg-muted/50 p-3 rounded-md">
            <p className="text-xs text-muted-foreground">
              <strong>Tips for great feedback:</strong>
              <ul className="list-disc list-inside mt-1 space-y-1">
                {type === 'bug' && (
                  <>
                    <li>Include steps to reproduce the issue</li>
                    <li>Mention your browser and OS</li>
                    <li>Attach screenshots if possible</li>
                  </>
                )}
                {type === 'feature' && (
                  <>
                    <li>Explain the problem you're trying to solve</li>
                    <li>Describe how you'd use this feature</li>
                    <li>Share any examples from other tools</li>
                  </>
                )}
                {(type === 'improvement' || type === 'other') && (
                  <>
                    <li>Be specific about what could be better</li>
                    <li>Explain why this matters to you</li>
                    <li>Share any examples or alternatives</li>
                  </>
                )}
              </ul>
            </p>
          </div>

          {/* Actions */}
          <div className="flex justify-end gap-2 pt-4">
            <Button
              type="button"
              variant="outline"
              onClick={handleClose}
              disabled={isSubmitting}
            >
              Cancel
            </Button>
            <Button type="submit" disabled={isSubmitting || !title.trim() || !description.trim()}>
              {isSubmitting ? (
                <>
                  <span className="animate-spin mr-2">⏳</span>
                  Submitting...
                </>
              ) : (
                <>
                  <Send className="h-4 w-4 mr-2" />
                  Submit Feedback
                </>
              )}
            </Button>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  );
});
