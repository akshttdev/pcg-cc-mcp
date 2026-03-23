import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  ArrowDown,
  ArrowUp,
  ArrowUpDown,
  ChevronDown,
  ChevronRight,
  Database,
  FileText,
  FolderOpen,
  MoreHorizontal,
  Pencil,
  Plus,
  Search,
  Trash2,
  Upload,
} from 'lucide-react';
import { useMemo, useState } from 'react';
import { Link } from 'react-router-dom';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { FormField } from '@/components/ui/form-field';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Textarea } from '@/components/ui/textarea';
import {
  DATA_TYPE_OPTIONS,
  type DataSourceRecord,
  dataSourcesApi,
  type UpdateDataSourceRequest,
} from '@/lib/api';
import { formatDate } from '@/lib/formatters';
import { dataSourceKeys } from '@/lib/query-keys';

// ── Add Data Source Dialog ────────────────────────────────────────────────────

function AddDataSourceDialog({
  orgId,
  projectEntries,
  open,
  onOpenChange,
  editingSource,
}: {
  orgId: string;
  projectEntries: { id: string; name: string }[];
  open: boolean;
  onOpenChange: (open: boolean) => void;
  editingSource?: DataSourceRecord | null;
}) {
  const queryClient = useQueryClient();
  const isEdit = !!editingSource;

  const [selectedProject, setSelectedProject] = useState<string>(editingSource?.project_id ?? '__none__');
  const [sourceType, setSourceType] = useState<string>(editingSource?.source_type ?? 'text');
  const [dataType, setDataType] = useState<string>(editingSource?.data_type ?? 'conversation');
  const [title, setTitle] = useState(editingSource?.title ?? '');
  const [description, setDescription] = useState(editingSource?.description ?? '');
  const [content, setContent] = useState(editingSource?.content ?? '');
  const [file, setFile] = useState<File | null>(null);
  const [folder, setFolder] = useState(editingSource?.folder ?? '');

  // Reset form when dialog opens/closes or editingSource changes
  const resetForm = () => {
    setSelectedProject(editingSource?.project_id ?? '__none__');
    setSourceType(editingSource?.source_type ?? 'text');
    setDataType(editingSource?.data_type ?? 'conversation');
    setTitle(editingSource?.title ?? '');
    setDescription(editingSource?.description ?? '');
    setContent(editingSource?.content ?? '');
    setFolder(editingSource?.folder ?? '');
    setFile(null);
  };

  const createMutation = useMutation({
    mutationFn: async () => {
      const projId = selectedProject !== '__none__' ? selectedProject : undefined;
      if (sourceType === 'file' && file) {
        const formData = new FormData();
        formData.append('file', file);
        formData.append('title', title.trim());
        formData.append('data_type', dataType);
        if (description.trim()) formData.append('description', description.trim());
        if (projId) formData.append('project_id', projId);
        formData.append('organization_id', orgId);
        if (folder.trim()) formData.append('folder', folder.trim());
        return dataSourcesApi.upload(formData);
      }
      return dataSourcesApi.create({
        organization_id: orgId,
        project_id: projId,
        title: title.trim(),
        description: description.trim() || undefined,
        data_type: dataType,
        source_type: sourceType,
        content: sourceType === 'text' && content.trim() ? content.trim() : undefined,
        folder: folder.trim() || undefined,
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: dataSourceKeys.list(orgId) });
      onOpenChange(false);
      resetForm();
    },
  });

  const updateMutation = useMutation({
    mutationFn: (data: UpdateDataSourceRequest) =>
      dataSourcesApi.update(editingSource!.id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: dataSourceKeys.list(orgId) });
      onOpenChange(false);
    },
  });

  const handleSubmit = () => {
    if (!title.trim()) return;
    if (isEdit) {
      updateMutation.mutate({
        title: title.trim(),
        description: description.trim() || undefined,
        data_type: dataType,
        source_type: sourceType,
        content: sourceType === 'text' && content.trim() ? content.trim() : undefined,
        folder: folder.trim() || undefined,
      });
    } else {
      createMutation.mutate();
    }
  };

  const isPending = createMutation.isPending || updateMutation.isPending;

  return (
    <Dialog open={open} onOpenChange={(v) => { onOpenChange(v); if (!v) resetForm(); }}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Database className="h-4 w-4" />
            {isEdit ? 'Edit Data Source' : 'Add Data Source'}
          </DialogTitle>
        </DialogHeader>
        <div className="space-y-4 py-2">
          {/* Source type radio */}
          <div className="space-y-2">
            <Label>Source</Label>
            <RadioGroup
              value={sourceType}
              onValueChange={setSourceType}
              className="flex gap-4"
            >
              <div className="flex items-center gap-1.5">
                <RadioGroupItem value="text" id="st-text" />
                <Label htmlFor="st-text" className="text-sm font-normal cursor-pointer">Text</Label>
              </div>
              <div className="flex items-center gap-1.5">
                <RadioGroupItem value="file" id="st-file" />
                <Label htmlFor="st-file" className="text-sm font-normal cursor-pointer">File Upload</Label>
              </div>
              <div className="flex items-center gap-1.5">
                <RadioGroupItem value="integration" id="st-integration" />
                <Label htmlFor="st-integration" className="text-sm font-normal cursor-pointer">Integration</Label>
              </div>
            </RadioGroup>
          </div>

          <FormField label="Title">
            <Input
              placeholder="e.g. Client kickoff call notes"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              autoFocus
            />
          </FormField>

          <FormField label="Data Type">
            <Select value={dataType} onValueChange={setDataType}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {DATA_TYPE_OPTIONS.map((opt) => (
                  <SelectItem key={opt.value} value={opt.value}>{opt.label}</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </FormField>

          <FormField label="Description (optional)">
            <Textarea
              placeholder="Brief description of this data source..."
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              rows={2}
            />
          </FormField>

          <FormField label="Folder (optional)" description='Use / to create subfolders. Leave blank to file under "Unfiled".'>
            <Input
              placeholder="e.g. Meetings or Meetings/Google Meet"
              value={folder}
              onChange={(e) => setFolder(e.target.value)}
            />
          </FormField>

          {/* Conditional input based on source type */}
          {sourceType === 'text' && (
            <FormField label="Content">
              <Textarea
                placeholder="Paste or type your content here..."
                value={content}
                onChange={(e) => setContent(e.target.value)}
                rows={6}
                className="font-mono text-xs"
              />
            </FormField>
          )}
          {sourceType === 'file' && !isEdit && (
            <FormField label="File">
              <div className="flex items-center gap-2">
                <Input
                  type="file"
                  onChange={(e) => setFile(e.target.files?.[0] ?? null)}
                  className="text-xs"
                />
                {file && (
                  <span className="text-xs text-muted-foreground shrink-0">
                    {(file.size / 1024).toFixed(0)} KB
                  </span>
                )}
              </div>
            </FormField>
          )}
          {sourceType === 'integration' && (
            <div className="rounded-md bg-muted/50 p-3 text-xs text-muted-foreground">
              Integration sources are populated automatically from connected services.
            </div>
          )}

          {projectEntries.length > 0 && (
            <div className="space-y-2">
              <Label>Project (optional)</Label>
              <Select value={selectedProject} onValueChange={setSelectedProject}>
                <SelectTrigger>
                  <SelectValue placeholder="No project (org-level)" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="__none__">No project (org-level)</SelectItem>
                  {projectEntries.map((p) => (
                    <SelectItem key={p.id} value={p.id}>{p.name}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          )}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>Cancel</Button>
          <Button onClick={handleSubmit} disabled={!title.trim() || isPending}>
            {isPending ? (isEdit ? 'Saving...' : 'Adding...') : (isEdit ? 'Save' : 'Add Source')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// ── Delete Confirmation Dialog ───────────────────────────────────────────────

function DeleteConfirmDialog({
  open,
  onOpenChange,
  sourceName,
  onConfirm,
  isPending,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  sourceName: string;
  onConfirm: () => void;
  isPending: boolean;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-sm">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2 text-destructive">
            <Trash2 className="h-4 w-4" />
            Delete Data Source
          </DialogTitle>
        </DialogHeader>
        <p className="text-sm text-muted-foreground">
          Are you sure you want to delete <strong>{sourceName}</strong>? This action cannot be undone.
        </p>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>Cancel</Button>
          <Button variant="destructive" onClick={onConfirm} disabled={isPending}>
            {isPending ? 'Deleting...' : 'Delete'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// ── Data Sources View ────────────────────────────────────────────────────────

type SortField = 'title' | 'data_type' | 'status' | 'created_at';
type SortDir = 'asc' | 'desc';

export function DataSourcesView({
  orgId,
  projectEntries,
}: {
  orgId: string;
  projectEntries: { id: string; name: string }[];
}) {
  const queryClient = useQueryClient();
  const [addOpen, setAddOpen] = useState(false);
  const [editingSource, setEditingSource] = useState<DataSourceRecord | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<DataSourceRecord | null>(null);
  const [sortField, setSortField] = useState<SortField>('created_at');
  const [sortDir, setSortDir] = useState<SortDir>('desc');
  const [selectedFolder, setSelectedFolder] = useState<string | null>(null);
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set(['Meetings', 'Documents']));
  const [search, setSearch] = useState('');

  const { data: sources = [], isLoading } = useQuery({
    queryKey: dataSourceKeys.list(orgId),
    queryFn: () => dataSourcesApi.listByOrganization(orgId),
    staleTime: 30_000,
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => dataSourcesApi.delete(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: dataSourceKeys.list(orgId) });
      setDeleteTarget(null);
    },
  });

  const toggleSort = (field: SortField) => {
    if (sortField === field) {
      setSortDir(d => d === 'asc' ? 'desc' : 'asc');
    } else {
      setSortField(field);
      setSortDir('asc');
    }
  };

  const SortIcon = ({ field }: { field: SortField }) => {
    if (sortField !== field) return <ArrowUpDown className="h-3 w-3 opacity-40" />;
    return sortDir === 'asc' ? <ArrowUp className="h-3 w-3" /> : <ArrowDown className="h-3 w-3" />;
  };

  // Build folder tree from sources
  const folderTree = useMemo(() => {
    const tree: Record<string, Record<string, number>> = {}; // root -> { sub: count }
    sources.forEach(s => {
      const f = s.folder || 'Unfiled';
      const parts = f.split('/');
      const root = parts[0];
      const sub = parts[1] || null;
      if (!tree[root]) tree[root] = {};
      if (sub) {
        tree[root][sub] = (tree[root][sub] || 0) + 1;
      } else {
        tree[root]['__self__'] = (tree[root]['__self__'] || 0) + 1;
      }
    });
    return tree;
  }, [sources]);

  const allFolderCount = sources.length;

  const sorted = useMemo(() => {
    let arr = [...sources];
    // Folder filter
    if (selectedFolder) {
      arr = arr.filter(s => {
        const f = s.folder || 'Unfiled';
        return f === selectedFolder || f.startsWith(selectedFolder + '/');
      });
    }
    // Search filter
    if (search.trim()) {
      const q = search.toLowerCase();
      arr = arr.filter(s => s.title.toLowerCase().includes(q) || (s.description || '').toLowerCase().includes(q));
    }
    arr.sort((a, b) => {
      let cmp = 0;
      switch (sortField) {
        case 'title': cmp = a.title.localeCompare(b.title); break;
        case 'data_type': cmp = a.data_type.localeCompare(b.data_type); break;
        case 'status': cmp = a.status.localeCompare(b.status); break;
        case 'created_at': cmp = a.created_at.localeCompare(b.created_at); break;
      }
      return sortDir === 'asc' ? cmp : -cmp;
    });
    return arr;
  }, [sources, sortField, sortDir, selectedFolder, search]);

  const dataTypeLabel = (dt: string) =>
    DATA_TYPE_OPTIONS.find(o => o.value === dt)?.label ?? dt;

  const formatFileSize = (bytes?: number) => {
    if (!bytes) return '';
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const statusBadge = (status: string) => {
    const variants: Record<string, string> = {
      ready: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400',
      pending: 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400',
      processing: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400',
      error: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
    };
    return (
      <span className={`inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium ${variants[status] ?? 'bg-muted text-muted-foreground'}`}>
        {status}
      </span>
    );
  };

  const toggleFolder = (f: string) => {
    setExpandedFolders(prev => {
      const next = new Set(prev);
      if (next.has(f)) next.delete(f); else next.add(f);
      return next;
    });
  };

  const folderCount = (f: string) =>
    sources.filter(s => {
      const sf = s.folder || 'Unfiled';
      return sf === f || sf.startsWith(f + '/');
    }).length;

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Database className="h-5 w-5 text-muted-foreground" />
          <h2 className="text-lg font-semibold">Data Sources</h2>
          <Badge variant="secondary">{sources.length}</Badge>
        </div>
        <Button size="sm" onClick={() => { setEditingSource(null); setAddOpen(true); }} className="gap-1">
          <Plus className="h-3.5 w-3.5" />
          Add Data Source
        </Button>
      </div>

      {isLoading ? (
        <div className="text-center py-12 text-muted-foreground text-sm">Loading data sources...</div>
      ) : sources.length === 0 ? (
        <div className="text-center py-12 text-muted-foreground">
          <Database className="h-8 w-8 mx-auto mb-2 opacity-40" />
          <p>No data sources yet</p>
          <Button variant="outline" size="sm" className="mt-4 gap-1" onClick={() => setAddOpen(true)}>
            <Plus className="h-3.5 w-3.5" />
            Add your first data source
          </Button>
        </div>
      ) : (
        <div className="flex gap-4">
          {/* Folder sidebar */}
          <div className="w-48 shrink-0">
            <div className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2 px-1">Folders</div>
            <nav className="space-y-0.5">
              {/* All */}
              <button
                onClick={() => setSelectedFolder(null)}
                className={`w-full flex items-center justify-between px-2 py-1.5 rounded text-sm hover:bg-muted/60 transition-colors ${!selectedFolder ? 'bg-muted font-medium' : ''}`}
              >
                <span className="flex items-center gap-1.5"><FolderOpen className="h-3.5 w-3.5 text-muted-foreground" />All</span>
                <span className="text-xs text-muted-foreground">{allFolderCount}</span>
              </button>
              {/* Root folders */}
              {Object.entries(folderTree).sort(([a],[b]) => a.localeCompare(b)).map(([root, subs]) => {
                const subKeys = Object.keys(subs).filter(k => k !== '__self__');
                const hasChildren = subKeys.length > 0;
                const isExpanded = expandedFolders.has(root);
                const cnt = folderCount(root);
                return (
                  <div key={root}>
                    <button
                      onClick={() => { if (hasChildren) toggleFolder(root); setSelectedFolder(root); }}
                      className={`w-full flex items-center justify-between px-2 py-1.5 rounded text-sm hover:bg-muted/60 transition-colors ${selectedFolder === root ? 'bg-muted font-medium' : ''}`}
                    >
                      <span className="flex items-center gap-1.5 min-w-0">
                        {hasChildren
                          ? (isExpanded ? <ChevronDown className="h-3 w-3 shrink-0 text-muted-foreground" /> : <ChevronRight className="h-3 w-3 shrink-0 text-muted-foreground" />)
                          : <span className="w-3 shrink-0" />}
                        <FolderOpen className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                        <span className="truncate">{root}</span>
                      </span>
                      <span className="text-xs text-muted-foreground shrink-0 ml-1">{cnt}</span>
                    </button>
                    {hasChildren && isExpanded && (
                      <div className="pl-4 space-y-0.5">
                        {subKeys.sort().map(sub => {
                          const fullPath = `${root}/${sub}`;
                          const subCnt = folderCount(fullPath);
                          return (
                            <button
                              key={sub}
                              onClick={() => setSelectedFolder(fullPath)}
                              className={`w-full flex items-center justify-between px-2 py-1 rounded text-xs hover:bg-muted/60 transition-colors ${selectedFolder === fullPath ? 'bg-muted font-medium' : ''}`}
                            >
                              <span className="flex items-center gap-1.5 min-w-0">
                                <FolderOpen className="h-3 w-3 shrink-0 text-muted-foreground" />
                                <span className="truncate">{sub}</span>
                              </span>
                              <span className="text-xs text-muted-foreground">{subCnt}</span>
                            </button>
                          );
                        })}
                      </div>
                    )}
                  </div>
                );
              })}
            </nav>
          </div>

          {/* Main content */}
          <div className="flex-1 min-w-0 space-y-3">
            {/* Search */}
            <div className="relative">
              <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
              <input
                type="text"
                placeholder="Search sources..."
                value={search}
                onChange={e => setSearch(e.target.value)}
                className="w-full pl-8 pr-3 py-1.5 text-sm border rounded bg-background focus:outline-none focus:ring-1 focus:ring-ring"
              />
            </div>
            {sorted.length === 0 ? (
              <div className="text-center py-8 text-sm text-muted-foreground">No sources in this folder.</div>
            ) : (
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="cursor-pointer select-none" onClick={() => toggleSort('title')}>
                  <span className="flex items-center gap-1">Title <SortIcon field="title" /></span>
                </TableHead>
                <TableHead className="cursor-pointer select-none w-[120px]" onClick={() => toggleSort('data_type')}>
                  <span className="flex items-center gap-1">Type <SortIcon field="data_type" /></span>
                </TableHead>
                <TableHead className="w-[100px]">Source</TableHead>
                <TableHead className="cursor-pointer select-none w-[80px]" onClick={() => toggleSort('status')}>
                  <span className="flex items-center gap-1">Status <SortIcon field="status" /></span>
                </TableHead>
                <TableHead className="cursor-pointer select-none w-[110px]" onClick={() => toggleSort('created_at')}>
                  <span className="flex items-center gap-1">Created <SortIcon field="created_at" /></span>
                </TableHead>
                <TableHead className="w-[40px]" />
              </TableRow>
            </TableHeader>
            <TableBody>
              {sorted.map((source) => (
                <TableRow key={source.id}>
                  <TableCell>
                    <div className="min-w-0">
                      <Link to={`/organizations/${orgId}/data-sources/${source.id}`} className="text-sm font-medium truncate hover:underline block">{source.title}</Link>
                      {source.description && (
                        <p className="text-xs text-muted-foreground truncate mt-0.5">{source.description}</p>
                      )}
                    </div>
                  </TableCell>
                  <TableCell>
                    <Badge variant="outline" className="text-xs">{dataTypeLabel(source.data_type)}</Badge>
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-1 text-xs text-muted-foreground">
                      {source.source_type === 'file' && source.file_name ? (
                        <>
                          <Upload className="h-3 w-3 shrink-0" />
                          <span className="truncate max-w-[60px]">{source.file_name}</span>
                          {source.file_size_bytes != null && (
                            <span className="shrink-0">({formatFileSize(source.file_size_bytes)})</span>
                          )}
                        </>
                      ) : source.source_type === 'file' ? (
                        <>
                          <Upload className="h-3 w-3 shrink-0" />
                          <span>File</span>
                        </>
                      ) : source.source_type === 'text' ? (
                        <>
                          <FileText className="h-3 w-3 shrink-0" />
                          <span>Text</span>
                        </>
                      ) : (
                        <>
                          <Database className="h-3 w-3 shrink-0" />
                          <span>Integration</span>
                        </>
                      )}
                    </div>
                  </TableCell>
                  <TableCell>{statusBadge(source.status)}</TableCell>
                  <TableCell className="text-xs text-muted-foreground">{formatDate(source.created_at)}</TableCell>
                  <TableCell>
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <Button variant="ghost" size="sm" className="h-7 w-7 p-0">
                          <MoreHorizontal className="h-3.5 w-3.5" />
                        </Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuItem onClick={() => { setEditingSource(source); setAddOpen(true); }}>
                          <Pencil className="h-3.5 w-3.5 mr-2" />
                          Edit
                        </DropdownMenuItem>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem
                          className="text-destructive focus:text-destructive"
                          onClick={() => setDeleteTarget(source)}
                        >
                          <Trash2 className="h-3.5 w-3.5 mr-2" />
                          Delete
                        </DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </Card>
            )}
          </div>
        </div>
      )}

      <AddDataSourceDialog
        orgId={orgId}
        projectEntries={projectEntries}
        open={addOpen}
        onOpenChange={setAddOpen}
        editingSource={editingSource}
      />

      <DeleteConfirmDialog
        open={!!deleteTarget}
        onOpenChange={(v) => { if (!v) setDeleteTarget(null); }}
        sourceName={deleteTarget?.title ?? ''}
        onConfirm={() => deleteTarget && deleteMutation.mutate(deleteTarget.id)}
        isPending={deleteMutation.isPending}
      />
    </div>
  );
}
