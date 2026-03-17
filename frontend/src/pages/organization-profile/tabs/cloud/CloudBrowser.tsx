import { useState, useCallback, useEffect } from 'react';
import { useQuery } from '@tanstack/react-query';
import {
  FileText, Image, Video, FileSpreadsheet, File, Download, Eye, X,
  Search, ChevronLeft, ChevronRight, Music, FileArchive, Code, FileCode,
} from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent } from '@/components/ui/card';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Skeleton } from '@/components/ui/skeleton';
import { orgCloudApi, type CloudFile, type CloudBrowseParams } from '@/lib/api/org-cloud';

interface CloudBrowserProps {
  orgId: string;
}

// Sovereign stack volumes only — no legacy Dropbox volumes
const VOLUME_OPTIONS = [
  { value: '', label: 'All Volumes' },
  { value: 'sovereign_personal', label: 'Personal' },
  { value: 'sovereign_org', label: 'Organization' },
  { value: 'media_pipeline', label: 'Media Pipeline' },
  { value: 'sovereign', label: 'Sovereign Storage' },
];

function getFileIcon(mimeType: string | null) {
  if (!mimeType) return File;
  if (mimeType.startsWith('image/')) return Image;
  if (mimeType.startsWith('video/')) return Video;
  if (mimeType.startsWith('audio/')) return Music;
  if (mimeType.includes('zip') || mimeType.includes('rar') || mimeType.includes('7z') || mimeType.includes('tar') || mimeType.includes('gzip')) return FileArchive;
  if (mimeType.includes('spreadsheet') || mimeType.includes('csv')) return FileSpreadsheet;
  if (mimeType.includes('javascript') || mimeType.includes('typescript') || mimeType.includes('x-python') || mimeType.includes('x-rust') || mimeType.includes('x-shellscript')) return FileCode;
  if (mimeType.includes('json') || mimeType.includes('xml') || mimeType.includes('yaml') || mimeType.includes('toml') || mimeType.includes('css')) return Code;
  if (mimeType.includes('pdf') || mimeType.includes('text') || mimeType.includes('document')) return FileText;
  return File;
}

function isPreviewable(mimeType: string | null): boolean {
  if (!mimeType) return false;
  return (
    mimeType.startsWith('image/') ||
    mimeType.startsWith('video/') ||
    mimeType.startsWith('audio/') ||
    mimeType === 'application/pdf' ||
    mimeType.startsWith('text/') ||
    mimeType === 'application/json' ||
    mimeType === 'application/xml' ||
    mimeType === 'image/svg+xml'
  );
}

