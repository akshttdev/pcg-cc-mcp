import { useState, useMemo } from 'react';
import { useParams } from 'react-router-dom';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useMutationWithToast } from '@/hooks/useMutationWithToast';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Search, List, FileText, Database,
  Upload, ChevronRight, Download, Trash2,
  Info, RefreshCw, Play, SortAsc, SortDesc,
  LayoutGrid,
} from 'lucide-react';
import { dataSourcesApi, type DataSourceRecord } from '@/lib/api';
import { dataSourceKeys } from '@/lib/query-keys';
import { formatDate } from '@/lib/formatters';
import { EmptyState } from '@/components/ui/empty-state';

import type { SortField } from './types';
import { buildFolderTree, getFolderContext, formatSize, hasLocalFile } from './types';
import { fileIcon } from './FileIcon';
import { FolderTreeNode, SECTION_ORDER } from './FolderTreeNode';
import { PreviewPanel } from './PreviewPanel';
import { UploadZone } from './UploadZone';
import { AddTextModal } from './AddTextModal';
import { RunWorkflowFromSourceDialog } from './RunWorkflowFromSourceDialog';

export function DataSourcesPage() {
  const { orgId, projectId } = useParams<{ orgId?: string; projectId?: string }>();
  const queryClient = useQueryClient();

  const [search, setSearch] = useState('');
  const [selectedPath, setSelectedPath] = useState('');
  const [typeFilter, setTypeFilter] = useState('all');
  const [sortField, setSortField] = useState<SortField>('created_at');
  const [sortDir, setSortDir] = useState<'asc' | 'desc'>('desc');
  const [selectedSource, setSelectedSource] = useState<DataSourceRecord | null>(null);
  const [showUploadZone, setShowUploadZone] = useState(false);
  const [showTextModal, setShowTextModal] = useState(false);
  const [viewMode, setViewMode] = useState<'list' | 'grid'>('list');
  const [runWorkflowSource, setRunWorkflowSource] = useState<DataSourceRecord | null>(null);

  const effectiveOrgId = orgId;
  const effectiveProjectId = projectId;

  const sourcesQuery = useQuery({
    queryKey: dataSourceKeys.list(effectiveOrgId, effectiveProjectId),
    queryFn: async () => {
      if (effectiveOrgId) return dataSourcesApi.listByOrganization(effectiveOrgId);
      if (effectiveProjectId) return dataSourcesApi.listByProject(effectiveProjectId);
      return dataSourcesApi.listAll();
    },
    staleTime: 30_000,
  });

  // Exclude personal data from org data sources — personal data lives in /intelligence
  const sources = useMemo(() => {
    const all = sourcesQuery.data || [];
    return all.filter((s: DataSourceRecord) => !s.folder?.startsWith('Personal/'));
  }, [sourcesQuery.data]);

  const deleteMutation = useMutationWithToast({
    mutationFn: (id: string) => dataSourcesApi.delete(id),
    successMessage: 'Deleted',
    errorMessage: 'Failed to delete',
    invalidateKeys: [dataSourceKeys.all],
    onSuccess: () => setSelectedSource(null),
  });

  const tree = useMemo(() => buildFolderTree(sources), [sources]);

  const filtered = useMemo(() => {
    let list = sources.filter((s: DataSourceRecord) => {
      const ctx = getFolderContext(s);
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
          <button
            className={`flex items-center gap-1.5 w-full text-left px-2 py-1.5 rounded-md text-sm transition-colors hover:bg-muted/60 ${!selectedPath ? 'bg-primary/10 text-primary font-medium' : ''}`}
            onClick={() => setSelectedPath('')}
          >
            <Database className="h-3.5 w-3.5 shrink-0" />
            <span>All Files</span>
            <span className="ml-auto text-[10px] text-muted-foreground">{sources.length}</span>
          </button>

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

export default DataSourcesPage;
