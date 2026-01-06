import { useState, useMemo, useCallback } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuItem,
} from '@/components/ui/dropdown-menu';
import {
  Inbox,
  MessageSquare,
  Heart,
  AtSign,
  Share2,
  Search,
  Filter,
  CheckCircle2,
  Clock,
  AlertTriangle,
  Send,
  MoreHorizontal,
  ExternalLink,
  Archive,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { SocialPlatform } from '@/types/social';
import { PLATFORM_ICONS } from '@/types/social';

// Local types for inbox items
type MentionType = 'comment' | 'mention' | 'reply' | 'dm' | 'like' | 'share' | 'follow';
type Sentiment = 'positive' | 'neutral' | 'negative' | 'mixed';
type Priority = 'urgent' | 'high' | 'normal' | 'low';
type MentionStatus = 'unread' | 'read' | 'responded' | 'archived';

export interface UnifiedInboxMention {
  id: string;
  platform: SocialPlatform;
  account_id: string;
  mention_type: MentionType;
  author_name: string;
  author_username: string;
  author_avatar?: string;
  content: string;
  post_url?: string;
  sentiment: Sentiment;
  priority: Priority;
  status: MentionStatus;
  created_at: string;
  responded_at?: string;
}

interface UnifiedInboxProps {
  mentions: UnifiedInboxMention[];
  onMentionClick?: (mention: UnifiedInboxMention) => void;
  onRespond?: (mentionId: string, response: string) => void;
  onArchive?: (mentionId: string) => void;
  onMarkRead?: (mentionId: string) => void;
  className?: string;
}

const platformFilterOptions: Array<{ value: SocialPlatform | 'all'; label: string }> = [
  { value: 'all', label: 'All Platforms' },
  { value: 'linkedin', label: 'LinkedIn' },
  { value: 'instagram', label: 'Instagram' },
  { value: 'twitter', label: 'Twitter / X' },
  { value: 'facebook', label: 'Facebook' },
  { value: 'tiktok', label: 'TikTok' },
  { value: 'bluesky', label: 'Bluesky' },
  { value: 'youtube', label: 'YouTube' },
  { value: 'pinterest', label: 'Pinterest' },
  { value: 'threads', label: 'Threads' },
];

const mentionTypeConfig: Record<MentionType, {
  label: string;
  icon: React.ReactNode;
  color: string;
}> = {
  comment: {
    label: 'Comment',
    icon: <MessageSquare className="h-4 w-4" />,
    color: 'text-blue-500',
  },
  mention: {
    label: 'Mention',
    icon: <AtSign className="h-4 w-4" />,
    color: 'text-purple-500',
  },
  reply: {
    label: 'Reply',
    icon: <MessageSquare className="h-4 w-4" />,
    color: 'text-cyan-500',
  },
  dm: {
    label: 'DM',
    icon: <Send className="h-4 w-4" />,
    color: 'text-green-500',
  },
  like: {
    label: 'Like',
    icon: <Heart className="h-4 w-4" />,
    color: 'text-red-500',
  },
  share: {
    label: 'Share',
    icon: <Share2 className="h-4 w-4" />,
    color: 'text-orange-500',
  },
  follow: {
    label: 'Follow',
    icon: <AtSign className="h-4 w-4" />,
    color: 'text-indigo-500',
  },
};

const sentimentConfig: Record<Sentiment, {
  label: string;
  color: string;
}> = {
  positive: { label: 'Positive', color: 'bg-green-100 text-green-700' },
  neutral: { label: 'Neutral', color: 'bg-gray-100 text-gray-700' },
  negative: { label: 'Negative', color: 'bg-red-100 text-red-700' },
  mixed: { label: 'Mixed', color: 'bg-yellow-100 text-yellow-700' },
};

const priorityConfig: Record<Priority, {
  label: string;
  icon: React.ReactNode;
  color: string;
}> = {
  urgent: {
    label: 'Urgent',
    icon: <AlertTriangle className="h-3 w-3" />,
    color: 'bg-red-500 text-white',
  },
  high: {
    label: 'High',
    icon: <Clock className="h-3 w-3" />,
    color: 'bg-orange-500 text-white',
  },
  normal: {
    label: 'Normal',
    icon: null,
    color: 'bg-gray-100 text-gray-700',
  },
  low: {
    label: 'Low',
    icon: null,
    color: 'bg-gray-50 text-gray-500',
  },
};

function MentionCard({
  mention,
  onClick,
  onRespond,
  onArchive,
  onMarkRead,
}: {
  mention: UnifiedInboxMention;
  onClick?: () => void;
  onRespond?: (response: string) => void;
  onArchive?: () => void;
  onMarkRead?: () => void;
}) {
  const [isExpanded, setIsExpanded] = useState(false);
  const [response, setResponse] = useState('');

  const typeConfig = mentionTypeConfig[mention.mention_type];
  const sentiment = sentimentConfig[mention.sentiment];
  const priority = priorityConfig[mention.priority];

  const formatTime = (dateString: string) => {
    const date = new Date(dateString);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;
    return date.toLocaleDateString();
  };

  const handleSubmitResponse = useCallback(() => {
    if (response.trim() && onRespond) {
      onRespond(response);
      setResponse('');
      setIsExpanded(false);
    }
  }, [response, onRespond]);

  return (
    <Card
      className={cn(
        'transition-all cursor-pointer hover:shadow-md',
        mention.status === 'unread' && 'border-l-4 border-l-blue-500 bg-blue-50/30'
      )}
      onClick={onClick}
    >
      <CardContent className="p-4">
        <div className="flex items-start gap-3">
          {/* Avatar */}
          <div className="relative">
            {mention.author_avatar ? (
              <img
                src={mention.author_avatar}
                alt={mention.author_name}
                className="w-10 h-10 rounded-full object-cover"
              />
            ) : (
              <div className="w-10 h-10 rounded-full bg-muted flex items-center justify-center">
                <span className="text-sm font-medium">
                  {mention.author_name.charAt(0).toUpperCase()}
                </span>
              </div>
            )}
            <div
              className={cn(
                'absolute -bottom-1 -right-1 w-5 h-5 rounded-full flex items-center justify-center text-white text-[10px] font-bold',
                mention.platform === 'linkedin' && 'bg-[#0A66C2]',
                mention.platform === 'instagram' && 'bg-[#E4405F]',
                mention.platform === 'twitter' && 'bg-black',
                mention.platform === 'facebook' && 'bg-[#1877F2]',
                !['linkedin', 'instagram', 'twitter', 'facebook'].includes(mention.platform) && 'bg-gray-600'
              )}
            >
              {PLATFORM_ICONS[mention.platform]}
            </div>
          </div>

          {/* Content */}
          <div className="flex-1 min-w-0">
            {/* Header */}
            <div className="flex items-center gap-2 mb-1">
              <span className="font-medium text-sm truncate">
                {mention.author_name}
              </span>
              <span className="text-xs text-muted-foreground">
                @{mention.author_username}
              </span>
              <span className="text-xs text-muted-foreground">
                • {formatTime(mention.created_at)}
              </span>
            </div>

            {/* Badges */}
            <div className="flex items-center gap-1.5 mb-2">
              <Badge variant="outline" className="text-xs gap-1">
                <span className={typeConfig.color}>{typeConfig.icon}</span>
                {typeConfig.label}
              </Badge>
              <Badge variant="outline" className={cn('text-xs', sentiment.color)}>
                {sentiment.label}
              </Badge>
              {mention.priority !== 'normal' && (
                <Badge className={cn('text-xs gap-1', priority.color)}>
                  {priority.icon}
                  {priority.label}
                </Badge>
              )}
              {mention.status === 'responded' && (
                <Badge variant="outline" className="text-xs bg-green-50 text-green-700">
                  <CheckCircle2 className="h-3 w-3 mr-1" />
                  Responded
                </Badge>
              )}
            </div>

            {/* Message content */}
            <p className="text-sm text-muted-foreground line-clamp-2">
              {mention.content}
            </p>

            {/* Expanded response area */}
            {isExpanded && (
              <div
                className="mt-3 space-y-2"
                onClick={(e) => e.stopPropagation()}
              >
                <Input
                  value={response}
                  onChange={(e) => setResponse(e.target.value)}
                  placeholder="Type your response..."
                  className="text-sm"
                />
                <div className="flex gap-2">
                  <Button
                    size="sm"
                    onClick={handleSubmitResponse}
                    disabled={!response.trim()}
                  >
                    <Send className="h-3 w-3 mr-1" />
                    Send
                  </Button>
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => setIsExpanded(false)}
                  >
                    Cancel
                  </Button>
                </div>
              </div>
            )}
          </div>

          {/* Actions */}
          <div className="flex items-center gap-1">
            {mention.post_url && (
              <Button
                variant="ghost"
                size="icon"
                asChild
                onClick={(e) => e.stopPropagation()}
              >
                <a href={mention.post_url} target="_blank" rel="noopener noreferrer">
                  <ExternalLink className="h-4 w-4" />
                </a>
              </Button>
            )}
            {!isExpanded && onRespond && mention.status !== 'responded' && (
              <Button
                variant="ghost"
                size="icon"
                onClick={(e) => {
                  e.stopPropagation();
                  setIsExpanded(true);
                }}
              >
                <MessageSquare className="h-4 w-4" />
              </Button>
            )}
            {(onArchive || onMarkRead) ? (
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={(e) => e.stopPropagation()}
                  >
                    <MoreHorizontal className="h-4 w-4" />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" className="w-40">
                  <DropdownMenuLabel>Quick Actions</DropdownMenuLabel>
                  <DropdownMenuSeparator />
                  {onMarkRead && mention.status === 'unread' && (
                    <DropdownMenuItem
                      onClick={(e) => {
                        e.stopPropagation();
                        onMarkRead();
                      }}
                    >
                      <CheckCircle2 className="h-3.5 w-3.5" />
                      Mark as read
                    </DropdownMenuItem>
                  )}
                  {onArchive && (
                    <DropdownMenuItem
                      onClick={(e) => {
                        e.stopPropagation();
                        onArchive();
                      }}
                    >
                      <Archive className="h-3.5 w-3.5" />
                      Archive
                    </DropdownMenuItem>
                  )}
                </DropdownMenuContent>
              </DropdownMenu>
            ) : (
              <Button
                variant="ghost"
                size="icon"
                onClick={(e) => e.stopPropagation()}
              >
                <MoreHorizontal className="h-4 w-4" />
              </Button>
            )}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

export function UnifiedInbox({
  mentions,
  onMentionClick,
  onRespond,
  onArchive,
  onMarkRead,
  className,
}: UnifiedInboxProps) {
  const [searchQuery, setSearchQuery] = useState('');
  const [activeTab, setActiveTab] = useState('all');
  const [platformFilter, setPlatformFilter] = useState<SocialPlatform | 'all'>('all');

  const filteredMentions = useMemo(() => {
    return mentions.filter((mention) => {
      // Search filter
      if (searchQuery) {
        const query = searchQuery.toLowerCase();
        const matchesSearch =
          mention.author_name.toLowerCase().includes(query) ||
          mention.author_username.toLowerCase().includes(query) ||
          mention.content.toLowerCase().includes(query);
        if (!matchesSearch) return false;
      }

      // Platform filter
      if (platformFilter !== 'all' && mention.platform !== platformFilter) {
        return false;
      }

      // Tab filter
      switch (activeTab) {
        case 'unread':
          return mention.status === 'unread';
        case 'urgent':
          return mention.priority === 'urgent' || mention.priority === 'high';
        case 'responded':
          return mention.status === 'responded';
        default:
          return mention.status !== 'archived';
      }
    });
  }, [mentions, searchQuery, platformFilter, activeTab]);

  const counts = useMemo(() => ({
    all: mentions.filter(m => m.status !== 'archived').length,
    unread: mentions.filter(m => m.status === 'unread').length,
    urgent: mentions.filter(m => m.priority === 'urgent' || m.priority === 'high').length,
    responded: mentions.filter(m => m.status === 'responded').length,
  }), [mentions]);

  return (
    <Card className={className}>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Inbox className="h-5 w-5 text-muted-foreground" />
            <CardTitle className="text-lg">Unified Inbox</CardTitle>
            {counts.unread > 0 && (
              <Badge className="bg-blue-500">{counts.unread} new</Badge>
            )}
          </div>
          <div className="flex items-center gap-2">
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="outline" size="sm" className="gap-1">
                  <Filter className="h-4 w-4" />
                  {platformFilter === 'all' ? 'All Platforms' : platformFilter}
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-48">
                <DropdownMenuLabel>Platform Filter</DropdownMenuLabel>
                <DropdownMenuSeparator />
                <DropdownMenuRadioGroup
                  value={platformFilter}
                  onValueChange={(value) =>
                    setPlatformFilter(value as SocialPlatform | 'all')
                  }
                >
                  {platformFilterOptions.map((option) => (
                    <DropdownMenuRadioItem key={option.value} value={option.value}>
                      <span className="flex items-center gap-2">
                        {option.value !== 'all' && (
                          <span className="text-xs font-semibold">
                            {PLATFORM_ICONS[option.value as SocialPlatform]}
                          </span>
                        )}
                        {option.label}
                      </span>
                    </DropdownMenuRadioItem>
                  ))}
                </DropdownMenuRadioGroup>
                {platformFilter !== 'all' && (
                  <>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem onClick={() => setPlatformFilter('all')}>
                      Clear Filter
                    </DropdownMenuItem>
                  </>
                )}
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>

        {/* Search */}
        <div className="relative mt-3">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search mentions..."
            className="pl-9"
          />
        </div>
      </CardHeader>

      <CardContent>
        <Tabs value={activeTab} onValueChange={setActiveTab}>
          <TabsList className="w-full justify-start mb-4">
            <TabsTrigger value="all" className="gap-1">
              All
              <Badge variant="secondary" className="ml-1 text-xs">
                {counts.all}
              </Badge>
            </TabsTrigger>
            <TabsTrigger value="unread" className="gap-1">
              Unread
              {counts.unread > 0 && (
                <Badge className="ml-1 text-xs bg-blue-500">
                  {counts.unread}
                </Badge>
              )}
            </TabsTrigger>
            <TabsTrigger value="urgent" className="gap-1">
              Urgent
              {counts.urgent > 0 && (
                <Badge className="ml-1 text-xs bg-red-500">
                  {counts.urgent}
                </Badge>
              )}
            </TabsTrigger>
            <TabsTrigger value="responded" className="gap-1">
              Responded
            </TabsTrigger>
          </TabsList>

          <TabsContent value={activeTab} className="mt-0">
            <div className="space-y-2">
              {filteredMentions.length === 0 ? (
                <div className="text-center py-12 text-muted-foreground">
                  <Inbox className="h-12 w-12 mx-auto mb-2 opacity-50" />
                  <p>No mentions found</p>
                  <p className="text-sm">
                    {searchQuery
                      ? 'Try adjusting your search'
                      : 'New mentions will appear here'}
                  </p>
                </div>
              ) : (
                filteredMentions.map((mention) => (
                  <MentionCard
                    key={mention.id}
                    mention={mention}
                    onClick={() => onMentionClick?.(mention)}
                    onRespond={
                      onRespond
                        ? (response) => onRespond(mention.id, response)
                        : undefined
                    }
                    onArchive={onArchive ? () => onArchive(mention.id) : undefined}
                    onMarkRead={onMarkRead ? () => onMarkRead(mention.id) : undefined}
                  />
                ))
              )}
            </div>
          </TabsContent>
        </Tabs>
      </CardContent>
    </Card>
  );
}
