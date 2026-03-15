import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  Building2,
  MapPin,
  Users,
  Briefcase,
  Globe,
  Mail,
  Pencil,
  Trash2,
  Linkedin,
  Phone,
  Activity,
} from 'lucide-react';
import { LIFECYCLE_STAGE_INFO, type LifecycleStage } from '@/types/crm';
import {
  crmDealsApi,
  crmActivitiesApi,
  crmApi,
  type CrmContactRecord,
  type CrmActivityRecord,
} from '@/lib/api';

export function ContactDetailModal({
  contact,
  orgId,
  open,
  onClose,
}: {
  contact: CrmContactRecord;
  orgId: string;
  open: boolean;
  onClose: () => void;
}) {
  const queryClient = useQueryClient();
  const [isEditing, setIsEditing] = useState(false);
  const [editData, setEditData] = useState({
    first_name: contact.first_name ?? '',
    last_name: contact.last_name ?? '',
    email: contact.email ?? '',
    phone: contact.phone ?? '',
    company_name: contact.company_name ?? '',
    job_title: contact.job_title ?? '',
    department: contact.department ?? '',
    linkedin_url: contact.linkedin_url ?? '',
    website: contact.website ?? '',
  });

  const stageInfo = LIFECYCLE_STAGE_INFO[contact.lifecycle_stage as LifecycleStage];

  const { data: deals = [] } = useQuery({
    queryKey: ['contact-deals', contact.id],
    queryFn: () => crmDealsApi.listDeals({ contact_id: contact.id }),
    enabled: open,
    staleTime: 30_000,
  });

  const { data: activities = [] } = useQuery<CrmActivityRecord[]>({
    queryKey: ['contact-activities', contact.id],
    queryFn: () => crmActivitiesApi.listActivities({ organization_id: orgId, contact_id: contact.id }),
    enabled: open,
    staleTime: 30_000,
  });

  const updateMutation = useMutation({
    mutationFn: () => crmApi.updateContact(contact.id, editData),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['crm-contacts'] });
      setIsEditing(false);
      toast.success('Contact updated');
    },
    onError: () => toast.error('Failed to update contact'),
  });

  const deleteMutation = useMutation({
    mutationFn: () => crmApi.deleteContact(contact.id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['crm-contacts'] });
      onClose();
      toast.success('Contact deleted');
    },
    onError: () => toast.error('Failed to delete contact'),
  });

  return (
    <Dialog open={open} onOpenChange={(v) => !v && onClose()}>
      <DialogContent className="max-w-2xl max-h-[85vh] overflow-y-auto">
        <DialogHeader>
          <div className="flex items-center justify-between">
            <DialogTitle className="text-lg">
              {contact.full_name || `${contact.first_name ?? ''} ${contact.last_name ?? ''}`.trim() || 'Unnamed Contact'}
            </DialogTitle>
            <div className="flex items-center gap-2">
              <Button size="sm" variant="ghost" onClick={() => setIsEditing(!isEditing)}>
                <Pencil className="h-3.5 w-3.5" />
              </Button>
              <Button size="sm" variant="ghost" className="text-destructive" onClick={() => {
                if (confirm('Delete this contact?')) deleteMutation.mutate();
              }}>
                <Trash2 className="h-3.5 w-3.5" />
              </Button>
            </div>
          </div>
        </DialogHeader>

        <div className="space-y-4">
          {isEditing ? (
            <div className="grid grid-cols-2 gap-3">
              <div>
                <Label className="text-xs">First Name</Label>
                <Input value={editData.first_name} onChange={(e) => setEditData(d => ({ ...d, first_name: e.target.value }))} />
              </div>
              <div>
                <Label className="text-xs">Last Name</Label>
                <Input value={editData.last_name} onChange={(e) => setEditData(d => ({ ...d, last_name: e.target.value }))} />
              </div>
              <div>
                <Label className="text-xs">Email</Label>
                <Input value={editData.email} onChange={(e) => setEditData(d => ({ ...d, email: e.target.value }))} />
              </div>
              <div>
                <Label className="text-xs">Phone</Label>
                <Input value={editData.phone} onChange={(e) => setEditData(d => ({ ...d, phone: e.target.value }))} />
              </div>
              <div>
                <Label className="text-xs">Company</Label>
                <Input value={editData.company_name} onChange={(e) => setEditData(d => ({ ...d, company_name: e.target.value }))} />
              </div>
              <div>
                <Label className="text-xs">Job Title</Label>
                <Input value={editData.job_title} onChange={(e) => setEditData(d => ({ ...d, job_title: e.target.value }))} />
              </div>
              <div>
                <Label className="text-xs">Department</Label>
                <Input value={editData.department} onChange={(e) => setEditData(d => ({ ...d, department: e.target.value }))} />
              </div>
              <div>
                <Label className="text-xs">LinkedIn URL</Label>
                <Input value={editData.linkedin_url} onChange={(e) => setEditData(d => ({ ...d, linkedin_url: e.target.value }))} />
              </div>
              <div className="col-span-2 flex gap-2 justify-end">
                <Button size="sm" variant="outline" onClick={() => setIsEditing(false)}>Cancel</Button>
                <Button size="sm" onClick={() => updateMutation.mutate()} disabled={updateMutation.isPending}>
                  {updateMutation.isPending ? 'Saving...' : 'Save'}
                </Button>
              </div>
            </div>
          ) : (
            <div className="space-y-3">
              <div className="flex items-center gap-3 flex-wrap">
                {stageInfo && (
                  <Badge variant="secondary" style={{ backgroundColor: stageInfo.color + '20', color: stageInfo.color }}>
                    {stageInfo.label}
                  </Badge>
                )}
                {contact.lead_score > 0 && (
                  <Badge variant="outline">Score: {contact.lead_score}</Badge>
                )}
                {contact.source && (
                  <Badge variant="outline" className="capitalize">{contact.source}</Badge>
                )}
              </div>

              <div className="grid grid-cols-2 gap-2 text-sm">
                {contact.email && (
                  <div className="flex items-center gap-2 text-muted-foreground">
                    <Mail className="h-3.5 w-3.5 shrink-0" />
                    <a href={`mailto:${contact.email}`} className="truncate hover:text-foreground">{contact.email}</a>
                  </div>
                )}
                {contact.phone && (
                  <div className="flex items-center gap-2 text-muted-foreground">
                    <Phone className="h-3.5 w-3.5 shrink-0" />
                    <span className="truncate">{contact.phone}</span>
                  </div>
                )}
                {contact.company_name && (
                  <div className="flex items-center gap-2 text-muted-foreground">
                    <Building2 className="h-3.5 w-3.5 shrink-0" />
                    <span className="truncate">{contact.company_name}</span>
                  </div>
                )}
                {contact.job_title && (
                  <div className="flex items-center gap-2 text-muted-foreground">
                    <Briefcase className="h-3.5 w-3.5 shrink-0" />
                    <span className="truncate">{contact.job_title}</span>
                  </div>
                )}
                {contact.department && (
                  <div className="flex items-center gap-2 text-muted-foreground">
                    <Users className="h-3.5 w-3.5 shrink-0" />
                    <span className="truncate">{contact.department}</span>
                  </div>
                )}
                {contact.linkedin_url && (
                  <div className="flex items-center gap-2 text-muted-foreground">
                    <Linkedin className="h-3.5 w-3.5 shrink-0" />
                    <a href={contact.linkedin_url} target="_blank" rel="noreferrer" className="truncate hover:text-foreground">LinkedIn</a>
                  </div>
                )}
                {(contact.city || contact.country) && (
                  <div className="flex items-center gap-2 text-muted-foreground">
                    <MapPin className="h-3.5 w-3.5 shrink-0" />
                    <span className="truncate">{[contact.city, contact.state, contact.country].filter(Boolean).join(', ')}</span>
                  </div>
                )}
                {contact.website && (
                  <div className="flex items-center gap-2 text-muted-foreground">
                    <Globe className="h-3.5 w-3.5 shrink-0" />
                    <a href={contact.website} target="_blank" rel="noreferrer" className="truncate hover:text-foreground">{contact.website}</a>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Deals */}
          {deals.length > 0 && (
            <div>
              <h4 className="text-xs font-medium text-muted-foreground mb-2 uppercase tracking-wider">Deals ({deals.length})</h4>
              <div className="space-y-1.5">
                {deals.map((deal: { id: string; name: string; amount?: number | null; currency?: string; stage?: string }) => (
                  <div key={deal.id} className="flex items-center justify-between p-2 rounded-lg bg-muted/30 text-sm">
                    <span className="truncate">{deal.name}</span>
                    <div className="flex items-center gap-2 shrink-0">
                      {deal.amount != null && (
                        <span className="text-xs font-medium">
                          {new Intl.NumberFormat('en-US', { style: 'currency', currency: deal.currency || 'USD' }).format(deal.amount)}
                        </span>
                      )}
                      {deal.stage && (
                        <Badge variant="outline" className="text-[10px]">{deal.stage}</Badge>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Recent Activity */}
          {activities.length > 0 && (
            <div>
              <h4 className="text-xs font-medium text-muted-foreground mb-2 uppercase tracking-wider">Recent Activity</h4>
              <div className="space-y-1.5 max-h-40 overflow-y-auto">
                {activities.slice(0, 10).map((activity) => (
                  <div key={activity.id} className="flex items-center justify-between p-2 rounded-lg bg-muted/30 text-sm">
                    <div className="flex items-center gap-2 min-w-0">
                      <Activity className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                      <span className="truncate">{activity.subject || activity.activity_type}</span>
                    </div>
                    <span className="text-[10px] text-muted-foreground shrink-0">
                      {new Date(activity.activity_at).toLocaleDateString()}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Metadata */}
          <div className="text-[10px] text-muted-foreground pt-2 border-t border-border/50 flex items-center justify-between">
            <span>Created {new Date(contact.created_at).toLocaleDateString()}</span>
            {contact.last_activity_at && (
              <span>Last active {new Date(contact.last_activity_at).toLocaleDateString()}</span>
            )}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
