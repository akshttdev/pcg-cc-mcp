import { useState, useMemo, useRef, useCallback } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Search, List, FileText, Table2, Music, Image,
  Film, Layers, Folder, MessageSquare, File, Database,
  Upload, ChevronRight, ChevronDown, Palette,
  Download, Trash2, X, Info, RefreshCw, Play,
  SortAsc, SortDesc, FolderOpen, LayoutGrid, Loader2,
} from 'lucide-react';
import { dataSourcesApi, workflowsApi, type DataSourceRecord } from '@/lib/api';
import { dataSourceKeys, workflowKeys } from '@/lib/query-keys';
import { toast } from 'sonner';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { formatDate } from '@/lib/formatters';
import { EmptyState } from '@/components/ui/empty-state';

// ─── Types ────────────────────────────────────────────────────────────────────

interface FolderNode {
  name: string;
  path: string;
  children: Record<string, FolderNode>;
  count: number;
}

type SortField = 'title' | 'data_type' | 'file_size' | 'created_at';
type SortDir = 'asc' | 'desc';

// ─── Helpers ──────────────────────────────────────────────────────────────────

function parseMetadata(raw: string): Record<string, any> {
  try { return JSON.parse(raw); } catch { return {}; }
}

function getFolderContext(source: DataSourceRecord): string {
  // Use the folder column (slash-delimited) as the canonical source
  if (source.folder) {
    return source.folder.split('/').map((p: string) => p.trim()).filter(Boolean).join(' > ');
  }
  const meta = parseMetadata(source.metadata);
  return meta.folder_context || '';
}

function buildFolderTree(sources: DataSourceRecord[]): FolderNode {
  const root: FolderNode = { name: 'root', path: '', children: {}, count: 0 };
  for (const s of sources) {
    const ctx = getFolderContext(s);
    if (!ctx) continue;
    const parts = ctx.split(' > ').map((p: string) => p.trim()).filter(Boolean);
    let node = root;
    let pathSoFar = '';
    for (const part of parts) {
      pathSoFar = pathSoFar ? `${pathSoFar} > ${part}` : part;
      if (!node.children[part]) {
        node.children[part] = { name: part, path: pathSoFar, children: {}, count: 0 };
      }
      node.children[part].count++;
      node = node.children[part];
    }
  }
  return root;
}

function fileIcon(source: DataSourceRecord, size = 'sm') {
  const cls = size === 'lg' ? 'h-10 w-10' : 'h-4 w-4';
  const ext = (source.file_type || '').toLowerCase();
  if (source.data_type === 'conversation') return <MessageSquare className={`${cls} text-amber-500`} />;
  if (['pdf'].includes(ext)) return <FileText className={`${cls} text-red-500`} />;
  if (['doc', 'docx', 'rtf', 'pages', 'txt', 'md'].includes(ext)) return <FileText className={`${cls} text-blue-500`} />;
  if (['xls', 'xlsx', 'csv', 'numbers'].includes(ext)) return <Table2 className={`${cls} text-green-500`} />;
  if (['ppt', 'pptx', 'key'].includes(ext)) return <FileText className={`${cls} text-orange-500`} />;
  if (['mp3', 'wav', 'aiff', 'm4a', 'flac', 'ogg'].includes(ext)) return <Music className={`${cls} text-purple-500`} />;
  if (['jpg', 'jpeg', 'png', 'gif', 'heic', 'tiff', 'tif', 'webp', 'svg'].includes(ext)) return <Image className={`${cls} text-amber-500`} />;
  if (['cr3', 'cr2', 'arw', 'dng', 'raw', 'nef'].includes(ext)) return <Image className={`${cls} text-amber-600`} />;
  if (['mp4', 'mov', 'avi', 'mkv'].includes(ext)) return <Film className={`${cls} text-blue-600`} />;
  if (['prproj', 'aep', 'ppro'].includes(ext)) return <Film className={`${cls} text-indigo-600`} />;
  if (['psd', 'psb', 'ai', 'sketch'].includes(ext)) return <Layers className={`${cls} text-indigo-500`} />;
  if (['cube', 'lrtemplate', 'xmp'].includes(ext)) return <Palette className={`${cls} text-pink-500`} />;
  return <File className={`${cls} text-muted-foreground`} />;
}