function isTextPreviewable(mimeType: string | null): boolean {
  if (!mimeType) return false;
  return (
    mimeType.startsWith('text/') ||
    mimeType === 'application/json' ||
    mimeType === 'application/xml' ||
    mimeType === 'application/sql'
  );
}

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(i > 0 ? 1 : 0)} ${units[i]}`;
}

const VOLUME_LABELS: Record<string, string> = {
  sovereign_personal: 'Personal',
  sovereign_org: 'Organization',
  media_pipeline: 'Media',
  sovereign: 'Sovereign',
  // Legacy labels kept for any existing DB rows that still render
  dropbox_personal: 'Personal (legacy)',
  dropbox_team: 'Organization (legacy)',
  data_sources: 'Data Sources',
  artifacts: 'Artifacts',
  dropbox: 'Dropbox',
};

export function CloudBrowser({ orgId }: CloudBrowserProps) {
  const [params, setParams] = useState<CloudBrowseParams>({ page: 1, per_page: 20 });
  const [searchInput, setSearchInput] = useState('');
  const [previewFile, setPreviewFile] = useState<CloudFile | null>(null);

  const { data, isLoading } = useQuery({
    queryKey: ['org-cloud', orgId, params],
    queryFn: () => orgCloudApi.browse(orgId, params),
    enabled: !!orgId,
    staleTime: 30_000,
  });

  const files = data?.files ?? [];
  const total = data?.total ?? 0;
  const page = data?.page ?? 1;
  const perPage = data?.per_page ?? 20;
  const totalPages = Math.ceil(total / perPage);

  const handleSearch = () => {
    setParams(p => ({ ...p, search: searchInput || undefined, page: 1 }));
  };

  const handleFileClick = useCallback((file: CloudFile) => {
    if (isPreviewable(file.mime_type)) {
      setPreviewFile(file);
    } else {
      const a = document.createElement('a');
      a.href = orgCloudApi.downloadUrl(orgId, file.id);
      a.download = file.file_name;
      a.target = '_blank';
      a.rel = 'noopener noreferrer';
      a.click();
    }
  }, [orgId]);

  return (
    <div className="space-y-4 mt-4">
      {/* Filters */}
      <div className="flex items-center gap-3 flex-wrap">
        <Select
          value={params.volume ?? ''}
          onValueChange={(v) => setParams(p => ({ ...p, volume: v || undefined, page: 1 }))}
        >
          <SelectTrigger className="w-[180px] h-8 text-xs">
            <SelectValue placeholder="All Volumes" />
          </SelectTrigger>
          <SelectContent>
            {VOLUME_OPTIONS.map(o => (
              <SelectItem key={o.value} value={o.value} className="text-xs">{o.label}</SelectItem>
            ))}
          </SelectContent>
        </Select>

        <Select
          value={params.mime_type ?? ''}
          onValueChange={(v) => setParams(p => ({ ...p, mime_type: v || undefined, page: 1 }))}
        >
          <SelectTrigger className="w-[140px] h-8 text-xs">
            <SelectValue placeholder="All Types" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="" className="text-xs">All Types</SelectItem>
            <SelectItem value="application/pdf" className="text-xs">PDFs</SelectItem>
            <SelectItem value="image/" className="text-xs">Images</SelectItem>
            <SelectItem value="video/" className="text-xs">Videos</SelectItem>
            <SelectItem value="audio/" className="text-xs">Audio</SelectItem>
            <SelectItem value="application/vnd" className="text-xs">Documents</SelectItem>
            <SelectItem value="text/" className="text-xs">Text Files</SelectItem>
          </SelectContent>
        </Select>

        <div className="flex items-center gap-1.5 flex-1 max-w-sm">
          <Input
            placeholder="Search files..."
            value={searchInput}
            onChange={e => setSearchInput(e.target.value)}
            onKeyDown={e => e.key === 'Enter' && handleSearch()}
            className="h-8 text-xs"
          />
          <Button variant="outline" size="sm" onClick={handleSearch} className="h-8 px-2">
            <Search className="h-3.5 w-3.5" />
          </Button>
        </div>

        <span className="text-xs text-muted-foreground ml-auto">
          {total} file{total !== 1 ? 's' : ''}
        </span>
      </div>

      {/* File Grid */}
      {isLoading ? (
        <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-3">
          {Array.from({ length: 10 }).map((_, i) => (
            <Skeleton key={i} className="h-32 rounded-lg" />
          ))}
        </div>
      ) : files.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-16 text-muted-foreground">
          <File className="h-10 w-10 mb-3 opacity-40" />
          <p className="text-sm">No files found</p>
          <p className="text-xs mt-1">Try adjusting filters or index your data sources</p>
        </div>
      ) : (
        <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-3">
          {files.map(file => (
            <FileCard key={file.id} file={file} orgId={orgId} onPreview={handleFileClick} />
          ))}
        </div>
      )}

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="flex items-center justify-between pt-2">
          <span className="text-xs text-muted-foreground">
            Showing {(page - 1) * perPage + 1}–{Math.min(page * perPage, total)} of {total}
          </span>
          <div className="flex items-center gap-1">
            <Button
              variant="outline" size="sm"
              disabled={page <= 1}
              onClick={() => setParams(p => ({ ...p, page: (p.page ?? 1) - 1 }))}
              className="h-7 w-7 p-0"
            >
              <ChevronLeft className="h-3.5 w-3.5" />
            </Button>
            <span className="text-xs px-2">{page} / {totalPages}</span>
            <Button
              variant="outline" size="sm"
              disabled={page >= totalPages}
              onClick={() => setParams(p => ({ ...p, page: (p.page ?? 1) + 1 }))}
              className="h-7 w-7 p-0"
            >
              <ChevronRight className="h-3.5 w-3.5" />
            </Button>
          </div>
        </div>
      )}

      {/* Preview Modal */}
      {previewFile && (
        <FilePreviewModal
          file={previewFile}
          orgId={orgId}
          onClose={() => setPreviewFile(null)}
        />
      )}
    </div>
  );
}

function FileCard({ file, orgId, onPreview }: { file: CloudFile; orgId: string; onPreview: (f: CloudFile) => void }) {
  const Icon = getFileIcon(file.mime_type);
  const downloadUrl = orgCloudApi.downloadUrl(orgId, file.id);
  const previewable = isPreviewable(file.mime_type);

  return (
    <Card
      className="group hover:border-primary/30 transition-colors cursor-pointer"
      onClick={() => onPreview(file)}
    >
      <CardContent className="p-3 space-y-2">
        <div className="flex items-start justify-between">
          <div className="p-1.5 rounded bg-muted">
            <Icon className="h-5 w-5 text-muted-foreground" />
          </div>
          <Badge variant="outline" className="text-[10px] h-4 py-0">
            {VOLUME_LABELS[file.storage_volume] ?? file.storage_volume}
          </Badge>
        </div>
        <div className="min-w-0">
          <p className="text-xs font-medium truncate" title={file.file_name}>
            {file.file_name}
          </p>
          <p className="text-[10px] text-muted-foreground">
            {formatSize(file.file_size_bytes)}
          </p>
        </div>
        <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
          <a
            href={downloadUrl}
            target="_blank"
            rel="noopener noreferrer"
            onClick={e => e.stopPropagation()}
          >
            <Button variant="ghost" size="sm" className="h-6 w-6 p-0">
              <Download className="h-3 w-3" />
            </Button>
          </a>
          {previewable && (
            <Button
              variant="ghost" size="sm" className="h-6 w-6 p-0"
              onClick={e => { e.stopPropagation(); onPreview(file); }}
            >
              <Eye className="h-3 w-3" />
            </Button>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

function TextPreview({ url, fileName }: { url: string; fileName: string }) {
  const [content, setContent] = useState<string | null>(null);
  const [error, setError] = useState(false);

  useEffect(() => {
    const controller = new AbortController();
    fetch(url, { signal: controller.signal, credentials: 'include' })
      .then(res => {
        if (!res.ok) throw new Error('Failed to fetch');
        return res.text();
      })
      .then(text => setContent(text))
      .catch(() => setError(true));
    return () => controller.abort();
  }, [url]);

  if (error) {
    return (
      <div className="flex flex-col items-center gap-2 p-8 text-muted-foreground">
        <FileText className="h-12 w-12" />
        <p className="text-sm">Unable to load preview</p>
      </div>
    );
  }

  if (content === null) {
    return <Skeleton className="w-full h-[60vh]" />;
  }

  const ext = fileName.split('.').pop()?.toLowerCase() ?? '';
  const isCode = ['js', 'jsx', 'ts', 'tsx', 'rs', 'py', 'go', 'java', 'rb', 'sh', 'bash',
    'css', 'scss', 'html', 'xml', 'json', 'yaml', 'yml', 'toml', 'sql', 'md'].includes(ext);

  return (
    <pre className={`w-full h-full min-h-[60vh] overflow-auto p-4 text-xs leading-relaxed ${
      isCode ? 'font-mono bg-zinc-950 text-green-400' : 'font-sans bg-white dark:bg-zinc-900 text-foreground'
    }`}>
      {content.length > 500_000 ? content.slice(0, 500_000) + '\n\n... (truncated)' : content}
    </pre>
  );
}

function FilePreviewModal({ file, orgId, onClose }: { file: CloudFile; orgId: string; onClose: () => void }) {
  const previewUrl = orgCloudApi.previewUrl(orgId, file.id);
  const downloadUrl = orgCloudApi.downloadUrl(orgId, file.id);
  const mime = file.mime_type ?? '';

  const handleBackdropClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget) onClose();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') onClose();
  };

  return (
    <div
      className="fixed inset-0 z-50 bg-black/80 flex items-center justify-center p-4"
      onClick={handleBackdropClick}
      onKeyDown={handleKeyDown}
      tabIndex={-1}
      ref={el => el?.focus()}
    >
      <div className="relative bg-background rounded-lg shadow-2xl max-w-5xl w-full max-h-[90vh] flex flex-col overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b shrink-0">
          <div className="min-w-0 flex-1 mr-4">
            <p className="text-sm font-medium truncate">{file.file_name}</p>
            <p className="text-xs text-muted-foreground">
              {formatSize(file.file_size_bytes)} &middot; {VOLUME_LABELS[file.storage_volume] ?? file.storage_volume}
            </p>
          </div>
          <div className="flex items-center gap-2 shrink-0">
            <a href={downloadUrl} target="_blank" rel="noopener noreferrer">
              <Button variant="outline" size="sm" className="h-7 gap-1.5 text-xs">
                <Download className="h-3 w-3" />
                Download
              </Button>
            </a>
            <Button variant="ghost" size="sm" className="h-7 w-7 p-0" onClick={onClose}>
              <X className="h-4 w-4" />
            </Button>
          </div>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-auto flex items-center justify-center bg-muted/30 min-h-[300px]">
          {mime.startsWith('image/') && mime !== 'image/svg+xml' && (
            <img
              src={previewUrl}
              alt={file.file_name}
              className="max-w-full max-h-[75vh] object-contain"
            />
          )}
          {mime === 'image/svg+xml' && (
            <iframe
              src={previewUrl}
              title={file.file_name}
              className="w-full h-full min-h-[75vh] bg-white"
              sandbox="allow-same-origin"
            />
          )}
          {mime.startsWith('video/') && (
            <video
              src={previewUrl}
              controls
              autoPlay
              className="max-w-full max-h-[75vh]"
            >
              Your browser does not support video playback.
            </video>
          )}
          {mime.startsWith('audio/') && (
            <div className="flex flex-col items-center gap-4 p-8">
              <Music className="h-16 w-16 text-muted-foreground" />
              <audio src={previewUrl} controls autoPlay className="w-full max-w-md">
                Your browser does not support audio playback.
              </audio>
            </div>
          )}
          {mime === 'application/pdf' && (
            <iframe
              src={previewUrl}
              title={file.file_name}
              className="w-full h-full min-h-[75vh]"
            />
          )}
          {isTextPreviewable(mime) && (
            <TextPreview url={previewUrl} fileName={file.file_name} />
          )}
        </div>
      </div>
    </div>
  );
}
