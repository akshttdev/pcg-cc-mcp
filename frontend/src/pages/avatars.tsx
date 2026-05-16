import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  CheckCircle2,
  Clock,
  Loader2,
  Plus,
  Sparkles,
  Trash2,
  Upload,
} from 'lucide-react';
import { useMemo, useRef, useState } from 'react';
import { avatars as tid } from 'shared/testids';
import type { AvatarProfile } from 'shared/types';

import {
  avatarsApi,
  type CharacterBible,
  parseBible,
  parsePortraitSet,
  type PortraitShot,
} from '@/lib/api/avatars';
import { cn } from '@/lib/utils';

const POLL_INTERVAL_MS = 4000;

const SHOT_CATEGORIES: Array<{
  key: 'identity' | 'wardrobe' | 'action';
  label: string;
  slots: string[];
}> = [
  {
    key: 'identity',
    label: 'Identity Sheet',
    slots: [
      'front',
      'three_quarter_left',
      'three_quarter_right',
      'profile_left',
      'profile_right',
      'full_body_front',
      'smile_warm',
      'serious_neutral',
    ],
  },
  {
    key: 'wardrobe',
    label: 'Wardrobe',
    slots: [
      'studio_black',
      'editorial_white',
      'casual_outdoor',
      'evening_dressed',
    ],
  },
  {
    key: 'action',
    label: 'Action',
    slots: [
      'laughing',
      'looking_down',
      'walking_three_quarter',
      'thinking_pose',
    ],
  },
];

const SLOT_LABELS: Record<string, string> = {
  front: 'Front',
  three_quarter_left: '3/4 Left',
  three_quarter_right: '3/4 Right',
  profile_left: 'Profile L',
  profile_right: 'Profile R',
  full_body_front: 'Full Body',
  smile_warm: 'Smile',
  serious_neutral: 'Serious',
  studio_black: 'Studio Black',
  editorial_white: 'Editorial White',
  casual_outdoor: 'Casual Outdoor',
  evening_dressed: 'Evening',
  laughing: 'Laughing',
  looking_down: 'Looking Down',
  walking_three_quarter: 'Walking',
  thinking_pose: 'Thinking',
};

export function AvatarsPage() {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);

  const avatarsQuery = useQuery({
    queryKey: ['avatars'],
    queryFn: () => avatarsApi.list(),
    refetchInterval: (q) => {
      const data = q.state.data;
      if (!data) return false;
      const generating = data.some(
        (a: AvatarProfile) =>
          a.profile_status === 'generating' || a.profile_status === 'pending'
      );
      return generating ? POLL_INTERVAL_MS : false;
    },
  });

  const avatars = avatarsQuery.data ?? [];
  const selected = avatars.find((a) => a.id === selectedId) ?? null;

  return (
    <div className="p-6 space-y-6" data-testid={tid.page}>
      <header className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold">Avatars</h1>
          <p className="text-sm text-zinc-500">
            One reference image in. A complete identity profile out.
          </p>
        </div>
      </header>

      {selected ? (
        <AvatarDetail avatar={selected} onBack={() => setSelectedId(null)} />
      ) : (
        <AvatarGrid
          avatars={avatars}
          loading={avatarsQuery.isLoading}
          creating={creating}
          onCreateStart={() => setCreating(true)}
          onCreateDone={() => setCreating(false)}
          onSelect={(id) => setSelectedId(id)}
        />
      )}
    </div>
  );
}

// ─── Grid ─────────────────────────────────────────────────────────────────────

interface GridProps {
  avatars: AvatarProfile[];
  loading: boolean;
  creating: boolean;
  onCreateStart: () => void;
  onCreateDone: () => void;
  onSelect: (id: string) => void;
}

function AvatarGrid({
  avatars,
  loading,
  creating,
  onCreateStart,
  onCreateDone,
  onSelect,
}: GridProps) {
  if (loading) {
    return (
      <div className="flex items-center gap-2 text-zinc-500">
        <Loader2 className="h-4 w-4 animate-spin" />
        Loading avatars…
      </div>
    );
  }
  return (
    <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
      {avatars.map((a) => (
        <AvatarCard key={a.id} avatar={a} onClick={() => onSelect(a.id)} />
      ))}
      {creating ? (
        <NewAvatarForm onDone={onCreateDone} />
      ) : (
        <button
          type="button"
          onClick={onCreateStart}
          data-testid={tid.newAvatarCard}
          className="aspect-[3/4] rounded-xl border-2 border-dashed border-zinc-300 dark:border-zinc-700 text-zinc-500 hover:border-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300 transition-colors flex flex-col items-center justify-center gap-2"
        >
          <Plus className="h-8 w-8" />
          <span className="text-sm font-medium">New Avatar</span>
        </button>
      )}
    </div>
  );
}

