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
import { Input } from '@/components/ui/input';
import { Skeleton } from '@/components/ui/skeleton';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  Mail,
  MailOpen,
  Star,
  Trash2,
  Search,
  Inbox,
  Clock,
  Paperclip,
  ChevronRight,
  RefreshCw,
  AlertCircle,
} from 'lucide-react';
import {
  emailMessagesApi,
  EmailMessageRecord,
  EmailInboxStats,
} from '@/lib/api';

interface EmailInboxProps {
  projectId: string;
  accountId?: string;
}

export function EmailInbox({ projectId, accountId }: EmailInboxProps) {
  const queryClient = useQueryClient();
  const [searchQuery, setSearchQuery] = useState('');
  const [filter, setFilter] = useState<'all' | 'unread' | 'starred' | 'needs_response'>('all');
  const [selectedMessage, setSelectedMessage] = useState<EmailMessageRecord | null>(null);

  const { data: stats, isLoading: statsLoading } = useQuery<EmailInboxStats>({
    queryKey: ['email-inbox-stats', projectId],
    queryFn: () => emailMessagesApi.getInboxStats(projectId),
    enabled: !!projectId,
  });

  const { data: messages = [], isLoading: messagesLoading, refetch } = useQuery<EmailMessageRecord[]>({
    queryKey: ['email-messages', projectId, accountId, filter],
    queryFn: () =>
      emailMessagesApi.listMessages({
        project_id: projectId,
        email_account_id: accountId,
        is_read: filter === 'unread' ? false : undefined,
        is_starred: filter === 'starred' ? true : undefined,
        needs_response: filter === 'needs_response' ? true : undefined,
        limit: 50,
      }),
    enabled: !!projectId,
  });

  const markAsReadMutation = useMutation({
    mutationFn: emailMessagesApi.markAsRead,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['email-messages'] });
      queryClient.invalidateQueries({ queryKey: ['email-inbox-stats'] });
    },
  });

  const toggleStarMutation = useMutation({
    mutationFn: emailMessagesApi.toggleStar,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['email-messages'] });
      queryClient.invalidateQueries({ queryKey: ['email-inbox-stats'] });
    },
  });

  const moveToTrashMutation = useMutation({
    mutationFn: emailMessagesApi.moveToTrash,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['email-messages'] });
      queryClient.invalidateQueries({ queryKey: ['email-inbox-stats'] });
      setSelectedMessage(null);
    },
  });

  const handleOpenMessage = async (message: EmailMessageRecord) => {
    setSelectedMessage(message);
    if (message.is_read === 0) {
      markAsReadMutation.mutate(message.id);
    }
  };

  const formatDate = (dateString: string) => {
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

  const filteredMessages = messages.filter((msg) => {
    if (!searchQuery) return true;
    const query = searchQuery.toLowerCase();
    return (
      msg.subject?.toLowerCase().includes(query) ||
      msg.from_address.toLowerCase().includes(query) ||
      msg.from_name?.toLowerCase().includes(query) ||
      msg.snippet?.toLowerCase().includes(query)
    );
  });

  const isLoading = statsLoading || messagesLoading;

  return (
    <div className="space-y-6">
      {/* Stats Cards */}
      {stats && (
        <div className="grid gap-4 md:grid-cols-4">
          <Card>
            <CardContent className="pt-6">
              <div className="flex items-center gap-3">
                <div className="p-2 bg-blue-100 rounded-lg">
                  <Inbox className="h-5 w-5 text-blue-600" />
                </div>
                <div>
                  <p className="text-sm text-muted-foreground">Total</p>
                  <p className="text-2xl font-bold">{stats.total}</p>
                </div>
              </div>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="pt-6">
              <div className="flex items-center gap-3">
                <div className="p-2 bg-red-100 rounded-lg">
                  <Mail className="h-5 w-5 text-red-600" />
                </div>
                <div>
                  <p className="text-sm text-muted-foreground">Unread</p>
                  <p className="text-2xl font-bold">{stats.unread}</p>
                </div>
              </div>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="pt-6">
              <div className="flex items-center gap-3">
                <div className="p-2 bg-yellow-100 rounded-lg">
                  <Star className="h-5 w-5 text-yellow-600" />
                </div>
                <div>
                  <p className="text-sm text-muted-foreground">Starred</p>
                  <p className="text-2xl font-bold">{stats.starred}</p>
                </div>
              </div>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="pt-6">
              <div className="flex items-center gap-3">
                <div className="p-2 bg-orange-100 rounded-lg">
                  <Clock className="h-5 w-5 text-orange-600" />
                </div>
                <div>
                  <p className="text-sm text-muted-foreground">Needs Response</p>
                  <p className="text-2xl font-bold">{stats.needs_response}</p>
                </div>
              </div>
            </CardContent>
          </Card>
        </div>
      )}

      {/* Messages List */}
      <Card>
        <CardHeader>
          <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
            <div>
              <CardTitle>Inbox</CardTitle>
              <CardDescription>
                View and manage your email messages
              </CardDescription>
            </div>
            <div className="flex gap-2">
              <div className="relative">
                <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                <Input
                  placeholder="Search emails..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="pl-9 w-64"
                />
              </div>
              <Select value={filter} onValueChange={(v) => setFilter(v as typeof filter)}>
                <SelectTrigger className="w-40">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">All Messages</SelectItem>
                  <SelectItem value="unread">Unread</SelectItem>
                  <SelectItem value="starred">Starred</SelectItem>
                  <SelectItem value="needs_response">Needs Response</SelectItem>
                </SelectContent>
              </Select>
              <Button variant="outline" size="icon" onClick={() => refetch()}>
                <RefreshCw className="h-4 w-4" />
              </Button>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="space-y-3">
              {[1, 2, 3, 4, 5].map((i) => (
                <Skeleton key={i} className="h-20 w-full" />
              ))}
            </div>
          ) : filteredMessages.length === 0 ? (
            <div className="text-center py-12 text-muted-foreground">
              <Inbox className="h-12 w-12 mx-auto mb-4 opacity-50" />
              <p className="text-lg font-medium">No emails yet</p>
              <p className="text-sm">
                {messages.length === 0
                  ? 'Connect an email account and sync to see your messages'
                  : 'No emails match your current filter'}
              </p>
            </div>
          ) : (
            <div className="divide-y">
              {filteredMessages.map((message) => (
                <EmailRow
                  key={message.id}
                  message={message}
                  onClick={() => handleOpenMessage(message)}
                  onStar={() => toggleStarMutation.mutate(message.id)}
                  onDelete={() => moveToTrashMutation.mutate(message.id)}
                  formatDate={formatDate}
                />
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Message Detail Dialog */}
      <Dialog open={!!selectedMessage} onOpenChange={(open) => !open && setSelectedMessage(null)}>
        <DialogContent className="max-w-3xl max-h-[80vh] overflow-y-auto">
          {selectedMessage && (
            <>
              <DialogHeader>
                <DialogTitle className="pr-8">{selectedMessage.subject || '(No Subject)'}</DialogTitle>
              </DialogHeader>
              <div className="space-y-4">
                <div className="flex items-center justify-between border-b pb-4">
                  <div className="flex items-center gap-3">
                    <div className="w-10 h-10 rounded-full bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center text-white font-semibold">
                      {(selectedMessage.from_name?.[0] || selectedMessage.from_address[0] || '?').toUpperCase()}
                    </div>
                    <div>
                      <p className="font-medium">
                        {selectedMessage.from_name || selectedMessage.from_address}
                      </p>
                      <p className="text-sm text-muted-foreground">
                        {selectedMessage.from_address}
                      </p>
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-sm text-muted-foreground">
                      {new Date(selectedMessage.received_at).toLocaleString()}
                    </span>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => toggleStarMutation.mutate(selectedMessage.id)}
                    >
                      <Star
                        className={`h-4 w-4 ${
                          selectedMessage.is_starred
                            ? 'fill-yellow-400 text-yellow-400'
                            : ''
                        }`}
                      />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => moveToTrashMutation.mutate(selectedMessage.id)}
                    >
                      <Trash2 className="h-4 w-4 text-red-500" />
                    </Button>
                  </div>
                </div>

                {selectedMessage.has_attachments === 1 && (
                  <div className="flex items-center gap-2 p-3 bg-muted rounded-lg">
                    <Paperclip className="h-4 w-4" />
                    <span className="text-sm">This email has attachments</span>
                  </div>
                )}

                <div className="prose prose-sm max-w-none">
                  {selectedMessage.body_html ? (
                    <div
                      dangerouslySetInnerHTML={{ __html: selectedMessage.body_html }}
                    />
                  ) : (
                    <pre className="whitespace-pre-wrap font-sans">
                      {selectedMessage.body_text || selectedMessage.snippet || 'No content'}
                    </pre>
                  )}
                </div>

                {selectedMessage.needs_response === 1 && (
                  <div className="flex items-center gap-2 p-3 bg-orange-50 border border-orange-200 rounded-lg">
                    <AlertCircle className="h-4 w-4 text-orange-600" />
                    <span className="text-sm text-orange-800">This email needs a response</span>
                  </div>
                )}
              </div>
            </>
          )}
        </DialogContent>
      </Dialog>
    </div>
  );
}

