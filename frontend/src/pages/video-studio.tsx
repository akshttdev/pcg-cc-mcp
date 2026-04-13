import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  AlertCircle,
  AudioLines,
  CheckCircle,
  ChevronRight,
  Clock,
  Download,
  Film,
  Grid2x2,
  Image as ImageIcon,
  Loader2,
  Maximize2,
  Pause,
  Play,
  Plus,
  RefreshCw,
  User,
  Video,
} from 'lucide-react';
import { useRef, useState } from 'react';

import { Loader } from '@/components/ui/loader';
import { apiClient } from '@/lib/api';

// ─── Types ───────────────────────────────────────────────────────────────────

interface VideoFile {
  filename: string;
  size_bytes: number;
  stream_url: string;
}

interface OverlayFile {
  filename: string;
  size_bytes: number;
  download_url: string;
}

interface OverlayEpisode {
  episode: string;
  files: OverlayFile[];
}

interface AvatarProfile {
  id: string;
  name: string;
  slug: string | null;
  heygen_avatar_id: string | null;
  heygen_avatar_type: string;
  elevenlabs_voice_id: string;
  identity_doc: string | null;
  style_notes: string | null;
  reference_image_url: string | null;
  thumbnail_url: string | null;
  default_background_url: string | null;
  status: string;
  error_message: string | null;
  created_at: string;
  updated_at: string;
}

