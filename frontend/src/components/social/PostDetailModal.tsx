import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  BarChart3,
  Calendar,
  ExternalLink,
  Facebook,
  Globe,
  Image,
  Instagram,
  Linkedin,
  Loader2,
  RefreshCw,
  Settings2,
  Sparkles,
  Trash2,
  Twitter,
  Upload,
  X,
  Youtube,
} from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { ResizableDrawer } from '@/components/ui/resizable-drawer';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Textarea } from '@/components/ui/textarea';
import { useAuth } from '@/contexts/AuthContext';
import { usersApi } from '@/lib/api/execution';
import { socialApi } from '@/lib/api/social';
import { cn } from '@/lib/utils';

// ── Platform config ───────────────────────────────────────────────────────────

const PLATFORMS = [
  {
    id: 'linkedin',
    label: 'LinkedIn',
    Icon: Linkedin,
    color: 'text-blue-600',
    charLimit: 3000,
    specific: ['article_url'],
  },
  {
    id: 'instagram',
    label: 'Instagram',
    Icon: Instagram,
    color: 'text-pink-500',
    charLimit: 2200,
    specific: ['alt_text', 'location', 'first_comment'],
  },
  {
    id: 'twitter',
    label: 'Twitter / X',
    Icon: Twitter,
    color: 'text-sky-400',
    charLimit: 280,
    specific: ['thread_mode'],
  },
  {
    id: 'facebook',
    label: 'Facebook',
    Icon: Facebook,
    color: 'text-blue-700',
    charLimit: 63206,
    specific: ['cta_type'],
  },
  {
    id: 'tiktok',
    label: 'TikTok',
    Icon: Globe,
    color: 'text-foreground',
    charLimit: 2200,
    specific: [],
  },
  {
    id: 'youtube',
    label: 'YouTube',
    Icon: Youtube,
    color: 'text-red-500',
    charLimit: 5000,
    specific: ['title', 'tags', 'visibility'],
  },
] as const;

const STATUS_OPTIONS = [
  { value: 'draft', label: 'Draft', color: 'bg-muted text-muted-foreground' },
  {
    value: 'pending_review',
    label: 'Pending Review',
    color:
      'bg-yellow-500/15 text-yellow-700 dark:text-yellow-400 border-yellow-300/50',
  },
  {
    value: 'approved',
    label: 'Approved',
    color: 'bg-blue-500/15 text-blue-700 dark:text-blue-400',
  },
  {
    value: 'scheduled',
    label: 'Scheduled',
    color: 'bg-purple-500/15 text-purple-700 dark:text-purple-400',
  },
  {
    value: 'published',
    label: 'Published',
    color: 'bg-green-500/15 text-green-700 dark:text-green-400',
  },
] as const;

const CATEGORIES = [
  'community',
  'events',
  'vibe',
  'drinks',
  'promo',
  'education',
  'announcement',
] as const;

// ── Helpers ───────────────────────────────────────────────────────────────────

