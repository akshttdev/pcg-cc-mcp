import { useState, useMemo } from 'react';
import { useQueries, useMutation, useQueryClient } from '@tanstack/react-query';
import { Card, CardContent } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Globe, Inbox } from 'lucide-react';
import { socialApi, type SocialMentionRecord } from '@/lib/api';
import { socialKeys } from '@/lib/query-keys';
import {
  PLATFORM_ICONS,
  PLATFORM_COLORS,
  PLATFORM_BG,
  STATUS_COLORS,
  SENTIMENT_COLORS,
  PRIORITY_COLORS,
} from '../../constants';
import { formatDate } from '../../helpers';

export function SocialInboxView({ projectEntries }: { projectEntries: { id: string; name: string }[] }) {
  const [statusFilter, setStatusFilter] = useState<string>('all');
  const [platformFilter, setPlatformFilter] = useState<string>('all');

  const mentionQueries = useQueries({
    queries: projectEntries.map(e => ({
      queryKey: socialKeys.mentions(e.id),
      queryFn: () => socialApi.listMentions(e.id, { limit: 50 }),
      staleTime: 30_000,
    })),
  });

  const allMentions = useMemo(() => {
    const list: (SocialMentionRecord & { _project: string })[] = [];
    mentionQueries.forEach((q, i) => {
      q.data?.forEach(m => list.push({ ...m, _project: projectEntries[i].name }));
    });
    return list.sort((a, b) => new Date(b.received_at).getTime() - new Date(a.received_at).getTime());
  }, [mentionQueries, projectEntries]);

  const platforms = useMemo(() => [...new Set(allMentions.map(m => m.platform))], [allMentions]);

  const filtered = useMemo(() => {
    return allMentions.filter(m => {
      if (statusFilter === 'unread' && m.status !== 'unread') return false;
      if (statusFilter === 'urgent' && m.priority !== 'urgent' && m.priority !== 'high') return false;
      if (statusFilter === 'replied' && m.status !== 'replied') return false;
      if (statusFilter === 'archived' && m.status !== 'archived') return false;
      if (platformFilter !== 'all' && m.platform !== platformFilter) return false;
      return true;
    });
  }, [allMentions, statusFilter, platformFilter]);

  const counts = useMemo(() => ({
    all: allMentions.length,
    unread: allMentions.filter(m => m.status === 'unread').length,
    urgent: allMentions.filter(m => m.priority === 'urgent' || m.priority === 'high').length,
    replied: allMentions.filter(m => m.status === 'replied').length,
    archived: allMentions.filter(m => m.status === 'archived').length,
  }), [allMentions]);

  const queryClient = useQueryClient();
  const updateMut = useMutation({
    mutationFn: ({ id, data }: { id: string; data: Parameters<typeof socialApi.updateMention>[1] }) =>
      socialApi.updateMention(id, data),
    onSuccess: () => {
      projectEntries.forEach(e => queryClient.invalidateQueries({ queryKey: socialKeys.mentions(e.id) }));
    },
  });

  const INBOX_TABS = [
    { key: 'all', label: 'All' },
    { key: 'unread', label: 'Unread' },
    { key: 'urgent', label: 'Urgent' },
    { key: 'replied', label: 'Replied' },
    { key: 'archived', label: 'Archived' },
  ];

  return (
    <div className="space-y-5">
      <div className="flex items-center justify-between gap-4 flex-wrap">
        <div className="flex gap-1 p-1 bg-muted/50 rounded-lg">
          {INBOX_TABS.map(({ key, label }) => (
            <button key={key} onClick={() => setStatusFilter(key)}
              className={`px-3 py-1 text-xs font-medium rounded-md transition-all ${statusFilter === key ? 'bg-background text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'}`}>
              {label}
              {(counts as any)[key] > 0 && <span className="ml-1.5 text-[10px] text-muted-foreground">{(counts as any)[key]}</span>}
            </button>
          ))}
        </div>
        {platforms.length > 1 && (
          <select value={platformFilter} onChange={e => setPlatformFilter(e.target.value)}
            className="text-xs border border-border rounded-md px-2 py-1.5 bg-background">
            <option value="all">All platforms</option>
            {platforms.map(p => <option key={p} value={p} className="capitalize">{p}</option>)}
          </select>
        )}
      </div>

      {filtered.length === 0 ? (
        <div className="text-center py-12 text-muted-foreground">
          <Inbox className="h-8 w-8 mx-auto mb-2 opacity-40" />
          <p>{statusFilter === 'all' ? 'No mentions yet' : `No ${statusFilter} mentions`}</p>
        </div>
      ) : (
        <div className="space-y-2">
          {filtered.map(mention => {
            const Icon = PLATFORM_ICONS[mention.platform] || Globe;
            const isUnread = mention.status === 'unread';
            return (
              <Card key={mention.id} className={`backdrop-blur-sm border-border/50 transition-colors ${isUnread ? 'bg-accent/10 border-accent/30' : 'bg-card/80'}`}>
                <CardContent className="pt-3 pb-3">
                  <div className="flex items-start gap-3">
                    <div className={`h-8 w-8 rounded-full flex items-center justify-center shrink-0 mt-0.5 ${PLATFORM_BG[mention.platform] || 'bg-muted'}`}>
                      <Icon className={`h-4 w-4 ${PLATFORM_COLORS[mention.platform] || 'text-muted-foreground'}`} />
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-2 flex-wrap">
                        <span className="text-sm font-medium">{mention.author_display_name || mention.author_username || 'Unknown'}</span>
                        {mention.author_is_verified && <span className="text-[10px] text-blue-500">&#10003; verified</span>}
                        <Badge variant="outline" className="text-[9px]">{mention.mention_type}</Badge>
                        <span className={`text-[10px] font-medium ${PRIORITY_COLORS[mention.priority]}`}>{mention.priority !== 'normal' ? mention.priority : ''}</span>
                        {isUnread && <span className="h-1.5 w-1.5 rounded-full bg-blue-500" />}
                      </div>
                      {mention.content && <p className="text-sm text-muted-foreground mt-1 line-clamp-3">{mention.content}</p>}
                      {mention.reply_content && (
                        <div className="mt-2 pl-3 border-l-2 border-primary/30">
                          <p className="text-xs text-muted-foreground">Reply: {mention.reply_content}</p>
                        </div>
                      )}
                      <div className="flex items-center gap-2 mt-2 flex-wrap">
                        {mention.sentiment && mention.sentiment !== 'unknown' && (
                          <span className={`text-[10px] ${SENTIMENT_COLORS[mention.sentiment]}`}>&#9679; {mention.sentiment}</span>
                        )}
                        <span className="text-[10px] text-muted-foreground">{formatDate(mention.received_at)}</span>
                        <Badge variant="outline" className="text-[9px]">{mention._project}</Badge>
                        <span className={`text-[10px] px-1.5 py-0.5 rounded border ${STATUS_COLORS[mention.status] || 'border-border text-muted-foreground'}`}>{mention.status}</span>
                      </div>
                    </div>
                    <div className="flex flex-col gap-1 shrink-0">
                      {isUnread && (
                        <button onClick={() => updateMut.mutate({ id: mention.id, data: { status: 'read' } })}
                          className="text-[10px] px-2 py-1 rounded border border-border hover:bg-muted transition-colors" title="Mark read">
                          Mark read
                        </button>
                      )}
                      {mention.status !== 'archived' && (
                        <button onClick={() => updateMut.mutate({ id: mention.id, data: { status: 'archived' } })}
                          className="text-[10px] px-2 py-1 rounded border border-border hover:bg-muted transition-colors">
                          Archive
                        </button>
                      )}
                      {mention.status !== 'replied' && (
                        <button onClick={() => updateMut.mutate({ id: mention.id, data: { status: 'flagged', priority: 'high' } })}
                          className="text-[10px] px-2 py-1 rounded border border-amber-300 hover:bg-amber-50 dark:hover:bg-amber-950/20 text-amber-600 transition-colors">
                          Flag
                        </button>
                      )}
                    </div>
                  </div>
                </CardContent>
              </Card>
            );
          })}
        </div>
      )}
    </div>
  );
}
