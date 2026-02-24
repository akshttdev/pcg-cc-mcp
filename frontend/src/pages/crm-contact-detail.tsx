import { useMemo } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import {
  ArrowLeft,
  Mail,
  Phone,
  Building2,
  Globe,
  Linkedin,
  Twitter,
  Star,
  DollarSign,
} from 'lucide-react';
import { crmApi, crmDealsApi } from '@/lib/api';
import type { CrmContactRecord, CrmDealRecord } from '@/lib/api';
import { LIFECYCLE_STAGE_INFO } from '@/types/crm';
import type { LifecycleStage } from '@/types/crm';
import { CrmActivityTimeline } from '@/components/crm/CrmActivityTimeline';
import { formatDistanceToNow } from 'date-fns';

export function CrmContactDetailPage() {
  const { projectId, contactId } = useParams<{
    projectId: string;
    contactId: string;
  }>();
  const navigate = useNavigate();

  const { data: contact, isLoading: contactLoading } = useQuery<CrmContactRecord>({
    queryKey: ['crm', 'contact', contactId],
    queryFn: () => crmApi.getContact(contactId!),
    enabled: !!contactId,
  });

  const { data: deals = [] } = useQuery<CrmDealRecord[]>({
    queryKey: ['crm', 'deals', 'contact', contactId],
    queryFn: () => crmDealsApi.listDeals({ contact_id: contactId }),
    enabled: !!contactId,
  });

  const totalRevenue = useMemo(() => {
    return deals.reduce((sum, d) => sum + (d.amount || 0), 0);
  }, [deals]);

  if (!projectId || !contactId) {
    return (
      <div className="h-full flex items-center justify-center text-muted-foreground">
        Contact not found
      </div>
    );
  }

  if (contactLoading) {
    return (
      <div className="p-6 space-y-6">
        <Skeleton className="h-8 w-48" />
        <div className="grid gap-6 md:grid-cols-3">
          <div className="md:col-span-2 space-y-4">
            <Skeleton className="h-40" />
            <Skeleton className="h-60" />
          </div>
          <Skeleton className="h-80" />
        </div>
      </div>
    );
  }

  if (!contact) {
    return (
      <div className="h-full flex items-center justify-center text-muted-foreground">
        Contact not found
      </div>
    );
  }

  const stageInfo = LIFECYCLE_STAGE_INFO[contact.lifecycle_stage as LifecycleStage] ?? {
    label: contact.lifecycle_stage,
    color: '#6B7280',
  };

  const formatCurrency = (amount: number) =>
    new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: 0,
    }).format(amount);

  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <div className="flex items-center gap-4">
        <Button
          variant="ghost"
          size="icon"
          onClick={() => navigate(`/projects/${projectId}/crm`)}
        >
          <ArrowLeft className="h-4 w-4" />
        </Button>
        <div className="flex items-center gap-4 flex-1">
          <div className="w-14 h-14 rounded-full bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center text-white text-xl font-semibold">
            {contact.avatar_url ? (
              <img
                src={contact.avatar_url}
                alt={contact.full_name || 'Contact'}
                className="w-full h-full rounded-full object-cover"
              />
            ) : (
              (
                contact.first_name?.[0] ||
                contact.email?.[0] ||
                'C'
              ).toUpperCase()
            )}
          </div>
          <div>
            <h1 className="text-2xl font-semibold">
              {contact.full_name || contact.email || 'Unnamed Contact'}
            </h1>
            <div className="flex items-center gap-2 mt-1">
              <Badge
                variant="outline"
                style={{ borderColor: stageInfo.color, color: stageInfo.color }}
              >
                {stageInfo.label}
              </Badge>
              {contact.lead_score > 0 && (
                <Badge className="bg-yellow-100 text-yellow-700">
                  <Star className="h-3 w-3 mr-1" />
                  Score: {contact.lead_score}
                </Badge>
              )}
              {contact.company_name && (
                <span className="text-sm text-muted-foreground flex items-center gap-1">
                  <Building2 className="h-3 w-3" />
                  {contact.company_name}
                </span>
              )}
            </div>
          </div>
        </div>
      </div>

      <div className="grid gap-6 md:grid-cols-3">
        {/* Main Content */}
        <div className="md:col-span-2 space-y-6">
          {/* Contact Info Card */}
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Contact Information</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <div className="grid gap-3 md:grid-cols-2">
                {contact.email && (
                  <div className="flex items-center gap-2 text-sm">
                    <Mail className="h-4 w-4 text-muted-foreground" />
                    <a
                      href={`mailto:${contact.email}`}
                      className="text-blue-600 hover:underline"
                    >
                      {contact.email}
                    </a>
                  </div>
                )}
                {contact.phone && (
                  <div className="flex items-center gap-2 text-sm">
                    <Phone className="h-4 w-4 text-muted-foreground" />
                    {contact.phone}
                  </div>
                )}
                {contact.job_title && (
                  <div className="flex items-center gap-2 text-sm">
                    <Building2 className="h-4 w-4 text-muted-foreground" />
                    {contact.job_title}
                    {contact.company_name && ` at ${contact.company_name}`}
                  </div>
                )}
                {contact.website && (
                  <div className="flex items-center gap-2 text-sm">
                    <Globe className="h-4 w-4 text-muted-foreground" />
                    <a
                      href={contact.website}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-blue-600 hover:underline truncate"
                    >
                      {contact.website}
                    </a>
                  </div>
                )}
                {contact.linkedin_url && (
                  <div className="flex items-center gap-2 text-sm">
                    <Linkedin className="h-4 w-4 text-muted-foreground" />
                    <a
                      href={contact.linkedin_url}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-blue-600 hover:underline truncate"
                    >
                      LinkedIn
                    </a>
                  </div>
                )}
                {contact.twitter_handle && (
                  <div className="flex items-center gap-2 text-sm">
                    <Twitter className="h-4 w-4 text-muted-foreground" />
                    <a
                      href={`https://twitter.com/${contact.twitter_handle}`}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-blue-600 hover:underline"
                    >
                      @{contact.twitter_handle}
                    </a>
                  </div>
                )}
              </div>
              <div className="flex items-center gap-4 pt-2 border-t text-xs text-muted-foreground">
                <span>
                  Last activity:{' '}
                  {contact.last_activity_at
                    ? formatDistanceToNow(new Date(contact.last_activity_at), {
                        addSuffix: true,
                      })
                    : 'Never'}
                </span>
                <span>{contact.email_count} emails</span>
                <span>{contact.meeting_count} meetings</span>
                <span>{contact.deal_count} deals</span>
              </div>
            </CardContent>
          </Card>

          {/* Linked Deals */}
          {deals.length > 0 && (
            <Card>
              <CardHeader>
                <div className="flex items-center justify-between">
                  <CardTitle className="text-base">Linked Deals</CardTitle>
                  <span className="text-sm text-muted-foreground">
                    Total: {formatCurrency(totalRevenue)}
                  </span>
                </div>
              </CardHeader>
              <CardContent>
                <div className="space-y-2">
                  {deals.map((deal) => (
                    <div
                      key={deal.id}
                      className="flex items-center justify-between p-3 border rounded-lg"
                    >
                      <div>
                        <p className="text-sm font-medium">{deal.name}</p>
                        <p className="text-xs text-muted-foreground">
                          {deal.stage} &middot; {deal.probability}% probability
                        </p>
                      </div>
                      {deal.amount && (
                        <span className="text-sm font-semibold text-green-600 flex items-center gap-1">
                          <DollarSign className="h-3 w-3" />
                          {formatCurrency(deal.amount)}
                        </span>
                      )}
                    </div>
                  ))}
                </div>
              </CardContent>
            </Card>
          )}

          {/* Activity Timeline */}
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Activity</CardTitle>
            </CardHeader>
            <CardContent>
              <CrmActivityTimeline
                projectId={projectId}
                contactId={contactId}
              />
            </CardContent>
          </Card>
        </div>

        {/* Sidebar Stats */}
        <div className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Quick Stats</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex justify-between">
                <span className="text-sm text-muted-foreground">Lead Score</span>
                <span className="text-sm font-medium">{contact.lead_score}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-sm text-muted-foreground">Total Revenue</span>
                <span className="text-sm font-medium text-green-600">
                  {formatCurrency(contact.total_revenue || 0)}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-sm text-muted-foreground">Deals</span>
                <span className="text-sm font-medium">{contact.deal_count}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-sm text-muted-foreground">Emails</span>
                <span className="text-sm font-medium">{contact.email_count}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-sm text-muted-foreground">Meetings</span>
                <span className="text-sm font-medium">{contact.meeting_count}</span>
              </div>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}
