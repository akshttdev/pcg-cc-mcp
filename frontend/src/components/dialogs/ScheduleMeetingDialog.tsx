import { useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import {
  Dialog,
  DialogContent,
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
import { X, Calendar, Clock } from 'lucide-react';
import { toast } from 'sonner';
import { proposalsApi, type PersonRecord } from '@/lib/api';
import { businessKeys } from '@/lib/query-keys';

interface InviteeEntry {
  person: PersonRecord;
  channel: string;
  address: string;
}

interface Props {
  open: boolean;
  onClose: () => void;
  proposalId: string;
  /** Pre-populated invitees (e.g. lead contact + org contacts) */
  defaultInvitees?: PersonRecord[];
}

const CHANNEL_OPTIONS = [
  { value: 'email',     label: 'Email' },
  { value: 'sms',       label: 'SMS' },
  { value: 'whatsapp',  label: 'WhatsApp' },
  { value: 'instagram', label: 'Instagram' },
  { value: 'linkedin',  label: 'LinkedIn' },
  { value: 'twitter',   label: 'Twitter / X' },
  { value: 'phone',     label: 'Phone call' },
  { value: 'in_person', label: 'In person' },
];

function defaultChannelFor(person: PersonRecord): string {
  return person.preferred_contact ?? person.onboarding_channel ?? 'email';
}

function defaultAddressFor(person: PersonRecord, channel: string): string {
  if (channel === 'email') return person.email ?? '';
  if (channel === 'sms' || channel === 'phone') return person.phone ?? '';
  return '';
}

export function ScheduleMeetingDialog({ open, onClose, proposalId, defaultInvitees = [] }: Props) {
  const queryClient = useQueryClient();
  const [saving, setSaving] = useState(false);

  const [scheduledAt, setScheduledAt] = useState('');
  const [duration,    setDuration]    = useState('60');
  const [location,    setLocation]    = useState('');
  const [agenda,      setAgenda]      = useState('');
  const [channel,     setChannel]     = useState('email');

  const [invitees, setInvitees] = useState<InviteeEntry[]>(() =>
    defaultInvitees.map((p) => {
      const ch = defaultChannelFor(p);
      return { person: p, channel: ch, address: defaultAddressFor(p, ch) };
    })
  );

  function updateInviteeChannel(personId: string, ch: string) {
    setInvitees((prev) =>
      prev.map((inv) =>
        inv.person.id === personId
          ? { ...inv, channel: ch, address: defaultAddressFor(inv.person, ch) }
          : inv
      )
    );
  }

  function updateInviteeAddress(personId: string, addr: string) {
    setInvitees((prev) =>
      prev.map((inv) =>
        inv.person.id === personId ? { ...inv, address: addr } : inv
      )
    );
  }

  function removeInvitee(personId: string) {
    setInvitees((prev) => prev.filter((inv) => inv.person.id !== personId));
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!scheduledAt) { toast.error('Meeting date/time is required'); return; }

    setSaving(true);
    try {
      const result = await proposalsApi.scheduleMeeting(proposalId, {
        scheduled_at: new Date(scheduledAt).toISOString(),
        duration_min: parseInt(duration, 10) || 60,
        location: location.trim() || undefined,
        agenda: agenda.trim() || undefined,
        channel,
        invitees: invitees.map((inv) => ({
          person_id: inv.person.id,
          channel: inv.channel,
          channel_address: inv.address,
        })),
      });

      const sent = result.dispatched.filter((d) => d.status === 'sent').length;
      const failed = result.dispatched.filter((d) => d.status === 'failed').length;

      if (failed > 0) {
        toast.warning(`Meeting scheduled — ${sent} invite${sent !== 1 ? 's' : ''} sent, ${failed} failed`);
      } else {
        toast.success(`Meeting scheduled — ${sent} invite${sent !== 1 ? 's' : ''} sent`);
      }

      queryClient.invalidateQueries({ queryKey: businessKeys.proposals() });
      queryClient.invalidateQueries({ queryKey: businessKeys.meetings(proposalId) });
      handleClose();
    } catch {
      toast.error('Failed to schedule meeting');
    } finally {
      setSaving(false);
    }
  }

  function handleClose() {
    setScheduledAt('');
    setDuration('60');
    setLocation('');
    setAgenda('');
    setChannel('email');
    setInvitees(
      defaultInvitees.map((p) => {
        const ch = defaultChannelFor(p);
        return { person: p, channel: ch, address: defaultAddressFor(p, ch) };
      })
    );
    onClose();
  }

  return (
    <Dialog open={open} onOpenChange={(v) => !v && handleClose()}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Calendar className="h-4 w-4" />
            Schedule Meeting
          </DialogTitle>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="space-y-4 pt-2">
          {/* Date/time + duration */}
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1">
              <Label htmlFor="meeting-dt">Date & time *</Label>
              <Input
                id="meeting-dt"
                type="datetime-local"
                value={scheduledAt}
                onChange={(e) => setScheduledAt(e.target.value)}
                required
              />
            </div>
            <div className="space-y-1">
              <Label htmlFor="meeting-dur" className="flex items-center gap-1">
                <Clock className="h-3 w-3" />
                Duration (min)
              </Label>
              <Input
                id="meeting-dur"
                type="number"
                min={15}
                step={15}
                value={duration}
                onChange={(e) => setDuration(e.target.value)}
              />
            </div>
          </div>

          {/* Primary invite channel */}
          <div className="space-y-1">
            <Label>Primary channel</Label>
            <Select value={channel} onValueChange={setChannel}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {CHANNEL_OPTIONS.map((c) => (
                  <SelectItem key={c.value} value={c.value}>{c.label}</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* Location */}
          <div className="space-y-1">
            <Label htmlFor="meeting-loc">Location / link</Label>
            <Input
              id="meeting-loc"
              placeholder="Zoom link, office address, Google Meet…"
              value={location}
              onChange={(e) => setLocation(e.target.value)}
            />
          </div>

          {/* Agenda */}
          <div className="space-y-1">
            <Label htmlFor="meeting-agenda">Agenda</Label>
            <Textarea
              id="meeting-agenda"
              rows={2}
              placeholder="Topics to cover…"
              value={agenda}
              onChange={(e) => setAgenda(e.target.value)}
            />
          </div>

          {/* Invitees */}
          {invitees.length > 0 && (
            <div className="space-y-2">
              <Label>Invitees</Label>
              <div className="space-y-2 max-h-48 overflow-y-auto pr-1">
                {invitees.map((inv) => (
                  <div key={inv.person.id} className="flex items-center gap-2 p-2 border rounded-md bg-muted/30">
                    <div className="flex-1 min-w-0">
                      <p className="text-sm font-medium truncate">{inv.person.full_name}</p>
                      <div className="flex items-center gap-2 mt-1">
                        <Select
                          value={inv.channel}
                          onValueChange={(ch) => updateInviteeChannel(inv.person.id, ch)}
                        >
                          <SelectTrigger className="h-6 text-xs w-28">
                            <SelectValue />
                          </SelectTrigger>
                          <SelectContent>
                            {CHANNEL_OPTIONS.map((c) => (
                              <SelectItem key={c.value} value={c.value} className="text-xs">
                                {c.label}
                              </SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                        <Input
                          className="h-6 text-xs flex-1"
                          placeholder="address / handle"
                          value={inv.address}
                          onChange={(e) => updateInviteeAddress(inv.person.id, e.target.value)}
                        />
                      </div>
                    </div>
                    <button
                      type="button"
                      onClick={() => removeInvitee(inv.person.id)}
                      className="text-muted-foreground hover:text-destructive shrink-0"
                    >
                      <X className="h-4 w-4" />
                    </button>
                  </div>
                ))}
              </div>
              <p className="text-xs text-muted-foreground">
                Invites will be dispatched via each person's selected channel.
              </p>
            </div>
          )}

          {invitees.length === 0 && (
            <div className="py-2 px-3 rounded-md border border-dashed text-xs text-muted-foreground">
              No invitees — add contacts to the proposal first, then open this dialog from their profile.
            </div>
          )}

          <DialogFooter className="pt-1">
            <Button type="button" variant="ghost" onClick={handleClose} disabled={saving}>
              Cancel
            </Button>
            <Button type="submit" disabled={saving}>
              {saving ? 'Scheduling…' : 'Send Invitations'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
