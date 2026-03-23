import { useState, useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Badge } from '@/components/ui/badge';
import { CardGrid } from '@/components/ui/card-grid';
import { Input } from '@/components/ui/input';
import {
  Search, FileText, Table2, Music, Image, Film, Layers,
  Folder, File, Download, Brain, FolderOpen,
  ChevronRight, ChevronDown, SortAsc, SortDesc, LayoutGrid, List,
  Palette, MessageSquare,
} from 'lucide-react';
import { dataSourcesApi, type DataSourceRecord } from '@/lib/api';
import { dataSourceKeys } from '@/lib/query-keys';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { formatDate } from '@/lib/formatters';
import { EmptyState } from '@/components/ui/empty-state';
import { useAuth } from '@/contexts/AuthContext';

// ─── Helpers ──────────────────────────────────────────────────────────────────

interface FolderNode {
  name: string;
  path: string;
  children: Record<string, FolderNode>;
  count: number;
}

type SortField = 'title' | 'data_type' | 'file_size' | 'created_at';
type SortDir = 'asc' | 'desc';

function parseMetadata(raw: string): Record<string, unknown> {
  try { return JSON.parse(raw); } catch { return {}; }
}

function getFolderPath(source: DataSourceRecord): string {
  if (source.folder) {
    // Strip "Personal/" prefix for display within intelligence page
    const folder = source.folder.startsWith('Personal/')
      ? source.folder.slice('Personal/'.length)
      : source.folder;
    return folder.split('/').map((p: string) => p.trim()).filter(Boolean).join(' > ');
  }
  return '';
}

function buildFolderTree(sources: DataSourceRecord[]): FolderNode {
  const root: FolderNode = { name: 'root', path: '', children: {}, count: 0 };
  for (const s of sources) {
    const ctx = getFolderPath(s);
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

// ─── Folder Tree Node ──────────────────────────────────────────────────────────

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
                onSelect={onSelect}
              />
            ))}
        </div>
      )}
    </div>
  );
}

// ─── Main Page ────────────────────────────────────────────────────────────────

