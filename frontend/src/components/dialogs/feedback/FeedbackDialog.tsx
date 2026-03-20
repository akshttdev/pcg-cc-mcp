import NiceModal, { useModal } from '@ebay/nice-modal-react';
import {
  AlertCircle,
  Bug,
  Frown,
  ImagePlus,
  Lightbulb,
  MessageCircleQuestion,
  Send,
  X,
} from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { toast } from 'sonner';

import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { FormDialogBody } from '@/components/ui/form-dialog-body';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import { resolveApiUrl } from '@/lib/api';

type FeedbackType =
  | 'bug'
  | 'feature'
  | 'improvement'
  | 'question'
  | 'friction'
  | 'other';

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
    value: 'friction' as const,
    label: 'Friction Report',
    icon: Frown,
    description: 'Something felt slow, confusing, or broken',
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
  const [severity, setSeverity] = useState<
    'low' | 'medium' | 'high' | 'critical'
  >('medium');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [screenshot, setScreenshot] = useState<string | null>(null);
  const [screenshotName, setScreenshotName] = useState<string>('');
  const fileInputRef = useRef<HTMLInputElement>(null);
  // Friction fields
  const [frictionPoint, setFrictionPoint] = useState('');
  const [userIntent, setUserIntent] = useState('');
  const [expectedBehavior, setExpectedBehavior] = useState('');
  const [frustrationLevel, setFrustrationLevel] = useState<number>(3);

  // Reset form state when modal opens
  useEffect(() => {
    if (modal.visible) {
      setType('bug');
      setTitle('');
      setDescription('');
      setEmail('');
      setSeverity('medium');
      setIsSubmitting(false);
      setScreenshot(null);
      setScreenshotName('');
      setFrictionPoint('');
      setUserIntent('');
      setExpectedBehavior('');
      setFrustrationLevel(3);
    }
  }, [modal.visible]);

  // Convert file to base64
  const handleFileSelect = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const file = e.target.files?.[0];
      if (!file) return;

      // Validate file type
      if (!file.type.startsWith('image/')) {
        toast.error('Please select an image file');
        return;
      }

      // Validate file size (max 5MB)
      if (file.size > 5 * 1024 * 1024) {
        toast.error('Image must be less than 5MB');
        return;
      }

      setScreenshotName(file.name);

      const reader = new FileReader();
      reader.onload = () => {
        const base64 = reader.result as string;
        setScreenshot(base64);
      };
      reader.readAsDataURL(file);
    },
    []
  );

  const removeScreenshot = useCallback(() => {
    setScreenshot(null);
    setScreenshotName('');
    if (fileInputRef.current) {
      fileInputRef.current.value = '';
    }
  }, []);

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
          screenshot: screenshot || undefined,
          // Friction fields
          page_url: type === 'friction' ? window.location.pathname : undefined,
          user_intent:
            type === 'friction' && userIntent.trim()
              ? userIntent.trim()
              : undefined,
          friction_point:
            type === 'friction' && frictionPoint.trim()
              ? frictionPoint.trim()
              : undefined,
          expected_behavior:
            type === 'friction' && expectedBehavior.trim()
              ? expectedBehavior.trim()
              : undefined,
          frustration_level: type === 'friction' ? frustrationLevel : undefined,
        }),
      });

      if (!response.ok) {
        throw new Error('Failed to submit feedback');
      }

      toast.success('Thank you! Your feedback has been submitted.');
      modal.hide();
    } catch (error) {
      console.error('Failed to submit feedback:', error);
      toast.error('Failed to submit feedback. Please try again.');
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleClose = () => {
    if (!isSubmitting) {
      modal.hide();
    }
  };

  return (
    <Dialog open={modal.visible} onOpenChange={handleClose}>
      <DialogContent className="max-w-2xl max-h-[90vh] flex flex-col overflow-hidden">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Icon className="h-5 w-5" />
            Submit Feedback
          </DialogTitle>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="flex-1 flex flex-col min-h-0">
          <FormDialogBody
            footer={
              <>
                <Button
                  type="button"
                  variant="outline"
                  onClick={handleClose}
                  disabled={isSubmitting}
                >
                  Cancel
                </Button>
                <Button
                  type="submit"
                  disabled={
                    isSubmitting || !title.trim() || !description.trim()
                  }
                >
                  {isSubmitting ? (
                    <>
                      <span className="animate-spin mr-2">&#9203;</span>
                      Submitting...
                    </>
                  ) : (
                    <>
                      <Send className="h-4 w-4 mr-2" />
                      Submit Feedback
                    </>
                  )}
                </Button>
              </>
            }
          >
            <div className="space-y-4">
              {/* Feedback Type */}
              <div>
                <Label className="text-sm font-medium">Feedback Type</Label>
                <Select
                  value={type}
                  onValueChange={(v) => setType(v as FeedbackType)}
                >
                  <SelectTrigger className="mt-1.5">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {FEEDBACK_TYPES.map((feedbackType) => {
                      const TypeIcon = feedbackType.icon;
                      return (
                        <SelectItem
                          key={feedbackType.value}
                          value={feedbackType.value}
                        >
                          <div className="flex items-center gap-2">
                            <TypeIcon className="h-4 w-4" />
                            <div>
                              <div className="font-medium">
                                {feedbackType.label}
                              </div>
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
                  <Select
                    value={severity}
                    onValueChange={(v) =>
                      setSeverity(v as 'low' | 'medium' | 'high' | 'critical')
                    }
                  >
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

              {/* Friction fields */}
              {type === 'friction' && (
                <div className="space-y-3 p-3 rounded-md border border-orange-200 dark:border-orange-800 bg-orange-50/50 dark:bg-orange-950/20">
                  <div>
                    <Label className="text-sm font-medium">
                      What were you trying to do?
                    </Label>
                    <Input
                      value={userIntent}
                      onChange={(e) => setUserIntent(e.target.value)}
                      placeholder="e.g. Move a deal to the next stage"
                      className="mt-1"
                      disabled={isSubmitting}
                    />
                  </div>
                  <div>
                    <Label className="text-sm font-medium">
                      What went wrong?
                    </Label>
                    <Input
                      value={frictionPoint}
                      onChange={(e) => setFrictionPoint(e.target.value)}
                      placeholder="e.g. Button was disabled with no explanation"
                      className="mt-1"
                      disabled={isSubmitting}
                    />
                  </div>
                  <div>
                    <Label className="text-sm font-medium">
                      What did you expect to happen?{' '}
                      <span className="text-muted-foreground font-normal">
                        (optional)
                      </span>
                    </Label>
                    <Input
                      value={expectedBehavior}
                      onChange={(e) => setExpectedBehavior(e.target.value)}
                      placeholder="e.g. The button should show why it's disabled"
                      className="mt-1"
                      disabled={isSubmitting}
                    />
                  </div>
                  <div>
                    <Label className="text-sm font-medium">
                      Frustration level
                    </Label>
                    <div className="flex items-center gap-2 mt-1">
                      {[1, 2, 3, 4, 5].map((level) => (
                        <button
                          key={level}
                          type="button"
                          onClick={() => setFrustrationLevel(level)}
                          className={`w-9 h-9 rounded-full text-sm font-medium transition-colors ${
                            frustrationLevel === level
                              ? level <= 2
                                ? 'bg-blue-500 text-white'
                                : level === 3
                                  ? 'bg-yellow-500 text-white'
                                  : 'bg-red-500 text-white'
                              : 'bg-muted hover:bg-muted/80'
                          }`}
                          disabled={isSubmitting}
                        >
                          {level}
                        </button>
                      ))}
                      <span className="text-xs text-muted-foreground ml-1">
                        {frustrationLevel <= 2
                          ? 'Minor'
                          : frustrationLevel === 3
                            ? 'Moderate'
                            : frustrationLevel === 4
                              ? 'High'
                              : 'Show-stopper'}
                      </span>
                    </div>
                  </div>
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
                        ? "Feature you'd like to see..."
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
                <Label
                  htmlFor="feedback-description"
                  className="text-sm font-medium"
                >
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
                  Email{' '}
                  <span className="text-muted-foreground">(optional)</span>
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

              {/* Screenshot Upload */}
              <div>
                <Label className="text-sm font-medium">
                  Screenshot{' '}
                  <span className="text-muted-foreground">(optional)</span>
                </Label>
                <input
                  ref={fileInputRef}
                  type="file"
                  accept="image/*"
                  onChange={handleFileSelect}
                  className="hidden"
                  disabled={isSubmitting}
                />
                {screenshot ? (
                  <div className="mt-1.5 relative">
                    <img
                      src={screenshot}
                      alt="Screenshot preview"
                      className="max-h-48 rounded-md border object-contain w-full bg-muted"
                    />
                    <Button
                      type="button"
                      variant="destructive"
                      size="sm"
                      className="absolute top-2 right-2 h-6 w-6 p-0"
                      onClick={removeScreenshot}
                      disabled={isSubmitting}
                      aria-label="Remove screenshot"
                    >
                      <X className="h-3 w-3" />
                    </Button>
                    <p className="text-xs text-muted-foreground mt-1">
                      {screenshotName}
                    </p>
                  </div>
                ) : (
                  <Button
                    type="button"
                    variant="outline"
                    className="mt-1.5 w-full"
                    onClick={() => fileInputRef.current?.click()}
                    disabled={isSubmitting}
                  >
                    <ImagePlus className="h-4 w-4 mr-2" />
                    Attach Screenshot
                  </Button>
                )}
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
                        <li>Attach a screenshot to help us understand</li>
                      </>
                    )}
                    {type === 'feature' && (
                      <>
                        <li>Explain the problem you're trying to solve</li>
                        <li>Describe how you'd use this feature</li>
                        <li>Share any examples from other tools</li>
                      </>
                    )}
                    {type === 'friction' && (
                      <>
                        <li>Describe what felt slow, confusing, or broken</li>
                        <li>Include the page URL (auto-captured)</li>
                        <li>Rate your frustration so we can prioritize</li>
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
            </div>
          </FormDialogBody>
        </form>
      </DialogContent>
    </Dialog>
  );
});
