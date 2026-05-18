import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  AlertTriangle,
  Brain,
  CheckCircle,
  ChevronDown,
  ChevronUp,
  Flame,
  Newspaper,
  Plus,
  Radio,
  Swords,
  TrendingUp,
  Users,
  XCircle,
} from 'lucide-react';
import { useState } from 'react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import {
  type ContentOpportunity,
  socialIntelligenceApi as intelligenceApi,
} from '@/lib/api/social';
import { intelligenceKeys } from '@/lib/query-keys';

// ── Icons ─────────────────────────────────────────────────────────────────────

const OPP_ICONS: Record<string, React.ElementType> = {
  trending_topic: TrendingUp,
  news_event: Newspaper,
  audience_question: Users,
  kol_signal: Flame,
  competitor_gap: Swords,
};

const ENTITY_ICONS: Record<string, React.ElementType> = {
  competitor: Swords,
  kol_influencer: Flame,
  thought_leader: Brain,
  trade_press: Newspaper,
};

const STATUS_COLORS: Record<string, string> = {
  detected: 'bg-blue-500/10 text-blue-500 border-blue-500/20',
  briefed: 'bg-purple-500/10 text-purple-500 border-purple-500/20',
  draft_created: 'bg-yellow-500/10 text-yellow-600 border-yellow-500/20',
  approved: 'bg-green-500/10 text-green-600 border-green-500/20',
  rejected: 'bg-red-500/10 text-red-500 border-red-500/20',
  expired: 'bg-muted text-muted-foreground border-border',
};

// ── Opportunity Card ──────────────────────────────────────────────────────────

