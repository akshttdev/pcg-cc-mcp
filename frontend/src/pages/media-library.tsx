import { useQuery } from '@tanstack/react-query';
import { Image as ImageIcon, Search, Trash2, Upload } from 'lucide-react';
import { useCallback, useRef, useState } from 'react';
import { useParams } from 'react-router-dom';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { CardGrid } from '@/components/ui/card-grid';
import { EmptyState } from '@/components/ui/empty-state';
import { Input } from '@/components/ui/input';
import { Loader } from '@/components/ui/loader';
import { useDebounce } from '@/hooks/useDebounce';
import { useMutationWithToast } from '@/hooks/useMutationWithToast';
import { mediaApi, type MediaAsset } from '@/lib/api';
import { mediaKeys } from '@/lib/query-keys';

function ShotTypeBadge({ type }: { type?: string }) {
  if (!type) return null;
  const colors: Record<string, string> = {
    wide: 'bg-blue-500/20 text-blue-400',
    medium: 'bg-green-500/20 text-green-400',
    close_up: 'bg-orange-500/20 text-orange-400',
    aerial: 'bg-purple-500/20 text-purple-400',
    extreme_close_up: 'bg-red-500/20 text-red-400',
  };
  const cls = colors[type] ?? 'bg-muted text-muted-foreground';
  return (
    <span
      className={`inline-block text-xs px-1.5 py-0.5 rounded font-medium ${cls}`}
    >
      {type.replace('_', ' ')}
    </span>
  );
}

function EnergyBar({ value }: { value: number }) {
  const pct = Math.round(value * 100);
  const color = value > 0.7 ? '#ef4444' : value > 0.4 ? '#f59e0b' : '#22c55e';
  return (
    <div className="flex items-center gap-1.5">
      <div className="flex-1 h-1.5 bg-muted rounded-full overflow-hidden">
        <div
          style={{ width: `${pct}%`, background: color }}
          className="h-full rounded-full"
        />
      </div>
      <span className="text-xs text-muted-foreground w-6 text-right">
        {pct}%
      </span>
    </div>
  );
}

function AssetThumb({ asset }: { asset: MediaAsset }) {
  const src = `/api/media/${asset.id}/file`;
  const isImage = asset.mime_type?.startsWith('image/');
  const isVideo = asset.mime_type?.startsWith('video/');

  if (isImage) {
    return (
      <img
        src={src}
        alt={asset.filename}
        className="w-full h-full object-cover"
        loading="lazy"
        data-testid={`media-thumb-${asset.id}`}
      />
    );
  }
  if (isVideo) {
    return (
      <video
        src={src}
        className="w-full h-full object-cover"
        muted
        playsInline
        preload="metadata"
        data-testid={`media-thumb-${asset.id}`}
      />
    );
  }
  return <ImageIcon className="h-10 w-10 text-muted-foreground/40" />;
}

