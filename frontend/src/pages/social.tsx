import { useEffect, useMemo, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import type { Project } from 'shared/types';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Skeleton } from '@/components/ui/skeleton';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { ContentCalendar } from '@/components/content-studio';
import { SocialAccountConnect } from '@/components/social/SocialAccountConnect';
import {
  UnifiedInbox,
  UnifiedInboxMention,
} from '@/components/social/UnifiedInbox';
import type {
  SocialAccount as UiSocialAccount,
  SocialPost as UiSocialPost,
  SocialPlatform,
  PlatformAdaptation,
  ContentBlock,
  ContentCategory,
  PostStatus,
} from '@/types/social';
import { PLATFORM_LIMITS } from '@/types/social';
import {
  projectsApi,
  socialApi,
  SocialAccountRecord,
  SocialPostRecord,
  SocialMentionRecord,
} from '@/lib/api';

export function SocialPage() {
  const {
    data: projects = [],
    isLoading: projectsLoading,
    error: projectsError,
  } = useQuery<Project[], Error>({
    queryKey: ['projects', 'social-command'],
    queryFn: projectsApi.getAll,
  });
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(null);
  const [searchParams, setSearchParams] = useSearchParams();
  const projectParam = searchParams.get('projectId');

  useEffect(() => {
    if (!selectedProjectId && projects.length > 0) {
      setSelectedProjectId(projects[0].id);
    }
  }, [projects, selectedProjectId]);

  useEffect(() => {
    if (projectParam && projectParam !== selectedProjectId) {
      setSelectedProjectId(projectParam);
    }
  }, [projectParam, selectedProjectId]);

  useEffect(() => {
    if (!selectedProjectId) return;
    if (projectParam === selectedProjectId) return;
    const next = new URLSearchParams();
    next.set('projectId', selectedProjectId);
    setSearchParams(next, { replace: true });
  }, [selectedProjectId, projectParam, setSearchParams]);

  const accountsQuery = useQuery<SocialAccountRecord[], Error>({
    queryKey: ['social-accounts', selectedProjectId],
    queryFn: () => socialApi.listAccounts(selectedProjectId ?? undefined),
    enabled: !!selectedProjectId,
  });

  const postsQuery = useQuery<SocialPostRecord[], Error>({
    queryKey: ['social-posts', selectedProjectId],
    queryFn: () => socialApi.listPosts(selectedProjectId ?? undefined),
    enabled: !!selectedProjectId,
  });

  const mentionsQuery = useQuery<SocialMentionRecord[], Error>({
    queryKey: ['social-mentions', selectedProjectId],
    queryFn: () => socialApi.listMentions(selectedProjectId as string, { limit: 60 }),
    enabled: !!selectedProjectId,
  });

  const statsQuery = useQuery({
    queryKey: ['social-inbox-stats', selectedProjectId],
    queryFn: () => socialApi.inboxStats(selectedProjectId as string),
    enabled: !!selectedProjectId,
  });

  const socialAccounts = useMemo(
    () => (accountsQuery.data ?? []).map(mapAccountRecord),
    [accountsQuery.data]
  );

  const calendarPosts = useMemo(
    () => mapPostRecords(postsQuery.data ?? [], socialAccounts),
    [postsQuery.data, socialAccounts]
  );

  const inboxMentions = useMemo(
    () => mapMentionRecords(mentionsQuery.data ?? []),
    [mentionsQuery.data]
  );

  const renderAccountsSection = () => {
    if (!selectedProjectId || accountsQuery.isLoading) {
      return <Skeleton className="h-48 w-full" />;
    }
    if (accountsQuery.error) {
      return (
        <Alert variant="destructive">
          <AlertDescription>
            {accountsQuery.error.message || 'Failed to load social accounts.'}
          </AlertDescription>
        </Alert>
      );
    }
    return (
      <SocialAccountConnect
        accounts={socialAccounts}
        className="space-y-4"
      />
    );
  };

  const renderCalendarSection = () => {
    if (!selectedProjectId || postsQuery.isLoading) {
      return <Skeleton className="h-80 w-full" />;
    }
    if (postsQuery.error) {
      return (
        <Alert variant="destructive">
          <AlertDescription>
            {postsQuery.error.message || 'Unable to load scheduled posts.'}
          </AlertDescription>
        </Alert>
      );
    }
    return (
      <ContentCalendar
        posts={calendarPosts}
        className="border"
      />
    );
  };

  const renderInboxSection = () => {
    if (!selectedProjectId || mentionsQuery.isLoading) {
      return <Skeleton className="h-64 w-full" />;
    }
    if (mentionsQuery.error) {
      return (
        <Alert variant="destructive">
          <AlertDescription>
            {mentionsQuery.error.message || 'Unable to load inbox.'}
          </AlertDescription>
        </Alert>
      );
    }
    return <UnifiedInbox mentions={inboxMentions} />;
  };

  return (
    <div className="p-6 space-y-6">
      <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
        <div>
          <CardTitle className="text-2xl">Social Command</CardTitle>
          <CardDescription>
            Connect accounts, schedule campaigns, and monitor engagement without leaving Mission Control.
          </CardDescription>
        </div>
        <div className="w-full max-w-xs space-y-1">
          <Label htmlFor="project-select">Project</Label>
          <Select
            value={selectedProjectId ?? ''}
            onValueChange={setSelectedProjectId}
            disabled={!projects.length}
          >
            <SelectTrigger id="project-select">
              <SelectValue placeholder="Select a project" />
            </SelectTrigger>
            <SelectContent>
              {projects.map((project) => (
                <SelectItem key={project.id} value={project.id}>
                  {project.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>

      {projectsLoading && <Skeleton className="h-8 w-1/3" />}

      {projectsError && (
        <Alert variant="destructive">
          <AlertDescription>
            {projectsError.message || 'Unable to load projects.'}
          </AlertDescription>
        </Alert>
      )}

      {!projects.length && !projectsLoading ? (
        <Alert>
          <AlertDescription>
            No projects detected. Create a project to enable social command.
          </AlertDescription>
        </Alert>
      ) : (
        <>
          <div className="grid gap-6 xl:grid-cols-[360px,1fr]">
            <Card>
              <CardHeader>
                <CardTitle>Connected Platforms</CardTitle>
                <CardDescription>
                  OAuth health, follower counts, and sync state per platform.
                </CardDescription>
              </CardHeader>
              <CardContent>{renderAccountsSection()}</CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>Content Calendar</CardTitle>
                <CardDescription>
                  Drag-and-drop posts across Kanban and calendar views.
                </CardDescription>
              </CardHeader>
              <CardContent>{renderCalendarSection()}</CardContent>
            </Card>
          </div>

          <Card>
            <CardHeader className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
              <div>
                <CardTitle>Unified Inbox</CardTitle>
                <CardDescription>
                  Mentions, comments, and DMs prioritized by Nora’s engagement agent.
                </CardDescription>
              </div>
              {statsQuery.isLoading ? (
                <Skeleton className="h-6 w-40" />
              ) : statsQuery.data ? (
                <div className="flex gap-3 text-sm text-muted-foreground">
                  <Badge variant="secondary">Unread: {statsQuery.data.total_unread}</Badge>
                  <Badge variant="outline">High Priority: {statsQuery.data.high_priority}</Badge>
                </div>
              ) : null}
            </CardHeader>
            <CardContent>{renderInboxSection()}</CardContent>
          </Card>
        </>
      )}
    </div>
  );
}

function mapAccountRecord(record: SocialAccountRecord): UiSocialAccount {
  const platform = normalizePlatform(record.platform);
  const statusMap: Record<string, UiSocialAccount['status']> = {
    active: 'active',
    inactive: 'disconnected',
    error: 'disconnected',
    expired: 'expired',
    pending_auth: 'pending',
  };

  return {
    id: record.id,
    platform,
    account_name:
      record.display_name || record.username || `${platform.toUpperCase()} Account`,
    account_type:
      (record.account_type as UiSocialAccount['account_type']) || 'business',
    status: statusMap[record.status] || 'disconnected',
    profile_url: record.profile_url || undefined,
    avatar_url: record.avatar_url || undefined,
    follower_count: record.follower_count ?? undefined,
    connected_at: record.created_at,
    last_synced_at: record.last_sync_at || undefined,
  };
}

function mapPostRecords(
  records: SocialPostRecord[],
  accounts: UiSocialAccount[]
): UiSocialPost[] {
  const accountMap = new Map(accounts.map((account) => [account.id, account]));

  return records.map((record) => {
    const blocks =
      safeParseJson<ContentBlock[]>(record.content_blocks) ??
      buildFallbackBlocks(record);
    const hashtags = safeParseJson<string[]>(record.hashtags) ?? [];
    const platforms = derivePlatforms(record, accountMap);
    const platformAdaptations = buildPlatformAdaptations(
      platforms,
      record.caption || blocks[0]?.content || 'Draft content',
      hashtags.length
    );

    return {
      id: record.id,
      account_id: record.social_account_id ?? '',
      task_id: record.task_id ?? undefined,
      content_blocks: blocks,
      platform_adaptations: platformAdaptations,
      category: (record.category as ContentCategory) || 'community',
      status: mapPostStatus(record.status),
      scheduled_for: record.scheduled_for || undefined,
      published_at: record.published_at || undefined,
      is_evergreen: !!record.is_evergreen,
      metrics: {
        impressions: record.impressions,
        reach: record.reach,
        engagement_rate: record.engagement_rate,
        likes: record.likes,
        comments: record.comments,
        shares: record.shares,
        saves: record.saves,
        clicks: record.clicks,
      },
      created_at: record.created_at,
      updated_at: record.updated_at,
    };
  });
}

function mapMentionRecords(records: SocialMentionRecord[]): UnifiedInboxMention[] {
  return records.map((record) => ({
    id: record.id,
    platform: normalizePlatform(record.platform),
    account_id: record.social_account_id,
    mention_type: mapMentionType(record.mention_type),
    author_name: record.author_display_name || record.author_username || 'Follower',
    author_username: record.author_username || '',
    author_avatar: record.author_avatar_url || undefined,
    content: record.content || '',
    post_url: record.parent_platform_id || undefined,
    sentiment: mapSentiment(record.sentiment),
    priority: mapPriority(record.priority),
    status: mapMentionStatus(record.status),
    created_at: record.received_at,
    responded_at: record.replied_at || undefined,
  }));
}

function normalizePlatform(value?: string | null): SocialPlatform {
  const platform = (value || 'linkedin').toLowerCase();
  const allowed: SocialPlatform[] = [
    'linkedin',
    'instagram',
    'twitter',
    'facebook',
    'tiktok',
    'bluesky',
    'youtube',
    'pinterest',
    'threads',
  ];
  return allowed.includes(platform as SocialPlatform)
    ? (platform as SocialPlatform)
    : 'linkedin';
}

function safeParseJson<T>(value?: string | null): T | null {
  if (!value) return null;
  try {
    return JSON.parse(value) as T;
  } catch (error) {
    console.warn('Failed to parse JSON', error);
    return null;
  }
}

function buildFallbackBlocks(record: SocialPostRecord): ContentBlock[] {
  const base = record.caption || 'Untitled social update';
  return [
    {
      id: `caption-${record.id}`,
      type: 'body',
      content: base,
    },
  ];
}

function derivePlatforms(
  record: SocialPostRecord,
  accounts: Map<string, UiSocialAccount>
): SocialPlatform[] {
  const fromJson = safeParseJson<string[]>(record.platforms) ?? [];
  const set = new Set<SocialPlatform>();
  fromJson.forEach((accountId) => {
    const account = accounts.get(accountId);
    if (account) set.add(account.platform);
  });
  if (record.social_account_id) {
    const account = accounts.get(record.social_account_id);
    if (account) set.add(account.platform);
  }
  if (!set.size) {
    set.add('linkedin');
  }
  return Array.from(set);
}

function buildPlatformAdaptations(
  platforms: SocialPlatform[],
  baseContent: string,
  hashtagCount: number
): Record<SocialPlatform, PlatformAdaptation> {
  return platforms.reduce((acc, platform) => {
    const limits = PLATFORM_LIMITS[platform];
    acc[platform] = {
      platform,
      content: baseContent,
      characterCount: baseContent.length,
      characterLimit: limits?.characterLimit ?? 3000,
      hashtagCount,
      hashtagLimit: limits?.hashtagLimit ?? 30,
      mediaIds: [],
      isValid: true,
      warnings: [],
    };
    return acc;
  }, {} as Record<SocialPlatform, PlatformAdaptation>);
}

function mapPostStatus(status: string): PostStatus {
  const normalized = status.toLowerCase();
  switch (normalized) {
    case 'pending_review':
      return 'pending_approval';
    case 'scheduled':
    case 'publishing':
      return 'scheduled';
    case 'approved':
      return 'approved';
    case 'published':
      return 'published';
    case 'failed':
      return 'failed';
    case 'cancelled':
      return 'archived';
    default:
      return 'draft';
  }
}

function mapMentionType(type: string) {
  const normalized = type.toLowerCase();
  if (['comment', 'mention', 'reply', 'dm', 'like', 'share'].includes(normalized)) {
    return normalized as UnifiedInboxMention['mention_type'];
  }
  if (normalized === 'quote' || normalized === 'tag' || normalized === 'review') {
    return 'mention';
  }
  return 'comment';
}

function mapSentiment(value?: string | null) {
  const normalized = (value || 'neutral').toLowerCase();
  if (['positive', 'neutral', 'negative', 'mixed'].includes(normalized)) {
    return normalized as UnifiedInboxMention['sentiment'];
  }
  return 'neutral';
}

function mapPriority(value: string): UnifiedInboxMention['priority'] {
  const normalized = value.toLowerCase();
  if (['urgent', 'high', 'normal', 'low'].includes(normalized)) {
    return normalized as UnifiedInboxMention['priority'];
  }
  return 'normal';
}

function mapMentionStatus(value: string): UnifiedInboxMention['status'] {
  const normalized = value.toLowerCase();
  if (normalized === 'replied') return 'responded';
  if (normalized === 'archived') return 'archived';
  if (normalized === 'read') return 'read';
  return 'unread';
}