export default function IntelligencePage() {
  const { user } = useAuth();
  const [search, setSearch] = useState('');
  const [selectedPath, setSelectedPath] = useState('');
  const [typeFilter, setTypeFilter] = useState('all');
  const [sortField, setSortField] = useState<SortField>('created_at');
  const [sortDir, setSortDir] = useState<SortDir>('desc');
  const [viewMode, setViewMode] = useState<'list' | 'grid'>('list');

  // Fetch all data sources, filter to personal ones
  const sourcesQuery = useQuery({
    queryKey: dataSourceKeys.personal(),
    queryFn: () => dataSourcesApi.listAll(),
    staleTime: 30_000,
  });

  // Filter to personal data only (folder starts with "Personal/")
  const personalSources = useMemo(() => {
    const all = sourcesQuery.data || [];
    return all.filter((s: DataSourceRecord) => s.folder?.startsWith('Personal/'));
  }, [sourcesQuery.data]);

  const tree = useMemo(() => buildFolderTree(personalSources), [personalSources]);

  const filtered = useMemo(() => {
    let list = personalSources.filter((s: DataSourceRecord) => {
      const ctx = getFolderPath(s);
      const matchesPath = !selectedPath || ctx === selectedPath || ctx.startsWith(selectedPath + ' >');
      const matchesSearch = !search || s.title.toLowerCase().includes(search.toLowerCase()) || ctx.toLowerCase().includes(search.toLowerCase());
      const matchesType = typeFilter === 'all' || s.data_type === typeFilter;
      return matchesPath && matchesSearch && matchesType;
    });

    list = [...list].sort((a, b) => {
      let av: string | number, bv: string | number;
      if (sortField === 'title') { av = a.title; bv = b.title; }
      else if (sortField === 'data_type') { av = a.data_type; bv = b.data_type; }
      else if (sortField === 'file_size') { av = a.file_size_bytes || 0; bv = b.file_size_bytes || 0; }
      else { av = a.created_at || ''; bv = b.created_at || ''; }
      if (av < bv) return sortDir === 'asc' ? -1 : 1;
      if (av > bv) return sortDir === 'asc' ? 1 : -1;
      return 0;
    });

    return list;
  }, [personalSources, selectedPath, search, typeFilter, sortField, sortDir]);

  const DATA_TYPES = useMemo(() => {
    const types = new Set(personalSources.map((s: DataSourceRecord) => s.data_type).filter(Boolean));
    return ['all', ...Array.from(types)] as string[];
  }, [personalSources]);

  function toggleSort(field: SortField) {
    if (sortField === field) setSortDir(d => d === 'asc' ? 'desc' : 'asc');
    else { setSortField(field); setSortDir('asc'); }
  }

  function SortIcon({ field }: { field: SortField }) {
    if (sortField !== field) return null;
    return sortDir === 'asc' ? <SortAsc className="h-3 w-3 ml-1" /> : <SortDesc className="h-3 w-3 ml-1" />;
  }

  const displayName = user?.full_name || user?.username || 'My';

  return (
    <div className="flex h-full">
      {/* Folder sidebar */}
      <div className="w-56 shrink-0 border-r bg-card overflow-y-auto p-2">
        <div className="flex items-center gap-2 px-2 py-2 mb-2">
          <Brain className="h-4 w-4 text-primary" />
          <span className="text-sm font-semibold">{displayName}&apos;s Intelligence</span>
        </div>
        <button
          className={`flex items-center gap-1.5 w-full text-left px-2 py-1 rounded-md text-sm transition-colors hover:bg-muted/60 ${!selectedPath ? 'bg-primary/10 text-primary font-medium' : ''}`}
          onClick={() => setSelectedPath('')}
        >
          <Folder className="h-3.5 w-3.5" />
          <span>All Files</span>
          <span className="ml-auto text-[10px] text-muted-foreground">{personalSources.length}</span>
        </button>
        {Object.values(tree.children)
          .sort((a, b) => b.count - a.count)
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
        {/* Header */}
        <div className="border-b px-4 py-3">
          <div className="flex items-center justify-between mb-3">
            <div>
              <h1 className="text-lg font-semibold">Personal Intelligence</h1>
              <p className="text-xs text-muted-foreground">
                {displayName}&apos;s personal files from Sovereign Stack
              </p>
            </div>
            <div className="flex items-center gap-2">
              <Badge variant="secondary" className="text-xs">
                {personalSources.length} files
              </Badge>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <div className="relative flex-1 max-w-sm">
              <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
              <Input
                placeholder="Search files..."
                value={search}
                onChange={e => setSearch(e.target.value)}
                className="pl-8 h-8 text-sm"
              />
            </div>
            <Select value={typeFilter} onValueChange={setTypeFilter}>
              <SelectTrigger className="h-8 w-32 text-xs">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {DATA_TYPES.map(t => (
                  <SelectItem key={t} value={t} className="text-xs capitalize">{t}</SelectItem>
                ))}
              </SelectContent>
            </Select>
            <div className="flex items-center border rounded-md">
              <button
                className={`p-1.5 ${viewMode === 'list' ? 'bg-muted' : ''}`}
                onClick={() => setViewMode('list')}
              >
                <List className="h-3.5 w-3.5" />
              </button>
              <button
                className={`p-1.5 ${viewMode === 'grid' ? 'bg-muted' : ''}`}
                onClick={() => setViewMode('grid')}
              >
                <LayoutGrid className="h-3.5 w-3.5" />
              </button>
            </div>
          </div>
        </div>

        {/* File list */}
        <div className="flex-1 overflow-y-auto">
          {sourcesQuery.isLoading ? (
            <div className="flex items-center justify-center h-32 text-muted-foreground text-sm">Loading...</div>
          ) : filtered.length === 0 ? (
            <EmptyState icon={Brain} title="No personal files" description="Personal files from your Sovereign Stack will appear here." />
          ) : viewMode === 'list' ? (
            <table className="w-full text-sm">
              <thead className="bg-muted/30 sticky top-0">
                <tr className="text-left text-xs text-muted-foreground">
                  <th className="px-4 py-2 cursor-pointer" onClick={() => toggleSort('title')}>
                    <span className="flex items-center">Name <SortIcon field="title" /></span>
                  </th>
                  <th className="px-4 py-2 cursor-pointer" onClick={() => toggleSort('data_type')}>
                    <span className="flex items-center">Type <SortIcon field="data_type" /></span>
                  </th>
                  <th className="px-4 py-2 cursor-pointer" onClick={() => toggleSort('file_size')}>
                    <span className="flex items-center">Size <SortIcon field="file_size" /></span>
                  </th>
                  <th className="px-4 py-2 cursor-pointer" onClick={() => toggleSort('created_at')}>
                    <span className="flex items-center">Added <SortIcon field="created_at" /></span>
                  </th>
                  <th className="px-4 py-2 w-10" />
                </tr>
              </thead>
              <tbody className="divide-y">
                {filtered.map(source => {
                  const meta = parseMetadata(source.metadata);
                  const hasFile = !!(meta.file_path || source.file_path || source.source_type === 'text');
                  return (
                    <tr key={source.id} className="hover:bg-muted/30 transition-colors">
                      <td className="px-4 py-2">
                        <div className="flex items-center gap-2 min-w-0">
                          {fileIcon(source)}
                          <span className="truncate">{source.title}</span>
                          {source.file_type && (
                            <Badge variant="outline" className="text-[10px] uppercase shrink-0">
                              {source.file_type}
                            </Badge>
                          )}
                        </div>
                      </td>
                      <td className="px-4 py-2 capitalize text-muted-foreground">{source.data_type}</td>
                      <td className="px-4 py-2 text-muted-foreground">{formatSize(source.file_size_bytes)}</td>
                      <td className="px-4 py-2 text-muted-foreground">{formatDate(source.created_at)}</td>
                      <td className="px-4 py-2">
                        {hasFile && (
                          <a href={dataSourcesApi.downloadUrl(source.id)} download={source.title}>
                            <Download className="h-3.5 w-3.5 text-muted-foreground hover:text-foreground" />
                          </a>
                        )}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          ) : (
            <CardGrid columns={{ sm: 2, md: 4, lg: 5 }} gap={3} className="p-4">
              {filtered.map(source => (
                <div
                  key={source.id}
                  className="border rounded-lg p-3 hover:bg-muted/30 transition-colors cursor-pointer"
                >
                  <div className="flex items-center justify-center h-16 mb-2">
                    {fileIcon(source, 'lg')}
                  </div>
                  <p className="text-xs font-medium truncate">{source.title}</p>
                  <p className="text-[10px] text-muted-foreground">
                    {formatSize(source.file_size_bytes)}
                  </p>
                </div>
              ))}
            </CardGrid>
          )}
        </div>
      </div>
    </div>
  );
}