function formatSize(bytes?: number | null): string {
  if (!bytes) return '—';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function hasLocalFile(source: DataSourceRecord): boolean {
  const meta = parseMetadata(source.metadata);
  return !!(meta.file_path || source.file_path || source.source_type === 'text');
}

// ─── FolderTree sidebar ───────────────────────────────────────────────────────

const SECTION_ORDER = ['ACTIVE CLIENTS', 'PROJECTS', 'PROPOSALS', 'SALES', 'SOCIAL MEDIA', 'RESOURCES', 'ARCHIVE'];

function FolderTreeNode({
  node, depth, selectedPath, onSelect,
}: {
  node: FolderNode; depth: number; selectedPath: string; onSelect: (path: string) => void;
}) {
  const hasChildren = Object.keys(node.children).length > 0;
  const isSelected = selectedPath === node.path;
  const isAncestor = selectedPath.startsWith(node.path + ' >');
  const [open, setOpen] = useState(depth < 1);

  return (
    <div>
      <button
        className={`flex items-center gap-1.5 w-full text-left px-2 py-1 rounded-md text-sm transition-colors hover:bg-muted/60
          ${isSelected ? 'bg-primary/10 text-primary font-medium' : ''}
          ${depth === 0 ? 'font-semibold text-[11px] tracking-wide uppercase mt-2 text-muted-foreground' : ''}
        `}
        style={{ paddingLeft: `${8 + depth * 12}px` }}
        onClick={() => {
          onSelect(node.path);
          if (hasChildren) setOpen(o => !o);
        }}
      >
        {hasChildren ? (
          open || isAncestor
            ? <ChevronDown className="h-3 w-3 shrink-0 text-muted-foreground" />
            : <ChevronRight className="h-3 w-3 shrink-0 text-muted-foreground" />
        ) : <span className="w-3 shrink-0" />}
        {depth === 0
          ? <FolderOpen className="h-3.5 w-3.5 shrink-0" />
          : <Folder className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
        }
        <span className="truncate">{node.name}</span>
        <span className="ml-auto text-[10px] text-muted-foreground shrink-0">{node.count}</span>
      </button>
      {(open || isAncestor) && hasChildren && (
        <div>
          {Object.values(node.children)
            .sort((a, b) => b.count - a.count)
            .map(child => (
              <FolderTreeNode
                key={child.path}
                node={child}
                depth={depth + 1}
                selectedPath={selectedPath}
                onSelect={setSelectedPath => onSelect(setSelectedPath)}
              />
            ))}
        </div>
      )}
    </div>
  );
}

// ─── Preview panel ────────────────────────────────────────────────────────────

function PreviewPanel({ source, onClose, onDelete, onRunWorkflow }: {
  source: DataSourceRecord;
  onClose: () => void;
  onDelete: (id: string) => void;
  onRunWorkflow?: (source: DataSourceRecord) => void;
}) {
  const meta = parseMetadata(source.metadata);
  const ext = (source.file_type || '').toLowerCase();
  const isImage = ['jpg', 'jpeg', 'png', 'gif', 'webp', 'svg'].includes(ext);
  const isText = source.source_type === 'text' || ['txt', 'md', 'csv'].includes(ext);
  const downloadable = hasLocalFile(source);

  const handleDownload = useCallback(() => {
    const url = dataSourcesApi.downloadUrl(source.id);
    const a = document.createElement('a');
    a.href = url;
    a.download = source.title;
    a.click();
  }, [source]);

  return (
    <div className="w-80 shrink-0 border-l bg-card flex flex-col h-full">
      {/* Header */}
      <div className="flex items-start gap-2 p-4 border-b">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1">
            {fileIcon(source)}
            <span className="text-sm font-medium truncate">{source.title}</span>
          </div>
          <div className="flex items-center gap-2 flex-wrap">
            <Badge variant="outline" className="text-[10px] capitalize">{source.data_type}</Badge>
            {source.file_type && <Badge variant="outline" className="text-[10px] uppercase">{source.file_type}</Badge>}
            {!downloadable && <Badge variant="secondary" className="text-[10px]">No local file</Badge>}
          </div>
        </div>
        <button onClick={onClose} className="p-1 rounded hover:bg-muted shrink-0">
          <X className="h-4 w-4 text-muted-foreground" />
        </button>
      </div>

      {/* Preview area */}
      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        {/* Visual preview */}
        {isImage && downloadable ? (
          <div className="rounded-lg overflow-hidden border bg-muted/30">
            <img
              src={dataSourcesApi.downloadUrl(source.id)}
              alt={source.title}
              className="w-full object-contain max-h-48"
              onError={(e) => { (e.target as HTMLImageElement).style.display = 'none'; }}
            />
          </div>
        ) : (
          <div className="rounded-lg border bg-muted/20 flex items-center justify-center h-32">
            {fileIcon(source, 'lg')}
          </div>
        )}

        {/* Summary / text preview */}
        {(source as any).summary && (
          <div>
            <p className="text-[11px] font-semibold text-muted-foreground uppercase tracking-wide mb-1">Summary</p>
            <p className="text-sm text-muted-foreground leading-relaxed">{(source as any).summary}</p>
          </div>
        )}

        {/* Text content preview */}
        {isText && source.content && (
          <div>
            <p className="text-[11px] font-semibold text-muted-foreground uppercase tracking-wide mb-1">Content Preview</p>
            <pre className="text-xs bg-muted/40 rounded-md p-3 overflow-auto max-h-40 whitespace-pre-wrap font-mono">
              {source.content.slice(0, 800)}{source.content.length > 800 ? '…' : ''}
            </pre>
          </div>
        )}

        {/* Metadata */}
        <div>
          <p className="text-[11px] font-semibold text-muted-foreground uppercase tracking-wide mb-2">Details</p>
          <div className="space-y-1.5 text-sm">
            {source.file_size_bytes && (
              <div className="flex justify-between">
                <span className="text-muted-foreground">Size</span>
                <span className="font-mono text-xs">{formatSize(source.file_size_bytes)}</span>
              </div>
            )}
            <div className="flex justify-between">
              <span className="text-muted-foreground">Type</span>
              <span className="capitalize">{source.source_type}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-muted-foreground">Added</span>
              <span>{formatDate(source.created_at)}</span>
            </div>
            {meta.dropbox_path && (
              <div className="flex flex-col gap-0.5">
                <span className="text-muted-foreground">Dropbox path</span>
                <span className="text-xs text-muted-foreground/70 truncate font-mono">{meta.dropbox_path}</span>
              </div>
            )}
            {getFolderContext(source) && (
              <div className="flex flex-col gap-0.5">
                <span className="text-muted-foreground">Location</span>
                <span className="text-xs text-muted-foreground/70">{getFolderContext(source)}</span>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Actions */}
      <div className="p-4 border-t flex flex-col gap-2">
        {source.status === 'ready' && onRunWorkflow && (
          <Button size="sm" variant="outline" className="w-full" onClick={() => onRunWorkflow(source)}>
            <Play className="h-3.5 w-3.5 mr-2" />
            Run Workflow
          </Button>
        )}
        {downloadable ? (
          <Button size="sm" className="w-full" onClick={handleDownload}>
            <Download className="h-3.5 w-3.5 mr-2" />
            Download
          </Button>
        ) : (
          <Button size="sm" variant="outline" className="w-full" disabled>
            <Download className="h-3.5 w-3.5 mr-2" />
            Not Available Locally
          </Button>
        )}
        <Button
          size="sm"
          variant="outline"
          className="w-full text-destructive hover:text-destructive"
          onClick={() => {
            if (confirm(`Delete "${source.title}"?`)) onDelete(source.id);
          }}
        >
          <Trash2 className="h-3.5 w-3.5 mr-2" />
          Delete
        </Button>
      </div>
    </div>
  );
}

// ─── Upload drop zone ─────────────────────────────────────────────────────────

function UploadZone({ orgId, projectId, onUploaded }: {
  orgId?: string; projectId?: string; onUploaded: () => void;
}) {
  const [dragging, setDragging] = useState(false);
  const [uploading, setUploading] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  const doUpload = useCallback(async (files: FileList | File[]) => {
    const fileArr = Array.from(files);
    if (!fileArr.length) return;
    setUploading(true);
    let ok = 0;
    for (const file of fileArr) {
      const fd = new FormData();
      fd.append('file', file);
      fd.append('title', file.name);
      fd.append('source_type', 'upload');
      fd.append('data_type', 'document');
      if (orgId) fd.append('organization_id', orgId);
      if (projectId) fd.append('project_id', projectId);
      try {
        await dataSourcesApi.upload(fd);
        ok++;
      } catch {
        toast.error(`Failed to upload ${file.name}`);
      }
    }
    if (ok > 0) {
      toast.success(`Uploaded ${ok} file${ok > 1 ? 's' : ''}`);
      onUploaded();
    }
    setUploading(false);
  }, [orgId, projectId, onUploaded]);

  const onDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setDragging(false);
    doUpload(e.dataTransfer.files);
  }, [doUpload]);

  return (
    <div
      onDragOver={(e) => { e.preventDefault(); setDragging(true); }}
      onDragLeave={() => setDragging(false)}
      onDrop={onDrop}
      className={`border-2 border-dashed rounded-xl p-8 text-center transition-colors cursor-pointer
        ${dragging ? 'border-primary bg-primary/5' : 'border-border hover:border-primary/50 hover:bg-muted/30'}`}
      onClick={() => inputRef.current?.click()}
    >
      <input
        ref={inputRef}
        type="file"
        multiple
        className="hidden"
        onChange={(e) => e.target.files && doUpload(e.target.files)}
      />
      <Upload className={`h-8 w-8 mx-auto mb-3 ${dragging ? 'text-primary' : 'text-muted-foreground'}`} />
      {uploading ? (
        <p className="text-sm text-muted-foreground">Uploading...</p>
      ) : (
        <>
          <p className="text-sm font-medium mb-1">Drop files here or click to upload</p>
          <p className="text-xs text-muted-foreground">Documents, images, audio, video — up to 50MB per file</p>
        </>
      )}
    </div>
  );
}

