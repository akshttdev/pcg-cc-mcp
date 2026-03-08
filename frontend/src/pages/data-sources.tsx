import { useState, useMemo } from 'react';
import { useParams, useNavigate, Link } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
  ArrowLeft, Search, LayoutGrid, List, FileText, Table2, Music, Image,
  Film, Layers, Folder, MessageSquare, File, Database,
  Upload, Plus, ChevronRight, ChevronDown, Users, TrendingUp, Zap,
  CheckCircle2, RefreshCw, Palette,
  BarChart3, Globe, BookOpen,
} from 'lucide-react';
import { dataSourcesApi, type DataSourceRecord } from '@/lib/api';

// ─── Types ────────────────────────────────────────────────────────────────────

interface FolderNode {
  name: string;
  path: string;
  children: Record<string, FolderNode>;
  count: number;
}

// ─── Constants ────────────────────────────────────────────────────────────────

const SECTION_COLORS: Record<string, string> = {
  'ACTIVE CLIENTS': 'text-blue-600 dark:text-blue-400',
  'PROPOSALS':      'text-amber-600 dark:text-amber-400',
  'SALES':          'text-green-600 dark:text-green-400',
  'SOCIAL MEDIA':   'text-pink-600 dark:text-pink-400',
  'RESOURCES':      'text-purple-600 dark:text-purple-400',
  'PROJECTS':       'text-orange-600 dark:text-orange-400',
  'ARCHIVE':        'text-slate-500 dark:text-slate-400',
};

const SECTION_BG: Record<string, string> = {
  'ACTIVE CLIENTS': 'bg-blue-50 dark:bg-blue-950/30 border-blue-200 dark:border-blue-800',
  'PROPOSALS':      'bg-amber-50 dark:bg-amber-950/30 border-amber-200 dark:border-amber-800',
  'SALES':          'bg-green-50 dark:bg-green-950/30 border-green-200 dark:border-green-800',
  'SOCIAL MEDIA':   'bg-pink-50 dark:bg-pink-950/30 border-pink-200 dark:border-pink-800',
  'RESOURCES':      'bg-purple-50 dark:bg-purple-950/30 border-purple-200 dark:border-purple-800',
  'PROJECTS':       'bg-orange-50 dark:bg-orange-950/30 border-orange-200 dark:border-orange-800',
  'ARCHIVE':        'bg-slate-50 dark:bg-slate-950/30 border-slate-200 dark:border-slate-800',
};

// ─── Helpers ──────────────────────────────────────────────────────────────────

function parseMetadata(raw: string): Record<string, any> {
  try { return JSON.parse(raw); } catch { return {}; }
}

function getFolderContext(source: DataSourceRecord): string {
  const meta = parseMetadata(source.metadata);
  return meta.folder_context || meta.dropbox_path?.split('/').slice(1, -1).join(' > ') || '';
}

function getTopSection(source: DataSourceRecord): string {
  const ctx = getFolderContext(source);
  return ctx.split(' > ')[0]?.toUpperCase() || 'UNCATEGORIZED';
}

function getClientName(source: DataSourceRecord): string {
  const ctx = getFolderContext(source);
  const parts = ctx.split(' > ');
  return parts.length >= 2 ? parts[1] : '';
}

