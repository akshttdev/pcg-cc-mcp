import { useState } from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
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
import { Loader2 } from 'lucide-react';
import { useCreateActivity } from '@/hooks/useCrmActivities';
import { toast } from 'sonner';

interface CrmActivityFormProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  projectId: string;
  contactId?: string;
  dealId?: string;
}

const ACTIVITY_TYPES = [
  { value: 'note_added', label: 'Note' },
  { value: 'call_made', label: 'Call Made' },
  { value: 'call_received', label: 'Call Received' },
  { value: 'email_sent', label: 'Email Sent' },
  { value: 'email_received', label: 'Email Received' },
  { value: 'meeting_scheduled', label: 'Meeting Scheduled' },
  { value: 'meeting_completed', label: 'Meeting Completed' },
  { value: 'task_created', label: 'Task Created' },
  { value: 'custom', label: 'Custom' },
];

export function CrmActivityForm({
  open,
  onOpenChange,
  projectId,
  contactId,
  dealId,
}: CrmActivityFormProps) {
  const createActivity = useCreateActivity();
  const [formData, setFormData] = useState({
    activity_type: 'note_added',
    subject: '',
    description: '',
    outcome: '',
    duration_minutes: '',
  });

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      await createActivity.mutateAsync({
        organization_id: projectId,
        crm_contact_id: contactId,
        crm_deal_id: dealId,
        activity_type: formData.activity_type,
        subject: formData.subject || undefined,
        description: formData.description || undefined,
        outcome: formData.outcome || undefined,
        duration_minutes: formData.duration_minutes
          ? parseInt(formData.duration_minutes)
          : undefined,
      });
      toast.success('Activity logged successfully.');
      onOpenChange(false);
      setFormData({
        activity_type: 'note_added',
        subject: '',
        description: '',
        outcome: '',
        duration_minutes: '',
      });
    } catch {
      toast.error('Failed to log activity.');
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[450px]">
        <DialogHeader>
          <DialogTitle>Log Activity</DialogTitle>
          <DialogDescription>
            Record an interaction or note for this contact.
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="activity_type">Type</Label>
            <Select
              value={formData.activity_type}
              onValueChange={(value) =>
                setFormData({ ...formData, activity_type: value })
              }
            >
              <SelectTrigger id="activity_type">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {ACTIVITY_TYPES.map((type) => (
                  <SelectItem key={type.value} value={type.value}>
                    {type.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-2">
            <Label htmlFor="subject">Subject</Label>
            <Input
              id="subject"
              value={formData.subject}
              onChange={(e) =>
                setFormData({ ...formData, subject: e.target.value })
              }
              placeholder="Brief summary of this activity"
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="description">Description</Label>
            <Textarea
              id="description"
              value={formData.description}
              onChange={(e) =>
                setFormData({ ...formData, description: e.target.value })
              }
              placeholder="Details..."
              rows={3}
            />
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="outcome">Outcome</Label>
              <Input
                id="outcome"
                value={formData.outcome}
                onChange={(e) =>
                  setFormData({ ...formData, outcome: e.target.value })
                }
                placeholder="e.g., Follow up next week"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="duration">Duration (min)</Label>
              <Input
                id="duration"
                type="number"
                value={formData.duration_minutes}
                onChange={(e) =>
                  setFormData({ ...formData, duration_minutes: e.target.value })
                }
                placeholder="0"
                min="0"
              />
            </div>
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
            >
              Cancel
            </Button>
            <Button type="submit" disabled={createActivity.isPending}>
              {createActivity.isPending && (
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
              )}
              Log Activity
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