// ─── Add text source modal ────────────────────────────────────────────────────

function AddTextModal({ orgId, projectId, onClose, onAdded }: {
  orgId?: string; projectId?: string; onClose: () => void; onAdded: () => void;
}) {
  const [title, setTitle] = useState('');
  const [content, setContent] = useState('');
  const [loading, setLoading] = useState(false);

  const handleSave = async () => {
    if (!title.trim()) return;
    setLoading(true);
    try {
      await dataSourcesApi.create({
        title: title.trim(),
        source_type: 'text',
        data_type: 'document',
        content: content || undefined,
        organization_id: orgId,
        project_id: projectId,
      } as any);
      toast.success('Text source added');
      onAdded();
      onClose();
    } catch {
      toast.error('Failed to add text source');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/50 z-50 flex items-center justify-center p-4" onClick={onClose}>
      <div className="bg-card rounded-xl border shadow-xl w-full max-w-lg" onClick={e => e.stopPropagation()}>
        <div className="flex items-center justify-between p-4 border-b">
          <h2 className="font-semibold">Add Text Source</h2>
          <button onClick={onClose}><X className="h-4 w-4 text-muted-foreground" /></button>
        </div>
        <div className="p-4 space-y-3">
          <div>
            <label className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Title</label>
            <Input value={title} onChange={e => setTitle(e.target.value)} placeholder="Document title..." className="mt-1" />
          </div>
          <div>
            <label className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Content</label>
            <textarea
              value={content}
              onChange={e => setContent(e.target.value)}
              placeholder="Paste or type content..."
              className="mt-1 w-full h-40 text-sm rounded-md border bg-background px-3 py-2 resize-none focus:outline-none focus:ring-2 focus:ring-ring"
            />
          </div>
        </div>
        <div className="flex justify-end gap-2 p-4 border-t">
          <Button variant="outline" size="sm" onClick={onClose}>Cancel</Button>
          <Button size="sm" onClick={handleSave} disabled={loading || !title.trim()}>
            {loading ? 'Saving...' : 'Add Source'}
          </Button>
        </div>
      </div>
    </div>
  );
}

// ─── Main page ────────────────────────────────────────────────────────────────

export default function DataSourcesPage() {
  const { orgId, projectId } = useParams<{ orgId?: string; projectId?: string }>();
  const queryClient = useQueryClient();

  const [search, setSearch] = useState('');
  const [selectedPath, setSelectedPath] = useState('');
  const [typeFilter, setTypeFilter] = useState('all');
  const [sortField, setSortField] = useState<SortField>('created_at');
  const [sortDir, setSortDir] = useState<SortDir>('desc');
  const [selectedSource, setSelectedSource] = useState<DataSourceRecord | null>(null);
  const [showUploadZone, setShowUploadZone] = useState(false);
  const [showTextModal, setShowTextModal] = useState(false);
  const [viewMode, setViewMode] = useState<'list' | 'grid'>('list');
  const [runWorkflowSource, setRunWorkflowSource] = useState<DataSourceRecord | null>(null);

  // Determine the org from URL
  const effectiveOrgId = orgId;
  const effectiveProjectId = projectId;

  // Fetch sources
  const sourcesQuery = useQuery({
    queryKey: dataSourceKeys.list(effectiveOrgId, effectiveProjectId),
    queryFn: async () => {
      if (effectiveOrgId) return dataSourcesApi.listByOrganization(effectiveOrgId);
      if (effectiveProjectId) return dataSourcesApi.listByProject(effectiveProjectId);
      // Global fallback: list all
      return dataSourcesApi.listAll();
    },
    staleTime: 30_000,
  });

  // Exclude personal data from org data sources — personal data lives in /intelligence
  const sources = useMemo(() => {
    const all = sourcesQuery.data || [];
    return all.filter((s: DataSourceRecord) => !s.folder?.startsWith('Personal/'));
  }, [sourcesQuery.data]);

  const deleteMutation = useMutation({
    mutationFn: (id: string) => dataSourcesApi.delete(id),
    onSuccess: () => {
      toast.success('Deleted');
      queryClient.invalidateQueries({ queryKey: dataSourceKeys.all });
      setSelectedSource(null);
    },
    onError: () => toast.error('Failed to delete'),
  });

  // Build folder tree
  const tree = useMemo(() => buildFolderTree(sources), [sources]);

  // Filter + sort
  const filtered = useMemo(() => {
    let list = sources.filter((s: DataSourceRecord) => {
      const ctx = getFolderContext(s);
      const matchesPath = !selectedPath || ctx === selectedPath || ctx.startsWith(selectedPath + ' >');
      const matchesSearch = !search || s.title.toLowerCase().includes(search.toLowerCase()) || ctx.toLowerCase().includes(search.toLowerCase());
      const matchesType = typeFilter === 'all' || s.data_type === typeFilter;
      return matchesPath && matchesSearch && matchesType;
    });

    list = [...list].sort((a, b) => {
      let av: any, bv: any;
      if (sortField === 'title') { av = a.title; bv = b.title; }
      else if (sortField === 'data_type') { av = a.data_type; bv = b.data_type; }
      else if (sortField === 'file_size') { av = a.file_size_bytes || 0; bv = b.file_size_bytes || 0; }
      else { av = a.created_at || ''; bv = b.created_at || ''; }
      if (av < bv) return sortDir === 'asc' ? -1 : 1;
      if (av > bv) return sortDir === 'asc' ? 1 : -1;
      return 0;
    });

    return list;
  }, [sources, selectedPath, search, typeFilter, sortField, sortDir]);

  const DATA_TYPES = useMemo(() => {
    const types = new Set(sources.map((s: DataSourceRecord) => s.data_type).filter(Boolean));
    return ['all', ...Array.from(types)] as string[];
  }, [sources]);

  function toggleSort(field: SortField) {
    if (sortField === field) setSortDir(d => d === 'asc' ? 'desc' : 'asc');
    else { setSortField(field); setSortDir('asc'); }
  }

  function SortIcon({ field }: { field: SortField }) {
    if (sortField !== field) return null;
    return sortDir === 'asc' ? <SortAsc className="h-3 w-3 ml-1" /> : <SortDesc className="h-3 w-3 ml-1" />;
  }

  const invalidate = () => queryClient.invalidateQueries({ queryKey: dataSourceKeys.all });

  // Stats
  const totalSize = sources.reduce((acc: number, s: DataSourceRecord) => acc + (s.file_size_bytes || 0), 0);
  const localFiles = sources.filter((s: DataSourceRecord) => hasLocalFile(s)).length;

  return (
    <div className="flex flex-col h-screen bg-background">
      {/* Top bar */}
      <div className="border-b px-6 py-3 flex items-center gap-4 bg-card shrink-0">
        <Database className="h-5 w-5 text-primary" />
        <div>
          <h1 className="text-base font-semibold">Data Sources</h1>
          <p className="text-xs text-muted-foreground">
            {sources.length.toLocaleString()} files · {formatSize(totalSize)} · {localFiles} local
          </p>
        </div>

        <div className="ml-auto flex items-center gap-2">
          {/* Toolbar controls */}
          <div className="relative">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
            <Input
              value={search}
              onChange={e => setSearch(e.target.value)}
              placeholder="Search files..."
              className="pl-8 h-8 text-sm w-56"
            />
          </div>

          <button
            onClick={invalidate}
            className="p-1.5 rounded hover:bg-muted text-muted-foreground"
            title="Refresh"
          >
            <RefreshCw className={`h-4 w-4 ${sourcesQuery.isFetching ? 'animate-spin' : ''}`} />
          </button>

          <div className="flex items-center gap-0.5 border rounded-md p-0.5 bg-muted/30">
            <button onClick={() => setViewMode('list')} className={`p-1 rounded ${viewMode === 'list' ? 'bg-background shadow-sm' : 'text-muted-foreground'}`}>
              <List className="h-3.5 w-3.5" />
            </button>
            <button onClick={() => setViewMode('grid')} className={`p-1 rounded ${viewMode === 'grid' ? 'bg-background shadow-sm' : 'text-muted-foreground'}`}>
              <LayoutGrid className="h-3.5 w-3.5" />
            </button>
          </div>

          <Button size="sm" variant="outline" onClick={() => setShowTextModal(true)}>
            <FileText className="h-3.5 w-3.5 mr-1.5" />
            Add Text
          </Button>
          <Button size="sm" onClick={() => setShowUploadZone(u => !u)}>
            <Upload className="h-3.5 w-3.5 mr-1.5" />
            Upload
          </Button>
        </div>
      </div>

      {/* Upload zone */}
      {showUploadZone && (
        <div className="px-6 py-4 border-b bg-muted/10">
          <UploadZone
            orgId={effectiveOrgId}
            projectId={effectiveProjectId}
            onUploaded={() => { invalidate(); setShowUploadZone(false); }}
          />
        </div>
      )}

      {/* Body */}
      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar */}
        <div className="w-56 shrink-0 border-r bg-card overflow-y-auto p-2">
          {/* All files */}
          <button
            className={`flex items-center gap-1.5 w-full text-left px-2 py-1.5 rounded-md text-sm transition-colors hover:bg-muted/60 ${!selectedPath ? 'bg-primary/10 text-primary font-medium' : ''}`}
            onClick={() => setSelectedPath('')}
          >
            <Database className="h-3.5 w-3.5 shrink-0" />
            <span>All Files</span>
            <span className="ml-auto text-[10px] text-muted-foreground">{sources.length}</span>
          </button>

          {/* Folder tree */}
          {Object.values(tree.children)
            .sort((a, b) => {
              const oa = SECTION_ORDER.indexOf(a.name.toUpperCase());
              const ob = SECTION_ORDER.indexOf(b.name.toUpperCase());
              return (oa === -1 ? 99 : oa) - (ob === -1 ? 99 : ob);
            })
            .map(node => (
              <FolderTreeNode
                key={node.path}
                node={node}
                depth={0}
                selectedPath={selectedPath}
                onSelect={setSelectedPath}
              />
            ))}
        </div>

        {/* Main content */}
        <div className="flex-1 flex flex-col overflow-hidden">
          {/* Sub-toolbar */}
          <div className="px-4 py-2 border-b flex items-center gap-3 bg-card/50">
            {/* Breadcrumb */}
            <div className="flex items-center gap-1 text-xs text-muted-foreground flex-1 min-w-0">
              <button className="hover:text-foreground" onClick={() => setSelectedPath('')}>Files</button>
              {selectedPath.split(' > ').filter(Boolean).map((part, i, arr) => (
                <span key={i} className="flex items-center gap-1">
                  <ChevronRight className="h-3 w-3" />
                  <button
                    className={i === arr.length - 1 ? 'text-foreground font-medium' : 'hover:text-foreground'}
                    onClick={() => setSelectedPath(arr.slice(0, i + 1).join(' > '))}
                  >
                    {part}
                  </button>
                </span>
              ))}
              <span className="ml-2 text-muted-foreground/60">({filtered.length})</span>
            </div>

            {/* Type filter pills */}
            <div className="flex items-center gap-1">
              {DATA_TYPES.map(t => (
                <button
                  key={t}
                  onClick={() => setTypeFilter(t)}
                  className={`px-2 py-0.5 rounded-full text-[11px] capitalize transition-colors border ${typeFilter === t ? 'bg-primary text-primary-foreground border-primary' : 'border-border text-muted-foreground hover:text-foreground hover:border-foreground/30'}`}
                >
                  {t}
                </button>
              ))}
            </div>
          </div>

          {/* File list */}
          <div className="flex-1 overflow-y-auto">
            {sourcesQuery.isLoading ? (
              <div className="flex items-center justify-center h-40 text-muted-foreground text-sm">
                <RefreshCw className="h-4 w-4 animate-spin mr-2" /> Loading files...
              </div>
            ) : filtered.length === 0 ? (
              <EmptyState
                icon={Database}
                title="No files found"
                description={search ? 'Try a different search term' : undefined}
                className="h-40"
              />
            ) : viewMode === 'list' ? (
              <table className="w-full text-sm">
                <thead className="sticky top-0 bg-card border-b">
                  <tr>
                    <th className="text-left px-4 py-2 text-xs font-medium text-muted-foreground w-1/2">
                      <button className="flex items-center hover:text-foreground" onClick={() => toggleSort('title')}>
                        Name <SortIcon field="title" />
                      </button>
                    </th>
                    <th className="text-left px-2 py-2 text-xs font-medium text-muted-foreground">
                      <button className="flex items-center hover:text-foreground" onClick={() => toggleSort('data_type')}>
                        Type <SortIcon field="data_type" />
                      </button>
                    </th>
                    <th className="text-right px-2 py-2 text-xs font-medium text-muted-foreground">
                      <button className="flex items-center ml-auto hover:text-foreground" onClick={() => toggleSort('file_size')}>
                        Size <SortIcon field="file_size" />
                      </button>
                    </th>
                    <th className="text-right px-4 py-2 text-xs font-medium text-muted-foreground">
                      <button className="flex items-center ml-auto hover:text-foreground" onClick={() => toggleSort('created_at')}>
                        Added <SortIcon field="created_at" />
                      </button>
                    </th>
                    <th className="px-4 py-2 w-24" />
                  </tr>
                </thead>
                <tbody>
                  {filtered.map((source: DataSourceRecord) => {
                    const isSelected = selectedSource?.id === source.id;
                    const downloadable = hasLocalFile(source);
                    return (
                      <tr
                        key={source.id}
                        className={`group border-b last:border-0 cursor-pointer transition-colors ${isSelected ? 'bg-primary/5' : 'hover:bg-muted/40'}`}
                        onClick={() => setSelectedSource(isSelected ? null : source)}
                      >
                        <td className="px-4 py-2">
                          <div className="flex items-center gap-2 min-w-0">
                            {fileIcon(source)}
                            <span className="truncate font-medium text-sm">{source.title}</span>
                            {!downloadable && (
                              <span className="text-[10px] text-muted-foreground/50 shrink-0">(cloud only)</span>
                            )}
                          </div>
                        </td>
                        <td className="px-2 py-2">
                          <Badge variant="outline" className="text-[10px] capitalize">{source.data_type}</Badge>
                        </td>
                        <td className="px-2 py-2 text-right font-mono text-xs text-muted-foreground">
                          {formatSize(source.file_size_bytes)}
                        </td>
                        <td className="px-4 py-2 text-right text-xs text-muted-foreground">
                          {formatDate(source.created_at)}
                        </td>
                        <td className="px-4 py-2">
                          <div className="flex items-center gap-1 justify-end opacity-0 group-hover:opacity-100 transition-opacity">
                            {downloadable && (
                              <button
                                title="Download"
                                className="p-1 rounded hover:bg-muted"
                                onClick={e => {
                                  e.stopPropagation();
                                  const url = dataSourcesApi.downloadUrl(source.id);
                                  const a = document.createElement('a');
                                  a.href = url;
                                  a.download = source.title;
                                  a.click();
                                }}
                              >
                                <Download className="h-3.5 w-3.5 text-muted-foreground" />
                              </button>
                            )}
                            <button
                              title="Delete"
                              className="p-1 rounded hover:bg-destructive/10"
                              onClick={e => {
                                e.stopPropagation();
                                if (confirm(`Delete "${source.title}"?`)) deleteMutation.mutate(source.id);
                              }}
                            >
                              <Trash2 className="h-3.5 w-3.5 text-muted-foreground hover:text-destructive" />
                            </button>
                            <button title="Info" className="p-1 rounded hover:bg-muted" onClick={e => { e.stopPropagation(); setSelectedSource(isSelected ? null : source); }}>
                              <Info className="h-3.5 w-3.5 text-muted-foreground" />
                            </button>
                            {source.status === 'ready' && (
                              <button
                                title="Run Workflow"
                                className="p-1 rounded hover:bg-primary/10"
                                onClick={e => { e.stopPropagation(); setRunWorkflowSource(source); }}
                              >
                                <Play className="h-3.5 w-3.5 text-muted-foreground hover:text-primary" />
                              </button>
                            )}
                          </div>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            ) : (
              /* Grid mode */
              <div className="p-4 grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-3">
                {filtered.map((source: DataSourceRecord) => {
                  const isSelected = selectedSource?.id === source.id;
                  const ext = (source.file_type || '').toLowerCase();
                  const isImage = ['jpg', 'jpeg', 'png', 'gif', 'webp'].includes(ext);
                  const downloadable = hasLocalFile(source);
                  return (
                    <div
                      key={source.id}
                      onClick={() => setSelectedSource(isSelected ? null : source)}
                      className={`group relative border rounded-xl p-3 cursor-pointer transition-all hover:shadow-md ${isSelected ? 'border-primary bg-primary/5' : 'bg-card hover:border-primary/40'}`}
                    >
                      {/* Thumbnail or icon */}
                      <div className="w-full aspect-[4/3] rounded-lg bg-muted/40 flex items-center justify-center mb-2 overflow-hidden">
                        {isImage && downloadable ? (
                          <img
                            src={dataSourcesApi.downloadUrl(source.id)}
                            alt={source.title}
                            className="w-full h-full object-cover"
                            onError={e => { (e.target as HTMLImageElement).style.display = 'none'; }}
                          />
                        ) : fileIcon(source, 'lg')}
                      </div>
                      <p className="text-xs font-medium truncate">{source.title}</p>
                      <p className="text-[10px] text-muted-foreground capitalize mt-0.5">{source.data_type} · {formatSize(source.file_size_bytes)}</p>

                      {/* Hover actions */}
                      <div className="absolute top-2 right-2 flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                        {downloadable && (
                          <button
                            className="p-1 rounded bg-background/90 shadow border hover:bg-muted"
                            onClick={e => {
                              e.stopPropagation();
                              const url = dataSourcesApi.downloadUrl(source.id);
                              const a = document.createElement('a');
                              a.href = url; a.download = source.title; a.click();
                            }}
                          >
                            <Download className="h-3 w-3" />
                          </button>
                        )}
                        <button
                          className="p-1 rounded bg-background/90 shadow border hover:bg-destructive/10"
                          onClick={e => {
                            e.stopPropagation();
                            if (confirm(`Delete "${source.title}"?`)) deleteMutation.mutate(source.id);
                          }}
                        >
                          <Trash2 className="h-3 w-3 text-muted-foreground" />
                        </button>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </div>

        {/* Preview panel */}
        {selectedSource && (
          <PreviewPanel
            source={selectedSource}
            onClose={() => setSelectedSource(null)}
            onDelete={(id) => deleteMutation.mutate(id)}
            onRunWorkflow={(s) => setRunWorkflowSource(s)}
          />
        )}
      </div>

      {/* Text modal */}
      {showTextModal && (
        <AddTextModal
          orgId={effectiveOrgId}
          projectId={effectiveProjectId}
          onClose={() => setShowTextModal(false)}
          onAdded={invalidate}
        />
      )}

      {/* Run Workflow dialog */}
      <RunWorkflowFromSourceDialog
        source={runWorkflowSource}
        onClose={() => setRunWorkflowSource(null)}
      />
    </div>
  );
}

// ─── Run Workflow from Data Source ─────────────────────────────────────────────

function RunWorkflowFromSourceDialog({ source, onClose }: {
  source: DataSourceRecord | null;
  onClose: () => void;
}) {
  const navigate = useNavigate();
  const [selectedWorkflowId, setSelectedWorkflowId] = useState('');

  const { data: allWorkflows = [] } = useQuery({
    queryKey: workflowKeys.definitions(),
    queryFn: () => workflowsApi.listDefinitions(),
    enabled: !!source,
  });

  // Show workflows that have a data_source node (purpose-built for processing sources),
  // plus any workflow as a fallback (all workflows can accept data source content)
  const workflows = useMemo(() => {
    const dataSourceWorkflows = allWorkflows.filter(wf =>
      wf.nodes?.some(n => n.type === 'data_source')
    );
    const otherWorkflows = allWorkflows.filter(wf =>
      !wf.nodes?.some(n => n.type === 'data_source')
    );
    return [...dataSourceWorkflows, ...otherWorkflows];
  }, [allWorkflows]);

  const runMutation = useMutation({
    mutationFn: () => dataSourcesApi.runWorkflow(source!.id, selectedWorkflowId),
    onSuccess: (data) => {
      if (data.workflow_run_id && data.staged_records > 0) {
        toast.success(`Workflow complete — ${data.staged_records} records staged`);
        handleClose();
        navigate('/workflows?tab=staging&run=' + data.workflow_run_id);
      } else {
        toast.info('Workflow completed with no new records');
        handleClose();
      }
    },
    onError: () => toast.error('Failed to run workflow'),
  });

  const handleClose = () => {
    setSelectedWorkflowId('');
    runMutation.reset();
    onClose();
  };

  return (
    <Dialog open={!!source} onOpenChange={(open) => { if (!open) handleClose(); }}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Play className="h-4 w-4" />
            Run Workflow
          </DialogTitle>
        </DialogHeader>

        <div className="space-y-4">
          <div>
            <p className="text-sm text-muted-foreground mb-1">Data Source</p>
            <div className="flex items-center gap-2 p-2 rounded-md bg-muted/50 text-sm">
              <FileText className="h-4 w-4 text-muted-foreground shrink-0" />
              <span className="truncate font-medium">{source?.title}</span>
            </div>
          </div>

          <div>
            <p className="text-sm text-muted-foreground mb-1">Workflow</p>
            {workflows.length === 0 ? (
              <EmptyState
                title="No workflows available yet"
                action={{ label: "Create a Workflow", onClick: () => { handleClose(); navigate('/workflows'); } }}
                className="py-4 border rounded-md bg-muted/30"
              />
            ) : (
              <Select value={selectedWorkflowId} onValueChange={setSelectedWorkflowId}>
                <SelectTrigger>
                  <SelectValue placeholder="Select a workflow..." />
                </SelectTrigger>
                <SelectContent>
                  {workflows.map((wf) => (
                    <SelectItem key={wf.id} value={wf.id}>
                      <span>{wf.name}</span>
                      {wf.is_system && <span className="ml-2 text-muted-foreground text-xs">(System)</span>}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )}
          </div>

          <div className="flex gap-2 justify-end">
            <Button variant="ghost" onClick={handleClose}>Cancel</Button>
            <Button
              onClick={() => runMutation.mutate()}
              disabled={!selectedWorkflowId || runMutation.isPending}
            >
              {runMutation.isPending
                ? <Loader2 className="h-3.5 w-3.5 mr-2 animate-spin" />
                : <Play className="h-3.5 w-3.5 mr-2" />}
              {runMutation.isPending ? 'Running...' : 'Run Workflow'}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