function AvatarCard({
  avatar,
  onClick,
}: {
  avatar: AvatarProfile;
  onClick: () => void;
}) {
  const status = avatar.profile_status;
  return (
    <button
      type="button"
      onClick={onClick}
      data-testid={tid.card(avatar.id)}
      className="aspect-[3/4] rounded-xl overflow-hidden bg-zinc-100 dark:bg-zinc-900 text-left relative group"
    >
      {avatar.thumbnail_url ? (
        <img
          src={avatar.thumbnail_url}
          alt={avatar.name}
          className="absolute inset-0 w-full h-full object-cover"
        />
      ) : avatar.reference_image_url ? (
        <img
          src={avatar.reference_image_url}
          alt={avatar.name}
          className="absolute inset-0 w-full h-full object-cover opacity-60"
        />
      ) : (
        <div className="absolute inset-0 flex items-center justify-center text-zinc-400">
          <Sparkles className="h-8 w-8" />
        </div>
      )}
      <div className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/80 to-transparent p-3">
        <div className="text-white font-medium truncate">{avatar.name}</div>
        <ProfileStatusBadge status={status} portrait={avatar.portrait_set} />
      </div>
    </button>
  );
}

function ProfileStatusBadge({
  status,
  portrait,
}: {
  status: string;
  portrait: string;
}) {
  const shots = parsePortraitSet(portrait);
  if (status === 'ready') {
    return (
      <div className="flex items-center gap-1 text-xs text-emerald-300">
        <CheckCircle2 className="h-3 w-3" />
        {shots.length} shots
      </div>
    );
  }
  if (status === 'generating' || status === 'pending') {
    return (
      <div className="flex items-center gap-1 text-xs text-amber-300">
        <Loader2 className="h-3 w-3 animate-spin" />
        Generating ({shots.length}/16)
      </div>
    );
  }
  if (status === 'partial') {
    return (
      <div className="flex items-center gap-1 text-xs text-amber-300">
        <CheckCircle2 className="h-3 w-3" />
        {shots.length}/16 shots — retry to fill in
      </div>
    );
  }
  if (status === 'failed') {
    return <div className="text-xs text-red-300">Failed</div>;
  }
  return (
    <div className="flex items-center gap-1 text-xs text-zinc-300">
      <Clock className="h-3 w-3" />
      No profile yet
    </div>
  );
}

// ─── New Avatar Form ──────────────────────────────────────────────────────────

