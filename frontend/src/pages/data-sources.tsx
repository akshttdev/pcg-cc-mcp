import { useState, useMemo, useRef, useCallback } from 'react';
import { useParams, useSearchParams } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Search, List, FileText, Table2, Music, Image,
  Film, Layers, Folder, MessageSquare, File, Database,
  Upload, ChevronRight, ChevronDown, Palette,
  Download, Trash2, X, Info, RefreshCw,
  SortAsc, SortDesc, FolderOpen, LayoutGrid,
  MonitorSmartphone, HardDrive, Cloud, CloudOff,
  AlertTriangle, CheckCircle2, Clock, Plus,
  FolderSync, Laptop, Server,
  ArrowDownToLine, ArrowUpFromLine, Shield,
} from 'lucide-react';
import { dataSourcesApi, syncApi, type DataSourceRecord, type SyncDevice } from '@/lib/api';
import { toast } from 'sonner';

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
  const meta = parseMetadata(source.metadata);
  return meta.folder_context || meta.dropbox_path?.split('/').slice(1, -1).join(' > ') || '';
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

function formatDate(iso?: string | null): string {
  if (!iso) return '—';
  try {
    return new Date(iso).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
  } catch { return '—'; }
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

function PreviewPanel({ source, onClose, onDelete }: {
  source: DataSourceRecord;
  onClose: () => void;
  onDelete: (id: string) => void;
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

// ─── Sync Management Panel ─────────────────────────────────────────────────

function SyncPanel({ orgId }: { orgId?: string }) {
  const queryClient = useQueryClient();
  const [showNewFolder, setShowNewFolder] = useState(false);
  const [newFolderName, setNewFolderName] = useState('');
  const [newFolderDesc, setNewFolderDesc] = useState('');
  const [selectedDevice, setSelectedDevice] = useState<string | null>(null);

  const overviewQuery = useQuery({
    queryKey: ['syncOverview', orgId],
    queryFn: () => orgId ? syncApi.getOrgOverview(orgId) : null,
    enabled: !!orgId,
    staleTime: 15_000,
  });

  const foldersQuery = useQuery({
    queryKey: ['syncFolders', orgId],
    queryFn: () => orgId ? syncApi.listFolders(orgId) : [],
    enabled: !!orgId,
    staleTime: 30_000,
  });

  const devicesQuery = useQuery({
    queryKey: ['syncDevices', orgId],
    queryFn: () => orgId ? syncApi.listDevices(orgId) : [],
    enabled: !!orgId,
    staleTime: 15_000,
  });

  const conflictsQuery = useQuery({
    queryKey: ['syncConflicts', selectedDevice],
    queryFn: () => selectedDevice ? syncApi.getConflicts(selectedDevice) : [],
    enabled: !!selectedDevice,
    staleTime: 10_000,
  });

  const deviceStateQuery = useQuery({
    queryKey: ['syncDeviceState', selectedDevice],
    queryFn: () => selectedDevice ? syncApi.getDeviceState(selectedDevice) : [],
    enabled: !!selectedDevice,
    staleTime: 15_000,
  });

  const createFolderMutation = useMutation({
    mutationFn: async () => {
      if (!orgId || !newFolderName.trim()) return;
      await syncApi.createFolder(orgId, {
        name: newFolderName.trim(),
        description: newFolderDesc.trim() || undefined,
      });
    },
    onSuccess: () => {
      toast.success('Sync folder created');
      setNewFolderName('');
      setNewFolderDesc('');
      setShowNewFolder(false);
      queryClient.invalidateQueries({ queryKey: ['syncFolders'] });
      queryClient.invalidateQueries({ queryKey: ['syncOverview'] });
    },
    onError: () => toast.error('Failed to create folder'),
  });

  const deleteFolderMutation = useMutation({
    mutationFn: (id: string) => syncApi.deleteFolder(id),
    onSuccess: () => {
      toast.success('Folder removed');
      queryClient.invalidateQueries({ queryKey: ['syncFolders'] });
      queryClient.invalidateQueries({ queryKey: ['syncOverview'] });
    },
  });

  const resolveConflictMutation = useMutation({
    mutationFn: ({ id, resolution }: { id: string; resolution: 'keep_local' | 'keep_remote' | 'keep_both' }) =>
      syncApi.resolveConflict(id, resolution),
    onSuccess: () => {
      toast.success('Conflict resolved');
      queryClient.invalidateQueries({ queryKey: ['syncConflicts'] });
      queryClient.invalidateQueries({ queryKey: ['syncDeviceState'] });
    },
  });

  const deactivateDeviceMutation = useMutation({
    mutationFn: (id: string) => syncApi.deactivateDevice(id),
    onSuccess: () => {
      toast.success('Device removed');
      setSelectedDevice(null);
      queryClient.invalidateQueries({ queryKey: ['syncDevices'] });
      queryClient.invalidateQueries({ queryKey: ['syncOverview'] });
    },
  });

  const overview = overviewQuery.data;
  const folders = foldersQuery.data || [];
  const devices = devicesQuery.data || [];
  const conflicts = conflictsQuery.data || [];
  const deviceState = deviceStateQuery.data || [];

  function deviceIcon(d: SyncDevice) {
    if (d.device_type === 'server') return <Server className="h-4 w-4" />;
    if (d.device_type === 'laptop') return <Laptop className="h-4 w-4" />;
    return <MonitorSmartphone className="h-4 w-4" />;
  }

  function isOnline(d: SyncDevice) {
    if (!d.last_seen_at) return false;
    const diff = Date.now() - new Date(d.last_seen_at).getTime();
    return diff < 5 * 60 * 1000;
  }

  function syncStatusBadge(status: string) {
    switch (status) {
      case 'synced': return <Badge className="bg-emerald-500/10 text-emerald-500 border-emerald-500/20 text-[10px]"><CheckCircle2 className="h-2.5 w-2.5 mr-1" />Synced</Badge>;
      case 'pending': return <Badge className="bg-amber-500/10 text-amber-500 border-amber-500/20 text-[10px]"><Clock className="h-2.5 w-2.5 mr-1" />Pending</Badge>;
      case 'conflict': return <Badge className="bg-red-500/10 text-red-500 border-red-500/20 text-[10px]"><AlertTriangle className="h-2.5 w-2.5 mr-1" />Conflict</Badge>;
      case 'error': return <Badge className="bg-red-500/10 text-red-500 border-red-500/20 text-[10px]"><CloudOff className="h-2.5 w-2.5 mr-1" />Error</Badge>;
      default: return <Badge variant="outline" className="text-[10px]">{status}</Badge>;
    }
  }

  if (!orgId) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-muted-foreground gap-2 p-12">
        <Cloud className="h-12 w-12 opacity-30" />
        <p className="text-sm font-medium">Select an organization to manage sync</p>
        <p className="text-xs">File sync is scoped to organizations</p>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full overflow-hidden">
      {/* Overview stats */}
      <div className="px-6 py-4 border-b bg-card/50">
        <div className="grid grid-cols-4 gap-4">
          <div className="rounded-lg border bg-card p-3">
            <div className="flex items-center gap-2 text-muted-foreground mb-1">
              <FolderSync className="h-3.5 w-3.5" />
              <span className="text-[11px] font-medium uppercase tracking-wide">Sync Folders</span>
            </div>
            <p className="text-xl font-semibold">{overview?.folder_count ?? folders.length}</p>
          </div>
          <div className="rounded-lg border bg-card p-3">
            <div className="flex items-center gap-2 text-muted-foreground mb-1">
              <MonitorSmartphone className="h-3.5 w-3.5" />
              <span className="text-[11px] font-medium uppercase tracking-wide">Devices</span>
            </div>
            <p className="text-xl font-semibold">
              {overview?.device_count ?? devices.length}
              {overview && overview.active_devices > 0 && (
                <span className="text-xs text-emerald-500 ml-2">{overview.active_devices} online</span>
              )}
            </p>
          </div>
          <div className="rounded-lg border bg-card p-3">
            <div className="flex items-center gap-2 text-muted-foreground mb-1">
              <CheckCircle2 className="h-3.5 w-3.5" />
              <span className="text-[11px] font-medium uppercase tracking-wide">Synced Files</span>
            </div>
            <p className="text-xl font-semibold">
              {overview?.sync_summary.synced ?? 0}
              {overview && overview.sync_summary.pending > 0 && (
                <span className="text-xs text-amber-500 ml-2">{overview.sync_summary.pending} pending</span>
              )}
            </p>
          </div>
          <div className="rounded-lg border bg-card p-3">
            <div className="flex items-center gap-2 text-muted-foreground mb-1">
              <AlertTriangle className="h-3.5 w-3.5" />
              <span className="text-[11px] font-medium uppercase tracking-wide">Conflicts</span>
            </div>
            <p className="text-xl font-semibold">
              {overview?.sync_summary.conflicts ?? 0}
              {overview && overview.sync_summary.errors > 0 && (
                <span className="text-xs text-red-500 ml-2">{overview.sync_summary.errors} errors</span>
              )}
            </p>
          </div>
        </div>
      </div>

      {/* Two-column layout */}
      <div className="flex flex-1 overflow-hidden">
        {/* Left: Folders + Devices */}
        <div className="w-80 shrink-0 border-r overflow-y-auto">
          {/* Sync Folders */}
          <div className="p-4 border-b">
            <div className="flex items-center justify-between mb-3">
              <h3 className="text-sm font-semibold flex items-center gap-2">
                <FolderSync className="h-4 w-4 text-primary" />
                Sync Folders
              </h3>
              <Button size="sm" variant="ghost" className="h-6 w-6 p-0" onClick={() => setShowNewFolder(v => !v)}>
                <Plus className="h-3.5 w-3.5" />
              </Button>
            </div>

            {showNewFolder && (
              <div className="mb-3 rounded-lg border p-3 bg-muted/30 space-y-2">
                <Input
                  value={newFolderName}
                  onChange={e => setNewFolderName(e.target.value)}
                  placeholder="Folder name..."
                  className="h-7 text-sm"
                />
                <Input
                  value={newFolderDesc}
                  onChange={e => setNewFolderDesc(e.target.value)}
                  placeholder="Description (optional)..."
                  className="h-7 text-sm"
                />
                <div className="flex gap-2">
                  <Button size="sm" className="h-6 text-xs" onClick={() => createFolderMutation.mutate()} disabled={!newFolderName.trim()}>
                    Create
                  </Button>
                  <Button size="sm" variant="ghost" className="h-6 text-xs" onClick={() => setShowNewFolder(false)}>
                    Cancel
                  </Button>
                </div>
              </div>
            )}

            {folders.length === 0 ? (
              <div className="text-center py-6 text-muted-foreground">
                <FolderSync className="h-8 w-8 mx-auto mb-2 opacity-30" />
                <p className="text-xs">No sync folders configured</p>
                <p className="text-[10px] mt-1">Create folders to organize synced files</p>
              </div>
            ) : (
              <div className="space-y-1">
                {folders.map(folder => (
                  <div key={folder.id} className="flex items-center gap-2 px-2 py-1.5 rounded-md hover:bg-muted/50 group">
                    <Folder className="h-3.5 w-3.5 text-primary shrink-0" />
                    <div className="flex-1 min-w-0">
                      <p className="text-sm font-medium truncate">{folder.name}</p>
                      <p className="text-[10px] text-muted-foreground truncate">{folder.path}</p>
                    </div>
                    <div className="flex items-center gap-1 shrink-0">
                      {folder.auto_sync ? (
                        <Cloud className="h-3 w-3 text-emerald-500" />
                      ) : (
                        <CloudOff className="h-3 w-3 text-muted-foreground" />
                      )}
                      <button
                        className="p-0.5 rounded hover:bg-destructive/10 opacity-0 group-hover:opacity-100 transition-opacity"
                        onClick={() => {
                          if (confirm(`Remove sync folder "${folder.name}"?`)) {
                            deleteFolderMutation.mutate(folder.id);
                          }
                        }}
                      >
                        <Trash2 className="h-3 w-3 text-muted-foreground hover:text-destructive" />
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Connected Devices */}
          <div className="p-4">
            <h3 className="text-sm font-semibold flex items-center gap-2 mb-3">
              <MonitorSmartphone className="h-4 w-4 text-primary" />
              Connected Devices
            </h3>

            {devices.length === 0 ? (
              <div className="text-center py-6 text-muted-foreground">
                <HardDrive className="h-8 w-8 mx-auto mb-2 opacity-30" />
                <p className="text-xs">No devices connected</p>
                <p className="text-[10px] mt-1">Install pcg-sync on a device to start</p>
                <div className="mt-3 text-left rounded-md bg-muted/50 p-2">
                  <p className="text-[10px] font-mono text-muted-foreground">
                    $ cargo install pcg-sync<br/>
                    $ pcg-sync configure<br/>
                    $ pcg-sync start
                  </p>
                </div>
              </div>
            ) : (
              <div className="space-y-1.5">
                {devices.map(device => {
                  const online = isOnline(device);
                  const isSelected = selectedDevice === device.id;
                  return (
                    <button
                      key={device.id}
                      onClick={() => setSelectedDevice(isSelected ? null : device.id)}
                      className={`w-full flex items-center gap-2.5 px-2.5 py-2 rounded-lg text-left transition-colors group
                        ${isSelected ? 'bg-primary/10 border border-primary/20' : 'hover:bg-muted/50 border border-transparent'}`}
                    >
                      <div className="relative shrink-0">
                        {deviceIcon(device)}
                        <span className={`absolute -bottom-0.5 -right-0.5 w-2 h-2 rounded-full border border-background ${online ? 'bg-emerald-500' : 'bg-muted-foreground/30'}`} />
                      </div>
                      <div className="flex-1 min-w-0">
                        <p className="text-sm font-medium truncate">{device.device_name}</p>
                        <div className="flex items-center gap-2 text-[10px] text-muted-foreground">
                          <span className="capitalize">{device.platform || device.device_type}</span>
                          <span>·</span>
                          {syncStatusBadge(device.sync_status)}
                        </div>
                      </div>
                      <button
                        className="p-1 rounded hover:bg-destructive/10 opacity-0 group-hover:opacity-100 transition-opacity shrink-0"
                        onClick={e => {
                          e.stopPropagation();
                          if (confirm(`Remove device "${device.device_name}"?`)) {
                            deactivateDeviceMutation.mutate(device.id);
                          }
                        }}
                      >
                        <Trash2 className="h-3 w-3 text-muted-foreground" />
                      </button>
                    </button>
                  );
                })}
              </div>
            )}
          </div>
        </div>

        {/* Right: Device detail / sync state */}
        <div className="flex-1 overflow-y-auto">
          {!selectedDevice ? (
            <div className="flex flex-col items-center justify-center h-full text-muted-foreground gap-2">
              <Shield className="h-12 w-12 opacity-20" />
              <p className="text-sm font-medium">Select a device to view sync details</p>
              <p className="text-xs">See file-level sync status, resolve conflicts, and manage subscriptions</p>
            </div>
          ) : (
            <div className="p-4 space-y-4">
              {/* Conflicts section */}
              {conflicts.length > 0 && (
                <div className="rounded-xl border border-red-500/20 bg-red-500/5 p-4">
                  <h4 className="text-sm font-semibold flex items-center gap-2 mb-3 text-red-500">
                    <AlertTriangle className="h-4 w-4" />
                    {conflicts.length} Conflict{conflicts.length !== 1 ? 's' : ''} to Resolve
                  </h4>
                  <div className="space-y-2">
                    {conflicts.map(c => (
                      <div key={c.id} className="flex items-center gap-3 rounded-lg border bg-card p-3">
                        <AlertTriangle className="h-4 w-4 text-red-500 shrink-0" />
                        <div className="flex-1 min-w-0">
                          <p className="text-sm font-medium truncate">{c.file_path}</p>
                          <p className="text-[10px] text-muted-foreground">
                            {c.conflict_type === 'both_modified' ? 'Modified on both device and server' : c.conflict_type || 'Unknown conflict'}
                          </p>
                        </div>
                        <div className="flex gap-1.5 shrink-0">
                          <Button
                            size="sm"
                            variant="outline"
                            className="h-6 text-[10px] px-2"
                            onClick={() => resolveConflictMutation.mutate({ id: c.id, resolution: 'keep_local' })}
                          >
                            <ArrowUpFromLine className="h-2.5 w-2.5 mr-1" />
                            Keep Local
                          </Button>
                          <Button
                            size="sm"
                            variant="outline"
                            className="h-6 text-[10px] px-2"
                            onClick={() => resolveConflictMutation.mutate({ id: c.id, resolution: 'keep_remote' })}
                          >
                            <ArrowDownToLine className="h-2.5 w-2.5 mr-1" />
                            Keep Remote
                          </Button>
                          <Button
                            size="sm"
                            variant="outline"
                            className="h-6 text-[10px] px-2"
                            onClick={() => resolveConflictMutation.mutate({ id: c.id, resolution: 'keep_both' })}
                          >
                            Keep Both
                          </Button>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* File sync state table */}
              <div>
                <h4 className="text-sm font-semibold mb-3 flex items-center gap-2">
                  <HardDrive className="h-4 w-4 text-primary" />
                  Sync State ({deviceState.length} files)
                </h4>

                {deviceState.length === 0 ? (
                  <div className="text-center py-8 text-muted-foreground border rounded-lg">
                    <Cloud className="h-8 w-8 mx-auto mb-2 opacity-30" />
                    <p className="text-xs">No files synced yet</p>
                    <p className="text-[10px] mt-1">Files will appear here once the device starts syncing</p>
                  </div>
                ) : (
                  <div className="rounded-lg border overflow-hidden">
                    <table className="w-full text-sm">
                      <thead className="bg-muted/30 border-b">
                        <tr>
                          <th className="text-left px-3 py-2 text-[11px] font-medium text-muted-foreground">File</th>
                          <th className="text-left px-3 py-2 text-[11px] font-medium text-muted-foreground w-24">Status</th>
                          <th className="text-left px-3 py-2 text-[11px] font-medium text-muted-foreground w-20">Direction</th>
                          <th className="text-right px-3 py-2 text-[11px] font-medium text-muted-foreground w-24">Size</th>
                          <th className="text-right px-3 py-2 text-[11px] font-medium text-muted-foreground w-32">Synced</th>
                        </tr>
                      </thead>
                      <tbody>
                        {deviceState.slice(0, 100).map(entry => (
                          <tr key={entry.id} className="border-b last:border-0 hover:bg-muted/20">
                            <td className="px-3 py-1.5">
                              <div className="flex items-center gap-2 min-w-0">
                                <File className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                                <span className="text-xs font-medium truncate">{entry.file_path}</span>
                              </div>
                            </td>
                            <td className="px-3 py-1.5">{syncStatusBadge(entry.sync_status)}</td>
                            <td className="px-3 py-1.5">
                              {entry.sync_direction === 'pull' && (
                                <span className="flex items-center gap-1 text-[10px] text-blue-500">
                                  <ArrowDownToLine className="h-2.5 w-2.5" /> Pull
                                </span>
                              )}
                              {entry.sync_direction === 'push' && (
                                <span className="flex items-center gap-1 text-[10px] text-emerald-500">
                                  <ArrowUpFromLine className="h-2.5 w-2.5" /> Push
                                </span>
                              )}
                            </td>
                            <td className="px-3 py-1.5 text-right text-xs text-muted-foreground font-mono">
                              {entry.file_size ? formatSize(entry.file_size) : '—'}
                            </td>
                            <td className="px-3 py-1.5 text-right text-[10px] text-muted-foreground">
                              {entry.synced_at ? formatDate(entry.synced_at) : '—'}
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                    {deviceState.length > 100 && (
                      <div className="px-3 py-2 text-[10px] text-muted-foreground text-center border-t bg-muted/10">
                        Showing 100 of {deviceState.length} files
                      </div>
                    )}
                  </div>
                )}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

// ─── Main page ────────────────────────────────────────────────────────────────

export default function DataSourcesPage() {
  const { orgId, projectId } = useParams<{ orgId?: string; projectId?: string }>();
  const queryClient = useQueryClient();

  const [searchParams, setSearchParams] = useSearchParams();
  const activeTab = (searchParams.get('tab') === 'sync' ? 'sync' : 'files') as 'files' | 'sync';
  const setActiveTab = (tab: 'files' | 'sync') => {
    const params = new URLSearchParams(searchParams);
    if (tab === 'files') params.delete('tab');
    else params.set('tab', tab);
    setSearchParams(params, { replace: true });
  };

  const [search, setSearch] = useState('');
  const [selectedPath, setSelectedPath] = useState('');
  const [typeFilter, setTypeFilter] = useState('all');
  const [sortField, setSortField] = useState<SortField>('created_at');
  const [sortDir, setSortDir] = useState<SortDir>('desc');
  const [selectedSource, setSelectedSource] = useState<DataSourceRecord | null>(null);
  const [showUploadZone, setShowUploadZone] = useState(false);
  const [showTextModal, setShowTextModal] = useState(false);
  const [viewMode, setViewMode] = useState<'list' | 'grid'>('list');

  // Determine the org from URL
  const effectiveOrgId = orgId;
  const effectiveProjectId = projectId;

  // Fetch sources
  const sourcesQuery = useQuery({
    queryKey: ['dataSources', effectiveOrgId, effectiveProjectId],
    queryFn: async () => {
      if (effectiveOrgId) return dataSourcesApi.listByOrganization(effectiveOrgId);
      if (effectiveProjectId) return dataSourcesApi.listByProject(effectiveProjectId);
      // Global fallback: list all by fetching a sentinel
      const r = await fetch('/api/data-sources?all=1', { credentials: 'include' });
      if (!r.ok) return [];
      const d = await r.json();
      return d.data || [];
    },
    staleTime: 30_000,
  });

  const sources = sourcesQuery.data || [];

  const deleteMutation = useMutation({
    mutationFn: (id: string) => dataSourcesApi.delete(id),
    onSuccess: () => {
      toast.success('Deleted');
      queryClient.invalidateQueries({ queryKey: ['dataSources'] });
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
      else if (sortField === 'file_size') { av = a.file_size || 0; bv = b.file_size || 0; }
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

  const invalidate = () => queryClient.invalidateQueries({ queryKey: ['dataSources'] });

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

        {/* Tab toggle */}
        <div className="flex items-center gap-0.5 border rounded-lg p-0.5 bg-muted/30 ml-4">
          <button
            onClick={() => setActiveTab('files')}
            className={`px-3 py-1 rounded-md text-xs font-medium transition-colors flex items-center gap-1.5
              ${activeTab === 'files' ? 'bg-background shadow-sm text-foreground' : 'text-muted-foreground hover:text-foreground'}`}
          >
            <Database className="h-3 w-3" />
            Files
          </button>
          <button
            onClick={() => setActiveTab('sync')}
            className={`px-3 py-1 rounded-md text-xs font-medium transition-colors flex items-center gap-1.5
              ${activeTab === 'sync' ? 'bg-background shadow-sm text-foreground' : 'text-muted-foreground hover:text-foreground'}`}
          >
            <FolderSync className="h-3 w-3" />
            Sync
          </button>
        </div>

        <div className="ml-auto flex items-center gap-2">
          {activeTab === 'files' && (
            <>
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
            </>
          )}
        </div>
      </div>

      {/* Sync tab */}
      {activeTab === 'sync' && (
        <div className="flex-1 overflow-hidden">
          <SyncPanel orgId={effectiveOrgId} />
        </div>
      )}

      {/* Upload zone */}
      {activeTab === 'files' && showUploadZone && (
        <div className="px-6 py-4 border-b bg-muted/10">
          <UploadZone
            orgId={effectiveOrgId}
            projectId={effectiveProjectId}
            onUploaded={() => { invalidate(); setShowUploadZone(false); }}
          />
        </div>
      )}

      {/* Body */}
      {activeTab === 'files' && <div className="flex flex-1 overflow-hidden">
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
              <div className="flex flex-col items-center justify-center h-40 text-muted-foreground">
                <Database className="h-8 w-8 mb-2 opacity-40" />
                <p className="text-sm">No files found</p>
                {search && <p className="text-xs mt-1">Try a different search term</p>}
              </div>
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
          />
        )}
      </div>}

      {/* Text modal */}
      {showTextModal && (
        <AddTextModal
          orgId={effectiveOrgId}
          projectId={effectiveProjectId}
          onClose={() => setShowTextModal(false)}
          onAdded={invalidate}
        />
      )}
    </div>
  );
}
