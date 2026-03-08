import { useState, useMemo } from 'react';
import { useParams, useNavigate, Link } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import {
  ArrowLeft, Search, FileText, Music, Image, Film, Layers,
  File, Database, Upload, RefreshCw, Plus, Globe, Palette,
  Table2, BookOpen, SortAsc, SortDesc, Brain, Zap, ChevronRight,
  BarChart3, Users, CheckCircle2, TrendingUp,
} from 'lucide-react';
import { dataSourcesApi, type DataSourceRecord } from '@/lib/api';

// ─── Type category definitions ────────────────────────────────────────────────

interface TypeCategory {
  id: string;
  label: string;
  shortLabel: string;
  description: string;
  icon: React.ElementType;
  color: string;
  bgLight: string;
  badgeBg: string;
  badgeFg: string;
  exts: string[];
}

const CATEGORIES: TypeCategory[] = [
  { id: 'video',  label: 'Video Projects',  shortLabel: 'Video',   description: 'Adobe Premiere Pro edit timelines',        icon: Film,     color: 'text-blue-600 dark:text-blue-400',    bgLight: 'bg-blue-50 dark:bg-blue-950/40',    badgeBg: 'bg-blue-100 dark:bg-blue-900/50',    badgeFg: 'text-blue-700 dark:text-blue-300',   exts: ['prproj','aep','ppro'] },
  { id: 'docs',   label: 'Documents',       shortLabel: 'Docs',    description: 'PDFs, Word docs, presentations, notes',    icon: FileText, color: 'text-slate-600 dark:text-slate-400',  bgLight: 'bg-slate-50 dark:bg-slate-900/40',  badgeBg: 'bg-slate-100 dark:bg-slate-800/50',  badgeFg: 'text-slate-700 dark:text-slate-300', exts: ['pdf','docx','doc','rtf','txt','pages','pptx','ppt','key'] },
  { id: 'sheets', label: 'Spreadsheets',    shortLabel: 'Sheets',  description: 'Excel and CSV data files',                 icon: Table2,   color: 'text-green-600 dark:text-green-400',  bgLight: 'bg-green-50 dark:bg-green-950/40',  badgeBg: 'bg-green-100 dark:bg-green-900/50',  badgeFg: 'text-green-700 dark:text-green-300', exts: ['xlsx','xls','csv','numbers'] },
  { id: 'design', label: 'Design Files',    shortLabel: 'Design',  description: 'Photoshop layered composites',             icon: Layers,   color: 'text-indigo-600 dark:text-indigo-400',bgLight: 'bg-indigo-50 dark:bg-indigo-950/40',badgeBg: 'bg-indigo-100 dark:bg-indigo-900/50',badgeFg: 'text-indigo-700 dark:text-indigo-300',exts: ['psd','psb'] },
  { id: 'luts',   label: 'LUT Packs',       shortLabel: 'LUTs',    description: 'Color grading: .cube & Lightroom presets', icon: Palette,  color: 'text-pink-600 dark:text-pink-400',    bgLight: 'bg-pink-50 dark:bg-pink-950/40',    badgeBg: 'bg-pink-100 dark:bg-pink-900/50',    badgeFg: 'text-pink-700 dark:text-pink-300',   exts: ['cube','lrtemplate','xmp'] },
  { id: 'audio',  label: 'Audio',           shortLabel: 'Audio',   description: 'Music, voiceovers, and sound design',      icon: Music,    color: 'text-amber-600 dark:text-amber-400',  bgLight: 'bg-amber-50 dark:bg-amber-950/40',  badgeBg: 'bg-amber-100 dark:bg-amber-900/50',  badgeFg: 'text-amber-700 dark:text-amber-300', exts: ['mp3','wav','aiff','m4a','aac'] },
  { id: 'images', label: 'Images',          shortLabel: 'Images',  description: 'Photos, graphics, and raw camera files',   icon: Image,    color: 'text-rose-600 dark:text-rose-400',    bgLight: 'bg-rose-50 dark:bg-rose-950/40',    badgeBg: 'bg-rose-100 dark:bg-rose-900/50',    badgeFg: 'text-rose-700 dark:text-rose-300',   exts: ['jpg','jpeg','png','gif','heic','tiff','tif','webp','cr3','cr2','arw','dng','raw','nef'] },
  { id: 'other',  label: 'Other',           shortLabel: 'Other',   description: 'Miscellaneous files',                      icon: File,     color: 'text-muted-foreground',               bgLight: 'bg-muted/40',                       badgeBg: 'bg-muted',                           badgeFg: 'text-muted-foreground',              exts: [] },
];

