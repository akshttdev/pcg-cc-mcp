import { useRef, useState } from 'react';
import { useParams } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { reviewApi, type ReviewComment, type ReviewSourceFile } from '@/lib/api';
import { reviewKeys } from '@/lib/query-keys';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { Badge } from '@/components/ui/badge';
import { Loader } from '@/components/ui/loader';
import { CheckCircle2, MessageSquare, Clock, Film, Download, ChevronUp, ChevronDown, Play } from 'lucide-react';

function formatTimecode(seconds?: number | null): string {
  if (seconds == null) return '';
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${m}:${s.toString().padStart(2, '0')}`;
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function CommentCard({
  comment,
  onSeek,
  onResolve,
}: {
  comment: ReviewComment;
  onSeek: (t: number) => void;
  onResolve: () => void;
}) {
  return (
    <div
      className={`border rounded-lg p-3 space-y-1.5 text-sm transition-opacity ${
        comment.is_resolved ? 'opacity-50' : ''
      }`}
    >
      <div className="flex items-center gap-2">
        <span className="font-medium">{comment.author_name}</span>
        {comment.timecode_seconds != null && (
          <button
            onClick={() => onSeek(comment.timecode_seconds!)}
            className="flex items-center gap-0.5 text-xs text-primary hover:underline"
          >
            <Clock className="h-3 w-3" />
            {formatTimecode(comment.timecode_seconds)}
          </button>
        )}
        {comment.is_resolved && (
          <Badge variant="secondary" className="text-[10px] ml-auto">
            resolved
          </Badge>
        )}
        {!comment.is_resolved && (
          <button
            onClick={onResolve}
            className="ml-auto text-xs text-muted-foreground hover:text-green-500"
            title="Mark resolved"
          >
            <CheckCircle2 className="h-4 w-4" />
          </button>
        )}
      </div>
      <p className="text-muted-foreground leading-snug">{comment.content}</p>
    </div>
  );
}

function SourceFilesDrawer({
  files,
  onSelect,
}: {
  files: ReviewSourceFile[];
  onSelect: (url: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [previewFile, setPreviewFile] = useState<ReviewSourceFile | null>(null);

  if (files.length === 0) return null;

  return (
    <div className="border-t border-border bg-background shrink-0">
      {/* Drawer handle */}
      <button
        onClick={() => setOpen((o) => !o)}
        className="w-full flex items-center gap-2 px-4 py-2 hover:bg-muted/50 transition-colors"
      >
        <Film className="h-4 w-4 text-muted-foreground" />
        <span className="text-sm font-medium">Source Files</span>
        <Badge variant="secondary" className="text-[10px]">{files.length}</Badge>
        <span className="ml-auto text-muted-foreground">
          {open ? <ChevronDown className="h-4 w-4" /> : <ChevronUp className="h-4 w-4" />}
        </span>
      </button>

      {open && (
        <div className="border-t border-border">
          <div className="flex" style={{ height: '220px' }}>
            {/* File list */}
            <div className="w-72 shrink-0 border-r border-border overflow-y-auto">
              {files.map((f) => (
                <div
                  key={f.name}
                  className={`flex items-center gap-2 px-3 py-2 cursor-pointer hover:bg-muted/50 group transition-colors ${
                    previewFile?.name === f.name ? 'bg-muted' : ''
                  }`}
                  onClick={() => setPreviewFile(f)}
                >
                  <Film className="h-3 w-3 text-muted-foreground shrink-0" />
                  <span className="text-xs truncate flex-1" title={f.name}>{f.name}</span>
                  <span className="text-[10px] text-muted-foreground shrink-0">{formatFileSize(f.size_bytes)}</span>
                  <a
                    href={f.url}
                    download={f.name}
                    className="opacity-0 group-hover:opacity-100 transition-opacity shrink-0"
                    title="Download"
                    onClick={(e) => e.stopPropagation()}
                  >
                    <Download className="h-3 w-3 text-muted-foreground hover:text-foreground" />
                  </a>
                </div>
              ))}
            </div>

            {/* Preview pane */}
            <div className="flex-1 bg-black flex flex-col items-center justify-center relative">
              {previewFile ? (
                <>
                  <video
                    key={previewFile.url}
                    src={previewFile.url}
                    controls
                    className="max-h-full max-w-full"
                    style={{ maxHeight: '180px' }}
                  />
                  <div className="absolute bottom-2 right-2 flex gap-1.5">
                    <button
                      onClick={() => onSelect(previewFile.url)}
                      className="flex items-center gap-1 bg-primary text-primary-foreground text-[10px] font-medium px-2 py-1 rounded hover:bg-primary/90 transition-colors"
                      title="Use this clip as main video"
                    >
                      <Play className="h-2.5 w-2.5" />
                      Use this cut
                    </button>
                  </div>
                </>
              ) : (
                <p className="text-white/40 text-xs">Select a file to preview</p>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export function ReviewPage() {
  const { token } = useParams<{ token: string }>();
  const queryClient = useQueryClient();
  const videoRef = useRef<HTMLVideoElement>(null);

  const [pendingTimecode, setPendingTimecode] = useState<number | null>(null);
  const [form, setForm] = useState({ author_name: '', author_email: '', content: '' });
  const [activeVideoUrl, setActiveVideoUrl] = useState<string | null>(null);

  const { data, isLoading, error } = useQuery({
    queryKey: reviewKeys.detail(token!),
    queryFn: () => reviewApi.getData(token!),
    enabled: !!token,
    refetchInterval: 30000,
  });

  const addCommentMutation = useMutation({
    mutationFn: () =>
      reviewApi.addComment(token!, {
        author_name: form.author_name || undefined,
        author_email: form.author_email || undefined,
        content: form.content,
        timecode_seconds: pendingTimecode ?? undefined,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: reviewKeys.detail(token!) });
      setForm((f) => ({ ...f, content: '' }));
      setPendingTimecode(null);
    },
  });

  const resolveMutation = useMutation({
    mutationFn: (commentId: string) => reviewApi.resolve(token!, commentId),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: reviewKeys.detail(token!) }),
  });

  const handleMarkTimecode = () => {
    if (!videoRef.current) return;
    const t = videoRef.current.currentTime;
    setPendingTimecode(t);
  };

  const seekTo = (t: number) => {
    if (videoRef.current) {
      videoRef.current.currentTime = t;
      videoRef.current.play();
    }
  };

  if (isLoading) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <Loader message="Loading review…" size={32} />
      </div>
    );
  }

  if (error || !data) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <div className="text-center space-y-2">
          <p className="text-lg font-semibold">Review link expired or invalid</p>
          <p className="text-muted-foreground text-sm">
            Please request a new link from the project team.
          </p>
        </div>
      </div>
    );
  }

  const { deliverable, comments, source_files = [] } = data;
  const videoUrl = activeVideoUrl ?? deliverable.working_file_url ?? deliverable.final_link;

  return (
    <div className="h-screen bg-background flex flex-col overflow-hidden">
      {/* Header */}
      <div className="border-b border-border px-6 py-3 flex items-center gap-3 shrink-0">
        <h1 className="font-semibold text-sm">{deliverable.title}</h1>
        <Badge variant="secondary" className="text-xs capitalize">
          {deliverable.status.replace('_', ' ')}
        </Badge>
        {activeVideoUrl && activeVideoUrl !== (deliverable.working_file_url ?? deliverable.final_link) && (
          <Badge variant="outline" className="text-xs text-amber-600 border-amber-500/50">
            previewing source cut
          </Badge>
        )}
        <span className="text-xs text-muted-foreground ml-auto">
          {data.token.view_count} view{data.token.view_count !== 1 ? 's' : ''}
        </span>
      </div>

      <div className="flex flex-1 min-h-0">
        {/* Left — video + source drawer */}
        <div className="flex-1 flex flex-col min-w-0">
          {/* Video player */}
          <div className="flex-1 bg-black flex items-center justify-center relative min-h-0">
            {videoUrl ? (
              <video
                key={videoUrl}
                ref={videoRef}
                src={videoUrl}
                controls
                className="max-h-full max-w-full"
              />
            ) : (
              <div className="text-white/50 text-sm text-center p-8">
                <p>No video file attached to this deliverable.</p>
                <p className="text-xs mt-1">
                  {deliverable.description || 'Review the description in the comments panel.'}
                </p>
              </div>
            )}

            {/* Mark timecode button + hint */}
            {videoUrl && (
              <div className="absolute bottom-4 left-1/2 -translate-x-1/2 flex flex-col items-center gap-2">
                {pendingTimecode != null ? (
                  <div className="bg-black/80 text-white text-xs px-3 py-1.5 rounded-full">
                    Comment at {formatTimecode(pendingTimecode)} — fill in your note on the right
                  </div>
                ) : (
                  <button
                    onClick={handleMarkTimecode}
                    className="flex items-center gap-1.5 bg-black/60 hover:bg-black/80 text-white/70 hover:text-white text-xs px-3 py-1.5 rounded-full transition-colors"
                  >
                    <Clock className="h-3 w-3" />
                    Mark timecode
                  </button>
                )}
              </div>
            )}
          </div>

          {/* Source files drawer — bottom of video column */}
          <SourceFilesDrawer
            files={source_files}
            onSelect={(url) => setActiveVideoUrl(url)}
          />
        </div>

        {/* Right panel — comments */}
        <div className="w-80 shrink-0 border-l border-border flex flex-col">
          <div className="p-3 border-b border-border">
            <h2 className="text-sm font-semibold flex items-center gap-1.5">
              <MessageSquare className="h-4 w-4" />
              Comments ({comments.length})
            </h2>
          </div>

          {/* Comments list */}
          <div className="flex-1 overflow-y-auto p-3 space-y-2">
            {comments.length === 0 && (
              <p className="text-xs text-muted-foreground text-center py-8">
                No comments yet. Click on the video to add a frame comment, or use the form below.
              </p>
            )}
            {comments.map((c) => (
              <CommentCard
                key={c.id}
                comment={c}
                onSeek={seekTo}
                onResolve={() => resolveMutation.mutate(c.id)}
              />
            ))}
          </div>

          {/* Add comment form */}
          <div className="border-t border-border p-3 space-y-2">
            {pendingTimecode != null && (
              <div className="flex items-center gap-1.5 text-xs text-primary">
                <Clock className="h-3 w-3" />
                <span>At {formatTimecode(pendingTimecode)}</span>
                <button
                  onClick={() => setPendingTimecode(null)}
                  className="ml-auto text-muted-foreground hover:text-foreground"
                >
                  ✕
                </button>
              </div>
            )}
            <Input
              placeholder="Your name (required)"
              value={form.author_name}
              onChange={(e) => setForm((f) => ({ ...f, author_name: e.target.value }))}
              className="text-xs"
            />
            <Input
              placeholder="Email (optional)"
              type="email"
              value={form.author_email}
              onChange={(e) => setForm((f) => ({ ...f, author_email: e.target.value }))}
              className="text-xs"
            />
            <Textarea
              placeholder="Your comment…"
              value={form.content}
              onChange={(e) => setForm((f) => ({ ...f, content: e.target.value }))}
              className="text-xs resize-none"
              rows={3}
            />
            <Button
              size="sm"
              className="w-full"
              disabled={!form.author_name.trim() || !form.content.trim() || addCommentMutation.isPending}
              onClick={() => addCommentMutation.mutate()}
            >
              Post Comment
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