function parseTags(raw: string | null | undefined): string[] {
  if (!raw) return [];
  try {
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

function parseMediaUrls(raw: string | null | undefined): string[] {
  return parseTags(raw);
}

function parsePlatforms(raw: string | null | undefined): string[] {
  return parseTags(raw);
}

function parsePlatformSpecific(
  raw: string | null | undefined
): Record<string, Record<string, string>> {
  if (!raw) return {};
  try {
    return JSON.parse(raw) as Record<string, Record<string, string>>;
  } catch {
    return {};
  }
}

function fmtScheduled(iso: string | null | undefined): string {
  if (!iso) return '';
  const d = new Date(iso);
  return d.toISOString().slice(0, 16);
}

function MetricCard({
  label,
  value,
}: {
  label: string;
  value: number | string;
}) {
  return (
    <div className="rounded-lg border p-4 text-center">
      <p className="text-2xl font-bold tabular-nums">
        {typeof value === 'number' ? value.toLocaleString() : value}
      </p>
      <p className="text-xs text-muted-foreground mt-1">{label}</p>
    </div>
  );
}

// ── Main component ────────────────────────────────────────────────────────────

interface Props {
  postId: string | null;
  open: boolean;
  onClose: () => void;
  onDeleted?: () => void;
}

export function PostDetailModal({ postId, open, onClose, onDeleted }: Props) {
  const queryClient = useQueryClient();
  const { user } = useAuth();

  const { data: post, isLoading } = useQuery({
    queryKey: ['social-post', postId],
    queryFn: () => socialApi.getPost(postId!),
    enabled: !!postId && open,
  });

  const { data: users = [] } = useQuery({
    queryKey: ['users-list'],
    queryFn: () => usersApi.list(),
    enabled: open,
  });

  // ── Editable state ──────────────────────────────────────────────────────────
  const [caption, setCaption] = useState('');
  const [hashtags, setHashtags] = useState<string[]>([]);
  const [hashtagInput, setHashtagInput] = useState('');
  const [mentions, setMentions] = useState<string[]>([]);
  const [mentionInput, setMentionInput] = useState('');
  const [selectedPlatforms, setSelectedPlatforms] = useState<string[]>([]);
  const [platformSpecific, setPlatformSpecific] = useState<
    Record<string, Record<string, string>>
  >({});
  const [mediaUrls, setMediaUrls] = useState<string[]>([]);
  const [scheduledFor, setScheduledFor] = useState('');
  const [status, setStatus] = useState('draft');
  const [category, setCategory] = useState('community');
  const [assigneeId, setAssigneeId] = useState('');
  const [isEvergreen, setIsEvergreen] = useState(false);
  const [recycleAfterDays, setRecycleAfterDays] = useState('');

  // ── AI caption state ────────────────────────────────────────────────────────
  const [aiTopic, setAiTopic] = useState('');
  const [aiOptions, setAiOptions] = useState<string[]>([]);
  const [generatingAi, setGeneratingAi] = useState(false);

  // ── Upload state ────────────────────────────────────────────────────────────
  const [uploading, setUploading] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [dragActive, setDragActive] = useState(false);

  // ── Populate form when post loads ──────────────────────────────────────────
  useEffect(() => {
    if (!post) return;
    setCaption(post.caption ?? '');
    setHashtags(parseTags(post.hashtags));
    setMentions(parseTags(post.mentions));
    setSelectedPlatforms(parsePlatforms(post.platforms));
    setPlatformSpecific(parsePlatformSpecific(post.platform_specific));
    setMediaUrls(parseMediaUrls(post.media_urls));
    setScheduledFor(fmtScheduled(post.scheduled_for));
    setStatus(post.status);
    setCategory(post.category ?? 'community');
    setAssigneeId(post.assignee_id ?? '');
    setIsEvergreen(post.is_evergreen ?? false);
    setRecycleAfterDays(post.recycle_after_days?.toString() ?? '');
    setAiOptions([]);
    setAiTopic('');
  }, [post]);

  // ── Save mutation ────────────────────────────────────────────────────────────
  const saveMutation = useMutation({
    mutationFn: async () => {
      if (!postId) return;
      return socialApi.updatePost(postId, {
        caption,
        hashtags,
        mentions,
        platforms: selectedPlatforms,
        platform_specific: platformSpecific as Record<string, unknown>,
        media_urls: mediaUrls,
        scheduled_for: scheduledFor
          ? new Date(scheduledFor).toISOString()
          : null,
        status,
        category,
        assignee_id: assigneeId || null,
        is_evergreen: isEvergreen,
        recycle_after_days: recycleAfterDays
          ? parseInt(recycleAfterDays)
          : null,
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['social-post', postId] });
      queryClient.invalidateQueries({ queryKey: ['calendar-posts'] });
      queryClient.invalidateQueries({ queryKey: ['social-posts'] });
      onClose();
    },
  });

  // ── Delete mutation ──────────────────────────────────────────────────────────
  const deleteMutation = useMutation({
    mutationFn: () => socialApi.deletePost(postId!),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['calendar-posts'] });
      queryClient.invalidateQueries({ queryKey: ['social-posts'] });
      onDeleted?.();
      onClose();
    },
  });

  // ── Media upload ─────────────────────────────────────────────────────────────
  const handleFiles = useCallback(
    async (files: FileList | null) => {
      if (!files || !postId || !post) return;
      setUploading(true);
      try {
        const newUrls: string[] = [];
        for (const file of Array.from(files)) {
          const result = await socialApi.uploadMedia({
            projectId: post.project_id,
            file,
            socialPostId: postId,
            uploadedBy: user?.id,
          });
          newUrls.push(result.public_url);
        }
        setMediaUrls((prev) => [...prev, ...newUrls]);
      } catch (err) {
        console.error('Upload failed', err);
      } finally {
        setUploading(false);
      }
    },
    [postId, post, user]
  );

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      setDragActive(false);
      handleFiles(e.dataTransfer.files);
    },
    [handleFiles]
  );

  // ── AI captions ──────────────────────────────────────────────────────────────
  async function handleGenerateAi() {
    if (!aiTopic.trim() || !selectedPlatforms.length) return;
    setGeneratingAi(true);
    setAiOptions([]);
    try {
      const captions = await socialApi.generateCaptions({
        platform: selectedPlatforms[0],
        topic: aiTopic.trim(),
      });
      setAiOptions(captions);
    } finally {
      setGeneratingAi(false);
    }
  }

  // ── Tag helpers ──────────────────────────────────────────────────────────────
  function addTag(
    input: string,
    setter: React.Dispatch<React.SetStateAction<string[]>>,
    inputSetter: React.Dispatch<React.SetStateAction<string>>
  ) {
    const tag = input.trim().replace(/^[#@]/, '');
    if (tag) setter((prev) => (prev.includes(tag) ? prev : [...prev, tag]));
    inputSetter('');
  }

  function removeTag(
    tag: string,
    setter: React.Dispatch<React.SetStateAction<string[]>>
  ) {
    setter((prev) => prev.filter((t) => t !== tag));
  }

  const isPublished = status === 'published';
  const statusCfg = STATUS_OPTIONS.find(
    (s) => s.value === (post?.status ?? status)
  );

  return (
    <ResizableDrawer
      open={open}
      onClose={onClose}
      defaultWidth={780}
      minWidth={520}
      storageKey="social:post-drawer-width"
    >
      <div className="flex flex-col h-full bg-background border-l shadow-xl">
        {/* ── Header ── */}
        <div className="flex items-start justify-between px-6 pt-5 pb-4 border-b shrink-0">
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 flex-wrap mb-1.5">
              {post &&
                parsePlatforms(post.platforms).map((p) => {
                  const cfg = PLATFORMS.find((x) => x.id === p);
                  if (!cfg) return null;
                  const { Icon, color } = cfg;
                  return (
                    <span
                      key={p}
                      className={`flex items-center gap-1 text-xs font-medium ${color}`}
                    >
                      <Icon className="h-3.5 w-3.5" />
                      {cfg.label}
                    </span>
                  );
                })}
              {statusCfg && (
                <Badge
                  variant="outline"
                  className={cn('text-xs', statusCfg.color)}
                >
                  {statusCfg.label}
                </Badge>
              )}
            </div>
            <h2 className="text-base font-semibold leading-tight line-clamp-2 text-foreground">
              {post?.caption
                ? post.caption.slice(0, 80) +
                  (post.caption.length > 80 ? '…' : '')
                : 'Social Post'}
            </h2>
            {post?.scheduled_for && (
              <p className="text-xs text-muted-foreground mt-0.5">
                Scheduled: {new Date(post.scheduled_for).toLocaleString()}
              </p>
            )}
          </div>

          <div className="flex items-center gap-1 ml-4 shrink-0">
            {post?.platform_url && (
              <Button
                variant="ghost"
                size="sm"
                className="h-8 gap-1.5 text-xs"
                onClick={() => window.open(post.platform_url!, '_blank')}
              >
                <ExternalLink className="h-3.5 w-3.5" />
                View Live
              </Button>
            )}
            <Button
              variant="ghost"
              size="sm"
              className="h-8 w-8 p-0 text-destructive hover:text-destructive hover:bg-destructive/10"
              onClick={() => deleteMutation.mutate()}
              disabled={deleteMutation.isPending}
            >
              <Trash2 className="h-4 w-4" />
            </Button>
            <Button
              variant="ghost"
              size="sm"
              className="h-8 w-8 p-0"
              onClick={onClose}
            >
              <X className="h-4 w-4" />
            </Button>
          </div>
        </div>

        {/* ── Body ── */}
        {isLoading || !post ? (
          <div className="flex-1 flex items-center justify-center">
            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
          </div>
        ) : (
          <Tabs defaultValue="content" className="flex-1 flex flex-col min-h-0">
            <div className="px-6 pt-3 shrink-0">
              <TabsList className="w-full justify-start gap-0 h-9 bg-transparent border-b rounded-none p-0">
                <TabsTrigger
                  value="content"
                  className="rounded-none border-b-2 border-transparent data-[state=active]:border-primary data-[state=active]:bg-transparent data-[state=active]:shadow-none gap-1.5 text-xs h-9 px-4"
                >
                  <Sparkles className="h-3.5 w-3.5" />
                  Content
                </TabsTrigger>
                <TabsTrigger
                  value="media"
                  className="rounded-none border-b-2 border-transparent data-[state=active]:border-primary data-[state=active]:bg-transparent data-[state=active]:shadow-none gap-1.5 text-xs h-9 px-4"
                >
                  <Image className="h-3.5 w-3.5" />
                  Media
                  {mediaUrls.length > 0 && (
                    <Badge
                      variant="secondary"
                      className="h-4 px-1 text-xs ml-0.5"
                    >
                      {mediaUrls.length}
                    </Badge>
                  )}
                </TabsTrigger>
                <TabsTrigger
                  value="platforms"
                  className="rounded-none border-b-2 border-transparent data-[state=active]:border-primary data-[state=active]:bg-transparent data-[state=active]:shadow-none gap-1.5 text-xs h-9 px-4"
                >
                  <Settings2 className="h-3.5 w-3.5" />
                  Platforms
                  {selectedPlatforms.length > 0 && (
                    <Badge
                      variant="secondary"
                      className="h-4 px-1 text-xs ml-0.5"
                    >
                      {selectedPlatforms.length}
                    </Badge>
                  )}
                </TabsTrigger>
                <TabsTrigger
                  value="schedule"
                  className="rounded-none border-b-2 border-transparent data-[state=active]:border-primary data-[state=active]:bg-transparent data-[state=active]:shadow-none gap-1.5 text-xs h-9 px-4"
                >
                  <Calendar className="h-3.5 w-3.5" />
                  Schedule
                </TabsTrigger>
                <TabsTrigger
                  value="performance"
                  className="rounded-none border-b-2 border-transparent data-[state=active]:border-primary data-[state=active]:bg-transparent data-[state=active]:shadow-none gap-1.5 text-xs h-9 px-4"
                  disabled={!isPublished}
                >
                  <BarChart3 className="h-3.5 w-3.5" />
                  Performance
                  {!isPublished && (
                    <span className="text-muted-foreground/50 text-xs ml-0.5 hidden sm:inline">
                      (post-publish)
                    </span>
                  )}
                </TabsTrigger>
              </TabsList>
            </div>

            <ScrollArea className="flex-1">
              {/* ── CONTENT TAB ── */}
              <TabsContent value="content" className="mt-0 px-6 py-5 space-y-5">
                {/* Media preview strip (if media exists) */}
                {mediaUrls.length > 0 && (
                  <div className="flex gap-2 overflow-x-auto pb-1">
                    {mediaUrls.map((url, i) => {
                      const isVideo =
                        url.match(/\.(mp4|mov|webm|avi)$/i) !== null;
                      return (
                        <div
                          key={i}
                          className="shrink-0 w-20 h-20 rounded-lg overflow-hidden border bg-muted"
                        >
                          {isVideo ? (
                            <video
                              src={url}
                              className="w-full h-full object-cover"
                              muted
                            />
                          ) : (
                            <img
                              src={url}
                              alt=""
                              className="w-full h-full object-cover"
                            />
                          )}
                        </div>
                      );
                    })}
                  </div>
                )}

                {/* AI Generator */}
                <div className="rounded-xl border border-dashed border-border p-4 bg-muted/20 space-y-3">
                  <Label className="text-xs text-muted-foreground flex items-center gap-1.5">
                    <Sparkles className="h-3.5 w-3.5 text-primary" />
                    AI Caption Generator
                  </Label>
                  <div className="flex gap-2">
                    <Input
                      placeholder="Topic (e.g. 'announcing our new venue partnership')"
                      value={aiTopic}
                      onChange={(e) => setAiTopic(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter') {
                          e.preventDefault();
                          handleGenerateAi();
                        }
                      }}
                      className="h-9 text-sm"
                    />
                    <Button
                      type="button"
                      size="sm"
                      variant="secondary"
                      className="h-9 shrink-0 px-4"
                      onClick={handleGenerateAi}
                      disabled={
                        generatingAi ||
                        !aiTopic.trim() ||
                        !selectedPlatforms.length
                      }
                    >
                      {generatingAi ? (
                        <Loader2 className="h-3.5 w-3.5 animate-spin" />
                      ) : (
                        'Generate'
                      )}
                    </Button>
                  </div>
                  {!selectedPlatforms.length && (
                    <p className="text-xs text-muted-foreground">
                      Select a platform on the Platforms tab first to generate
                      captions.
                    </p>
                  )}
                  {aiOptions.length > 0 && (
                    <div className="space-y-2 pt-1">
                      {aiOptions.map((opt, i) => (
                        <button
                          key={i}
                          type="button"
                          onClick={() => {
                            setCaption(opt);
                            setAiOptions([]);
                          }}
                          className="w-full text-left text-sm p-3 rounded-lg border hover:bg-muted/60 line-clamp-3 border-border/50 transition-colors"
                        >
                          {opt}
                        </button>
                      ))}
                    </div>
                  )}
                </div>

                {/* Caption */}
                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <Label>Caption</Label>
                    <span
                      className={cn(
                        'text-xs',
                        selectedPlatforms.includes('twitter') &&
                          caption.length > 280
                          ? 'text-destructive font-medium'
                          : 'text-muted-foreground'
                      )}
                    >
                      {caption.length} chars
                      {selectedPlatforms.includes('twitter') && (
                        <span className="ml-1 text-muted-foreground">
                          (X limit: 280)
                        </span>
                      )}
                    </span>
                  </div>
                  <Textarea
                    value={caption}
                    onChange={(e) => setCaption(e.target.value)}
                    placeholder="Write your post caption…"
                    rows={7}
                    className="resize-y text-sm leading-relaxed"
                  />
                </div>

                {/* Hashtags */}
                <div className="space-y-2">
                  <Label className="text-sm">Hashtags</Label>
                  {hashtags.length > 0 && (
                    <div className="flex flex-wrap gap-1.5">
                      {hashtags.map((tag) => (
                        <span
                          key={tag}
                          className="flex items-center gap-1 bg-blue-500/10 text-blue-700 dark:text-blue-400 border border-blue-300/40 px-2 py-0.5 rounded-full text-xs"
                        >
                          #{tag}
                          <button
                            onClick={() => removeTag(tag, setHashtags)}
                            className="hover:text-foreground"
                          >
                            <X className="h-2.5 w-2.5" />
                          </button>
                        </span>
                      ))}
                    </div>
                  )}
                  <div className="flex gap-2">
                    <Input
                      placeholder="Add hashtag (without #)"
                      value={hashtagInput}
                      onChange={(e) => setHashtagInput(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter' || e.key === ',') {
                          e.preventDefault();
                          addTag(hashtagInput, setHashtags, setHashtagInput);
                        }
                      }}
                      className="h-9 text-sm"
                    />
                    <Button
                      type="button"
                      size="sm"
                      variant="outline"
                      className="h-9"
                      onClick={() =>
                        addTag(hashtagInput, setHashtags, setHashtagInput)
                      }
                    >
                      Add
                    </Button>
                  </div>
                </div>

                {/* Mentions */}
                <div className="space-y-2">
                  <Label className="text-sm">Mentions</Label>
                  {mentions.length > 0 && (
                    <div className="flex flex-wrap gap-1.5">
                      {mentions.map((m) => (
                        <span
                          key={m}
                          className="flex items-center gap-1 bg-purple-500/10 text-purple-700 dark:text-purple-400 border border-purple-300/40 px-2 py-0.5 rounded-full text-xs"
                        >
                          @{m}
                          <button
                            onClick={() => removeTag(m, setMentions)}
                            className="hover:text-foreground"
                          >
                            <X className="h-2.5 w-2.5" />
                          </button>
                        </span>
                      ))}
                    </div>
                  )}
                  <div className="flex gap-2">
                    <Input
                      placeholder="Add mention (without @)"
                      value={mentionInput}
                      onChange={(e) => setMentionInput(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter' || e.key === ',') {
                          e.preventDefault();
                          addTag(mentionInput, setMentions, setMentionInput);
                        }
                      }}
                      className="h-9 text-sm"
                    />
                    <Button
                      type="button"
                      size="sm"
                      variant="outline"
                      className="h-9"
                      onClick={() =>
                        addTag(mentionInput, setMentions, setMentionInput)
                      }
                    >
                      Add
                    </Button>
                  </div>
                </div>
              </TabsContent>

              {/* ── MEDIA TAB ── */}
              <TabsContent value="media" className="mt-0 px-6 py-5 space-y-5">
                {/* Drop zone */}
                <div
                  className={cn(
                    'border-2 border-dashed rounded-xl p-10 text-center transition-colors cursor-pointer',
                    dragActive
                      ? 'border-primary bg-primary/5'
                      : 'border-border hover:border-primary/50 hover:bg-muted/30'
                  )}
                  onDragEnter={(e) => {
                    e.preventDefault();
                    setDragActive(true);
                  }}
                  onDragOver={(e) => {
                    e.preventDefault();
                    setDragActive(true);
                  }}
                  onDragLeave={() => setDragActive(false)}
                  onDrop={handleDrop}
                  onClick={() => fileInputRef.current?.click()}
                >
                  {uploading ? (
                    <div className="flex flex-col items-center gap-3">
                      <Loader2 className="h-10 w-10 animate-spin text-muted-foreground" />
                      <p className="text-sm text-muted-foreground">
                        Uploading…
                      </p>
                    </div>
                  ) : (
                    <div className="flex flex-col items-center gap-3">
                      <Upload className="h-10 w-10 text-muted-foreground" />
                      <div>
                        <p className="text-sm font-medium">
                          Drop images or videos here
                        </p>
                        <p className="text-xs text-muted-foreground mt-1">
                          or click to browse — max 50 MB per file
                        </p>
                      </div>
                      <p className="text-xs text-muted-foreground">
                        Supports JPG, PNG, GIF, WebP, MP4, MOV, WebM
                      </p>
                    </div>
                  )}
                  <input
                    ref={fileInputRef}
                    type="file"
                    className="hidden"
                    accept="image/*,video/*"
                    multiple
                    onChange={(e) => handleFiles(e.target.files)}
                  />
                </div>

                {/* Media grid */}
                {mediaUrls.length > 0 ? (
                  <div className="grid grid-cols-3 gap-3">
                    {mediaUrls.map((url, i) => {
                      const isVideo =
                        url.match(/\.(mp4|mov|webm|avi)$/i) !== null;
                      return (
                        <div
                          key={i}
                          className="relative group rounded-xl overflow-hidden border bg-muted aspect-square"
                        >
                          {isVideo ? (
                            <video
                              src={url}
                              className="w-full h-full object-cover"
                              muted
                            />
                          ) : (
                            <img
                              src={url}
                              alt={`Media ${i + 1}`}
                              className="w-full h-full object-cover"
                            />
                          )}
                          <div className="absolute inset-0 bg-black/50 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center gap-2">
                            <Button
                              variant="ghost"
                              size="sm"
                              className="h-8 w-8 p-0 text-white hover:bg-white/20"
                              onClick={() => window.open(url, '_blank')}
                            >
                              <ExternalLink className="h-4 w-4" />
                            </Button>
                            <Button
                              variant="ghost"
                              size="sm"
                              className="h-8 w-8 p-0 text-red-400 hover:bg-red-500/20"
                              onClick={() =>
                                setMediaUrls((prev) =>
                                  prev.filter((_, idx) => idx !== i)
                                )
                              }
                            >
                              <Trash2 className="h-4 w-4" />
                            </Button>
                          </div>
                          {isVideo && (
                            <div className="absolute bottom-1.5 left-1.5 bg-black/60 text-white text-xs px-1.5 py-0.5 rounded">
                              Video
                            </div>
                          )}
                        </div>
                      );
                    })}
                  </div>
                ) : (
                  <p className="text-xs text-muted-foreground text-center py-4">
                    No media attached yet
                  </p>
                )}
              </TabsContent>

              {/* ── PLATFORMS TAB ── */}
              <TabsContent
                value="platforms"
                className="mt-0 px-6 py-5 space-y-5"
              >
                <div>
                  <Label className="text-sm mb-3 block">
                    Distribution Channels
                  </Label>
                  <div className="flex flex-wrap gap-2">
                    {PLATFORMS.map(({ id, label, Icon, color }) => {
                      const active = selectedPlatforms.includes(id);
                      return (
                        <button
                          key={id}
                          type="button"
                          onClick={() =>
                            setSelectedPlatforms((prev) =>
                              active
                                ? prev.filter((p) => p !== id)
                                : [...prev, id]
                            )
                          }
                          className={cn(
                            'flex items-center gap-2 px-4 py-2 rounded-full border text-sm font-medium transition-colors',
                            active
                              ? 'bg-primary text-primary-foreground border-primary'
                              : 'border-border text-muted-foreground hover:text-foreground hover:border-border/80'
                          )}
                        >
                          <Icon
                            className={cn('h-4 w-4', active ? '' : color)}
                          />
                          {label}
                        </button>
                      );
                    })}
                  </div>
                </div>

                {selectedPlatforms.length > 0 && (
                  <div className="space-y-4">
                    <Label className="text-sm">Per-Platform Fine-tuning</Label>
                    {selectedPlatforms.map((pid) => {
                      const cfg = PLATFORMS.find((p) => p.id === pid);
                      if (!cfg) return null;
                      const { Icon, color, label, charLimit, specific } = cfg;
                      const override = platformSpecific[pid] ?? {};
                      const captionOverride = override.caption ?? '';
                      const effectiveCaption = captionOverride || caption;
                      const overLimit = effectiveCaption.length > charLimit;

                      return (
                        <div
                          key={pid}
                          className="rounded-xl border p-4 space-y-4"
                        >
                          <div className="flex items-center justify-between">
                            <div
                              className={`flex items-center gap-2 text-sm font-semibold ${color}`}
                            >
                              <Icon className="h-4 w-4" />
                              {label}
                            </div>
                            <span
                              className={cn(
                                'text-xs',
                                overLimit
                                  ? 'text-destructive font-medium'
                                  : 'text-muted-foreground'
                              )}
                            >
                              {effectiveCaption.length} /{' '}
                              {charLimit.toLocaleString()}
                            </span>
                          </div>

                          {/* Caption override */}
                          <div className="space-y-1.5">
                            <Label className="text-xs text-muted-foreground">
                              Custom caption for {label}{' '}
                              <span className="italic">
                                (leave blank to use main caption)
                              </span>
                            </Label>
                            <Textarea
                              value={captionOverride}
                              onChange={(e) =>
                                setPlatformSpecific((prev) => ({
                                  ...prev,
                                  [pid]: {
                                    ...prev[pid],
                                    caption: e.target.value,
                                  },
                                }))
                              }
                              placeholder={`Override caption for ${label}…`}
                              rows={3}
                              className="text-sm resize-none"
                            />
                          </div>

                          {/* Platform-specific fields */}
                          {specific.includes('alt_text' as never) && (
                            <div className="space-y-1.5">
                              <Label className="text-xs text-muted-foreground">
                                Alt text (accessibility)
                              </Label>
                              <Input
                                value={override.alt_text ?? ''}
                                onChange={(e) =>
                                  setPlatformSpecific((prev) => ({
                                    ...prev,
                                    [pid]: {
                                      ...prev[pid],
                                      alt_text: e.target.value,
                                    },
                                  }))
                                }
                                placeholder="Describe the image for screen readers"
                                className="h-9 text-sm"
                              />
                            </div>
                          )}

                          {specific.includes('first_comment' as never) && (
                            <div className="space-y-1.5">
                              <Label className="text-xs text-muted-foreground">
                                First comment (hashtags strategy)
                              </Label>
                              <Input
                                value={override.first_comment ?? ''}
                                onChange={(e) =>
                                  setPlatformSpecific((prev) => ({
                                    ...prev,
                                    [pid]: {
                                      ...prev[pid],
                                      first_comment: e.target.value,
                                    },
                                  }))
                                }
                                placeholder="#hashtag #hashtag2"
                                className="h-9 text-sm"
                              />
                            </div>
                          )}

                          {specific.includes('location' as never) && (
                            <div className="space-y-1.5">
                              <Label className="text-xs text-muted-foreground">
                                Location tag
                              </Label>
                              <Input
                                value={override.location ?? ''}
                                onChange={(e) =>
                                  setPlatformSpecific((prev) => ({
                                    ...prev,
                                    [pid]: {
                                      ...prev[pid],
                                      location: e.target.value,
                                    },
                                  }))
                                }
                                placeholder="City or venue name"
                                className="h-9 text-sm"
                              />
                            </div>
                          )}

                          {specific.includes('article_url' as never) && (
                            <div className="space-y-1.5">
                              <Label className="text-xs text-muted-foreground">
                                Article / link preview URL
                              </Label>
                              <Input
                                value={override.article_url ?? ''}
                                onChange={(e) =>
                                  setPlatformSpecific((prev) => ({
                                    ...prev,
                                    [pid]: {
                                      ...prev[pid],
                                      article_url: e.target.value,
                                    },
                                  }))
                                }
                                placeholder="https://…"
                                className="h-9 text-sm"
                              />
                            </div>
                          )}

                          {specific.includes('title' as never) && (
                            <div className="space-y-1.5">
                              <Label className="text-xs text-muted-foreground">
                                Video title
                              </Label>
                              <Input
                                value={override.title ?? ''}
                                onChange={(e) =>
                                  setPlatformSpecific((prev) => ({
                                    ...prev,
                                    [pid]: {
                                      ...prev[pid],
                                      title: e.target.value,
                                    },
                                  }))
                                }
                                placeholder="YouTube video title"
                                className="h-9 text-sm"
                              />
                            </div>
                          )}

                          {specific.includes('tags' as never) && (
                            <div className="space-y-1.5">
                              <Label className="text-xs text-muted-foreground">
                                Video tags (comma separated)
                              </Label>
                              <Input
                                value={override.tags ?? ''}
                                onChange={(e) =>
                                  setPlatformSpecific((prev) => ({
                                    ...prev,
                                    [pid]: {
                                      ...prev[pid],
                                      tags: e.target.value,
                                    },
                                  }))
                                }
                                placeholder="tag1, tag2, tag3"
                                className="h-9 text-sm"
                              />
                            </div>
                          )}

                          {specific.includes('visibility' as never) && (
                            <div className="space-y-1.5">
                              <Label className="text-xs text-muted-foreground">
                                Visibility
                              </Label>
                              <Select
                                value={override.visibility ?? 'public'}
                                onValueChange={(v) =>
                                  setPlatformSpecific((prev) => ({
                                    ...prev,
                                    [pid]: { ...prev[pid], visibility: v },
                                  }))
                                }
                              >
                                <SelectTrigger className="h-9 text-sm">
                                  <SelectValue />
                                </SelectTrigger>
                                <SelectContent>
                                  <SelectItem value="public">Public</SelectItem>
                                  <SelectItem value="unlisted">
                                    Unlisted
                                  </SelectItem>
                                  <SelectItem value="private">
                                    Private
                                  </SelectItem>
                                </SelectContent>
                              </Select>
                            </div>
                          )}

                          {specific.includes('cta_type' as never) && (
                            <div className="space-y-1.5">
                              <Label className="text-xs text-muted-foreground">
                                Call-to-action button
                              </Label>
                              <Select
                                value={override.cta_type ?? 'none'}
                                onValueChange={(v) =>
                                  setPlatformSpecific((prev) => ({
                                    ...prev,
                                    [pid]: { ...prev[pid], cta_type: v },
                                  }))
                                }
                              >
                                <SelectTrigger className="h-9 text-sm">
                                  <SelectValue />
                                </SelectTrigger>
                                <SelectContent>
                                  <SelectItem value="none">None</SelectItem>
                                  <SelectItem value="learn_more">
                                    Learn More
                                  </SelectItem>
                                  <SelectItem value="shop_now">
                                    Shop Now
                                  </SelectItem>
                                  <SelectItem value="sign_up">
                                    Sign Up
                                  </SelectItem>
                                  <SelectItem value="book_now">
                                    Book Now
                                  </SelectItem>
                                  <SelectItem value="contact_us">
                                    Contact Us
                                  </SelectItem>
                                </SelectContent>
                              </Select>
                            </div>
                          )}

                          {specific.includes('thread_mode' as never) && (
                            <div className="flex items-center gap-2.5">
                              <input
                                type="checkbox"
                                id={`thread-${pid}`}
                                checked={override.thread_mode === 'true'}
                                onChange={(e) =>
                                  setPlatformSpecific((prev) => ({
                                    ...prev,
                                    [pid]: {
                                      ...prev[pid],
                                      thread_mode: e.target.checked
                                        ? 'true'
                                        : 'false',
                                    },
                                  }))
                                }
                                className="h-4 w-4"
                              />
                              <Label
                                htmlFor={`thread-${pid}`}
                                className="text-sm cursor-pointer"
                              >
                                Post as thread (splits at 280 chars)
                              </Label>
                            </div>
                          )}
                        </div>
                      );
                    })}
                  </div>
                )}
              </TabsContent>

              {/* ── SCHEDULE TAB ── */}
              <TabsContent
                value="schedule"
                className="mt-0 px-6 py-5 space-y-5"
              >
                <div className="grid grid-cols-2 gap-4">
                  <div className="space-y-2">
                    <Label>Status</Label>
                    <Select value={status} onValueChange={setStatus}>
                      <SelectTrigger className="h-10">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {STATUS_OPTIONS.map((s) => (
                          <SelectItem key={s.value} value={s.value}>
                            {s.label}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>

                  <div className="space-y-2">
                    <Label>Category</Label>
                    <Select value={category} onValueChange={setCategory}>
                      <SelectTrigger className="h-10">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {CATEGORIES.map((c) => (
                          <SelectItem key={c} value={c}>
                            {c.charAt(0).toUpperCase() + c.slice(1)}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                </div>

                <div className="space-y-2">
                  <Label>Schedule Date & Time</Label>
                  <Input
                    type="datetime-local"
                    value={scheduledFor}
                    onChange={(e) => setScheduledFor(e.target.value)}
                    className="h-10"
                  />
                </div>

                <div className="space-y-2">
                  <Label>Assignee</Label>
                  <Select value={assigneeId} onValueChange={setAssigneeId}>
                    <SelectTrigger className="h-10">
                      <SelectValue placeholder="Unassigned" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="">Unassigned</SelectItem>
                      {(
                        users as Array<{
                          id: string;
                          full_name?: string;
                          username?: string;
                        }>
                      ).map((u) => (
                        <SelectItem key={u.id} value={u.id}>
                          {u.full_name || u.username || u.id}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>

                <div className="rounded-xl border p-4 space-y-3">
                  <div className="flex items-center gap-2.5">
                    <input
                      type="checkbox"
                      id="evergreen-toggle"
                      checked={isEvergreen}
                      onChange={(e) => setIsEvergreen(e.target.checked)}
                      className="h-4 w-4"
                    />
                    <Label
                      htmlFor="evergreen-toggle"
                      className="cursor-pointer flex items-center gap-1.5 text-sm"
                    >
                      <RefreshCw className="h-4 w-4 text-muted-foreground" />
                      Evergreen post (recycle after publishing)
                    </Label>
                  </div>
                  {isEvergreen && (
                    <div className="space-y-1.5 pl-6">
                      <Label className="text-xs text-muted-foreground">
                        Recycle after (days)
                      </Label>
                      <Input
                        type="number"
                        min="1"
                        value={recycleAfterDays}
                        onChange={(e) => setRecycleAfterDays(e.target.value)}
                        placeholder="e.g. 30"
                        className="h-9 w-36 text-sm"
                      />
                    </div>
                  )}
                </div>

                {/* Post metadata */}
                <div className="rounded-xl bg-muted/20 border p-4 space-y-1.5 text-xs text-muted-foreground">
                  {post.created_at && (
                    <p>Created: {new Date(post.created_at).toLocaleString()}</p>
                  )}
                  {post.published_at && (
                    <p>
                      Published: {new Date(post.published_at).toLocaleString()}
                    </p>
                  )}
                  {post.publish_attempt > 0 && (
                    <p>Publish attempts: {post.publish_attempt}</p>
                  )}
                  {post.publish_error && (
                    <p className="text-destructive">
                      Last error: {post.publish_error}
                    </p>
                  )}
                </div>
              </TabsContent>

              {/* ── PERFORMANCE TAB ── */}
              <TabsContent
                value="performance"
                className="mt-0 px-6 py-5 space-y-5"
              >
                {isPublished ? (
                  <>
                    <div className="grid grid-cols-4 gap-3">
                      <MetricCard
                        label="Impressions"
                        value={post.impressions}
                      />
                      <MetricCard label="Reach" value={post.reach} />
                      <MetricCard label="Likes" value={post.likes} />
                      <MetricCard label="Comments" value={post.comments} />
                    </div>
                    <div className="grid grid-cols-4 gap-3">
                      <MetricCard label="Shares" value={post.shares} />
                      <MetricCard label="Saves" value={post.saves} />
                      <MetricCard label="Link Clicks" value={post.clicks} />
                      <MetricCard
                        label="Engagement Rate"
                        value={`${(post.engagement_rate * 100).toFixed(2)}%`}
                      />
                    </div>
                    {post.platform_url && (
                      <Button
                        variant="outline"
                        size="sm"
                        className="gap-1.5"
                        onClick={() =>
                          window.open(post.platform_url!, '_blank')
                        }
                      >
                        <ExternalLink className="h-3.5 w-3.5" />
                        View on{' '}
                        {parsePlatforms(post.platforms)[0] ?? 'platform'}
                      </Button>
                    )}
                  </>
                ) : (
                  <div className="flex flex-col items-center justify-center py-16 gap-3 text-muted-foreground">
                    <BarChart3 className="h-12 w-12 opacity-20" />
                    <p className="text-sm font-medium">
                      Analytics available after publishing
                    </p>
                    <p className="text-xs">
                      Current status:{' '}
                      <span className="font-medium">
                        {status.replace(/_/g, ' ')}
                      </span>
                    </p>
                  </div>
                )}
              </TabsContent>

              {/* bottom padding for scroll area */}
              <div className="h-6" />
            </ScrollArea>

            {/* ── Footer ── */}
            <div className="px-6 py-4 border-t shrink-0 flex items-center justify-between bg-background">
              <p className="text-xs text-muted-foreground">
                {post?.updated_at && (
                  <>Last saved {new Date(post.updated_at).toLocaleString()}</>
                )}
              </p>
              <div className="flex gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={onClose}
                  className="h-9 px-4"
                >
                  Cancel
                </Button>
                <Button
                  size="sm"
                  onClick={() => saveMutation.mutate()}
                  disabled={saveMutation.isPending || isLoading}
                  className="h-9 px-5"
                >
                  {saveMutation.isPending ? (
                    <>
                      <Loader2 className="h-3.5 w-3.5 animate-spin mr-1.5" />
                      Saving…
                    </>
                  ) : (
                    'Save Changes'
                  )}
                </Button>
              </div>
            </div>
          </Tabs>
        )}
      </div>
    </ResizableDrawer>
  );
}