function EmailRow({
  message,
  onClick,
  onStar,
  onDelete,
  formatDate,
}: {
  message: EmailMessageRecord;
  onClick: () => void;
  onStar: () => void;
  onDelete: () => void;
  formatDate: (date: string) => string;
}) {
  const isUnread = message.is_read === 0;

  return (
    <div
      className={`flex items-center gap-4 p-4 hover:bg-muted/50 cursor-pointer transition-colors ${
        isUnread ? 'bg-blue-50/50' : ''
      }`}
      onClick={onClick}
    >
      {/* Star */}
      <Button
        variant="ghost"
        size="icon"
        className="shrink-0"
        onClick={(e) => {
          e.stopPropagation();
          onStar();
        }}
      >
        <Star
          className={`h-4 w-4 ${
            message.is_starred ? 'fill-yellow-400 text-yellow-400' : 'text-muted-foreground'
          }`}
        />
      </Button>

      {/* Read/Unread indicator */}
      <div className="shrink-0">
        {isUnread ? (
          <Mail className="h-5 w-5 text-blue-600" />
        ) : (
          <MailOpen className="h-5 w-5 text-muted-foreground" />
        )}
      </div>

      {/* Sender Avatar */}
      <div className="w-10 h-10 rounded-full bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center text-white font-semibold shrink-0">
        {(message.from_name?.[0] || message.from_address[0] || '?').toUpperCase()}
      </div>

      {/* Content */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-1">
          <span className={`truncate ${isUnread ? 'font-semibold' : ''}`}>
            {message.from_name || message.from_address}
          </span>
          {message.has_attachments === 1 && (
            <Paperclip className="h-3 w-3 text-muted-foreground shrink-0" />
          )}
          {message.needs_response === 1 && (
            <Badge variant="outline" className="text-xs shrink-0 text-orange-600 border-orange-300">
              Needs Response
            </Badge>
          )}
        </div>
        <p className={`text-sm truncate ${isUnread ? 'font-medium' : 'text-muted-foreground'}`}>
          {message.subject || '(No Subject)'}
        </p>
        <p className="text-xs text-muted-foreground truncate">
          {message.snippet}
        </p>
      </div>

      {/* Date & Actions */}
      <div className="flex items-center gap-2 shrink-0">
        <span className="text-xs text-muted-foreground">
          {formatDate(message.received_at)}
        </span>
        <Button
          variant="ghost"
          size="icon"
          className="opacity-0 group-hover:opacity-100"
          onClick={(e) => {
            e.stopPropagation();
            onDelete();
          }}
        >
          <Trash2 className="h-4 w-4 text-muted-foreground hover:text-red-500" />
        </Button>
        <ChevronRight className="h-4 w-4 text-muted-foreground" />
      </div>
    </div>
  );
}

export default EmailInbox;