function AssetCard({
  asset,
  onClick,
  onDelete,
}: {
  asset: MediaAsset;
  onClick: () => void;
  onDelete: () => void;
}) {
  const tags: string[] = (() => {
    try {
      return JSON.parse(asset.scene_tags) as string[];
    } catch {
      return [];
    }
  })();

  return (
    <div
      className="group relative border border-border rounded-lg overflow-hidden bg-card hover:border-primary/50 transition-colors cursor-pointer"
      onClick={onClick}
    >
      {/* Thumbnail / preview from the streaming endpoint */}
      <div className="aspect-video bg-muted flex items-center justify-center relative overflow-hidden">
        <AssetThumb asset={asset} />
        {asset.analysis_status === 'running' && (
          <div className="absolute inset-0 bg-black/50 flex items-center justify-center">
            <Loader message="Analysing…" size={20} />
          </div>
        )}
        {asset.analysis_status === 'pending' && (
          <div className="absolute top-2 right-2">
            <Badge variant="outline" className="text-xs">
              pending
            </Badge>
          </div>
        )}
      </div>

      <div className="p-2 space-y-1.5">
        <div className="flex items-start justify-between gap-1">
          <p
            className="text-xs font-medium truncate flex-1"
            title={asset.filename}
          >
            {asset.filename}
          </p>
          <button
            onClick={(e) => {
              e.stopPropagation();
              onDelete();
            }}
            className="opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-destructive transition-opacity"
          >
            <Trash2 className="h-3 w-3" />
          </button>
        </div>

        <div className="flex items-center gap-1 flex-wrap">
          <ShotTypeBadge type={asset.shot_type ?? undefined} />
          {asset.analysis_status === 'done' && (
            <span className="text-xs text-muted-foreground ml-auto">
              {Math.round(asset.ai_confidence * 100)}% confidence
            </span>
          )}
        </div>

        {asset.analysis_status === 'done' && (
          <EnergyBar value={asset.energy_level} />
        )}

        {tags.length > 0 && (
          <div className="flex flex-wrap gap-1">
            {tags.slice(0, 3).map((tag) => (
              <span
                key={tag}
                className="text-xs bg-muted px-1 rounded text-muted-foreground"
              >
                {tag}
              </span>
            ))}
            {tags.length > 3 && (
              <span className="text-xs text-muted-foreground">
                +{tags.length - 3}
              </span>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

function AssetDetailPanel({
  asset,
  onClose,
}: {
  asset: MediaAsset;
  onClose: () => void;
}) {
  const tags: string[] = (() => {
    try {
      return JSON.parse(asset.scene_tags) as string[];
    } catch {
      return [];
    }
  })();

  return (
    <div className="w-72 shrink-0 border-l border-border p-4 space-y-4 overflow-y-auto">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold">Asset Details</h3>
        <button
          onClick={onClose}
          className="text-muted-foreground hover:text-foreground text-xs"
        >
          ✕
        </button>
      </div>

      <div className="aspect-video bg-muted rounded-lg flex items-center justify-center overflow-hidden">
        {asset.mime_type?.startsWith('video/') ? (
          <video
            src={`/api/media/${asset.id}/file`}
            className="w-full h-full object-contain bg-black"
            controls
            playsInline
            preload="metadata"
            data-testid={`media-detail-${asset.id}`}
          />
        ) : asset.mime_type?.startsWith('image/') ? (
          <img
            src={`/api/media/${asset.id}/file`}
            alt={asset.filename}
            className="w-full h-full object-contain"
            data-testid={`media-detail-${asset.id}`}
          />
        ) : (
          <ImageIcon className="h-12 w-12 text-muted-foreground/40" />
        )}
      </div>

      <div className="space-y-1">
        <p className="text-xs font-medium">{asset.filename}</p>
        <p className="text-xs text-muted-foreground">
          {asset.mime_type} · {(asset.file_size_bytes / 1024 / 1024).toFixed(2)}{' '}
          MB
        </p>
      </div>

      {asset.ai_description && (
        <div>
          <p className="text-xs font-medium mb-1 text-muted-foreground uppercase tracking-wide">
            AI Description
          </p>
          <p className="text-xs">{asset.ai_description}</p>
        </div>
      )}

      <div className="space-y-2">
        {asset.shot_type && (
          <div className="flex items-center justify-between">
            <span className="text-xs text-muted-foreground">Shot type</span>
            <ShotTypeBadge type={asset.shot_type} />
          </div>
        )}
        <div className="flex items-center justify-between">
          <span className="text-xs text-muted-foreground">Energy</span>
          <div className="w-24">
            <EnergyBar value={asset.energy_level} />
          </div>
        </div>
        <div className="flex items-center justify-between">
          <span className="text-xs text-muted-foreground">Confidence</span>
          <span className="text-xs">
            {Math.round(asset.ai_confidence * 100)}%
          </span>
        </div>
      </div>

      {tags.length > 0 && (
        <div>
          <p className="text-xs font-medium mb-1 text-muted-foreground uppercase tracking-wide">
            Tags
          </p>
          <div className="flex flex-wrap gap-1">
            {tags.map((tag) => (
              <Badge key={tag} variant="secondary" className="text-xs">
                {tag}
              </Badge>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

export function MediaLibraryPage() {
  const { projectId } = useParams<{ projectId: string }>();
  const fileRef = useRef<HTMLInputElement>(null);

  const [search, setSearch] = useState('');
  const [selectedAsset, setSelectedAsset] = useState<MediaAsset | null>(null);
  const debouncedSearch = useDebounce(search, 300);

  const queryKey = ['media', projectId, debouncedSearch];

  const { data: assets = [], isLoading } = useQuery({
    queryKey,
    queryFn: () =>
      debouncedSearch.trim()
        ? mediaApi.search(projectId!, debouncedSearch)
        : mediaApi.list(projectId!),
    enabled: !!projectId,
    refetchInterval: 10000, // poll for analysis updates
  });

  const uploadMutation = useMutationWithToast({
    mutationFn: (file: File) => {
      const fd = new FormData();
      fd.append('file', file);
      return mediaApi.upload(projectId!, fd);
    },
    successMessage: 'Uploaded — AI analysis started.',
    errorMessage: 'Upload failed',
    invalidateKeys: [mediaKeys.library(projectId!)],
  });

  const deleteMutation = useMutationWithToast({
    mutationFn: (id: string) => mediaApi.delete(id),
    successMessage: 'Asset deleted',
    errorMessage: 'Failed to delete asset',
    invalidateKeys: [mediaKeys.library(projectId!)],
    onSuccess: () => {
      if (selectedAsset) setSelectedAsset(null);
    },
  });

  const handleFiles = useCallback(
    (files: FileList | null) => {
      if (!files) return;
      Array.from(files).forEach((f) => uploadMutation.mutate(f));
    },
    [uploadMutation]
  );

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center gap-3 p-4 border-b border-border">
        <div className="relative flex-1">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            className="pl-8"
            placeholder="Search media by description, tags, shot type…"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>
        <Button
          size="sm"
          onClick={() => fileRef.current?.click()}
          disabled={uploadMutation.isPending}
        >
          <Upload className="h-4 w-4 mr-1.5" />
          Upload
        </Button>
        <input
          ref={fileRef}
          type="file"
          className="hidden"
          multiple
          accept="video/*,image/*"
          onChange={(e) => handleFiles(e.target.files)}
        />
      </div>

      {/* Content area */}
      <div className="flex flex-1 min-h-0">
        <div className="flex-1 overflow-y-auto p-4">
          {isLoading ? (
            <div className="flex items-center justify-center h-48">
              <Loader message="Loading media…" size={24} />
            </div>
          ) : assets.length === 0 ? (
            <EmptyState
              icon={ImageIcon}
              title={search ? 'No results for that search' : 'No media yet'}
              description={search ? undefined : 'Upload files to get started'}
              className="h-48"
            />
          ) : (
            <CardGrid columns={{ sm: 2, md: 3, lg: 4, xl: 5 }} gap={3}>
              {assets.map((asset) => (
                <AssetCard
                  key={asset.id}
                  asset={asset}
                  onClick={() => setSelectedAsset(asset)}
                  onDelete={() => deleteMutation.mutate(asset.id)}
                />
              ))}
            </CardGrid>
          )}
        </div>

        {selectedAsset && (
          <AssetDetailPanel
            asset={selectedAsset}
            onClose={() => setSelectedAsset(null)}
          />
        )}
      </div>
    </div>
  );
}