function OpportunityCard({
  opp,
  orgId,
  onAction,
}: {
  opp: ContentOpportunity;
  orgId: string;
  onAction: () => void;
}) {
  const [expanded, setExpanded] = useState(false);
  const qc = useQueryClient();

  const approveMutation = useMutation({
    mutationFn: () => intelligenceApi.approveOpportunity(orgId, opp.id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: intelligenceKeys.opportunities(orgId) });
      onAction();
    },
  });

  const rejectMutation = useMutation({
    mutationFn: () => intelligenceApi.rejectOpportunity(orgId, opp.id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: intelligenceKeys.opportunities(orgId) });
      onAction();
    },
  });

  const Icon = OPP_ICONS[opp.type] || Radio;
  const isPending = opp.status === 'detected' || opp.status === 'draft_created';
  const relevancePct = Math.round(opp.relevance_score * 100);

  return (
    <Card className="bg-card/80 border-border/50 transition-all hover:border-border">
      <CardContent className="pt-4 pb-3">
        <div className="flex items-start gap-3">
          <div className="rounded-lg p-2 bg-muted/60 shrink-0">
            <Icon className="h-4 w-4 text-muted-foreground" />
          </div>
          <div className="flex-1 min-w-0">
            <div className="flex items-start justify-between gap-2 flex-wrap">
              <div className="flex items-center gap-2 flex-wrap">
                <span className="text-sm font-medium leading-tight">
                  {opp.title}
                </span>
                <span
                  className={`text-[10px] px-1.5 py-0.5 rounded border font-medium capitalize ${STATUS_COLORS[opp.status] ?? 'bg-muted text-muted-foreground'}`}
                >
                  {opp.status.replace('_', ' ')}
                </span>
              </div>
              <div className="flex items-center gap-1 shrink-0">
                <div
                  className="h-1.5 rounded-full bg-muted overflow-hidden"
                  style={{ width: 40 }}
                  title={`Relevance: ${relevancePct}%`}
                >
                  <div
                    className="h-full rounded-full bg-primary"
                    style={{ width: `${relevancePct}%` }}
                  />
                </div>
                <span className="text-[10px] text-muted-foreground">
                  {relevancePct}%
                </span>
              </div>
            </div>
            <div className="flex items-center gap-2 mt-1 text-[10px] text-muted-foreground flex-wrap">
              <span className="capitalize">{opp.type.replace('_', ' ')}</span>
              {opp.suggested_format && <span>· {opp.suggested_format}</span>}
              {opp.has_draft && (
                <span className="text-green-500">· Draft ready</span>
              )}
              {opp.expires_at && (
                <span>
                  · Expires {new Date(opp.expires_at).toLocaleDateString()}
                </span>
              )}
            </div>

            {expanded && opp.brief && (
              <p className="mt-2 text-xs text-muted-foreground leading-relaxed border-t border-border/40 pt-2">
                {opp.brief}
              </p>
            )}

            {opp.brief && (
              <button
                onClick={() => setExpanded((v) => !v)}
                className="mt-1 flex items-center gap-0.5 text-[10px] text-muted-foreground hover:text-foreground transition-colors"
              >
                {expanded ? (
                  <ChevronUp className="h-3 w-3" />
                ) : (
                  <ChevronDown className="h-3 w-3" />
                )}
                {expanded ? 'Less' : 'More'}
              </button>
            )}
          </div>
        </div>

        {isPending && (
          <div className="flex gap-2 mt-3 justify-end">
            <Button
              size="sm"
              variant="outline"
              className="h-6 text-xs px-2 text-red-500 border-red-500/30 hover:bg-red-500/10"
              onClick={() => rejectMutation.mutate()}
              disabled={rejectMutation.isPending}
            >
              <XCircle className="h-3 w-3 mr-1" />
              Reject
            </Button>
            <Button
              size="sm"
              className="h-6 text-xs px-2"
              onClick={() => approveMutation.mutate()}
              disabled={approveMutation.isPending}
            >
              <CheckCircle className="h-3 w-3 mr-1" />
              Approve
            </Button>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

// ── Add Tracked Entity Dialog ─────────────────────────────────────────────────

function AddEntityDialog({ orgId }: { orgId: string }) {
  const [open, setOpen] = useState(false);
  const [name, setName] = useState('');
  const [entityType, setEntityType] = useState<string>('competitor');
  const [instagram, setInstagram] = useState('');
  const [twitter, setTwitter] = useState('');
  const [linkedin, setLinkedin] = useState('');
  const [notes, setNotes] = useState('');
  const qc = useQueryClient();

  const addMutation = useMutation({
    mutationFn: () =>
      intelligenceApi.addTrackedEntity({
        org_id: orgId,
        name,
        entity_type: entityType as
          | 'competitor'
          | 'kol_influencer'
          | 'thought_leader'
          | 'trade_press',
        instagram_handle: instagram || undefined,
        twitter_handle: twitter || undefined,
        linkedin_url: linkedin || undefined,
        notes: notes || undefined,
      }),
    onSuccess: () => {
      qc.invalidateQueries({
        queryKey: intelligenceKeys.trackedEntities(orgId),
      });
      setOpen(false);
      setName('');
      setInstagram('');
      setTwitter('');
      setLinkedin('');
      setNotes('');
    },
  });

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button size="sm" variant="outline" className="h-7 text-xs gap-1">
          <Plus className="h-3 w-3" />
          Add Source
        </Button>
      </DialogTrigger>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle className="text-sm">Track New Source</DialogTitle>
        </DialogHeader>
        <div className="space-y-3 py-2">
          <div>
            <Label className="text-xs">Name</Label>
            <Input
              className="h-8 text-sm mt-1"
              placeholder="e.g. HubSpot, Gary Vee, TechCrunch"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </div>
          <div>
            <Label className="text-xs">Type</Label>
            <Select value={entityType} onValueChange={setEntityType}>
              <SelectTrigger className="h-8 text-sm mt-1">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="competitor">Competitor</SelectItem>
                <SelectItem value="kol_influencer">KOL / Influencer</SelectItem>
                <SelectItem value="thought_leader">Thought Leader</SelectItem>
                <SelectItem value="trade_press">Trade Press</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div className="grid grid-cols-2 gap-2">
            <div>
              <Label className="text-xs">Instagram handle</Label>
              <Input
                className="h-8 text-sm mt-1"
                placeholder="@handle"
                value={instagram}
                onChange={(e) => setInstagram(e.target.value)}
              />
            </div>
            <div>
              <Label className="text-xs">Twitter/X handle</Label>
              <Input
                className="h-8 text-sm mt-1"
                placeholder="@handle"
                value={twitter}
                onChange={(e) => setTwitter(e.target.value)}
              />
            </div>
          </div>
          <div>
            <Label className="text-xs">LinkedIn URL</Label>
            <Input
              className="h-8 text-sm mt-1"
              placeholder="https://linkedin.com/in/..."
              value={linkedin}
              onChange={(e) => setLinkedin(e.target.value)}
            />
          </div>
          <div>
            <Label className="text-xs">Notes</Label>
            <Textarea
              className="text-sm mt-1 min-h-[60px]"
              placeholder="Why track this source?"
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
            />
          </div>
          <Button
            className="w-full h-8 text-sm"
            onClick={() => addMutation.mutate()}
            disabled={!name || addMutation.isPending}
          >
            {addMutation.isPending ? 'Adding...' : 'Add to Tracking'}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}

// ── Main View ─────────────────────────────────────────────────────────────────

export function SocialIntelligenceView({
  orgId,
}: {
  projectEntries: { id: string; name: string }[];
  orgId?: string;
}) {
  const [statusFilter, setStatusFilter] = useState<string>('active');
  const [typeFilter, setTypeFilter] = useState<string>('');
  const [entityTypeFilter, setEntityTypeFilter] = useState<string>('');

  const activeOrg = orgId ?? '';

  const apiStatus = statusFilter === 'active' ? undefined : statusFilter;
  const apiType = typeFilter || undefined;

  const { data: oppsData, isLoading: oppsLoading } = useQuery({
    queryKey: intelligenceKeys.opportunities(activeOrg, apiStatus, apiType),
    queryFn: () =>
      intelligenceApi.listOpportunities({
        orgId: activeOrg,
        status: apiStatus,
        opportunityType: apiType,
        limit: 20,
      }),
    enabled: !!activeOrg,
    staleTime: 60_000,
  });

  const { data: entitiesData, isLoading: entitiesLoading } = useQuery({
    queryKey: intelligenceKeys.trackedEntities(
      activeOrg,
      entityTypeFilter || undefined
    ),
    queryFn: () =>
      intelligenceApi.listTrackedEntities(
        activeOrg,
        entityTypeFilter || undefined
      ),
    enabled: !!activeOrg,
    staleTime: 120_000,
  });

  const { data: insightsData } = useQuery({
    queryKey: intelligenceKeys.audienceInsights(activeOrg, 30),
    queryFn: () => intelligenceApi.getAudienceInsights(activeOrg, 30),
    enabled: !!activeOrg,
    staleTime: 120_000,
  });

  const { data: sovData } = useQuery({
    queryKey: intelligenceKeys.shareOfVoice(activeOrg, 30),
    queryFn: () => intelligenceApi.getShareOfVoice(activeOrg, 30),
    enabled: !!activeOrg,
    staleTime: 300_000,
  });

  const qc = useQueryClient();

  if (!activeOrg) {
    return (
      <div className="text-center py-16 text-muted-foreground">
        <Brain className="h-10 w-10 mx-auto mb-3 opacity-30" />
        <p className="text-sm">Select an organization to view intelligence</p>
      </div>
    );
  }

  const opportunities = oppsData?.opportunities ?? [];
  const entities = entitiesData?.entities ?? [];
  const insights = insightsData?.insights ?? [];
  const sovPeriods = sovData?.periods ?? [];
  const latestSov = sovPeriods[0];

  const pendingCount = opportunities.filter(
    (o) => o.status === 'detected' || o.status === 'draft_created'
  ).length;

  return (
    <div className="space-y-6">
      {/* SOV + Insights summary bar */}
      {(latestSov || insights.length > 0) && (
        <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
          {latestSov && (
            <>
              <Card className="bg-card/80 border-border/50">
                <CardContent className="pt-4 pb-3">
                  <p className="text-[10px] text-muted-foreground mb-1 uppercase tracking-wide">
                    Share of Voice
                  </p>
                  <p className="text-xl font-semibold">
                    {latestSov.share_of_voice_pct ?? '—'}
                  </p>
                  <p className="text-[10px] text-muted-foreground mt-0.5">
                    {latestSov.own_mentions} / {latestSov.total_niche_mentions}{' '}
                    mentions
                  </p>
                </CardContent>
              </Card>
              <Card className="bg-card/80 border-border/50">
                <CardContent className="pt-4 pb-3">
                  <p className="text-[10px] text-muted-foreground mb-1 uppercase tracking-wide">
                    Niche Mentions
                  </p>
                  <p className="text-xl font-semibold">
                    {latestSov.total_niche_mentions.toLocaleString()}
                  </p>
                  <p className="text-[10px] text-muted-foreground mt-0.5">
                    last 7 days
                  </p>
                </CardContent>
              </Card>
            </>
          )}
          <Card className="bg-card/80 border-border/50">
            <CardContent className="pt-4 pb-3">
              <p className="text-[10px] text-muted-foreground mb-1 uppercase tracking-wide">
                Opportunities
              </p>
              <p className="text-xl font-semibold">{opportunities.length}</p>
              {pendingCount > 0 && (
                <p className="text-[10px] text-yellow-500 mt-0.5 flex items-center gap-1">
                  <AlertTriangle className="h-3 w-3" />
                  {pendingCount} need review
                </p>
              )}
            </CardContent>
          </Card>
          <Card className="bg-card/80 border-border/50">
            <CardContent className="pt-4 pb-3">
              <p className="text-[10px] text-muted-foreground mb-1 uppercase tracking-wide">
                Tracked Sources
              </p>
              <p className="text-xl font-semibold">{entities.length}</p>
              <p className="text-[10px] text-muted-foreground mt-0.5">
                competitors · KOLs · press
              </p>
            </CardContent>
          </Card>
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Opportunities panel (2/3 width) */}
        <div className="lg:col-span-2 space-y-3">
          <div className="flex items-center justify-between flex-wrap gap-2">
            <h3 className="text-sm font-medium flex items-center gap-2">
              <Radio className="h-4 w-4 text-primary" />
              Content Opportunities
              {pendingCount > 0 && (
                <Badge variant="secondary" className="text-[10px] h-4 px-1.5">
                  {pendingCount} pending
                </Badge>
              )}
            </h3>
            <div className="flex items-center gap-2">
              <Select value={statusFilter} onValueChange={setStatusFilter}>
                <SelectTrigger className="h-7 text-xs w-[120px]">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="active">Active</SelectItem>
                  <SelectItem value="detected">Detected</SelectItem>
                  <SelectItem value="draft_created">Draft Ready</SelectItem>
                  <SelectItem value="approved">Approved</SelectItem>
                  <SelectItem value="rejected">Rejected</SelectItem>
                </SelectContent>
              </Select>
              <Select value={typeFilter} onValueChange={setTypeFilter}>
                <SelectTrigger className="h-7 text-xs w-[130px]">
                  <SelectValue placeholder="All types" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="">All types</SelectItem>
                  <SelectItem value="trending_topic">Trending</SelectItem>
                  <SelectItem value="news_event">News</SelectItem>
                  <SelectItem value="kol_signal">KOL Signal</SelectItem>
                  <SelectItem value="competitor_gap">Competitor Gap</SelectItem>
                  <SelectItem value="audience_question">Audience Q</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>

          {oppsLoading ? (
            <div className="space-y-2">
              {[1, 2, 3].map((i) => (
                <div
                  key={i}
                  className="h-20 rounded-lg bg-muted/40 animate-pulse"
                />
              ))}
            </div>
          ) : opportunities.length === 0 ? (
            <Card className="bg-card/80 border-border/50">
              <CardContent className="py-10 text-center">
                <Radio className="h-8 w-8 mx-auto mb-2 text-muted-foreground opacity-40" />
                <p className="text-sm text-muted-foreground">
                  No opportunities found
                </p>
                <p className="text-xs text-muted-foreground mt-1">
                  Signal processors run hourly. Add tracked sources to start
                  detecting opportunities.
                </p>
              </CardContent>
            </Card>
          ) : (
            <ScrollArea className="max-h-[500px]">
              <div className="space-y-2 pr-1">
                {opportunities.map((opp) => (
                  <OpportunityCard
                    key={opp.id}
                    opp={opp}
                    orgId={activeOrg}
                    onAction={() =>
                      qc.invalidateQueries({
                        queryKey: intelligenceKeys.opportunities(activeOrg),
                      })
                    }
                  />
                ))}
              </div>
            </ScrollArea>
          )}

          {/* Audience Insights */}
          {insights.length > 0 && (
            <div className="mt-4">
              <h4 className="text-xs font-medium text-muted-foreground uppercase tracking-wide mb-2 flex items-center gap-1.5">
                <Users className="h-3.5 w-3.5" /> Audience Signals
              </h4>
              <div className="grid grid-cols-2 gap-2">
                {insights.slice(0, 4).map((insight, i) => (
                  <Card key={i} className="bg-card/60 border-border/40">
                    <CardContent className="pt-3 pb-2">
                      <div className="flex items-center justify-between gap-1 mb-1">
                        <span className="text-[10px] text-muted-foreground capitalize">
                          {insight.type.replace('_', ' ')}
                        </span>
                        <span className="text-[10px] font-medium">
                          {insight.mention_count}×
                        </span>
                      </div>
                      <p className="text-xs font-medium line-clamp-1">
                        {insight.topic}
                      </p>
                      {insight.content_angle && (
                        <p className="text-[10px] text-muted-foreground mt-0.5 line-clamp-1">
                          → {insight.content_angle}
                        </p>
                      )}
                    </CardContent>
                  </Card>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Tracked Sources panel (1/3 width) */}
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <h3 className="text-sm font-medium flex items-center gap-2">
              <Swords className="h-4 w-4 text-muted-foreground" />
              Tracked Sources
            </h3>
            <AddEntityDialog orgId={activeOrg} />
          </div>

          <Select value={entityTypeFilter} onValueChange={setEntityTypeFilter}>
            <SelectTrigger className="h-7 text-xs">
              <SelectValue placeholder="All types" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="">All types</SelectItem>
              <SelectItem value="competitor">Competitors</SelectItem>
              <SelectItem value="kol_influencer">KOL / Influencers</SelectItem>
              <SelectItem value="thought_leader">Thought Leaders</SelectItem>
              <SelectItem value="trade_press">Trade Press</SelectItem>
            </SelectContent>
          </Select>

          {entitiesLoading ? (
            <div className="space-y-2">
              {[1, 2].map((i) => (
                <div
                  key={i}
                  className="h-14 rounded-lg bg-muted/40 animate-pulse"
                />
              ))}
            </div>
          ) : entities.length === 0 ? (
            <Card className="bg-card/80 border-border/50">
              <CardContent className="py-8 text-center">
                <Swords className="h-7 w-7 mx-auto mb-2 text-muted-foreground opacity-30" />
                <p className="text-xs text-muted-foreground">
                  No tracked sources yet
                </p>
                <p className="text-[10px] text-muted-foreground mt-1">
                  Add competitors, KOLs, and press to monitor
                </p>
              </CardContent>
            </Card>
          ) : (
            <ScrollArea className="max-h-[500px]">
              <div className="space-y-2 pr-1">
                {entities.map((entity) => {
                  const Icon = ENTITY_ICONS[entity.type] || Radio;
                  return (
                    <Card
                      key={entity.id}
                      className="bg-card/80 border-border/40"
                    >
                      <CardContent className="pt-3 pb-2">
                        <div className="flex items-center gap-2">
                          <Icon className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                          <div className="flex-1 min-w-0">
                            <p className="text-xs font-medium truncate">
                              {entity.name}
                            </p>
                            <p className="text-[10px] text-muted-foreground capitalize">
                              {entity.type.replace('_', ' ')}
                            </p>
                          </div>
                          <div
                            className="h-1 rounded-full bg-muted overflow-hidden shrink-0"
                            style={{ width: 28 }}
                            title={`Relevance: ${Math.round(entity.relevance * 100)}%`}
                          >
                            <div
                              className="h-full rounded-full bg-primary"
                              style={{ width: `${entity.relevance * 100}%` }}
                            />
                          </div>
                        </div>
                        {(entity.instagram || entity.twitter) && (
                          <div className="flex gap-2 mt-1 text-[10px] text-muted-foreground">
                            {entity.instagram && (
                              <span>@{entity.instagram}</span>
                            )}
                            {entity.twitter && <span>@{entity.twitter}</span>}
                          </div>
                        )}
                      </CardContent>
                    </Card>
                  );
                })}
              </div>
            </ScrollArea>
          )}

          {/* Share of Voice history */}
          {sovPeriods.length > 1 && (
            <div className="mt-2">
              <h4 className="text-[10px] font-medium text-muted-foreground uppercase tracking-wide mb-2">
                SOV History
              </h4>
              <div className="space-y-1">
                {sovPeriods.slice(0, 4).map((period, i) => (
                  <div
                    key={i}
                    className="flex items-center justify-between text-xs"
                  >
                    <span className="text-muted-foreground text-[10px]">
                      {new Date(period.period_end).toLocaleDateString()}
                    </span>
                    <span className="font-medium">
                      {period.share_of_voice_pct ?? '0%'}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