function NewAvatarForm({ onDone }: { onDone: () => void }) {
  const qc = useQueryClient();
  const fileRef = useRef<HTMLInputElement>(null);
  const [name, setName] = useState('');
  const [file, setFile] = useState<File | null>(null);
  const [error, setError] = useState<string | null>(null);

  const createMutation = useMutation({
    mutationFn: async () => {
      if (!name.trim()) throw new Error('Name is required');
      if (!file) throw new Error('Reference image is required');
      const created = await avatarsApi.create({
        organization_id: null,
        name: name.trim(),
        slug: null,
        identity_doc: null,
        style_notes: null,
        heygen_avatar_id: null,
        heygen_avatar_type: null,
        elevenlabs_voice_id: null,
        reference_image_url: null,
        thumbnail_url: null,
        default_background_url: null,
      });
      await avatarsApi.uploadReference(created.id, file);
      return created;
    },
    onSuccess: async () => {
      await qc.invalidateQueries({ queryKey: ['avatars'] });
      onDone();
    },
    onError: (e) => setError(e instanceof Error ? e.message : String(e)),
  });

  const preview = useMemo(
    () => (file ? URL.createObjectURL(file) : null),
    [file]
  );

  return (
    <div
      className="aspect-[3/4] rounded-xl bg-zinc-50 dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-800 p-3 flex flex-col gap-3"
      data-testid={tid.newAvatarDropzone}
    >
      <button
        type="button"
        onClick={() => fileRef.current?.click()}
        className={cn(
          'flex-1 rounded-lg border-2 border-dashed flex items-center justify-center text-zinc-500 overflow-hidden',
          file ? 'border-emerald-500' : 'border-zinc-300 dark:border-zinc-700'
        )}
      >
        {preview ? (
          <img
            src={preview}
            alt="preview"
            className="w-full h-full object-cover"
          />
        ) : (
          <div className="flex flex-col items-center gap-1 text-xs">
            <Upload className="h-6 w-6" />
            <span>Drop image</span>
          </div>
        )}
      </button>
      <input
        ref={fileRef}
        type="file"
        accept="image/*"
        className="hidden"
        onChange={(e) => setFile(e.target.files?.[0] ?? null)}
      />
      <input
        type="text"
        placeholder="Name (e.g. Sami Satoshi)"
        value={name}
        onChange={(e) => setName(e.target.value)}
        data-testid={tid.newAvatarNameInput}
        className="w-full text-sm rounded-md border border-zinc-300 dark:border-zinc-700 bg-transparent px-2 py-1.5"
      />
      {error ? <div className="text-xs text-red-500">{error}</div> : null}
      <div className="flex gap-2">
        <button
          type="button"
          onClick={onDone}
          className="text-xs text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300 px-2 py-1"
        >
          Cancel
        </button>
        <button
          type="button"
          disabled={!name || !file || createMutation.isPending}
          onClick={() => {
            setError(null);
            createMutation.mutate();
          }}
          data-testid={tid.newAvatarSubmit}
          className="flex-1 text-xs bg-zinc-900 dark:bg-zinc-100 text-white dark:text-zinc-900 rounded-md px-2 py-1.5 disabled:opacity-50 flex items-center justify-center gap-1"
        >
          {createMutation.isPending ? (
            <Loader2 className="h-3 w-3 animate-spin" />
          ) : null}
          Create
        </button>
      </div>
    </div>
  );
}

// ─── Detail ───────────────────────────────────────────────────────────────────

function AvatarDetail({
  avatar,
  onBack,
}: {
  avatar: AvatarProfile;
  onBack: () => void;
}) {
  const qc = useQueryClient();
  // Refresh while generating
  useQuery({
    queryKey: ['avatar', avatar.id],
    queryFn: () => avatarsApi.get(avatar.id),
    refetchInterval:
      avatar.profile_status === 'generating' ||
      avatar.profile_status === 'pending'
        ? POLL_INTERVAL_MS
        : false,
    initialData: avatar,
  });

  const generateMutation = useMutation({
    mutationFn: () => avatarsApi.generateProfile(avatar.id),
    onSuccess: async () => {
      await qc.invalidateQueries({ queryKey: ['avatars'] });
      await qc.invalidateQueries({ queryKey: ['avatar', avatar.id] });
    },
  });

  const deleteMutation = useMutation({
    mutationFn: () => avatarsApi.delete(avatar.id),
    onSuccess: async () => {
      await qc.invalidateQueries({ queryKey: ['avatars'] });
      onBack();
    },
  });

  const shots = parsePortraitSet(avatar.portrait_set);
  const bible = parseBible(avatar.bible_json);
  const shotsBySlot = new Map(shots.map((s) => [s.slot, s]));

  const canGenerate =
    !!avatar.reference_image_url &&
    avatar.profile_status !== 'generating' &&
    avatar.profile_status !== 'pending';

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <button
          type="button"
          onClick={onBack}
          className="text-sm text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300"
        >
          ← Back to avatars
        </button>
        <div className="flex gap-2">
          <button
            type="button"
            onClick={() => deleteMutation.mutate()}
            disabled={deleteMutation.isPending}
            className="text-xs text-red-500 hover:text-red-700 px-3 py-1.5 rounded-md inline-flex items-center gap-1"
          >
            <Trash2 className="h-3 w-3" />
            Delete
          </button>
          <button
            type="button"
            onClick={() => generateMutation.mutate()}
            disabled={!canGenerate || generateMutation.isPending}
            data-testid={tid.generateProfile(avatar.id)}
            className="text-sm bg-zinc-900 dark:bg-zinc-100 text-white dark:text-zinc-900 rounded-md px-3 py-1.5 disabled:opacity-50 inline-flex items-center gap-1"
          >
            <Sparkles className="h-3 w-3" />
            {avatar.profile_status === 'generating' ||
            avatar.profile_status === 'pending'
              ? `Generating ${shots.length}/16…`
              : avatar.profile_status === 'ready'
                ? 'Regenerate Profile'
                : avatar.profile_status === 'partial'
                  ? `Retry (${shots.length}/16 done)`
                  : 'Generate Profile'}
          </button>
        </div>
      </div>

      <div className="flex gap-6 items-start">
        <div className="w-40 shrink-0 aspect-[3/4] rounded-xl overflow-hidden bg-zinc-100 dark:bg-zinc-900">
          {avatar.reference_image_url ? (
            <img
              src={avatar.reference_image_url}
              alt={avatar.name}
              className="w-full h-full object-cover"
            />
          ) : null}
        </div>
        <div>
          <h2 className="text-xl font-semibold">{avatar.name}</h2>
          <div className="text-xs text-zinc-500 mt-1">{avatar.id}</div>
          <div className="mt-3">
            <ProfileStatusBadge
              status={avatar.profile_status}
              portrait={avatar.portrait_set}
            />
          </div>
          {avatar.profile_error ? (
            <div className="mt-2 text-xs text-red-500 max-w-md">
              {avatar.profile_error}
            </div>
          ) : null}
        </div>
      </div>

      {SHOT_CATEGORIES.map((cat) => (
        <section key={cat.key} className="space-y-2">
          <h3 className="text-sm font-semibold uppercase tracking-wide text-zinc-500">
            {cat.label}
          </h3>
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
            {cat.slots.map((slot) => (
              <ShotTile key={slot} slot={slot} shot={shotsBySlot.get(slot)} />
            ))}
          </div>
        </section>
      ))}

      {bible ? <BibleView bible={bible} /> : null}
    </div>
  );
}