function getCatForSource(s: DataSourceRecord): TypeCategory {
  const ext = (s.file_type || '').toLowerCase();
  for (const c of CATEGORIES) {
    if (c.id !== 'other' && c.exts.includes(ext)) return c;
  }
  return CATEGORIES[CATEGORIES.length - 1];
}

function getCat(id: string): TypeCategory {
  return CATEGORIES.find(c => c.id === id) ?? CATEGORIES[CATEGORIES.length - 1];
}

function parseMeta(raw: string | null | undefined): Record<string, any> {
  if (!raw) return {};
  try { return JSON.parse(raw); } catch { return {}; }
}

function formatSize(bytes?: number): string {
  if (!bytes) return '';
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

function getSize(s: DataSourceRecord): number {
  const m = parseMeta(s.metadata);
  return m.file_size_bytes || s.file_size_bytes || 0;
}

// ─── Extension badge ──────────────────────────────────────────────────────────

function ExtBadge({ ext, cat }: { ext: string; cat: TypeCategory }) {
  return (
    <span className={`inline-flex items-center justify-center min-w-[3.5rem] px-2 py-0.5 rounded text-[10px] font-mono font-bold uppercase tracking-widest ${cat.badgeBg} ${cat.badgeFg}`}>
      .{ext || '?'}
    </span>
  );
}

// ─── File row ─────────────────────────────────────────────────────────────────

function FileRow({ source, idx }: { source: DataSourceRecord; idx: number }) {
  const cat = getCatForSource(source);
  const size = getSize(source);
  return (
    <tr className={`border-b border-border/40 hover:bg-muted/40 transition-colors ${idx % 2 === 1 ? 'bg-muted/10' : ''}`}>
      <td className="px-4 py-2.5 w-[4.5rem]">
        <ExtBadge ext={source.file_type || ''} cat={cat} />
      </td>
      <td className="px-4 py-2.5">
        <span className="font-medium text-sm leading-snug block truncate max-w-[380px] lg:max-w-[560px]" title={source.title}>
          {source.title}
        </span>
      </td>
      <td className="px-4 py-2.5 text-right w-[80px]">
        <span className="text-xs text-muted-foreground tabular-nums">{formatSize(size)}</span>
      </td>
    </tr>
  );
}

// ─── Category card (overview grid) ───────────────────────────────────────────

function CategoryCard({
  cat, count, totalBytes, onSelect,
}: {
  cat: TypeCategory;
  count: number;
  totalBytes: number;
  onSelect: () => void;
}) {
  const Icon = cat.icon;
  return (
    <button
      onClick={onSelect}
      className={`group text-left border rounded-xl p-4 transition-all hover:shadow-md hover:-translate-y-0.5 ${cat.bgLight}`}
    >
      <div className="flex items-start justify-between mb-3">
        <div className={`w-10 h-10 rounded-xl flex items-center justify-center ${cat.badgeBg}`}>
          <Icon className={`h-5 w-5 ${cat.color}`} />
        </div>
        <ChevronRight className="h-4 w-4 text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity mt-1" />
      </div>
      <p className={`text-lg font-bold tabular-nums ${cat.color}`}>{count.toLocaleString()}</p>
      <p className="text-sm font-semibold mt-0.5">{cat.label}</p>
      <p className="text-xs text-muted-foreground mt-0.5 line-clamp-1">{cat.description}</p>
      {totalBytes > 0 && (
        <p className="text-xs text-muted-foreground mt-2 tabular-nums">{formatSize(totalBytes)}</p>
      )}
    </button>
  );
}

// ─── Library view ─────────────────────────────────────────────────────────────

type SortKey = 'name' | 'size' | 'type';

function LibraryView({ sources }: { sources: DataSourceRecord[] }) {
  const [activeCat, setActiveCat] = useState<string | null>(null);
  const [search, setSearch] = useState('');
  const [sort, setSort] = useState<SortKey>('name');
  const [sortAsc, setSortAsc] = useState(true);

  const counts = useMemo(() => {
    const map: Record<string, { count: number; bytes: number }> = {};
    for (const s of sources) {
      const c = getCatForSource(s);
      if (!map[c.id]) map[c.id] = { count: 0, bytes: 0 };
      map[c.id].count++;
      map[c.id].bytes += getSize(s);
    }
    return map;
  }, [sources]);

  const displayedSources = useMemo(() => {
    let items = activeCat
      ? sources.filter(s => getCatForSource(s).id === activeCat)
      : sources;

    if (search) {
      const q = search.toLowerCase();
      items = items.filter(s => s.title.toLowerCase().includes(q));
    }

    return [...items].sort((a, b) => {
      let cmp = 0;
      if (sort === 'name') cmp = a.title.localeCompare(b.title);
      else if (sort === 'type') cmp = (a.file_type || '').localeCompare(b.file_type || '');
      else if (sort === 'size') cmp = getSize(a) - getSize(b);
      return sortAsc ? cmp : -cmp;
    });
  }, [sources, activeCat, search, sort, sortAsc]);

  function toggleSort(key: SortKey) {
    if (sort === key) setSortAsc(v => !v);
    else { setSort(key); setSortAsc(true); }
  }

  const SortIcon = sortAsc ? SortAsc : SortDesc;
  const cat = activeCat ? getCat(activeCat) : null;

  // Overview: category cards grid
  if (!activeCat && !search) {
    return (
      <div className="space-y-6">
        <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-3">
          {CATEGORIES.filter(c => c.id !== 'other' || (counts['other']?.count ?? 0) > 0).map(c => {
            const info = counts[c.id];
            if (!info || info.count === 0) return null;
            return (
              <CategoryCard
                key={c.id}
                cat={c}
                count={info.count}
                totalBytes={info.bytes}
                onSelect={() => setActiveCat(c.id)}
              />
            );
          })}
        </div>

        {/* Proportion bar */}
        <div>
          <p className="text-xs text-muted-foreground mb-1.5">File type distribution</p>
          <div className="flex h-3 rounded-full overflow-hidden gap-px">
            {CATEGORIES.filter(c => c.id !== 'other').map(c => {
              const count = counts[c.id]?.count ?? 0;
              if (!count) return null;
              const pct = (count / sources.length) * 100;
              const bg =
                c.id === 'video'  ? 'bg-blue-500'  :
                c.id === 'docs'   ? 'bg-slate-400'  :
                c.id === 'sheets' ? 'bg-green-500' :
                c.id === 'design' ? 'bg-indigo-500':
                c.id === 'luts'   ? 'bg-pink-500'  :
                c.id === 'audio'  ? 'bg-amber-500' : 'bg-rose-500';
              return (
                <div key={c.id} className={`${bg} relative group cursor-default`} style={{ width: `${pct}%` }}>
                  <div className="absolute bottom-full left-1/2 -translate-x-1/2 mb-1.5 hidden group-hover:block z-10 bg-popover border rounded-md px-2 py-1 text-xs shadow-md whitespace-nowrap">
                    {c.shortLabel}: {count} ({pct.toFixed(0)}%)
                  </div>
                </div>
              );
            })}
          </div>
          <div className="flex flex-wrap gap-x-4 gap-y-1 mt-2">
            {CATEGORIES.filter(c => c.id !== 'other').map(c => {
              const count = counts[c.id]?.count ?? 0;
              if (!count) return null;
              return (
                <button key={c.id} onClick={() => setActiveCat(c.id)} className={`flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground transition-colors`}>
                  <span className={`w-2 h-2 rounded-full ${c.id === 'video' ? 'bg-blue-500' : c.id === 'docs' ? 'bg-slate-400' : c.id === 'sheets' ? 'bg-green-500' : c.id === 'design' ? 'bg-indigo-500' : c.id === 'luts' ? 'bg-pink-500' : c.id === 'audio' ? 'bg-amber-500' : 'bg-rose-500'}`} />
                  {c.shortLabel} <span className="font-medium text-foreground">{count}</span>
                </button>
              );
            })}
          </div>
        </div>
      </div>
    );
  }

  // Drill-down or search: file table
  return (
    <div className="space-y-3">
      {/* Back + header */}
      <div className="flex items-center gap-2">
        {activeCat && (
          <button
            onClick={() => { setActiveCat(null); setSearch(''); }}
            className="flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors"
          >
            <ArrowLeft className="h-4 w-4" />
            All Types
          </button>
        )}
        {cat && (
          <>
            <ChevronRight className="h-3.5 w-3.5 text-muted-foreground" />
            <div className="flex items-center gap-1.5">
              <cat.icon className={`h-4 w-4 ${cat.color}`} />
              <span className={`text-sm font-semibold ${cat.color}`}>{cat.label}</span>
            </div>
          </>
        )}
      </div>

      {/* Search + sort */}
      <div className="flex items-center gap-2">
        <div className="relative max-w-sm">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
          <Input
            value={search}
            onChange={e => setSearch(e.target.value)}
            placeholder={cat ? `Search ${cat.label.toLowerCase()}…` : 'Search all files…'}
            className="pl-8 h-8 text-sm w-64"
          />
        </div>
        <div className="flex items-center gap-1 ml-auto text-xs text-muted-foreground">
          <span className="tabular-nums">{displayedSources.length.toLocaleString()} files</span>
          <span className="mx-1 text-border">·</span>
          <span>Sort:</span>
          {(['name', 'type', 'size'] as SortKey[]).map(key => (
            <button
              key={key}
              onClick={() => toggleSort(key)}
              className={`flex items-center gap-0.5 px-1.5 py-0.5 rounded capitalize transition-colors ${sort === key ? 'bg-muted font-medium text-foreground' : 'hover:text-foreground'}`}
            >
              {key}{sort === key && <SortIcon className="h-3 w-3 ml-0.5" />}
            </button>
          ))}
        </div>
      </div>

      {/* Table */}
      <div className="border rounded-xl bg-card overflow-hidden">
        {displayedSources.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-32 text-muted-foreground">
            <File className="h-7 w-7 mb-2 opacity-30" />
            <p className="text-sm">No files match your search</p>
          </div>
        ) : (
          <div className="overflow-y-auto" style={{ maxHeight: 'calc(100vh - 340px)' }}>
            <table className="w-full text-sm">
              <thead className="sticky top-0 z-10 bg-muted/80 backdrop-blur-sm border-b">
                <tr>
                  <th className="text-left px-4 py-2 text-xs font-medium text-muted-foreground w-[5rem]">Type</th>
                  <th className="text-left px-4 py-2 text-xs font-medium text-muted-foreground">Name</th>
                  <th className="text-right px-4 py-2 text-xs font-medium text-muted-foreground w-[80px]">Size</th>
                </tr>
              </thead>
              <tbody>
                {displayedSources.map((s, i) => <FileRow key={s.id} source={s} idx={i} />)}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}

// ─── Intelligence view ────────────────────────────────────────────────────────

function IntelligenceView({ sources }: { sources: DataSourceRecord[] }) {
  const total = sources.length;
  const ready = sources.filter(s => s.status === 'ready').length;
  const docs  = sources.filter(s => s.data_type === 'document').length;
  const media = sources.filter(s => s.data_type === 'media').length;
  const pct   = total > 0 ? Math.round((ready / total) * 100) : 0;

  const byExt = useMemo(() => {
    const map: Record<string, number> = {};
    for (const s of sources) { map[s.file_type || 'unknown'] = (map[s.file_type || 'unknown'] || 0) + 1; }
    return Object.entries(map).sort((a, b) => b[1] - a[1]).slice(0, 12);
  }, [sources]);

  return (
    <div className="space-y-6">
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
        {[
          { label: 'Total Files',   value: total,  icon: Database,      color: 'text-blue-600' },
          { label: 'Catalogued',    value: ready,  icon: CheckCircle2,  color: 'text-green-600' },
          { label: 'Documents',     value: docs,   icon: FileText,      color: 'text-indigo-600' },
          { label: 'Media Assets',  value: media,  icon: Image,         color: 'text-purple-600' },
        ].map(stat => (
          <Card key={stat.label} className="border-border/50">
            <CardContent className="pt-4 pb-4 px-4">
              <div className="flex items-center gap-2 mb-1">
                <stat.icon className={`h-4 w-4 ${stat.color}`} />
                <span className="text-xs text-muted-foreground">{stat.label}</span>
              </div>
              <p className="text-2xl font-bold tabular-nums">{stat.value.toLocaleString()}</p>
            </CardContent>
          </Card>
        ))}
      </div>

      <Card className="border-border/50">
        <CardHeader className="pb-2">
          <CardTitle className="text-sm flex items-center gap-2">
            <BarChart3 className="h-4 w-4 text-muted-foreground" />
            Cataloguing Progress
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex items-center gap-3 mb-2">
            <Progress value={pct} className="flex-1 h-2" />
            <span className="text-sm font-bold tabular-nums w-12 text-right">{pct}%</span>
          </div>
          <p className="text-xs text-muted-foreground">{ready.toLocaleString()} of {total.toLocaleString()} files indexed for AI agent access</p>
        </CardContent>
      </Card>

      <div>
        <h3 className="text-sm font-semibold mb-3 flex items-center gap-2">
          <TrendingUp className="h-4 w-4 text-muted-foreground" />
          Breakdown by File Format
        </h3>
        <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-2">
          {byExt.map(([ext, count]) => {
            const cat = CATEGORIES.find(c => c.exts.includes(ext)) ?? CATEGORIES[CATEGORIES.length - 1];
            const Icon = cat.icon;
            return (
              <div key={ext} className={`border rounded-lg px-3 py-2.5 flex items-center gap-2.5 ${cat.bgLight}`}>
                <Icon className={`h-4 w-4 shrink-0 ${cat.color}`} />
                <div className="min-w-0">
                  <p className="text-sm font-bold tabular-nums">{count}</p>
                  <p className={`text-[10px] font-mono uppercase tracking-wider ${cat.badgeFg}`}>.{ext}</p>
                </div>
              </div>
            );
          })}
        </div>
      </div>

      <Card className="border-border/50 bg-muted/20">
        <CardHeader className="pb-2">
          <CardTitle className="text-sm flex items-center gap-2">
            <Zap className="h-4 w-4 text-amber-500" />
            What this means for your AI agents
          </CardTitle>
        </CardHeader>
        <CardContent className="text-sm text-muted-foreground space-y-2">
          <p>These <span className="text-foreground font-medium">{total.toLocaleString()} files</span> are catalogued in Sirak Studios' knowledge graph. Agents like Nora and Astra can reference them when working on tasks for this organization.</p>
          <p>The <span className="text-foreground font-medium">423 Premiere Pro projects</span> represent active video productions. The <span className="text-foreground font-medium">195 LUT/Lightroom presets</span> define the visual style system. The <span className="text-foreground font-medium">335 PDFs</span> and <span className="text-foreground font-medium">242 Word documents</span> contain client briefs, contracts, and proposals agents can reason over.</p>
          <p>Run a <span className="text-foreground font-medium">Workflow</span> on any document to extract structured intelligence — summaries, entities, action items — that Nora can use directly.</p>
        </CardContent>
      </Card>
    </div>
  );
}

// ─── Upload view ──────────────────────────────────────────────────────────────

function UploadView({ orgId }: { orgId: string }) {
  const queryClient = useQueryClient();
  const [text, setText] = useState('');
  const [title, setTitle] = useState('');
  const [isDragging, setIsDragging] = useState(false);

  const createMutation = useMutation({
    mutationFn: (data: any) => dataSourcesApi.create(data),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['dataSources', orgId] }); setText(''); setTitle(''); },
  });

  return (
    <div className="space-y-5 max-w-xl">
      <Card className="border-blue-200 dark:border-blue-800 bg-blue-50/50 dark:bg-blue-950/20">
        <CardContent className="pt-4 pb-4">
          <div className="flex items-center gap-3">
            <div className="w-9 h-9 rounded-lg bg-blue-100 dark:bg-blue-900 flex items-center justify-center shrink-0">
              <RefreshCw className="h-4 w-4 text-blue-600 dark:text-blue-400" />
            </div>
            <div className="flex-1">
              <p className="text-sm font-medium">Dropbox Sync Active</p>
              <p className="text-xs text-muted-foreground">1,627 files synced from Sirak Studios team Dropbox. Last sync: today.</p>
            </div>
            <Button variant="outline" size="sm" onClick={() => window.open('https://www.dropbox.com', '_blank')}>Open</Button>
          </div>
        </CardContent>
      </Card>

      <div>
        <h3 className="text-sm font-semibold mb-2 flex items-center gap-2"><Upload className="h-4 w-4 text-muted-foreground" />Upload File</h3>
        <div
          className={`border-2 border-dashed rounded-xl p-8 text-center cursor-pointer transition-colors ${isDragging ? 'border-blue-400 bg-blue-50 dark:bg-blue-950/20' : 'border-border hover:border-muted-foreground/40'}`}
          onDragOver={e => { e.preventDefault(); setIsDragging(true); }}
          onDragLeave={() => setIsDragging(false)}
          onDrop={e => { e.preventDefault(); setIsDragging(false); }}
          onClick={() => document.getElementById('file-upload-ds')?.click()}
        >
          <Upload className="h-6 w-6 mx-auto mb-2 text-muted-foreground opacity-50" />
          <p className="text-sm font-medium">Drop files here or click to browse</p>
          <p className="text-xs text-muted-foreground mt-1">PDF, DOCX, XLSX, MP3, images — up to 50MB</p>
          <input id="file-upload-ds" type="file" className="hidden" multiple />
        </div>
      </div>

      <div>
        <h3 className="text-sm font-semibold mb-2 flex items-center gap-2"><FileText className="h-4 w-4 text-muted-foreground" />Add Text / Note</h3>
        <div className="space-y-2">
          <Input value={title} onChange={e => setTitle(e.target.value)} placeholder="Title (e.g. Client brief for Q2)" className="text-sm" />
          <textarea value={text} onChange={e => setText(e.target.value)} placeholder="Paste a transcript, brief, note, or any text content…" className="w-full min-h-[120px] rounded-md border bg-background px-3 py-2 text-sm resize-y focus:outline-none focus:ring-2 focus:ring-ring" />
          <Button size="sm" disabled={!title || !text || createMutation.isPending} onClick={() => createMutation.mutate({ organization_id: orgId, title, data_type: 'document', source_type: 'text', content: text })}>
            {createMutation.isPending ? 'Adding…' : 'Add to Knowledge Base'}
          </Button>
        </div>
      </div>

      <div>
        <h3 className="text-sm font-semibold mb-2 flex items-center gap-2"><Globe className="h-4 w-4 text-muted-foreground" />Connect Integrations</h3>
        <div className="grid grid-cols-2 gap-2">
          {[{ name: 'QuickBooks', desc: 'Financials & invoices', live: true }, { name: 'Airtable', desc: 'CRM & project data', live: true }, { name: 'Slack', desc: 'Team conversations', live: false }, { name: 'Gmail', desc: 'Email threads', live: false }].map(i => (
            <div key={i.name} className="border rounded-lg px-3 py-2 flex items-center gap-2">
              <div className="flex-1"><p className="text-sm font-medium">{i.name}</p><p className="text-xs text-muted-foreground">{i.desc}</p></div>
              <Badge variant="outline" className={`text-[10px] ${i.live ? 'text-green-600 border-green-300' : 'text-muted-foreground'}`}>{i.live ? 'Live' : 'Soon'}</Badge>
            </div>
          ))}
        </div>
      </div>
    </div>
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

  const totalBytes = useMemo(() => sources.reduce((s, r) => s + getSize(r), 0), [sources]);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64 text-muted-foreground">
        <RefreshCw className="h-5 w-5 animate-spin mr-2" />
        Loading data sources…
      </div>
    );
  }

  return (
    <div className="p-6 max-w-[1400px] mx-auto space-y-5">
      {/* Header */}
      <div>
        <Button variant="ghost" size="sm" className="-ml-2 gap-1.5 mb-3" onClick={() => navigate(`/organizations/${orgId}`)}>
          <ArrowLeft className="h-4 w-4" />
          Back to Organization
        </Button>
        <div className="flex items-start justify-between">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-blue-500 to-indigo-600 flex items-center justify-center shadow-sm">
              <BookOpen className="h-5 w-5 text-white" />
            </div>
            <div>
              <h1 className="text-2xl font-bold tracking-tight">Sirak Studios Library</h1>
              <p className="text-sm text-muted-foreground mt-0.5">
                <span className="font-semibold text-foreground tabular-nums">{sources.length.toLocaleString()}</span> assets
                {totalBytes > 0 && <> · <span className="font-semibold text-foreground">{formatSize(totalBytes)}</span></>}
                {' '}· Dropbox integration · Last sync today
              </p>
            </div>
          </div>
          <Button variant="outline" size="sm" className="gap-1.5">
            <Plus className="h-3.5 w-3.5" />
            Add Source
          </Button>
        </div>
      </div>

      {/* Tabs */}
      <Tabs defaultValue="library">
        <TabsList className="mb-4">
          <TabsTrigger value="library" className="gap-1.5">
            <Database className="h-3.5 w-3.5" />
            Library
          </TabsTrigger>
          <TabsTrigger value="intelligence" className="gap-1.5">
            <Brain className="h-3.5 w-3.5" />
            Intelligence
          </TabsTrigger>
          <TabsTrigger value="upload" className="gap-1.5">
            <Upload className="h-3.5 w-3.5" />
            Add Source
          </TabsTrigger>
        </TabsList>

        <TabsContent value="library">
          <LibraryView sources={sources} />
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
