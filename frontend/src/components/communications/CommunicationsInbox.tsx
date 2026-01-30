import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  Phone,
  PhoneIncoming,
  PhoneOutgoing,
  PhoneMissed,
  MessageSquare,
  Star,
  Clock,
  Play,
  ChevronRight,
  RefreshCw,
  User,
} from 'lucide-react';
import {
  communicationsApi,
  CallLogRecord,
  SmsMessageRecord,
  CallStats,
  SmsStats,
} from '@/lib/api';

interface CommunicationsInboxProps {
  projectId: string;
}

export function CommunicationsInbox({ projectId }: CommunicationsInboxProps) {
  const queryClient = useQueryClient();
  const [activeTab, setActiveTab] = useState<'calls' | 'sms'>('calls');
  const [selectedCall, setSelectedCall] = useState<CallLogRecord | null>(null);
  const [selectedSms, setSelectedSms] = useState<SmsMessageRecord | null>(null);

  // Calls queries
  const { data: callStats } = useQuery<CallStats>({
    queryKey: ['call-stats', projectId],
    queryFn: () => communicationsApi.getCallStats(projectId),
    enabled: !!projectId,
  });

  const { data: calls = [], isLoading: callsLoading, refetch: refetchCalls } = useQuery<CallLogRecord[]>({
    queryKey: ['calls', projectId],
    queryFn: () => communicationsApi.listCalls({ project_id: projectId, limit: 50 }),
    enabled: !!projectId,
  });

  // SMS queries
  const { data: smsStats } = useQuery<SmsStats>({
    queryKey: ['sms-stats', projectId],
    queryFn: () => communicationsApi.getSmsStats(projectId),
    enabled: !!projectId,
  });

  const { data: smsMessages = [], isLoading: smsLoading, refetch: refetchSms } = useQuery<SmsMessageRecord[]>({
    queryKey: ['sms', projectId],
    queryFn: () => communicationsApi.listSms({ project_id: projectId, limit: 50 }),
    enabled: !!projectId,
  });

  const markSmsReadMutation = useMutation({
    mutationFn: communicationsApi.markSmsRead,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['sms'] });
      queryClient.invalidateQueries({ queryKey: ['sms-stats'] });
    },
  });

  const toggleSmsStarMutation = useMutation({
    mutationFn: communicationsApi.toggleSmsStar,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['sms'] });
    },
  });

  const handleOpenSms = (sms: SmsMessageRecord) => {
    setSelectedSms(sms);
    if (sms.is_read === 0) {
      markSmsReadMutation.mutate(sms.id);
    }
  };

  const formatDuration = (seconds: number | null): string => {
    if (!seconds) return '0:00';
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  };

  const formatDate = (dateString: string | null): string => {
    if (!dateString) return '';
    const date = new Date(dateString);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffDays === 0) {
      return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    }
    if (diffDays === 1) return 'Yesterday';
    if (diffDays < 7) return date.toLocaleDateString([], { weekday: 'short' });
    return date.toLocaleDateString([], { month: 'short', day: 'numeric' });
  };

  const formatPhoneNumber = (phone: string): string => {
    // Simple formatting - can be enhanced
    if (phone.length === 10) {
      return `(${phone.slice(0, 3)}) ${phone.slice(3, 6)}-${phone.slice(6)}`;
    }
    if (phone.startsWith('+1') && phone.length === 12) {
      return `(${phone.slice(2, 5)}) ${phone.slice(5, 8)}-${phone.slice(8)}`;
    }
    return phone;
  };

  const getCallIcon = (call: CallLogRecord) => {
    if (call.direction === 'inbound') {
      if (call.status === 'completed') {
        return <PhoneIncoming className="h-5 w-5 text-green-600" />;
      }
      return <PhoneMissed className="h-5 w-5 text-red-500" />;
    }
    return <PhoneOutgoing className="h-5 w-5 text-blue-600" />;
  };

  const getCallStatusBadge = (status: string) => {
    const variants: Record<string, { variant: 'default' | 'secondary' | 'destructive' | 'outline'; label: string }> = {
      'completed': { variant: 'default', label: 'Completed' },
      'in-progress': { variant: 'secondary', label: 'In Progress' },
      'ringing': { variant: 'secondary', label: 'Ringing' },
      'busy': { variant: 'outline', label: 'Busy' },
      'no-answer': { variant: 'destructive', label: 'No Answer' },
      'failed': { variant: 'destructive', label: 'Failed' },
      'canceled': { variant: 'outline', label: 'Canceled' },
    };
    const config = variants[status] || { variant: 'outline' as const, label: status };
    return <Badge variant={config.variant}>{config.label}</Badge>;
  };

  return (
    <div className="space-y-6">
      {/* Stats Overview */}
      <div className="grid gap-4 md:grid-cols-4">
        <Card>
          <CardContent className="pt-6">
            <div className="flex items-center gap-3">
              <div className="p-2 bg-blue-100 rounded-lg">
                <Phone className="h-5 w-5 text-blue-600" />
              </div>
              <div>
                <p className="text-sm text-muted-foreground">Total Calls</p>
                <p className="text-2xl font-bold">{callStats?.total || 0}</p>
              </div>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <div className="flex items-center gap-3">
              <div className="p-2 bg-green-100 rounded-lg">
                <PhoneIncoming className="h-5 w-5 text-green-600" />
              </div>
              <div>
                <p className="text-sm text-muted-foreground">Inbound</p>
                <p className="text-2xl font-bold">{callStats?.inbound || 0}</p>
              </div>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <div className="flex items-center gap-3">
              <div className="p-2 bg-purple-100 rounded-lg">
                <MessageSquare className="h-5 w-5 text-purple-600" />
              </div>
              <div>
                <p className="text-sm text-muted-foreground">SMS Messages</p>
                <p className="text-2xl font-bold">{smsStats?.total || 0}</p>
              </div>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <div className="flex items-center gap-3">
              <div className="p-2 bg-red-100 rounded-lg">
                <Clock className="h-5 w-5 text-red-600" />
              </div>
              <div>
                <p className="text-sm text-muted-foreground">Unread SMS</p>
                <p className="text-2xl font-bold">{smsStats?.unread || 0}</p>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Tabs for Calls and SMS */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>Communications</CardTitle>
              <CardDescription>Phone calls and text messages</CardDescription>
            </div>
            <Button
              variant="outline"
              size="icon"
              onClick={() => {
                refetchCalls();
                refetchSms();
              }}
            >
              <RefreshCw className="h-4 w-4" />
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as 'calls' | 'sms')}>
            <TabsList className="mb-4">
              <TabsTrigger value="calls" className="gap-2">
                <Phone className="h-4 w-4" />
                Calls
                {callStats && callStats.total > 0 && (
                  <Badge variant="secondary" className="ml-1">{callStats.total}</Badge>
                )}
              </TabsTrigger>
              <TabsTrigger value="sms" className="gap-2">
                <MessageSquare className="h-4 w-4" />
                Text Messages
                {smsStats && smsStats.unread > 0 && (
                  <Badge variant="destructive" className="ml-1">{smsStats.unread}</Badge>
                )}
              </TabsTrigger>
            </TabsList>

            <TabsContent value="calls">
              {callsLoading ? (
                <div className="space-y-3">
                  {[1, 2, 3].map((i) => (
                    <Skeleton key={i} className="h-16 w-full" />
                  ))}
                </div>
              ) : calls.length === 0 ? (
                <div className="text-center py-12 text-muted-foreground">
                  <Phone className="h-12 w-12 mx-auto mb-4 opacity-50" />
                  <p className="text-lg font-medium">No calls yet</p>
                  <p className="text-sm">Calls made through Nora will appear here</p>
                </div>
              ) : (
                <div className="divide-y">
                  {calls.map((call) => (
                    <div
                      key={call.id}
                      className="flex items-center gap-4 p-4 hover:bg-muted/50 cursor-pointer"
                      onClick={() => setSelectedCall(call)}
                    >
                      {getCallIcon(call)}
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                          <span className="font-medium">
                            {call.caller_name || formatPhoneNumber(
                              call.direction === 'inbound' ? call.from_number : call.to_number
                            )}
                          </span>
                          {getCallStatusBadge(call.status)}
                        </div>
                        <p className="text-sm text-muted-foreground">
                          {call.direction === 'inbound' ? 'Incoming' : 'Outgoing'} call
                          {call.duration_seconds ? ` • ${formatDuration(call.duration_seconds)}` : ''}
                        </p>
                      </div>
                      <div className="flex items-center gap-2 text-sm text-muted-foreground">
                        <span>{formatDate(call.start_time)}</span>
                        <ChevronRight className="h-4 w-4" />
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </TabsContent>

            <TabsContent value="sms">
              {smsLoading ? (
                <div className="space-y-3">
                  {[1, 2, 3].map((i) => (
                    <Skeleton key={i} className="h-16 w-full" />
                  ))}
                </div>
              ) : smsMessages.length === 0 ? (
                <div className="text-center py-12 text-muted-foreground">
                  <MessageSquare className="h-12 w-12 mx-auto mb-4 opacity-50" />
                  <p className="text-lg font-medium">No text messages yet</p>
                  <p className="text-sm">SMS messages will appear here</p>
                </div>
              ) : (
                <div className="divide-y">
                  {smsMessages.map((sms) => (
                    <div
                      key={sms.id}
                      className={`flex items-center gap-4 p-4 hover:bg-muted/50 cursor-pointer ${
                        sms.is_read === 0 ? 'bg-blue-50/50' : ''
                      }`}
                      onClick={() => handleOpenSms(sms)}
                    >
                      <Button
                        variant="ghost"
                        size="icon"
                        className="shrink-0"
                        onClick={(e) => {
                          e.stopPropagation();
                          toggleSmsStarMutation.mutate(sms.id);
                        }}
                      >
                        <Star
                          className={`h-4 w-4 ${
                            sms.is_starred ? 'fill-yellow-400 text-yellow-400' : 'text-muted-foreground'
                          }`}
                        />
                      </Button>
                      <div className="w-10 h-10 rounded-full bg-gradient-to-br from-purple-500 to-pink-600 flex items-center justify-center text-white shrink-0">
                        <User className="h-5 w-5" />
                      </div>
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                          <span className={sms.is_read === 0 ? 'font-semibold' : ''}>
                            {formatPhoneNumber(sms.direction === 'inbound' ? sms.from_number : sms.to_number)}
                          </span>
                          {sms.direction === 'inbound' ? (
                            <Badge variant="outline" className="text-xs">Received</Badge>
                          ) : (
                            <Badge variant="secondary" className="text-xs">Sent</Badge>
                          )}
                        </div>
                        <p className="text-sm text-muted-foreground truncate">{sms.body}</p>
                      </div>
                      <div className="flex items-center gap-2 text-sm text-muted-foreground">
                        <span>{formatDate(sms.date_sent || sms.created_at)}</span>
                        <ChevronRight className="h-4 w-4" />
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </TabsContent>
          </Tabs>
        </CardContent>
      </Card>

      {/* Call Detail Dialog */}
      <Dialog open={!!selectedCall} onOpenChange={(open) => !open && setSelectedCall(null)}>
        <DialogContent className="max-w-2xl">
          {selectedCall && (
            <>
              <DialogHeader>
                <DialogTitle className="flex items-center gap-2">
                  {getCallIcon(selectedCall)}
                  <span>
                    {selectedCall.direction === 'inbound' ? 'Incoming' : 'Outgoing'} Call
                  </span>
                </DialogTitle>
              </DialogHeader>
              <div className="space-y-4">
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <p className="text-sm text-muted-foreground">From</p>
                    <p className="font-medium">{formatPhoneNumber(selectedCall.from_number)}</p>
                  </div>
                  <div>
                    <p className="text-sm text-muted-foreground">To</p>
                    <p className="font-medium">{formatPhoneNumber(selectedCall.to_number)}</p>
                  </div>
                  <div>
                    <p className="text-sm text-muted-foreground">Status</p>
                    {getCallStatusBadge(selectedCall.status)}
                  </div>
                  <div>
                    <p className="text-sm text-muted-foreground">Duration</p>
                    <p className="font-medium">{formatDuration(selectedCall.duration_seconds)}</p>
                  </div>
                  <div>
                    <p className="text-sm text-muted-foreground">Started</p>
                    <p className="font-medium">
                      {selectedCall.start_time
                        ? new Date(selectedCall.start_time).toLocaleString()
                        : 'N/A'}
                    </p>
                  </div>
                  <div>
                    <p className="text-sm text-muted-foreground">Ended</p>
                    <p className="font-medium">
                      {selectedCall.end_time
                        ? new Date(selectedCall.end_time).toLocaleString()
                        : 'N/A'}
                    </p>
                  </div>
                </div>

                {selectedCall.recording_url && (
                  <div className="p-4 bg-muted rounded-lg">
                    <p className="text-sm font-medium mb-2">Recording</p>
                    <Button variant="outline" size="sm" className="gap-2">
                      <Play className="h-4 w-4" />
                      Play Recording
                    </Button>
                  </div>
                )}

                {selectedCall.transcription && (
                  <div>
                    <p className="text-sm font-medium mb-2">Transcription</p>
                    <div className="p-4 bg-muted rounded-lg text-sm">
                      {selectedCall.transcription}
                    </div>
                  </div>
                )}

                {selectedCall.summary && (
                  <div>
                    <p className="text-sm font-medium mb-2">Summary</p>
                    <p className="text-sm text-muted-foreground">{selectedCall.summary}</p>
                  </div>
                )}
              </div>
            </>
          )}
        </DialogContent>
      </Dialog>

      {/* SMS Detail Dialog */}
      <Dialog open={!!selectedSms} onOpenChange={(open) => !open && setSelectedSms(null)}>
        <DialogContent className="max-w-lg">
          {selectedSms && (
            <>
              <DialogHeader>
                <DialogTitle className="flex items-center gap-2">
                  <MessageSquare className="h-5 w-5" />
                  Text Message
                </DialogTitle>
              </DialogHeader>
              <div className="space-y-4">
                <div className="flex items-center justify-between border-b pb-4">
                  <div className="flex items-center gap-3">
                    <div className="w-10 h-10 rounded-full bg-gradient-to-br from-purple-500 to-pink-600 flex items-center justify-center text-white">
                      <User className="h-5 w-5" />
                    </div>
                    <div>
                      <p className="font-medium">
                        {formatPhoneNumber(
                          selectedSms.direction === 'inbound'
                            ? selectedSms.from_number
                            : selectedSms.to_number
                        )}
                      </p>
                      <p className="text-sm text-muted-foreground">
                        {selectedSms.direction === 'inbound' ? 'Received' : 'Sent'}
                      </p>
                    </div>
                  </div>
                  <div className="text-sm text-muted-foreground">
                    {selectedSms.date_sent
                      ? new Date(selectedSms.date_sent).toLocaleString()
                      : new Date(selectedSms.created_at).toLocaleString()}
                  </div>
                </div>

                <div className="p-4 bg-muted rounded-lg">
                  <p className="whitespace-pre-wrap">{selectedSms.body}</p>
                </div>

                {selectedSms.auto_response && (
                  <div>
                    <p className="text-sm font-medium mb-2">Auto Response (Nora)</p>
                    <div className="p-4 bg-blue-50 rounded-lg text-sm">
                      {selectedSms.auto_response}
                    </div>
                  </div>
                )}

                <div className="flex items-center justify-between pt-2">
                  <Badge variant={selectedSms.status === 'delivered' ? 'default' : 'secondary'}>
                    {selectedSms.status}
                  </Badge>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => toggleSmsStarMutation.mutate(selectedSms.id)}
                  >
                    <Star
                      className={`h-4 w-4 mr-1 ${
                        selectedSms.is_starred ? 'fill-yellow-400 text-yellow-400' : ''
                      }`}
                    />
                    {selectedSms.is_starred ? 'Starred' : 'Star'}
                  </Button>
                </div>
              </div>
            </>
          )}
        </DialogContent>
      </Dialog>
    </div>
  );
}

export default CommunicationsInbox;