function ShotTile({
  slot,
  shot,
}: {
  slot: string;
  shot: PortraitShot | undefined;
}) {
  return (
    <div
      className="aspect-square rounded-lg bg-zinc-100 dark:bg-zinc-900 overflow-hidden relative"
      data-testid={tid.shot(slot)}
    >
      {shot ? (
        <img
          src={shot.url}
          alt={SLOT_LABELS[slot] ?? slot}
          className="w-full h-full object-cover"
        />
      ) : (
        <div className="absolute inset-0 flex items-center justify-center text-zinc-400 text-xs">
          {SLOT_LABELS[slot] ?? slot}
        </div>
      )}
      <div className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/70 to-transparent text-white text-[10px] px-2 py-1">
        {SLOT_LABELS[slot] ?? slot}
      </div>
    </div>
  );
}

function BibleView({ bible }: { bible: CharacterBible }) {
  return (
    <section
      className="space-y-2 border border-zinc-200 dark:border-zinc-800 rounded-lg p-4"
      data-testid={tid.bibleSection}
    >
      <h3 className="text-sm font-semibold uppercase tracking-wide text-zinc-500">
        Character Bible
      </h3>
      <dl className="grid grid-cols-2 gap-x-6 gap-y-2 text-sm">
        <Field label="Age" value={bible.age_range} />
        <Field label="Presentation" value={bible.gender_presentation} />
        <Field label="Hair" value={bible.hair} />
        <Field label="Eyes" value={bible.eyes} />
        <Field label="Skin" value={bible.skin} />
        <Field label="Build" value={bible.build} />
        <Field label="Wardrobe" value={bible.wardrobe_defaults} />
        <Field label="Vibe" value={bible.style_vibe} />
      </dl>
      {bible.distinguishing_features?.length ? (
        <div className="text-sm">
          <span className="text-zinc-500">Features: </span>
          {bible.distinguishing_features.join(', ')}
        </div>
      ) : null}
      {bible.do_not_do?.length ? (
        <div className="text-sm text-amber-600 dark:text-amber-400">
          <span className="font-medium">DO NOT: </span>
          {bible.do_not_do.join('; ')}
        </div>
      ) : null}
    </section>
  );
}

function Field({
  label,
  value,
}: {
  label: string;
  value: string | null | undefined;
}) {
  if (!value) return null;
  return (
    <div>
      <dt className="text-xs text-zinc-500">{label}</dt>
      <dd className="text-sm">{value}</dd>
    </div>
  );
}
