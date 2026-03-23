import { useMemo } from 'react';
import { useQuery, useQueries } from '@tanstack/react-query';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { CardGrid } from '@/components/ui/card-grid';
import { Users, CalendarDays, Inbox, CheckCircle, Share2, FileText, Globe } from 'lucide-react';
import {
  organizationsApi,
  socialApi,
  type SocialAccountRecord,
  type SocialPostRecord,
  type SocialMentionRecord,
} from '@/lib/api';
import { socialKeys, organizationKeys } from '@/lib/query-keys';
import {
  PLATFORM_ICONS,
  PLATFORM_COLORS,
  PLATFORM_BG,
  STATUS_COLORS,
  SENTIMENT_COLORS,
} from '../../constants';
import { formatDate } from '../../helpers';
import { formatCompactNumber } from '@/lib/formatters';

export function SocialOverviewView({
  projectEntries,
  orgId,
  onSwitchView,
}: {
  projectEntries: { id: string; name: string }[];
  orgId: string;
  onSwitchView: (v: string) => void;
}) {
  const { data: brandProfile } = useQuery({
    queryKey: organizationKeys.brandProfile(orgId),
    queryFn: () => organizationsApi.getBrandProfile(orgId),
    staleTime: 300_000,
    enabled: !!orgId,
  });

  const accountQueries = useQueries({
    queries: projectEntries.map(e => ({
      queryKey: socialKeys.accounts(e.id),
      queryFn: () => socialApi.listAccounts(e.id),
      staleTime: 60_000,
    })),
  });

  const postQueries = useQueries({
    queries: projectEntries.map(e => ({
      queryKey: socialKeys.posts(e.id),
      queryFn: () => socialApi.listPostsFiltered({ projectId: e.id, limit: 20 }),
      staleTime: 60_000,
    })),
  });

  const mentionQueries = useQueries({
    queries: projectEntries.map(e => ({
      queryKey: socialKeys.mentionsOverview(e.id),
      queryFn: () => socialApi.listMentions(e.id, { limit: 10 }),
      staleTime: 60_000,
    })),
  });

  const agg = useMemo(() => {
    const allAccounts: (SocialAccountRecord & { _project: string })[] = [];
    const allPosts: (SocialPostRecord & { _project: string })[] = [];
    const allMentions: (SocialMentionRecord & { _project: string })[] = [];

    accountQueries.forEach((q, i) => {
      q.data?.forEach(a => allAccounts.push({ ...a, _project: projectEntries[i].name }));
    });
    postQueries.forEach((q, i) => {
      q.data?.forEach(p => allPosts.push({ ...p, _project: projectEntries[i].name }));
    });
    mentionQueries.forEach((q, i) => {
      q.data?.forEach(m => allMentions.push({ ...m, _project: projectEntries[i].name }));
    });

    const totalFollowers = allAccounts.reduce((s, a) => s + (a.follower_count || 0), 0);
    const scheduled = allPosts.filter(p => p.status === 'scheduled').length;
    const published = allPosts.filter(p => p.status === 'published').length;
    const unread = allMentions.filter(m => m.status === 'unread').length;
    const urgent = allMentions.filter(m => m.priority === 'urgent' || m.priority === 'high').length;

    const recentPosts = [...allPosts]
      .filter(p => p.status === 'published' || p.status === 'scheduled')
      .sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime())
      .slice(0, 6);

    const recentMentions = [...allMentions]
      .sort((a, b) => new Date(b.received_at).getTime() - new Date(a.received_at).getTime())
      .slice(0, 5);

    // Platform breakdown
    const byPlatform: Record<string, { accounts: number; followers: number }> = {};
    allAccounts.forEach(a => {
      if (!byPlatform[a.platform]) byPlatform[a.platform] = { accounts: 0, followers: 0 };
      byPlatform[a.platform].accounts++;
      byPlatform[a.platform].followers += a.follower_count || 0;
    });

    // Brand profile platforms (if no connected accounts)
    const brandHandles: { platform: string; handle: string }[] = [];
    if (allAccounts.length === 0 && brandProfile) {
      if (brandProfile.socialInstagram) brandHandles.push({ platform: 'instagram', handle: brandProfile.socialInstagram });
      if (brandProfile.socialLinkedin) brandHandles.push({ platform: 'linkedin', handle: brandProfile.socialLinkedin });
      if (brandProfile.socialTwitter) brandHandles.push({ platform: 'twitter', handle: brandProfile.socialTwitter });
      if (brandProfile.socialTiktok) brandHandles.push({ platform: 'tiktok', handle: brandProfile.socialTiktok });
      if (brandProfile.socialYoutube) brandHandles.push({ platform: 'youtube', handle: brandProfile.socialYoutube });
    }

    return { totalFollowers, scheduled, published, unread, urgent, recentPosts, recentMentions, byPlatform, brandHandles, totalAccounts: allAccounts.length };
  }, [accountQueries, postQueries, mentionQueries, projectEntries, brandProfile]);

  return (
    <div className="space-y-6">
      {/* KPI row */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        {[
          { label: 'Total Followers', value: formatCompactNumber(agg.totalFollowers), icon: Users, color: 'text-blue-500', action: () => onSwitchView('accounts') },
          { label: 'Scheduled Posts', value: agg.scheduled, icon: CalendarDays, color: 'text-purple-500', action: () => onSwitchView('content') },
          { label: 'Unread Mentions', value: agg.unread, icon: Inbox, color: agg.unread > 0 ? 'text-amber-500' : 'text-muted-foreground', action: () => onSwitchView('inbox') },
          { label: 'Published', value: agg.published, icon: CheckCircle, color: 'text-green-500', action: () => onSwitchView('content') },
        ].map(({ label, value, icon: Icon, color, action }) => (
          <Card key={label} className="bg-card/80 backdrop-blur-sm border-border/50 cursor-pointer hover:border-border transition-colors" onClick={action}>
            <CardContent className="pt-5">
              <div className="flex items-start justify-between">
                <div>
                  <p className="text-xs text-muted-foreground mb-1">{label}</p>
                  <p className="text-2xl font-bold">{value}</p>
                </div>
                <Icon className={`h-5 w-5 ${color} mt-0.5`} />
              </div>
            </CardContent>
          </Card>
        ))}
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Platform breakdown */}
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium flex items-center gap-2">
              <Share2 className="h-4 w-4 text-pink-500" />
              Platforms
            </CardTitle>
          </CardHeader>
          <CardContent>
            {Object.keys(agg.byPlatform).length === 0 && agg.brandHandles.length === 0 ? (
              <div className="text-center py-6 text-muted-foreground">
                <Share2 className="h-6 w-6 mx-auto mb-2 opacity-40" />
                <p className="text-xs">No accounts connected</p>
                <button onClick={() => onSwitchView('accounts')} className="mt-2 text-xs text-primary hover:underline">Connect accounts &rarr;</button>
              </div>
            ) : agg.brandHandles.length > 0 ? (
              <div className="space-y-2">
                <p className="text-[10px] text-muted-foreground mb-2">From brand profile</p>
                {agg.brandHandles.map(({ platform, handle }) => {
                  const Icon = PLATFORM_ICONS[platform] || Globe;
                  return (
                    <div key={platform} className="flex items-center gap-2.5 p-2 rounded-lg bg-muted/40">
                      <Icon className={`h-4 w-4 shrink-0 ${PLATFORM_COLORS[platform] || 'text-muted-foreground'}`} />
                      <div className="min-w-0 flex-1">
                        <p className="text-xs font-medium truncate">{handle}</p>
                        <p className="text-[10px] text-muted-foreground capitalize">{platform}</p>
                      </div>
                    </div>
                  );
                })}
              </div>
            ) : (
              <div className="space-y-2">
                {Object.entries(agg.byPlatform).map(([platform, data]) => {
                  const Icon = PLATFORM_ICONS[platform] || Globe;
                  return (
                    <div key={platform} className={`flex items-center gap-2.5 p-2 rounded-lg ${PLATFORM_BG[platform] || 'bg-muted/40'}`}>
                      <Icon className={`h-4 w-4 shrink-0 ${PLATFORM_COLORS[platform] || 'text-muted-foreground'}`} />
                      <div className="min-w-0 flex-1">
                        <p className="text-xs font-medium capitalize">{platform}</p>
                        <p className="text-[10px] text-muted-foreground">{formatCompactNumber(data.followers)} followers</p>
                      </div>
                      <span className="text-[10px] text-muted-foreground">{data.accounts} acct{data.accounts !== 1 ? 's' : ''}</span>
                    </div>
                  );
                })}
              </div>
            )}
          </CardContent>
        </Card>

        {/* Recent posts */}
        <Card className="bg-card/80 backdrop-blur-sm border-border/50 lg:col-span-2">
          <CardHeader className="pb-3">
            <div className="flex items-center justify-between">
              <CardTitle className="text-sm font-medium flex items-center gap-2">
                <FileText className="h-4 w-4 text-purple-500" />
                Recent Content
              </CardTitle>
              <button onClick={() => onSwitchView('content')} className="text-xs text-primary hover:underline">View all &rarr;</button>
            </div>
          </CardHeader>
          <CardContent>
            {agg.recentPosts.length === 0 ? (
              <div className="text-center py-6 text-muted-foreground">
                <FileText className="h-6 w-6 mx-auto mb-2 opacity-40" />
                <p className="text-xs">No posts yet</p>
                <button onClick={() => onSwitchView('content')} className="mt-2 text-xs text-primary hover:underline">Create first post &rarr;</button>
              </div>
            ) : (
              <div className="space-y-2">
                {agg.recentPosts.map(post => {
                  const platforms: string[] = (() => { try { return JSON.parse(post.platforms); } catch { return [post.platforms]; } })();
                  return (
                    <div key={post.id} className="flex items-start gap-3 p-2.5 rounded-lg border border-border/40 hover:bg-muted/30 transition-colors">
                      <div className="flex gap-1 mt-0.5">
                        {platforms.slice(0, 3).map(p => {
                          const Icon = PLATFORM_ICONS[p] || Globe;
                          return <Icon key={p} className={`h-3.5 w-3.5 ${PLATFORM_COLORS[p] || 'text-muted-foreground'}`} />;
                        })}
                      </div>
                      <div className="min-w-0 flex-1">
                        <p className="text-xs line-clamp-1 font-medium">{post.caption || '(no caption)'}</p>
                        <div className="flex items-center gap-2 mt-0.5">
                          <span className={`text-[10px] px-1.5 py-0.5 rounded border ${STATUS_COLORS[post.status] || ''}`}>{post.status}</span>
                          {post.scheduled_for && <span className="text-[10px] text-muted-foreground">{formatDate(post.scheduled_for)}</span>}
                          <span className="text-[10px] text-muted-foreground">{post._project}</span>
                        </div>
                      </div>
                      {(post.likes > 0 || post.impressions > 0) && (
                        <div className="text-right shrink-0">
                          {post.impressions > 0 && <p className="text-[10px] text-muted-foreground">{formatCompactNumber(post.impressions)} views</p>}
                          {post.likes > 0 && <p className="text-[10px] text-muted-foreground">{post.likes} &#9829;</p>}
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      {/* Recent inbox */}
      {agg.recentMentions.length > 0 && (
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-3">
            <div className="flex items-center justify-between">
              <CardTitle className="text-sm font-medium flex items-center gap-2">
                <Inbox className="h-4 w-4 text-amber-500" />
                Recent Inbox
                {agg.unread > 0 && <Badge variant="default" className="text-[10px]">{agg.unread} unread</Badge>}
                {agg.urgent > 0 && <Badge variant="destructive" className="text-[10px]">{agg.urgent} urgent</Badge>}
              </CardTitle>
              <button onClick={() => onSwitchView('inbox')} className="text-xs text-primary hover:underline">View all &rarr;</button>
            </div>
          </CardHeader>
          <CardContent>
            <CardGrid columns={{ md: 2, lg: 3 }} gap={2}>
              {agg.recentMentions.map(m => {
                const Icon = PLATFORM_ICONS[m.platform] || Globe;
                return (
                  <div key={m.id} className={`p-2.5 rounded-lg border border-border/40 ${m.status === 'unread' ? 'bg-accent/20' : ''}`}>
                    <div className="flex items-center gap-2 mb-1">
                      <Icon className={`h-3.5 w-3.5 shrink-0 ${PLATFORM_COLORS[m.platform] || 'text-muted-foreground'}`} />
                      <span className="text-xs font-medium truncate flex-1">{m.author_display_name || m.author_username || 'Unknown'}</span>
                      {m.status === 'unread' && <span className="h-1.5 w-1.5 rounded-full bg-blue-500 shrink-0" />}
                    </div>
                    {m.content && <p className="text-[11px] text-muted-foreground line-clamp-2">{m.content}</p>}
                    <div className="flex items-center gap-1.5 mt-1">
                      <Badge variant="outline" className="text-[9px]">{m.mention_type}</Badge>
                      {m.sentiment && m.sentiment !== 'unknown' && (
                        <span className={`text-[9px] ${SENTIMENT_COLORS[m.sentiment]}`}>{m.sentiment}</span>
                      )}
                    </div>
                  </div>
                );
              })}
            </CardGrid>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