interface VideoJob {
  id: string;
  avatar_profile_id: string;
  script_text: string;
  background_url: string | null;
  tts_status: string;
  tts_audio_url: string | null;
  heygen_video_id: string | null;
  heygen_status: string | null;
  raw_video_url: string | null;
  final_video_url: string | null;
  thumbnail_url: string | null;
  duration_seconds: number | null;
  status: string;
  error_message: string | null;
  created_at: string;
  updated_at: string;
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

function formatTime(s: number): string {
  const m = Math.floor(s / 60);
  const sec = Math.floor(s % 60);
  return `${m}:${sec.toString().padStart(2, '0')}`;
}

function formatBytes(b: number): string {
  if (b > 1_000_000) return `${(b / 1_000_000).toFixed(1)} MB`;
  return `${(b / 1_000).toFixed(0)} KB`;
}

function timeAgo(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const m = Math.floor(diff / 60_000);
  if (m < 1) return 'just now';
  if (m < 60) return `${m}m ago`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h}h ago`;
  return `${Math.floor(h / 24)}d ago`;
}

const IN_PROGRESS = ['pending', 'tts_generating', 'avatar_generating'];

// ─── Status Badge ─────────────────────────────────────────────────────────────

function JobStatusBadge({ status }: { status: string }) {
  const map: Record<
    string,
    { label: string; color: string; spinning: boolean }
  > = {
    pending: { label: 'Pending', color: '#6B7280', spinning: false },
    tts_generating: { label: 'TTS', color: '#3B82F6', spinning: true },
    avatar_generating: { label: 'Rendering', color: '#8B5CF6', spinning: true },
    ready: { label: 'Ready', color: '#10B981', spinning: false },
    failed: { label: 'Failed', color: '#EF4444', spinning: false },
  };
  const c = map[status] ?? map['pending'];
  return (
    <span
      className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-semibold"
      style={{ background: `${c.color}20`, color: c.color }}
    >
      {c.spinning ? (
        <Loader2 className="h-3 w-3 animate-spin" />
      ) : status === 'ready' ? (
        <CheckCircle className="h-3 w-3" />
      ) : status === 'failed' ? (
        <AlertCircle className="h-3 w-3" />
      ) : (
        <Clock className="h-3 w-3" />
      )}
      {c.label.toUpperCase()}
    </span>
  );
}

// ─── Overlay Timeline ─────────────────────────────────────────────────────────

const OVERLAY_TIMELINE = [
  {
    time: 0,
    label: 'Intro Animation',
    description: 'PCG logo scale-in + rule draw + slide-up',
    color: '#AF9041',
  },
  {
    time: 4.5,
    label: 'Presenter Overlay',
    description: 'Sami Satoshi · Digital Asset Strategist',
    color: '#AF9041',
  },
  {
    time: 5.2,
    label: 'Ticker Strip',
    description: 'PCG Tech Briefing · Scrolling headlines',
    color: '#7A6228',
  },
  {
    time: 9,
    label: 'B-Roll Cut → AI',
    description: 'Next-gen chip architecture footage',
    color: '#3B82F6',
  },
  {
    time: 17,
    label: 'Stat Callout',
    description: '3x More Efficient - Next-Gen Chip Architecture',
    color: '#AF9041',
  },
  {
    time: 38,
    label: 'B-Roll Cut → Crypto',
    description: 'Blockchain infrastructure footage',
    color: '#3B82F6',
  },
  {
    time: 46,
    label: 'Breaking Alert',
    description: '3 of Top 10 Asset Managers Allocated to Crypto',
    color: '#EF4444',
  },
  {
    time: 57,
    label: 'B-Roll Cut → Data',
    description: 'Data sovereignty / infrastructure footage',
    color: '#3B82F6',
  },
  {
    time: 64,
    label: 'Definition Box',
    description: 'Data Sovereignty - the right to control your data',
    color: '#AF9041',
  },
  {
    time: 79,
    label: 'Pull Quote',
    description: 'Most defensible competitive asset - Sami Satoshi',
    color: '#D4B96A',
  },
  {
    time: 87,
    label: 'PCG Positioning',
    description: 'Agents · Sovereign Infrastructure · Tokenized Design',
    color: '#AF9041',
  },
  {
    time: 101,
    label: 'CTA Overlay',
    description: 'Stay Sharp · @powerclubglobal',
    color: '#AF9041',
  },
];

function OverlayTimeline({ currentTime }: { currentTime: number }) {
  const active = OVERLAY_TIMELINE.filter((o) => currentTime >= o.time);
  const current = active[active.length - 1];
  return (
    <div className="space-y-1">
      {OVERLAY_TIMELINE.map((o, i) => {
        const isActive = currentTime >= o.time;
        const isCurrent = current === o;
        return (
          <div
            key={i}
            className={`flex items-start gap-3 px-3 py-2 rounded-lg transition-all ${isCurrent ? 'bg-card border border-border/60' : isActive ? 'opacity-50' : 'opacity-30'}`}
          >
            <div
              className="mt-1 w-2 h-2 rounded-full flex-shrink-0"
              style={{ background: isActive ? o.color : '#374151' }}
            />
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2">
                <span
                  className="text-xs font-mono"
                  style={{ color: isActive ? o.color : '#6B7280' }}
                >
                  {formatTime(o.time)}
                </span>
                <span
                  className={`text-sm font-medium truncate ${isCurrent ? 'text-white' : 'text-muted-foreground'}`}
                >
                  {o.label}
                </span>
                {isCurrent && (
                  <span
                    className="ml-auto text-xs px-1.5 py-0.5 rounded"
                    style={{ background: `${o.color}20`, color: o.color }}
                  >
                    LIVE
                  </span>
                )}
              </div>
              {isCurrent && (
                <p className="text-xs text-muted-foreground mt-0.5 truncate">
                  {o.description}
                </p>
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
}

const OVERLAY_LABELS: Record<string, string> = {
  '01_show_bug': 'Show Bug (Persistent)',
  '02_host_id_sami': 'Host ID - Sami Satoshi',
  '03_broll_ctx_ai': 'B-Roll - AI / Inference',
  '04_broll_ctx_crypto': 'B-Roll - Blockchain',
  '05_broll_ctx_pcg': 'B-Roll - PCG / Sovereign',
  '06_intro_title': 'Intro Title Card',
  '07_thumbnail': 'Thumbnail Graphic',
  '08_outro': 'Outro / Follow CTA',
};

// ─── Simple Modal (no external dependencies) ──────────────────────────────────

function Modal({
  open,
  onClose,
  title,
  children,
}: {
  open: boolean;
  onClose: () => void;
  title: string;
  children: React.ReactNode;
}) {
  if (!open) return null;
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/60" onClick={onClose} />
      <div className="relative z-10 w-full max-w-md bg-background border border-border rounded-2xl shadow-2xl p-6 mx-4">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-base font-semibold">{title}</h2>
          <button
            onClick={onClose}
            className="p-1 rounded hover:bg-muted/50 transition-colors text-muted-foreground text-xl leading-none"
          >
            &times;
          </button>
        </div>
        {children}
      </div>
    </div>
  );
}

// ─── New Anchor Modal ─────────────────────────────────────────────────────────

function NewAnchorModal({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const [name, setName] = useState('');
  const [heygenId, setHeygenId] = useState('');
  const [voiceId, setVoiceId] = useState('ZtcPZrt9K4w8e1OB9M6w');
  const [styleNotes, setStyleNotes] = useState('');
  const qc = useQueryClient();

  const mutation = useMutation({
    mutationFn: () =>
      apiClient
        .post<AvatarProfile>('/video-gen/avatars', {
          name,
          heygen_avatar_id: heygenId || null,
          elevenlabs_voice_id: voiceId || null,
          style_notes: styleNotes || null,
        })
        .then((r) => r.data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['video-avatars'] });
      onClose();
      setName('');
      setHeygenId('');
      setVoiceId('ZtcPZrt9K4w8e1OB9M6w');
      setStyleNotes('');
    },
  });

  return (
    <Modal open={open} onClose={onClose} title="New Anchor Profile">
      <div className="space-y-3">
        <div>
          <label className="text-xs text-muted-foreground">Name *</label>
          <input
            className="mt-1 w-full bg-muted border border-border rounded-lg px-3 py-2 text-sm focus:outline-none"
            placeholder="e.g. Sami Satoshi"
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
        </div>
        <div>
          <label className="text-xs text-muted-foreground">
            HeyGen Avatar ID
          </label>
          <input
            className="mt-1 w-full bg-muted border border-border rounded-lg px-3 py-2 text-sm font-mono focus:outline-none"
            placeholder="From HeyGen dashboard"
            value={heygenId}
            onChange={(e) => setHeygenId(e.target.value)}
          />
        </div>
        <div>
          <label className="text-xs text-muted-foreground">
            ElevenLabs Voice ID
          </label>
          <input
            className="mt-1 w-full bg-muted border border-border rounded-lg px-3 py-2 text-sm font-mono focus:outline-none"
            value={voiceId}
            onChange={(e) => setVoiceId(e.target.value)}
          />
        </div>
        <div>
          <label className="text-xs text-muted-foreground">Style Notes</label>
          <textarea
            rows={2}
            className="mt-1 w-full bg-muted border border-border rounded-lg px-3 py-2 text-sm focus:outline-none resize-none"
            placeholder="Tone, pacing, character notes..."
            value={styleNotes}
            onChange={(e) => setStyleNotes(e.target.value)}
          />
        </div>
        {mutation.isError && (
          <p className="text-xs text-red-400">Failed to create anchor.</p>
        )}
        <div className="flex gap-2 pt-1">
          <button
            onClick={onClose}
            className="flex-1 px-4 py-2 rounded-lg text-sm border border-border hover:bg-muted/50 transition-colors"
          >
            Cancel
          </button>
          <button
            onClick={() => mutation.mutate()}
            disabled={!name || mutation.isPending}
            className="flex-1 px-4 py-2 rounded-lg text-sm font-medium text-black disabled:opacity-50"
            style={{ background: '#AF9041' }}
          >
            {mutation.isPending ? 'Creating...' : 'Create'}
          </button>
        </div>
      </div>
    </Modal>
  );
}

// ─── New Job Modal ─────────────────────────────────────────────────────────────

function NewJobModal({
  open,
  onClose,
  avatars,
  defaultAnchorId,
}: {
  open: boolean;
  onClose: () => void;
  avatars: AvatarProfile[];
  defaultAnchorId: string | null;
}) {
  const [anchorId, setAnchorId] = useState(defaultAnchorId ?? '');
  const [script, setScript] = useState('');
  const qc = useQueryClient();

  const selectedAvatar = avatars.find((a) => a.id === anchorId);

  const mutation = useMutation({
    mutationFn: () =>
      apiClient
        .post<VideoJob>('/video-gen/jobs', {
          avatar_profile_id: anchorId,
          script_text: script,
        })
        .then((r) => r.data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['video-jobs'] });
      onClose();
      setScript('');
    },
  });

  return (
    <Modal open={open} onClose={onClose} title="Produce New Video">
      <div className="space-y-3">
        <div>
          <label className="text-xs text-muted-foreground">Anchor *</label>
          <select
            className="mt-1 w-full bg-muted border border-border rounded-lg px-3 py-2 text-sm focus:outline-none"
            value={anchorId}
            onChange={(e) => setAnchorId(e.target.value)}
          >
            <option value="">Select anchor...</option>
            {avatars.map((a) => (
              <option key={a.id} value={a.id}>
                {a.name}
              </option>
            ))}
          </select>
          {selectedAvatar && !selectedAvatar.heygen_avatar_id && (
            <p className="mt-1 text-xs text-amber-500">
              No HeyGen ID set - job will fail at render stage.
            </p>
          )}
        </div>
        <div>
          <label className="text-xs text-muted-foreground">Script *</label>
          <textarea
            rows={8}
            className="mt-1 w-full bg-muted border border-border rounded-lg px-3 py-2 text-sm focus:outline-none resize-none font-mono"
            placeholder="Paste the full script here..."
            value={script}
            onChange={(e) => setScript(e.target.value)}
          />
          <p className="mt-1 text-[10px] text-muted-foreground">
            {script.length} chars
          </p>
        </div>
        {mutation.isError && (
          <p className="text-xs text-red-400">Failed to queue job.</p>
        )}
        <div className="flex gap-2 pt-1">
          <button
            onClick={onClose}
            className="flex-1 px-4 py-2 rounded-lg text-sm border border-border hover:bg-muted/50 transition-colors"
          >
            Cancel
          </button>
          <button
            onClick={() => mutation.mutate()}
            disabled={!anchorId || !script.trim() || mutation.isPending}
            className="flex-1 px-4 py-2 rounded-lg text-sm font-medium text-black disabled:opacity-50 flex items-center justify-center gap-2"
            style={{ background: '#AF9041' }}
          >
            {mutation.isPending ? (
              <>
                <Loader2 className="h-3.5 w-3.5 animate-spin" /> Queuing...
              </>
            ) : (
              <>
                <Video className="h-3.5 w-3.5" /> Produce
              </>
            )}
          </button>
        </div>
      </div>
    </Modal>
  );
}

// ─── Types ────────────────────────────────────────────────────────────────────

interface PlayerSource {
  url: string;
  label: string;
}

type LeftTab = 'library' | 'anchors' | 'jobs';
type RightTab = 'timeline' | 'assets';

// ─── Main Page ────────────────────────────────────────────────────────────────

export function VideoStudioPage() {
  const videoRef = useRef<HTMLVideoElement>(null);
  const [currentTime, setCurrentTime] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [playerSource, setPlayerSource] = useState<PlayerSource | null>(null);
  const [activeJobId, setActiveJobId] = useState<string | null>(null);
  const [duration, setDuration] = useState(0);
  const [leftTab, setLeftTab] = useState<LeftTab>('library');
  const [rightTab, setRightTab] = useState<RightTab>('timeline');
  const [showNewAnchor, setShowNewAnchor] = useState(false);
  const [showNewJob, setShowNewJob] = useState(false);
  const [newJobAnchorId, setNewJobAnchorId] = useState<string | null>(null);

  const { data: files, isLoading: filesLoading } = useQuery({
    queryKey: ['video-gen-files'],
    queryFn: () =>
      apiClient.get<VideoFile[]>('/video-gen/files').then((r) => r.data),
  });

  const { data: overlays, isLoading: overlaysLoading } = useQuery({
    queryKey: ['video-gen-overlays'],
    queryFn: () =>
      apiClient
        .get<OverlayEpisode[]>('/video-gen/overlays')
        .then((r) => r.data),
  });

  const { data: avatars = [], isLoading: avatarsLoading } = useQuery({
    queryKey: ['video-avatars'],
    queryFn: () =>
      apiClient.get<AvatarProfile[]>('/video-gen/avatars').then((r) => r.data),
  });

  const { data: jobs = [], isLoading: jobsLoading } = useQuery({
    queryKey: ['video-jobs'],
    queryFn: () =>
      apiClient.get<VideoJob[]>('/video-gen/jobs').then((r) => r.data),
    refetchInterval: (q) => {
      const data = q.state.data;
      if (!data) return false;
      return data.some((j: VideoJob) => IN_PROGRESS.includes(j.status))
        ? 10_000
        : false;
    },
  });

  const defaultFile =
    files?.find((f) => f.filename.includes('v17')) ?? files?.[0];
  const resolvedSource: PlayerSource | null =
    playerSource ??
    (defaultFile
      ? {
          url: `/api${defaultFile.stream_url}`,
          label: defaultFile.filename.replace(/_/g, ' ').toUpperCase(),
        }
      : null);

  const loadFile = (f: VideoFile) => {
    setPlayerSource({
      url: `/api${f.stream_url}`,
      label: f.filename.replace(/_/g, ' ').toUpperCase(),
    });
    setActiveJobId(null);
  };

  const loadJob = (job: VideoJob) => {
    const url = job.final_video_url ?? `/api/video-gen/video/${job.id}`;
    const anchor = avatars.find((a) => a.id === job.avatar_profile_id);
    setPlayerSource({ url, label: `${anchor?.name ?? 'Video'} - Job` });
    setActiveJobId(job.id);
  };

  const togglePlay = () => {
    if (!videoRef.current) return;
    if (playing) {
      videoRef.current.pause();
    } else {
      videoRef.current.play();
    }
    setPlaying((p) => !p);
  };

  const seek = (e: React.MouseEvent<HTMLDivElement>) => {
    if (!videoRef.current || !duration) return;
    const rect = e.currentTarget.getBoundingClientRect();
    videoRef.current.currentTime =
      ((e.clientX - rect.left) / rect.width) * duration;
  };

  const progress = duration > 0 ? (currentTime / duration) * 100 : 0;
  const inProgressCount = jobs.filter((j: VideoJob) =>
    IN_PROGRESS.includes(j.status)
  ).length;

  const leftTabs: { key: LeftTab; label: string; icon: React.ReactNode }[] = [
    {
      key: 'library',
      label: 'Library',
      icon: <Grid2x2 className="h-3.5 w-3.5" />,
    },
    {
      key: 'anchors',
      label: 'Anchors',
      icon: <User className="h-3.5 w-3.5" />,
    },
    { key: 'jobs', label: 'Jobs', icon: <Video className="h-3.5 w-3.5" /> },
  ];

  return (
    <div className="h-full flex flex-col bg-background">
      {/* Header */}
      <div className="flex items-center gap-3 px-6 py-4 border-b border-border/40">
        <Film className="h-5 w-5 text-amber-500" />
        <h1 className="text-lg font-semibold">Video Studio</h1>
        {inProgressCount > 0 && (
          <span
            className="text-xs px-2 py-0.5 rounded-full flex items-center gap-1"
            style={{ background: '#3B82F620', color: '#3B82F6' }}
          >
            <Loader2 className="h-3 w-3 animate-spin" />
            {inProgressCount} rendering
          </span>
        )}
      </div>

      <div className="flex-1 flex overflow-hidden">
        {/* Left Panel */}
        <div className="w-64 border-r border-border/40 flex flex-col">
          <div className="flex border-b border-border/40">
            {leftTabs.map((t) => (
              <button
                key={t.key}
                onClick={() => setLeftTab(t.key)}
                className={`flex-1 flex items-center justify-center gap-1 py-2.5 text-[11px] font-medium transition-colors ${leftTab === t.key ? 'text-amber-400 border-b-2 border-amber-400' : 'text-muted-foreground hover:text-foreground'}`}
              >
                {t.icon}
                {t.label}
                {t.key === 'jobs' && jobs.length > 0 && (
                  <span className="ml-0.5 text-[9px] px-1 rounded-full bg-muted text-muted-foreground">
                    {jobs.length}
                  </span>
                )}
              </button>
            ))}
          </div>

          {/* Library tab */}
          {leftTab === 'library' && (
            <>
              <div className="px-4 py-2 text-[10px] font-semibold text-muted-foreground uppercase tracking-widest border-b border-border/30">
                Produced Videos
              </div>
              {filesLoading && <Loader message="Loading..." className="m-4" />}
              <div className="flex-1 overflow-y-auto py-2">
                {files?.map((f) => (
                  <button
                    key={f.filename}
                    onClick={() => loadFile(f)}
                    className={`w-full text-left px-4 py-2.5 flex items-center gap-2 hover:bg-muted/50 transition-colors ${resolvedSource?.url === `/api${f.stream_url}` && !activeJobId ? 'bg-muted' : ''}`}
                  >
                    <ChevronRight
                      className="h-3 w-3 flex-shrink-0"
                      style={{
                        color:
                          resolvedSource?.url === `/api${f.stream_url}` &&
                          !activeJobId
                            ? '#AF9041'
                            : 'transparent',
                      }}
                    />
                    <div className="min-w-0">
                      <p className="text-xs font-medium truncate">
                        {f.filename}
                      </p>
                      <p className="text-[10px] text-muted-foreground">
                        {formatBytes(f.size_bytes)}
                      </p>
                    </div>
                  </button>
                ))}
              </div>
            </>
          )}

          {/* Anchors tab */}
          {leftTab === 'anchors' && (
            <>
              <div className="flex items-center justify-between px-4 py-2 border-b border-border/30">
                <span className="text-[10px] font-semibold text-muted-foreground uppercase tracking-widest">
                  Anchor Profiles
                </span>
                <button
                  onClick={() => setShowNewAnchor(true)}
                  className="p-1 rounded hover:bg-muted/50 transition-colors"
                >
                  <Plus className="h-3.5 w-3.5 text-amber-500" />
                </button>
              </div>
              {avatarsLoading && (
                <Loader message="Loading..." className="m-4" />
              )}
              <div className="flex-1 overflow-y-auto py-2">
                {!avatarsLoading && avatars.length === 0 && (
                  <div className="text-center p-6">
                    <User className="h-8 w-8 mx-auto mb-2 opacity-20" />
                    <p className="text-xs text-muted-foreground">
                      No anchors yet
                    </p>
                    <button
                      onClick={() => setShowNewAnchor(true)}
                      className="mt-3 text-xs px-3 py-1.5 rounded-lg font-medium text-black"
                      style={{ background: '#AF9041' }}
                    >
                      + Add Anchor
                    </button>
                  </div>
                )}
                {avatars.map((a: AvatarProfile) => (
                  <div
                    key={a.id}
                    className="mx-3 mb-2 rounded-xl border border-border/50 bg-card/50 p-3 space-y-2"
                  >
                    <div className="flex items-start gap-2">
                      <div
                        className="w-9 h-9 rounded-lg flex items-center justify-center flex-shrink-0"
                        style={{ background: '#AF904120' }}
                      >
                        <User
                          className="h-4 w-4"
                          style={{ color: '#AF9041' }}
                        />
                      </div>
                      <div className="flex-1 min-w-0">
                        <p className="text-sm font-semibold truncate">
                          {a.name}
                        </p>
                        <div className="flex items-center gap-1.5 mt-0.5">
                          <span
                            className="w-1.5 h-1.5 rounded-full"
                            style={{
                              background:
                                a.status === 'active' ? '#10B981' : '#6B7280',
                            }}
                          />
                          <span className="text-[10px] text-muted-foreground capitalize">
                            {a.status}
                          </span>
                        </div>
                      </div>
                    </div>
                    <div className="space-y-1">
                      <div className="flex items-center gap-2 text-[10px] text-muted-foreground">
                        <Video className="h-3 w-3 flex-shrink-0" />
                        <span className="truncate font-mono">
                          {a.heygen_avatar_id ?? 'No HeyGen ID'}
                        </span>
                      </div>
                      <div className="flex items-center gap-2 text-[10px] text-muted-foreground">
                        <AudioLines className="h-3 w-3 flex-shrink-0" />
                        <span className="truncate font-mono">
                          {a.elevenlabs_voice_id}
                        </span>
                      </div>
                    </div>
                    {a.style_notes && (
                      <p className="text-[10px] text-muted-foreground leading-relaxed line-clamp-2">
                        {a.style_notes}
                      </p>
                    )}
                    <button
                      onClick={() => {
                        setNewJobAnchorId(a.id);
                        setShowNewJob(true);
                      }}
                      className="w-full px-3 py-1.5 rounded-lg text-xs font-medium text-black flex items-center justify-center gap-1.5 hover:opacity-90 transition-opacity"
                      style={{ background: '#AF9041' }}
                    >
                      <Video className="h-3 w-3" /> Produce Video
                    </button>
                  </div>
                ))}
              </div>
            </>
          )}

          {/* Jobs tab */}
          {leftTab === 'jobs' && (
            <>
              <div className="flex items-center justify-between px-4 py-2 border-b border-border/30">
                <span className="text-[10px] font-semibold text-muted-foreground uppercase tracking-widest">
                  Video Jobs
                </span>
                <div className="flex items-center gap-1">
                  {inProgressCount > 0 && (
                    <RefreshCw className="h-3 w-3 animate-spin text-blue-400" />
                  )}
                  <button
                    onClick={() => setShowNewJob(true)}
                    className="p-1 rounded hover:bg-muted/50 transition-colors"
                  >
                    <Plus className="h-3.5 w-3.5 text-amber-500" />
                  </button>
                </div>
              </div>
              {jobsLoading && <Loader message="Loading..." className="m-4" />}
              <div className="flex-1 overflow-y-auto py-2">
                {!jobsLoading && jobs.length === 0 && (
                  <div className="text-center p-6">
                    <Video className="h-8 w-8 mx-auto mb-2 opacity-20" />
                    <p className="text-xs text-muted-foreground">No jobs yet</p>
                    <button
                      onClick={() => setShowNewJob(true)}
                      className="mt-3 text-xs px-3 py-1.5 rounded-lg font-medium text-black"
                      style={{ background: '#AF9041' }}
                    >
                      + Produce Video
                    </button>
                  </div>
                )}
                {jobs.map((job: VideoJob) => {
                  const anchor = avatars.find(
                    (a: AvatarProfile) => a.id === job.avatar_profile_id
                  );
                  const canPlay =
                    job.status === 'ready' &&
                    (job.final_video_url || job.raw_video_url);
                  const preview =
                    job.script_text.slice(0, 80) +
                    (job.script_text.length > 80 ? '...' : '');
                  return (
                    <div
                      key={job.id}
                      onClick={canPlay ? () => loadJob(job) : undefined}
                      className={`mx-3 mb-2 rounded-xl border p-3 space-y-2 transition-colors ${canPlay ? 'cursor-pointer' : ''} ${activeJobId === job.id ? 'border-amber-500/40 bg-amber-500/5' : 'border-border/50 bg-card/50 hover:border-border'}`}
                    >
                      <div className="flex items-start justify-between gap-2">
                        <div className="flex-1 min-w-0">
                          <p className="text-xs font-medium text-muted-foreground truncate">
                            {anchor?.name ?? 'Unknown'}
                          </p>
                          <p className="text-xs text-foreground/80 leading-relaxed mt-0.5 line-clamp-2">
                            {preview}
                          </p>
                        </div>
                        <JobStatusBadge status={job.status} />
                      </div>
                      {IN_PROGRESS.includes(job.status) && (
                        <p className="text-[10px] text-blue-400 flex items-center gap-1">
                          <Loader2 className="h-3 w-3 animate-spin" />
                          {job.status === 'tts_generating'
                            ? 'Generating speech...'
                            : job.status === 'avatar_generating'
                              ? 'Rendering avatar...'
                              : 'Queued...'}
                        </p>
                      )}
                      {job.status === 'failed' && job.error_message && (
                        <p className="text-[10px] text-red-400 truncate">
                          {job.error_message}
                        </p>
                      )}
                      <div className="flex items-center justify-between">
                        <span className="text-[10px] text-muted-foreground">
                          {timeAgo(job.created_at)}
                        </span>
                        {job.duration_seconds != null && (
                          <span className="text-[10px] text-muted-foreground">
                            {formatTime(job.duration_seconds)}
                          </span>
                        )}
                        {canPlay && (
                          <span
                            className="text-[10px]"
                            style={{ color: '#AF9041' }}
                          >
                            Play
                          </span>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
            </>
          )}
        </div>

        {/* Center: Player */}
        <div className="flex-1 flex flex-col items-center justify-center bg-black/50 p-6 gap-4">
          {resolvedSource ? (
            <>
              <div
                className="relative rounded-xl overflow-hidden shadow-2xl"
                style={{
                  height: 'min(calc(100vh - 280px), 520px)',
                  aspectRatio: '9/16',
                }}
              >
                <video
                  key={resolvedSource.url}
                  ref={videoRef}
                  src={resolvedSource.url}
                  className="w-full h-full object-cover"
                  onTimeUpdate={() =>
                    setCurrentTime(videoRef.current?.currentTime ?? 0)
                  }
                  onLoadedMetadata={() =>
                    setDuration(videoRef.current?.duration ?? 0)
                  }
                  onPlay={() => setPlaying(true)}
                  onPause={() => setPlaying(false)}
                  onEnded={() => setPlaying(false)}
                />
                <div
                  className="absolute top-0 left-0 w-8 h-8 border-t-2 border-l-2 rounded-tl-xl"
                  style={{ borderColor: '#AF9041' }}
                />
                <div
                  className="absolute bottom-0 right-0 w-8 h-8 border-b-2 border-r-2 rounded-br-xl"
                  style={{ borderColor: '#AF9041' }}
                />
              </div>

              <div className="w-full max-w-sm space-y-2">
                <div
                  className="relative h-1.5 bg-muted rounded-full cursor-pointer"
                  onClick={seek}
                >
                  <div
                    className="h-full rounded-full"
                    style={{
                      width: `${progress}%`,
                      background: 'linear-gradient(90deg, #D4B96A, #AF9041)',
                    }}
                  />
                  {OVERLAY_TIMELINE.map((o, i) => (
                    <div
                      key={i}
                      className="absolute top-1/2 -translate-y-1/2 w-1 h-3 rounded-full opacity-60"
                      style={{
                        left: `${(o.time / (duration || 105)) * 100}%`,
                        background: o.color,
                      }}
                    />
                  ))}
                </div>
                <div className="flex items-center gap-3">
                  <button
                    onClick={togglePlay}
                    className="w-8 h-8 rounded-full flex items-center justify-center"
                    style={{ background: '#AF9041' }}
                  >
                    {playing ? (
                      <Pause className="h-4 w-4 text-black" />
                    ) : (
                      <Play className="h-4 w-4 text-black" />
                    )}
                  </button>
                  <span className="text-xs font-mono text-muted-foreground">
                    {formatTime(currentTime)} / {formatTime(duration)}
                  </span>
                  <button
                    onClick={() => videoRef.current?.requestFullscreen()}
                    className="ml-auto p-1.5 rounded hover:bg-muted/50 transition-colors"
                  >
                    <Maximize2 className="h-3.5 w-3.5 text-muted-foreground" />
                  </button>
                </div>
                <p className="text-center text-xs text-muted-foreground font-medium truncate">
                  {resolvedSource.label}
                </p>
              </div>
            </>
          ) : (
            <div className="text-center text-muted-foreground">
              <Film className="h-12 w-12 mx-auto mb-3 opacity-20" />
              <p className="text-sm">Select a video or produce a new one</p>
            </div>
          )}
        </div>

        {/* Right Panel */}
        <div className="w-72 border-l border-border/40 flex flex-col">
          <div className="flex border-b border-border/40">
            {(['timeline', 'assets'] as RightTab[]).map((tab) => (
              <button
                key={tab}
                onClick={() => setRightTab(tab)}
                className={`flex-1 px-3 py-2.5 text-xs font-medium transition-colors ${rightTab === tab ? 'text-amber-400 border-b-2 border-amber-400' : 'text-muted-foreground hover:text-foreground'}`}
              >
                {tab === 'timeline' ? 'Timeline' : 'Overlay Assets'}
              </button>
            ))}
          </div>

          {rightTab === 'timeline' && (
            <>
              <div className="flex-1 overflow-y-auto py-2 px-2">
                <OverlayTimeline currentTime={currentTime} />
              </div>
              <div className="px-4 py-3 border-t border-border/40">
                <p className="text-[10px] text-muted-foreground">
                  {OVERLAY_TIMELINE.length} overlays · 3 B-roll cuts ·{' '}
                  {formatTime(duration)} runtime
                </p>
              </div>
            </>
          )}

          {rightTab === 'assets' && (
            <div className="flex-1 overflow-y-auto">
              {overlaysLoading && (
                <Loader message="Loading assets..." className="m-4" />
              )}
              {overlays?.map((ep: OverlayEpisode) => (
                <div key={ep.episode}>
                  <div className="px-4 py-2 text-[10px] font-semibold text-amber-600/80 uppercase tracking-widest border-b border-border/30 bg-muted/20">
                    {ep.episode}
                  </div>
                  {ep.files.map((f: OverlayFile) => {
                    const stem = f.filename.replace(/\.(png|svg)$/, '');
                    const label =
                      OVERLAY_LABELS[stem] ?? stem.replace(/_/g, ' ');
                    const isTransparent =
                      parseInt(stem.split('_')[0] ?? '9') <= 5;
                    return (
                      <div
                        key={f.filename}
                        className="flex items-center gap-3 px-4 py-2.5 border-b border-border/20 hover:bg-muted/30 transition-colors group"
                      >
                        <div
                          className="w-7 h-7 rounded flex items-center justify-center flex-shrink-0"
                          style={{
                            background: isTransparent
                              ? 'repeating-conic-gradient(#333 0% 25%, #222 0% 50%) 0 0 / 8px 8px'
                              : '#1a1a1a',
                          }}
                        >
                          <ImageIcon className="h-3.5 w-3.5 text-muted-foreground" />
                        </div>
                        <div className="flex-1 min-w-0">
                          <p className="text-xs font-medium truncate">
                            {label}
                          </p>
                          <p className="text-[10px] text-muted-foreground">
                            {isTransparent ? 'Transparent' : 'Full-Frame'} ·{' '}
                            {formatBytes(f.size_bytes)}
                          </p>
                        </div>
                        <a
                          href={`/api${f.download_url}`}
                          download={f.filename}
                          className="opacity-0 group-hover:opacity-100 transition-opacity p-1.5 rounded hover:bg-muted"
                        >
                          <Download className="h-3.5 w-3.5 text-amber-500" />
                        </a>
                      </div>
                    );
                  })}
                </div>
              ))}
              {!overlaysLoading && !overlays?.length && (
                <div className="text-center text-muted-foreground p-8">
                  <ImageIcon className="h-8 w-8 mx-auto mb-2 opacity-20" />
                  <p className="text-xs">No overlay assets found</p>
                </div>
              )}
            </div>
          )}
        </div>
      </div>

      {/* Modals */}
      <NewAnchorModal
        open={showNewAnchor}
        onClose={() => setShowNewAnchor(false)}
      />
      <NewJobModal
        open={showNewJob}
        onClose={() => {
          setShowNewJob(false);
          setNewJobAnchorId(null);
        }}
        avatars={avatars}
        defaultAnchorId={newJobAnchorId}
      />
    </div>
  );
}