function buildFolderTree(sources: DataSourceRecord[]): FolderNode {
  const root: FolderNode = { name: 'root', path: '', children: {}, count: 0 };
  for (const s of sources) {
    const ctx = getFolderContext(s);
    if (!ctx) continue;
    const parts = ctx.split(' > ').map(p => p.trim()).filter(Boolean);
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

function fileIcon(source: DataSourceRecord) {
  const ext = (source.file_type || '').toLowerCase();
  const isFolder = source.title.startsWith('📁');

  if (isFolder) return <Folder className="h-4 w-4 text-slate-400" />;
  if (source.data_type === 'conversation') return <MessageSquare className="h-4 w-4 text-amber-500" />;

  if (['pdf'].includes(ext)) return <FileText className="h-4 w-4 text-red-500" />;
  if (['doc', 'docx', 'rtf', 'pages', 'txt'].includes(ext)) return <FileText className="h-4 w-4 text-blue-500" />;
  if (['xls', 'xlsx', 'csv', 'numbers'].includes(ext)) return <Table2 className="h-4 w-4 text-green-500" />;
  if (['ppt', 'pptx', 'key'].includes(ext)) return <FileText className="h-4 w-4 text-orange-500" />;
  if (['mp3', 'wav', 'aiff', 'm4a'].includes(ext)) return <Music className="h-4 w-4 text-purple-500" />;
  if (['jpg', 'jpeg', 'png', 'gif', 'heic', 'tiff', 'tif', 'webp'].includes(ext)) return <Image className="h-4 w-4 text-amber-500" />;
  if (['cr3', 'cr2', 'arw', 'dng', 'raw', 'nef'].includes(ext)) return <Image className="h-4 w-4 text-amber-600" />;
  if (['prproj', 'aep', 'ppro'].includes(ext)) return <Film className="h-4 w-4 text-blue-600" />;
  if (['psd', 'psb'].includes(ext)) return <Layers className="h-4 w-4 text-indigo-500" />;
  if (['cube', 'lrtemplate', 'xmp'].includes(ext)) return <Palette className="h-4 w-4 text-pink-500" />;
  return <File className="h-4 w-4 text-muted-foreground" />;
}

function formatSize(bytes?: number): string {
  if (!bytes) return '';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

// ─── FolderTree sidebar ───────────────────────────────────────────────────────

function FolderTreeNode({
  node, depth, selectedPath, onSelect,
}: {
  node: FolderNode; depth: number; selectedPath: string; onSelect: (path: string) => void;
}) {
  const hasChildren = Object.keys(node.children).length > 0;
  const isSelected = selectedPath === node.path;
  const isAncestor = selectedPath.startsWith(node.path + ' >') || selectedPath === node.path;
  const [open, setOpen] = useState(depth < 2);
  const sectionColor = depth === 0 ? (SECTION_COLORS[node.name.toUpperCase()] || 'text-foreground') : '';

  return (
    <div>
      <button
        className={`flex items-center gap-1.5 w-full text-left px-2 py-1 rounded-md text-sm transition-colors hover:bg-muted/60
          ${isSelected ? 'bg-muted font-medium' : ''}
          ${depth === 0 ? 'font-semibold text-xs tracking-wide uppercase mt-2' : ''}
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
        ) : <span className="w-3" />}
        {depth === 0
          ? <span className={sectionColor}>{node.name}</span>
          : <span className="truncate">{node.name}</span>
        }
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

// ─── Library tab ──────────────────────────────────────────────────────────────

function LibraryView({ sources, orgId }: { sources: DataSourceRecord[]; orgId: string }) {
  const [search, setSearch] = useState('');
  const [viewMode, setViewMode] = useState<'list' | 'grid'>('list');
  const [selectedPath, setSelectedPath] = useState('');
  const [typeFilter, setTypeFilter] = useState('all');

  const tree = useMemo(() => buildFolderTree(sources), [sources]);

  const filtered = useMemo(() => {
    return sources.filter(s => {
      const ctx = getFolderContext(s);
      const matchesPath = !selectedPath || ctx === selectedPath || ctx.startsWith(selectedPath + ' >');
      const matchesSearch = !search || s.title.toLowerCase().includes(search.toLowerCase()) || ctx.toLowerCase().includes(search.toLowerCase());
      const matchesType = typeFilter === 'all' || s.data_type === typeFilter;
      return matchesPath && matchesSearch && matchesType;
    });
  }, [sources, selectedPath, search, typeFilter]);

  const DATA_TYPES = ['all', 'document', 'media', 'dataset', 'conversation'];

  return (
    <div className="flex gap-4 h-[calc(100vh-220px)]">
      {/* Folder sidebar */}
      <div className="w-56 shrink-0 border rounded-lg bg-card overflow-y-auto p-2">
        <button
          className={`flex items-center gap-1.5 w-full text-left px-2 py-1 rounded-md text-sm transition-colors hover:bg-muted/60 ${!selectedPath ? 'bg-muted font-medium' : ''}`}
          onClick={() => setSelectedPath('')}
        >
          <Database className="h-3.5 w-3.5 text-muted-foreground" />
          <span>All Sources</span>
          <span className="ml-auto text-[10px] text-muted-foreground">{sources.length}</span>
        </button>
        {Object.values(tree.children)
          .sort((a, b) => {
            const order = ['ACTIVE CLIENTS', 'PROJECTS', 'PROPOSALS', 'SALES', 'SOCIAL MEDIA', 'RESOURCES', 'ARCHIVE'];
            return (order.indexOf(a.name.toUpperCase()) + 1 || 99) - (order.indexOf(b.name.toUpperCase()) + 1 || 99);
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

      {/* Content area */}
      <div className="flex-1 flex flex-col min-w-0">
        {/* Toolbar */}
        <div className="flex items-center gap-2 mb-3">
          <div className="relative flex-1">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
            <Input
              value={search}
              onChange={e => setSearch(e.target.value)}
              placeholder="Search files..."
              className="pl-8 h-8 text-sm"
            />
          </div>
          <div className="flex items-center gap-1 border rounded-md p-0.5 bg-muted/30">
            {DATA_TYPES.map(t => (
              <button
                key={t}
                onClick={() => setTypeFilter(t)}
                className={`px-2 py-0.5 rounded text-xs capitalize transition-colors ${typeFilter === t ? 'bg-background shadow-sm font-medium' : 'text-muted-foreground hover:text-foreground'}`}
              >
                {t}
              </button>
            ))}
          </div>
          <div className="flex items-center gap-0.5 border rounded-md p-0.5 bg-muted/30">
            <button onClick={() => setViewMode('list')} className={`p-1 rounded ${viewMode === 'list' ? 'bg-background shadow-sm' : 'text-muted-foreground'}`}>
              <List className="h-3.5 w-3.5" />
            </button>
            <button onClick={() => setViewMode('grid')} className={`p-1 rounded ${viewMode === 'grid' ? 'bg-background shadow-sm' : 'text-muted-foreground'}`}>
              <LayoutGrid className="h-3.5 w-3.5" />
            </button>
          </div>
        </div>

        {/* Breadcrumb + count */}
        <div className="flex items-center gap-2 mb-2 text-xs text-muted-foreground">
          {selectedPath ? (
            <>
              <span className="flex items-center gap-1">
                {selectedPath.split(' > ').map((part, i, arr) => (
                  <span key={i} className="flex items-center gap-1">
                    {i > 0 && <ChevronRight className="h-3 w-3" />}
                    <span className={i === arr.length - 1 ? 'text-foreground font-medium' : ''}>{part}</span>
                  </span>
                ))}
              </span>
              <span>·</span>
            </>
          ) : null}
          <span>{filtered.length} {filtered.length === 1 ? 'file' : 'files'}</span>
        </div>

        {/* File list / grid */}
        <div className="flex-1 overflow-y-auto">
          {filtered.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-40 text-muted-foreground">
              <File className="h-8 w-8 mb-2 opacity-30" />
              <p className="text-sm">No files match your filters</p>
            </div>
          ) : viewMode === 'list' ? (
            <div className="border rounded-lg overflow-hidden">
              <table className="w-full text-sm">
                <thead className="bg-muted/40 border-b">
                  <tr>
                    <th className="text-left px-3 py-2 text-xs font-medium text-muted-foreground">Name</th>
                    <th className="text-left px-3 py-2 text-xs font-medium text-muted-foreground w-[100px]">Type</th>
                    <th className="text-left px-3 py-2 text-xs font-medium text-muted-foreground w-[80px]">Size</th>
                    <th className="text-left px-3 py-2 text-xs font-medium text-muted-foreground w-[120px]">Folder</th>
                    <th className="text-left px-3 py-2 text-xs font-medium text-muted-foreground w-[90px]">Status</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-border/50">
                  {filtered.map(source => {
                    const ctx = getFolderContext(source);
                    const section = getTopSection(source);
                    return (
                      <tr key={source.id} className="hover:bg-muted/30 transition-colors group">
                        <td className="px-3 py-1.5">
                          <Link
                            to={`/organizations/${orgId}/data-sources/${source.id}`}
                            className="flex items-center gap-2 min-w-0 hover:underline"
                          >
                            {fileIcon(source)}
                            <span className="truncate max-w-[280px]">{source.title}</span>
                          </Link>
                        </td>
                        <td className="px-3 py-1.5">
                          <span className="text-xs text-muted-foreground capitalize">{source.file_type || source.data_type}</span>
                        </td>
                        <td className="px-3 py-1.5 text-xs text-muted-foreground">
                          {formatSize(source.file_size_bytes)}
                        </td>
                        <td className="px-3 py-1.5">
                          <span className={`text-xs truncate max-w-[100px] block ${SECTION_COLORS[section] || 'text-muted-foreground'}`}>
                            {ctx.split(' > ').slice(-1)[0] || ctx}
                          </span>
                        </td>
                        <td className="px-3 py-1.5">
                          <StatusPill status={source.status} />
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          ) : (
            <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-2">
              {filtered.map(source => {
                const section = getTopSection(source);
                return (
                  <Link
                    key={source.id}
                    to={`/organizations/${orgId}/data-sources/${source.id}`}
                    className="border rounded-lg p-3 hover:bg-muted/40 transition-colors flex flex-col gap-1.5 group"
                  >
                    <div className="flex items-center justify-between">
                      {fileIcon(source)}
                      <StatusPill status={source.status} />
                    </div>
                    <p className="text-xs font-medium leading-snug line-clamp-2 group-hover:underline">{source.title}</p>
                    <p className={`text-[10px] truncate ${SECTION_COLORS[section] || 'text-muted-foreground'}`}>
                      {getFolderContext(source).split(' > ').slice(-1)[0]}
                    </p>
                    {source.file_size_bytes ? (
                      <p className="text-[10px] text-muted-foreground">{formatSize(source.file_size_bytes)}</p>
                    ) : null}
                  </Link>
                );
              })}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

// ─── Clients tab ──────────────────────────────────────────────────────────────

function ClientsView({ sources, orgId }: { sources: DataSourceRecord[]; orgId: string }) {
  const clientSources = useMemo(() => sources.filter(s => getTopSection(s) === 'ACTIVE CLIENTS'), [sources]);

  const clients = useMemo(() => {
    const map: Record<string, { name: string; files: DataSourceRecord[] }> = {};
    for (const s of clientSources) {
      const name = getClientName(s);
      if (!name) continue;
      if (!map[name]) map[name] = { name, files: [] };
      map[name].files.push(s);
    }
    return Object.values(map).sort((a, b) => b.files.length - a.files.length);
  }, [clientSources]);

  const typeCount = (files: DataSourceRecord[], type: string) => files.filter(f => f.data_type === type).length;

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2">
        <Users className="h-4 w-4 text-muted-foreground" />
        <h3 className="font-semibold">{clients.length} Active Clients</h3>
        <span className="text-xs text-muted-foreground">— {clientSources.length} files across all clients</span>
      </div>
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
        {clients.map(client => {
          const docs = typeCount(client.files, 'document');
          const media = typeCount(client.files, 'media');
          const datasets = typeCount(client.files, 'dataset');
          const exts = [...new Set(client.files.map(f => f.file_type).filter(Boolean))].slice(0, 5);
          return (
            <Card key={client.name} className="border border-border/50 hover:border-blue-300 dark:hover:border-blue-700 transition-colors">
              <CardHeader className="pb-2 pt-4 px-4">
                <div className="flex items-start justify-between">
                  <div>
                    <CardTitle className="text-sm font-semibold">{client.name}</CardTitle>
                    <p className="text-xs text-muted-foreground mt-0.5">{client.files.length} files</p>
                  </div>
                  <div className="w-8 h-8 rounded-full bg-blue-100 dark:bg-blue-950 flex items-center justify-center">
                    <span className="text-xs font-bold text-blue-600 dark:text-blue-400">
                      {client.name.charAt(0)}
                    </span>
                  </div>
                </div>
              </CardHeader>
              <CardContent className="px-4 pb-4 space-y-3">
                <div className="flex gap-2">
                  {docs > 0 && <span className="text-[10px] bg-blue-50 dark:bg-blue-950/50 text-blue-700 dark:text-blue-300 px-1.5 py-0.5 rounded">{docs} docs</span>}
                  {media > 0 && <span className="text-[10px] bg-purple-50 dark:bg-purple-950/50 text-purple-700 dark:text-purple-300 px-1.5 py-0.5 rounded">{media} media</span>}
                  {datasets > 0 && <span className="text-[10px] bg-green-50 dark:bg-green-950/50 text-green-700 dark:text-green-300 px-1.5 py-0.5 rounded">{datasets} datasets</span>}
                </div>
                {exts.length > 0 && (
                  <div className="flex gap-1 flex-wrap">
                    {exts.map(ext => (
                      <span key={ext} className="text-[10px] font-mono bg-muted px-1 py-0.5 rounded text-muted-foreground">.{ext}</span>
                    ))}
                  </div>
                )}
                <Button
                  variant="outline"
                  size="sm"
                  className="w-full h-7 text-xs"
                  asChild
                >
                  <Link to={`/organizations/${orgId}?tab=knowledge&view=datasources`}>
                    Browse Files
                  </Link>
                </Button>
              </CardContent>
            </Card>
          );
        })}
      </div>
    </div>
  );
}

// ─── Intelligence tab ─────────────────────────────────────────────────────────

function IntelligenceView({ sources }: { sources: DataSourceRecord[] }) {
  const sections = useMemo(() => {
    const map: Record<string, { files: DataSourceRecord[]; processed: number }> = {};
    for (const s of sources) {
      const section = getTopSection(s);
      if (!map[section]) map[section] = { files: [], processed: 0 };
      map[section].files.push(s);
      if (s.status === 'ready' || s.status === 'processing') map[section].processed++;
    }
    return Object.entries(map)
      .sort((a, b) => b[1].files.length - a[1].files.length)
      .map(([name, data]) => ({
        name,
        total: data.files.length,
        processed: data.processed,
        coverage: Math.round((data.processed / data.files.length) * 100),
        docs: data.files.filter(f => f.data_type === 'document').length,
        media: data.files.filter(f => f.data_type === 'media').length,
        datasets: data.files.filter(f => f.data_type === 'dataset').length,
      }));
  }, [sources]);

  const totalFiles = sources.length;
  const processedFiles = sources.filter(s => s.status === 'ready').length;
  const docFiles = sources.filter(s => s.data_type === 'document').length;
  const mediaFiles = sources.filter(s => s.data_type === 'media').length;

  return (
    <div className="space-y-6">
      {/* Summary stats */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
        {[
          { label: 'Total Files', value: totalFiles, icon: Database, color: 'text-blue-600' },
          { label: 'Catalogued', value: processedFiles, icon: CheckCircle2, color: 'text-green-600' },
          { label: 'Documents', value: docFiles, icon: FileText, color: 'text-indigo-600' },
          { label: 'Media Assets', value: mediaFiles, icon: Image, color: 'text-purple-600' },
        ].map(stat => (
          <Card key={stat.label} className="border border-border/50">
            <CardContent className="pt-4 pb-4 px-4">
              <div className="flex items-center gap-2 mb-1">
                <stat.icon className={`h-4 w-4 ${stat.color}`} />
                <span className="text-xs text-muted-foreground">{stat.label}</span>
              </div>
              <p className="text-2xl font-bold">{stat.value.toLocaleString()}</p>
            </CardContent>
          </Card>
        ))}
      </div>

      {/* Coverage by section */}
      <div>
        <h3 className="text-sm font-semibold mb-3 flex items-center gap-2">
          <BarChart3 className="h-4 w-4 text-muted-foreground" />
          Knowledge Coverage by Section
        </h3>
        <div className="space-y-2">
          {sections.map(section => (
            <div key={section.name} className={`border rounded-lg px-4 py-3 ${SECTION_BG[section.name] || 'bg-muted/20 border-border'}`}>
              <div className="flex items-center justify-between mb-1.5">
                <div className="flex items-center gap-2">
                  <span className={`text-sm font-semibold ${SECTION_COLORS[section.name] || ''}`}>{section.name}</span>
                  <span className="text-xs text-muted-foreground">{section.total} files</span>
                </div>
                <div className="flex items-center gap-2 text-xs text-muted-foreground">
                  {section.docs > 0 && <span>{section.docs} docs</span>}
                  {section.media > 0 && <span>{section.media} media</span>}
                  {section.datasets > 0 && <span>{section.datasets} datasets</span>}
                  <span className="font-medium text-foreground">{section.coverage}% catalogued</span>
                </div>
              </div>
              <div className="h-1.5 bg-black/10 dark:bg-white/10 rounded-full overflow-hidden">
                <div
                  className="h-full rounded-full bg-current transition-all"
                  style={{ width: `${section.coverage}%`, color: section.name === 'ACTIVE CLIENTS' ? '#3b82f6' : section.name === 'PROPOSALS' ? '#f59e0b' : section.name === 'SALES' ? '#22c55e' : section.name === 'SOCIAL MEDIA' ? '#ec4899' : section.name === 'RESOURCES' ? '#a855f7' : section.name === 'PROJECTS' ? '#f97316' : '#64748b' }}
                />
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* What this means */}
      <Card className="border border-border/50 bg-muted/20">
        <CardHeader className="pb-2">
          <CardTitle className="text-sm flex items-center gap-2">
            <Zap className="h-4 w-4 text-amber-500" />
            What this means for your AI agents
          </CardTitle>
        </CardHeader>
        <CardContent className="text-sm text-muted-foreground space-y-2">
          <p>These {totalFiles.toLocaleString()} files are catalogued in Sirak Studios' knowledge graph. Agents like Nora and Astra can reference them when working on tasks for this organization.</p>
          <p>To deepen agent awareness, run a <span className="text-foreground font-medium">workflow</span> on any file from the Library tab — this extracts structured intelligence (summaries, entities, action items) that Nora can reason over directly.</p>
          <p>Priority areas to process: <span className="text-foreground font-medium">PROPOSALS</span> (for drafting new ones), <span className="text-foreground font-medium">SALES</span> (for CRM enrichment), and <span className="text-foreground font-medium">ACTIVE CLIENTS</span> (for project context).</p>
        </CardContent>
      </Card>
    </div>
  );
}

// ─── Upload tab ───────────────────────────────────────────────────────────────

function UploadView({ orgId }: { orgId: string }) {
  const queryClient = useQueryClient();
  const [text, setText] = useState('');
  const [title, setTitle] = useState('');
  const [isDragging, setIsDragging] = useState(false);

  const createMutation = useMutation({
    mutationFn: (data: any) => dataSourcesApi.create(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['dataSources', orgId] });
      setText('');
      setTitle('');
    },
  });

  const handleDropboxSync = () => {
    window.open('https://www.dropbox.com/scl/fo/ry27oara1fzt0vyn6310u/AI78hc6CdzKRo41lplPx41o?rlkey=zw3ub5rrdo24zked384qdglb0', '_blank');
  };

  return (
    <div className="space-y-6 max-w-2xl">
      {/* Dropbox sync */}
      <Card className="border border-blue-200 dark:border-blue-800 bg-blue-50/50 dark:bg-blue-950/20">
        <CardContent className="pt-4 pb-4">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-lg bg-blue-100 dark:bg-blue-900 flex items-center justify-center shrink-0">
              <RefreshCw className="h-5 w-5 text-blue-600 dark:text-blue-400" />
            </div>
            <div className="flex-1">
              <p className="text-sm font-medium">Dropbox Integration</p>
              <p className="text-xs text-muted-foreground mt-0.5">
                1,627 files synced from Sirak Studios team Dropbox. Last sync: today.
              </p>
            </div>
            <Button variant="outline" size="sm" className="shrink-0" onClick={handleDropboxSync}>
              Open Dropbox
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* File upload */}
      <div>
        <h3 className="text-sm font-semibold mb-2 flex items-center gap-2">
          <Upload className="h-4 w-4 text-muted-foreground" />
          Upload a File
        </h3>
        <div
          className={`border-2 border-dashed rounded-xl p-10 text-center transition-colors cursor-pointer
            ${isDragging ? 'border-blue-400 bg-blue-50 dark:bg-blue-950/20' : 'border-border hover:border-muted-foreground/40'}`}
          onDragOver={e => { e.preventDefault(); setIsDragging(true); }}
          onDragLeave={() => setIsDragging(false)}
          onDrop={e => { e.preventDefault(); setIsDragging(false); /* TODO: handle file */ }}
          onClick={() => document.getElementById('file-upload')?.click()}
        >
          <Upload className="h-8 w-8 mx-auto mb-2 text-muted-foreground opacity-50" />
          <p className="text-sm font-medium">Drop files here or click to browse</p>
          <p className="text-xs text-muted-foreground mt-1">PDF, DOCX, XLSX, MP3, images — up to 50MB</p>
          <input id="file-upload" type="file" className="hidden" multiple />
        </div>
      </div>

      {/* Paste text */}
      <div>
        <h3 className="text-sm font-semibold mb-2 flex items-center gap-2">
          <FileText className="h-4 w-4 text-muted-foreground" />
          Add Text / Note
        </h3>
        <div className="space-y-2">
          <Input
            value={title}
            onChange={e => setTitle(e.target.value)}
            placeholder="Title (e.g. Client brief for Bvlgari Q2)"
            className="text-sm"
          />
          <textarea
            value={text}
            onChange={e => setText(e.target.value)}
            placeholder="Paste a transcript, brief, note, or any text content..."
            className="w-full min-h-[160px] rounded-md border bg-background px-3 py-2 text-sm resize-y focus:outline-none focus:ring-2 focus:ring-ring"
          />
          <Button
            size="sm"
            disabled={!title || !text || createMutation.isPending}
            onClick={() => createMutation.mutate({
              organization_id: orgId,
              title,
              data_type: 'document',
              source_type: 'text',
              content: text,
            })}
          >
            {createMutation.isPending ? 'Adding...' : 'Add to Knowledge Base'}
          </Button>
        </div>
      </div>

      {/* Integration sources */}
      <div>
        <h3 className="text-sm font-semibold mb-2 flex items-center gap-2">
          <Globe className="h-4 w-4 text-muted-foreground" />
          Connect Integrations
        </h3>
        <div className="grid grid-cols-2 gap-2">
          {[
            { name: 'QuickBooks', desc: 'Financials & invoices', status: 'connected' },
            { name: 'Airtable',   desc: 'CRM & project data',   status: 'connected' },
            { name: 'Slack',      desc: 'Team conversations',    status: 'coming_soon' },
            { name: 'Gmail',      desc: 'Email threads',         status: 'coming_soon' },
          ].map(integration => (
            <div key={integration.name} className="border rounded-lg px-3 py-2.5 flex items-center gap-2">
              <div className="flex-1">
                <p className="text-sm font-medium">{integration.name}</p>
                <p className="text-xs text-muted-foreground">{integration.desc}</p>
              </div>
              {integration.status === 'connected' ? (
                <Badge variant="outline" className="text-[10px] text-green-600 border-green-300">Live</Badge>
              ) : (
                <Badge variant="outline" className="text-[10px] text-muted-foreground">Soon</Badge>
              )}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

// ─── Status pill ──────────────────────────────────────────────────────────────

function StatusPill({ status }: { status: string }) {
  const styles: Record<string, string> = {
    ready:      'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400',
    pending:    'bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400',
    processing: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400',
    error:      'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
  };
  return (
    <span className={`inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium ${styles[status] ?? 'bg-muted text-muted-foreground'}`}>
      {status}
    </span>
  );
}

// ─── Main page ────────────────────────────────────────────────────────────────

export function DataSourcesPage() {
  const { orgId } = useParams<{ orgId: string }>();
  const navigate = useNavigate();

  const { data: sources = [], isLoading } = useQuery({
    queryKey: ['dataSources', orgId],
    queryFn: () => dataSourcesApi.listByOrganization(orgId!),
    staleTime: 60_000,
    enabled: !!orgId,
  });

  const stats = useMemo(() => ({
    total: sources.length,
    document: sources.filter(s => s.data_type === 'document').length,
    media: sources.filter(s => s.data_type === 'media').length,
    dataset: sources.filter(s => s.data_type === 'dataset').length,
    conversation: sources.filter(s => s.data_type === 'conversation').length,
  }), [sources]);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64 text-muted-foreground">
        <RefreshCw className="h-5 w-5 animate-spin mr-2" />
        Loading data sources...
      </div>
    );
  }

  return (
    <div className="p-6 space-y-5 max-w-[1400px] mx-auto">
      {/* Header */}
      <div className="space-y-3">
        <Button variant="ghost" size="sm" className="-ml-2 gap-1.5" onClick={() => navigate(`/organizations/${orgId}`)}>
          <ArrowLeft className="h-4 w-4" />
          Back to Organization
        </Button>
        <div className="flex items-start justify-between">
          <div>
            <h1 className="text-2xl font-bold tracking-tight">Data Sources</h1>
            <p className="text-sm text-muted-foreground mt-0.5">
              Sirak Studios knowledge base — {stats.total.toLocaleString()} files indexed from Dropbox and integrations
            </p>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" className="gap-1.5" onClick={() => navigate(`/organizations/${orgId}/data-sources/upload`)}>
              <Plus className="h-3.5 w-3.5" />
              Add Source
            </Button>
          </div>
        </div>

        {/* Stat bar */}
        <div className="flex items-center gap-4 text-sm">
          {[
            { label: 'Documents', count: stats.document, color: 'text-blue-600 dark:text-blue-400' },
            { label: 'Media',     count: stats.media,    color: 'text-purple-600 dark:text-purple-400' },
            { label: 'Datasets',  count: stats.dataset,  color: 'text-green-600 dark:text-green-400' },
            { label: 'Conversations', count: stats.conversation, color: 'text-amber-600 dark:text-amber-400' },
          ].map((s, i) => (
            <span key={s.label} className="flex items-center gap-1.5 text-xs">
              {i > 0 && <span className="text-border">·</span>}
              <span className={`font-semibold ${s.color}`}>{s.count.toLocaleString()}</span>
              <span className="text-muted-foreground">{s.label}</span>
            </span>
          ))}
        </div>
      </div>

      {/* Tabs */}
      <Tabs defaultValue="library">
        <TabsList className="tab-grid-4 mb-4">
          <TabsTrigger value="library" className="gap-1.5">
            <BookOpen className="h-3.5 w-3.5" />
            Library
          </TabsTrigger>
          <TabsTrigger value="clients" className="gap-1.5">
            <Users className="h-3.5 w-3.5" />
            Clients
          </TabsTrigger>
          <TabsTrigger value="intelligence" className="gap-1.5">
            <TrendingUp className="h-3.5 w-3.5" />
            Intelligence
          </TabsTrigger>
          <TabsTrigger value="upload" className="gap-1.5">
            <Upload className="h-3.5 w-3.5" />
            Upload
          </TabsTrigger>
        </TabsList>

        <TabsContent value="library">
          <LibraryView sources={sources} orgId={orgId!} />
        </TabsContent>
        <TabsContent value="clients">
          <ClientsView sources={sources} orgId={orgId!} />
        </TabsContent>
        <TabsContent value="intelligence">
          <IntelligenceView sources={sources} />
        </TabsContent>
        <TabsContent value="upload">
          <UploadView orgId={orgId!} />
        </TabsContent>
      </Tabs>
    </div>
  );
}
