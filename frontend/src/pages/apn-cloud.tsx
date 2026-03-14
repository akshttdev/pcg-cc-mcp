import { useCallback, useRef, useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  sharedStorageApi,
  type StorageVolume,
  type StorageEntry,
  type StorageSearchResult,
} from '@/lib/api';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
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
  DialogFooter,
} from '@/components/ui/dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { toast } from 'sonner';
import {
  Folder,
  File,
  FileText,
  FileImage,
  FileVideo,
  FileAudio,
  FileArchive,
  Upload,
  FolderPlus,
  Download,
  Trash2,
  Pencil,
  Search,
  HardDrive,
  LayoutGrid,
  List,
  MoreVertical,
  ChevronRight,
  Home,
  Loader2,
  X,
} from 'lucide-react';
import { useDebounce } from '@/hooks/useDebounce';

// ── Helpers ──────────────────────────────────────────────────────────────────

function formatFileSize(bytes: number | null): string {
  if (bytes === null || bytes === undefined) return '--';
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  const value = bytes / Math.pow(1024, i);
  return `${value.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

function formatDate(dateStr: string | null): string {
  if (!dateStr) return '--';
  const date = new Date(dateStr);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMin = Math.floor(diffMs / 60000);
  const diffHrs = Math.floor(diffMs / 3600000);
  const diffDays = Math.floor(diffMs / 86400000);

  if (diffMin < 1) return 'Just now';
  if (diffMin < 60) return `${diffMin}m ago`;
  if (diffHrs < 24) return `${diffHrs}h ago`;
  if (diffDays < 7) return `${diffDays}d ago`;
  return date.toLocaleDateString('en-US', {
    month: 'short',
    day: 'numeric',
    year: date.getFullYear() !== now.getFullYear() ? 'numeric' : undefined,
  });
}

const IMAGE_EXTS = new Set([
  'jpg', 'jpeg', 'png', 'gif', 'bmp', 'svg', 'webp', 'heic', 'heif',
  'tiff', 'tif', 'cr2', 'cr3', 'nef', 'arw', 'dng', 'psd', 'ico',
]);
const VIDEO_EXTS = new Set([
  'mp4', 'mov', 'avi', 'mkv', 'wmv', 'flv', 'webm', 'cine', 'insv', 'mxf', 'mts', 'prores',
]);
const AUDIO_EXTS = new Set([
  'wav', 'mp3', 'm4a', 'flac', 'aac', 'ogg', 'wma', 'aiff',
]);
const ARCHIVE_EXTS = new Set([
  'zip', 'rar', 'tar', 'gz', '7z', 'bz2', 'xz', 'zst', 'dmg', 'iso',
]);
const DOC_EXTS = new Set([
  'pdf', 'doc', 'docx', 'txt', 'csv', 'xlsx', 'xls', 'pptx', 'ppt',
  'rtf', 'odt', 'md', 'json', 'xml', 'yaml', 'yml', 'toml', 'log',
]);

function getFileIcon(entry: StorageEntry | StorageSearchResult) {
  if (entry.entry_type === 'directory') return Folder;
  const ext = entry.name.split('.').pop()?.toLowerCase() ?? '';
  if (IMAGE_EXTS.has(ext)) return FileImage;
  if (VIDEO_EXTS.has(ext)) return FileVideo;
  if (AUDIO_EXTS.has(ext)) return FileAudio;
  if (ARCHIVE_EXTS.has(ext)) return FileArchive;
  if (DOC_EXTS.has(ext)) return FileText;
  return File;
}

function getFileIconColor(entry: StorageEntry | StorageSearchResult): string {
  if (entry.entry_type === 'directory') return 'text-blue-400';
  const ext = entry.name.split('.').pop()?.toLowerCase() ?? '';
  if (IMAGE_EXTS.has(ext)) return 'text-green-400';
  if (VIDEO_EXTS.has(ext)) return 'text-purple-400';
  if (AUDIO_EXTS.has(ext)) return 'text-amber-400';
  if (ARCHIVE_EXTS.has(ext)) return 'text-orange-400';
  if (DOC_EXTS.has(ext)) return 'text-rose-400';
  return 'text-muted-foreground';
}

function joinPath(...parts: string[]): string {
  return parts
    .join('/')
    .replace(/\/+/g, '/')
    .replace(/\/$/, '') || '/';
}

// ── Main Component ───────────────────────────────────────────────────────────

export function ApnCloudPage() {
  const queryClient = useQueryClient();
  const fileInputRef = useRef<HTMLInputElement>(null);

  // State
  const [currentVolume, setCurrentVolume] = useState('sovereign');
  const [currentPath, setCurrentPath] = useState('/');
  const [viewMode, setViewMode] = useState<'grid' | 'list'>('list');
  const [searchQuery, setSearchQuery] = useState('');
  const [isSearching, setIsSearching] = useState(false);
  const [showNewFolderDialog, setShowNewFolderDialog] = useState(false);
  const [newFolderName, setNewFolderName] = useState('');
  const [renameTarget, setRenameTarget] = useState<StorageEntry | null>(null);
  const [renameValue, setRenameValue] = useState('');
  const [isDragOver, setIsDragOver] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<StorageEntry | null>(null);
  const [uploadingFiles, setUploadingFiles] = useState<string[]>([]);

  const debouncedSearch = useDebounce(searchQuery, 300);

  // ── Queries ──────────────────────────────────────────────────────────────

  const { data: volumes } = useQuery({
    queryKey: ['shared-storage', 'volumes'],
    queryFn: () => sharedStorageApi.listVolumes(),
  });

  const { data: browseData, isLoading } = useQuery({
    queryKey: ['shared-storage', 'browse', currentVolume, currentPath],
    queryFn: () => sharedStorageApi.browse(currentVolume, currentPath),
    enabled: !isSearching,
  });

  const { data: searchResults, isLoading: isSearchLoading } = useQuery({
    queryKey: ['shared-storage', 'search', currentVolume, debouncedSearch],
    queryFn: () => sharedStorageApi.search(debouncedSearch, currentVolume),
    enabled: isSearching && debouncedSearch.length > 1,
  });

  // ── Mutations ────────────────────────────────────────────────────────────

  const uploadMutation = useMutation({
    mutationFn: (file: File) =>
      sharedStorageApi.upload(file, currentVolume, currentPath),
    onMutate: (file) => {
      setUploadingFiles((prev) => [...prev, file.name]);
    },
    onSuccess: (_data, file) => {
      setUploadingFiles((prev) => prev.filter((n) => n !== file.name));
      queryClient.invalidateQueries({
        queryKey: ['shared-storage', 'browse', currentVolume, currentPath],
      });
      toast.success(`Uploaded ${file.name}`);
    },
    onError: (err, file) => {
      setUploadingFiles((prev) => prev.filter((n) => n !== file.name));
      toast.error(`Failed to upload ${file.name}: ${(err as Error).message}`);
    },
  });

  const mkdirMutation = useMutation({
    mutationFn: (name: string) =>
      sharedStorageApi.mkdir(currentVolume, joinPath(currentPath, name)),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ['shared-storage', 'browse', currentVolume, currentPath],
      });
      setShowNewFolderDialog(false);
      setNewFolderName('');
      toast.success('Folder created');
    },
    onError: (err) => {
      toast.error(`Failed to create folder: ${(err as Error).message}`);
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (entry: StorageEntry) =>
      sharedStorageApi.deleteEntry(
        currentVolume,
        joinPath(currentPath, entry.name)
      ),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ['shared-storage', 'browse', currentVolume, currentPath],
      });
      setDeleteTarget(null);
      toast.success('Deleted successfully');
    },
    onError: (err) => {
      toast.error(`Failed to delete: ${(err as Error).message}`);
    },
  });

  const renameMutation = useMutation({
    mutationFn: ({ entry, newName }: { entry: StorageEntry; newName: string }) =>
      sharedStorageApi.rename(
        currentVolume,
        joinPath(currentPath, entry.name),
        joinPath(currentPath, newName)
      ),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ['shared-storage', 'browse', currentVolume, currentPath],
      });
      setRenameTarget(null);
      setRenameValue('');
      toast.success('Renamed successfully');
    },
    onError: (err) => {
      toast.error(`Failed to rename: ${(err as Error).message}`);
    },
  });

  // ── Navigation helpers ───────────────────────────────────────────────────

  const navigateToFolder = useCallback(
    (name: string) => {
      setCurrentPath(joinPath(currentPath, name));
      setIsSearching(false);
      setSearchQuery('');
    },
    [currentPath]
  );

  const navigateToPath = useCallback((path: string) => {
    setCurrentPath(path || '/');
    setIsSearching(false);
    setSearchQuery('');
  }, []);

  const getBreadcrumbs = useCallback((): { label: string; path: string }[] => {
    if (currentPath === '/') return [];
    const segments = currentPath.split('/').filter(Boolean);
    return segments.map((seg, i) => ({
      label: seg,
      path: '/' + segments.slice(0, i + 1).join('/'),
    }));
  }, [currentPath]);

  // ── Upload handlers ──────────────────────────────────────────────────────

  const handleFiles = useCallback(
    (files: FileList | File[]) => {
      Array.from(files).forEach((file) => uploadMutation.mutate(file));
    },
    [uploadMutation]
  );

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragOver(true);
  }, []);

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragOver(false);
  }, []);

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      e.stopPropagation();
      setIsDragOver(false);
      if (e.dataTransfer.files.length > 0) {
        handleFiles(e.dataTransfer.files);
      }
    },
    [handleFiles]
  );

  // ── Entry click ──────────────────────────────────────────────────────────

  const handleEntryClick = useCallback(
    (entry: StorageEntry) => {
      if (entry.entry_type === 'directory') {
        navigateToFolder(entry.name);
      } else {
        const url = sharedStorageApi.getDownloadUrl(
          currentVolume,
          joinPath(currentPath, entry.name)
        );
        window.open(url, '_blank');
      }
    },
    [currentVolume, currentPath, navigateToFolder]
  );

  const handleSearchResultClick = useCallback(
    (result: StorageSearchResult) => {
      if (result.entry_type === 'directory') {
        setCurrentVolume(result.volume);
        setCurrentPath(result.path);
        setIsSearching(false);
        setSearchQuery('');
      } else {
        const url = sharedStorageApi.getDownloadUrl(result.volume, result.path);
        window.open(url, '_blank');
      }
    },
    []
  );

  // ── Search toggle ────────────────────────────────────────────────────────

  const handleSearchChange = useCallback((value: string) => {
    setSearchQuery(value);
    setIsSearching(value.length > 0);
  }, []);

  const clearSearch = useCallback(() => {
    setSearchQuery('');
    setIsSearching(false);
  }, []);

  // ── Current volume info ──────────────────────────────────────────────────

  const currentVolumeInfo = volumes?.find((v: StorageVolume) => v.id === currentVolume);
  const isWritable = currentVolumeInfo?.writable ?? false;
  const entries = browseData?.entries ?? [];
  const breadcrumbs = getBreadcrumbs();

  // Sort: directories first, then alphabetical
  const sortedEntries = [...entries].sort((a, b) => {
    if (a.entry_type !== b.entry_type) {
      return a.entry_type === 'directory' ? -1 : 1;
    }
    return a.name.localeCompare(b.name, undefined, { sensitivity: 'base' });
  });

  // ── Render ─────────────────────────────────────────────────────────────

  return (
    <div className="flex flex-col h-full">
      {/* ── Top Bar ──────────────────────────────────────────────────────── */}
      <div className="flex-shrink-0 border-b border-border bg-background px-4 py-3">
        <div className="flex items-center gap-3 flex-wrap">
          {/* Volume selector */}
          <Select value={currentVolume} onValueChange={(v) => {
            setCurrentVolume(v);
            setCurrentPath('/');
            setIsSearching(false);
            setSearchQuery('');
          }}>
            <SelectTrigger className="w-[200px]">
              <HardDrive className="h-4 w-4 mr-2 text-muted-foreground" />
              <SelectValue placeholder="Select volume" />
            </SelectTrigger>
            <SelectContent>
              {volumes?.map((vol: StorageVolume) => (
                <SelectItem key={vol.id} value={vol.id}>
                  <div className="flex items-center gap-2">
                    <span>{vol.name}</span>
                    {vol.writable ? (
                      <Badge variant="outline" className="text-[10px] py-0 px-1">rw</Badge>
                    ) : (
                      <Badge variant="secondary" className="text-[10px] py-0 px-1">ro</Badge>
                    )}
                  </div>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          {/* Breadcrumb */}
          <div className="flex items-center gap-1 text-sm min-w-0 flex-1">
            <button
              onClick={() => navigateToPath('/')}
              className="text-muted-foreground hover:text-foreground transition-colors p-1 rounded hover:bg-muted"
              title="Root"
            >
              <Home className="h-4 w-4" />
            </button>
            {breadcrumbs.map((crumb) => (
              <span key={crumb.path} className="flex items-center gap-1 min-w-0">
                <ChevronRight className="h-3 w-3 text-muted-foreground flex-shrink-0" />
                <button
                  onClick={() => navigateToPath(crumb.path)}
                  className="text-muted-foreground hover:text-foreground transition-colors truncate max-w-[150px]"
                  title={crumb.label}
                >
                  {crumb.label}
                </button>
              </span>
            ))}
          </div>

          {/* Search */}
          <div className="relative w-[250px]">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              value={searchQuery}
              onChange={(e) => handleSearchChange(e.target.value)}
              placeholder="Search files..."
              className="pl-9 pr-8 h-9"
            />
            {searchQuery && (
              <button
                onClick={clearSearch}
                className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
              >
                <X className="h-4 w-4" />
              </button>
            )}
          </div>

          {/* View toggle */}
          <div className="flex items-center border border-border rounded-md">
            <button
              onClick={() => setViewMode('list')}
              className={`p-1.5 rounded-l-md transition-colors ${
                viewMode === 'list'
                  ? 'bg-muted text-foreground'
                  : 'text-muted-foreground hover:text-foreground'
              }`}
              title="List view"
            >
              <List className="h-4 w-4" />
            </button>
            <button
              onClick={() => setViewMode('grid')}
              className={`p-1.5 rounded-r-md transition-colors ${
                viewMode === 'grid'
                  ? 'bg-muted text-foreground'
                  : 'text-muted-foreground hover:text-foreground'
              }`}
              title="Grid view"
            >
              <LayoutGrid className="h-4 w-4" />
            </button>
          </div>
        </div>
      </div>

      {/* ── Action Bar ───────────────────────────────────────────────────── */}
      {isWritable && !isSearching && (
        <div className="flex-shrink-0 border-b border-border bg-background px-4 py-2">
          <div className="flex items-center gap-2">
            <Button
              size="sm"
              onClick={() => fileInputRef.current?.click()}
            >
              <Upload className="h-4 w-4 mr-1.5" />
              Upload
            </Button>
            <Button
              size="sm"
              variant="outline"
              onClick={() => {
                setNewFolderName('');
                setShowNewFolderDialog(true);
              }}
            >
              <FolderPlus className="h-4 w-4 mr-1.5" />
              New Folder
            </Button>
            <input
              ref={fileInputRef}
              type="file"
              multiple
              className="hidden"
              onChange={(e) => {
                if (e.target.files?.length) {
                  handleFiles(e.target.files);
                  e.target.value = '';
                }
              }}
            />

            {/* Upload progress indicators */}
            {uploadingFiles.length > 0 && (
              <div className="flex items-center gap-2 ml-4 text-sm text-muted-foreground">
                <Loader2 className="h-4 w-4 animate-spin" />
                <span>
                  Uploading {uploadingFiles.length} file{uploadingFiles.length > 1 ? 's' : ''}...
                </span>
              </div>
            )}
          </div>
        </div>
      )}

      {/* ── Main Content Area ────────────────────────────────────────────── */}
      <div
        className="flex-1 overflow-auto relative"
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
      >
        {/* Drag overlay */}
        {isDragOver && (
          <div className="absolute inset-0 z-50 bg-blue-500/10 border-2 border-dashed border-blue-500 rounded-lg flex items-center justify-center pointer-events-none">
            <div className="bg-background/90 rounded-lg px-6 py-4 text-center shadow-lg">
              <Upload className="h-8 w-8 text-blue-500 mx-auto mb-2" />
              <p className="text-sm font-medium">Drop files to upload</p>
              <p className="text-xs text-muted-foreground mt-1">
                to {currentVolume}:{currentPath}
              </p>
            </div>
          </div>
        )}

        {/* Loading state */}
        {(isLoading || isSearchLoading) && (
          <div className="flex items-center justify-center py-20">
            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
          </div>
        )}

        {/* Search results */}
        {isSearching && !isSearchLoading && (
          <div className="p-4">
            {debouncedSearch.length <= 1 ? (
              <p className="text-sm text-muted-foreground text-center py-10">
                Type at least 2 characters to search
              </p>
            ) : searchResults && searchResults.length > 0 ? (
              <div>
                <p className="text-sm text-muted-foreground mb-3">
                  {searchResults.length} result{searchResults.length !== 1 ? 's' : ''} for "{debouncedSearch}"
                </p>
                <div className="border border-border rounded-lg overflow-hidden">
                  <table className="w-full">
                    <thead>
                      <tr className="border-b border-border bg-muted/50 text-xs text-muted-foreground">
                        <th className="text-left py-2 px-3 font-medium">Name</th>
                        <th className="text-left py-2 px-3 font-medium w-[120px]">Volume</th>
                        <th className="text-left py-2 px-3 font-medium">Path</th>
                        <th className="text-right py-2 px-3 font-medium w-[80px]">Size</th>
                      </tr>
                    </thead>
                    <tbody>
                      {searchResults.map((result: StorageSearchResult, i: number) => {
                        const Icon = getFileIcon(result);
                        const iconColor = getFileIconColor(result);
                        return (
                          <tr
                            key={`${result.volume}-${result.path}-${i}`}
                            className="border-b border-border last:border-b-0 hover:bg-muted/30 cursor-pointer transition-colors"
                            onClick={() => handleSearchResultClick(result)}
                          >
                            <td className="py-2 px-3">
                              <div className="flex items-center gap-2">
                                <Icon className={`h-4 w-4 flex-shrink-0 ${iconColor}`} />
                                <span className="text-sm truncate">{result.name}</span>
                              </div>
                            </td>
                            <td className="py-2 px-3">
                              <Badge variant="outline" className="text-[10px]">{result.volume}</Badge>
                            </td>
                            <td className="py-2 px-3 text-xs text-muted-foreground truncate max-w-[300px]">
                              {result.path}
                            </td>
                            <td className="py-2 px-3 text-xs text-muted-foreground text-right">
                              {formatFileSize(result.size)}
                            </td>
                          </tr>
                        );
                      })}
                    </tbody>
                  </table>
                </div>
              </div>
            ) : (
              <p className="text-sm text-muted-foreground text-center py-10">
                No results found for "{debouncedSearch}"
              </p>
            )}
          </div>
        )}

        {/* Browse - Empty state */}
        {!isSearching && !isLoading && sortedEntries.length === 0 && (
          <div className="flex flex-col items-center justify-center py-20 text-center">
            <Folder className="h-12 w-12 text-muted-foreground/40 mb-3" />
            <p className="text-sm font-medium text-muted-foreground">This folder is empty</p>
            {isWritable && (
              <p className="text-xs text-muted-foreground mt-1">
                Drag files here or use the Upload button
              </p>
            )}
          </div>
        )}

        {/* Browse - List view */}
        {!isSearching && !isLoading && sortedEntries.length > 0 && viewMode === 'list' && (
          <div className="p-4">
            <div className="border border-border rounded-lg overflow-hidden">
              <table className="w-full">
                <thead>
                  <tr className="border-b border-border bg-muted/50 text-xs text-muted-foreground">
                    <th className="text-left py-2 px-3 font-medium">Name</th>
                    <th className="text-right py-2 px-3 font-medium w-[100px]">Size</th>
                    <th className="text-left py-2 px-3 font-medium w-[120px]">Modified</th>
                    {isWritable && <th className="w-[40px]" />}
                  </tr>
                </thead>
                <tbody>
                  {sortedEntries.map((entry) => {
                    const Icon = getFileIcon(entry);
                    const iconColor = getFileIconColor(entry);
                    return (
                      <tr
                        key={entry.name}
                        className="border-b border-border last:border-b-0 hover:bg-muted/30 cursor-pointer transition-colors group"
                        onClick={() => handleEntryClick(entry)}
                      >
                        <td className="py-2 px-3">
                          <div className="flex items-center gap-2">
                            <Icon className={`h-4 w-4 flex-shrink-0 ${iconColor}`} />
                            <span className="text-sm truncate">{entry.name}</span>
                          </div>
                        </td>
                        <td className="py-2 px-3 text-xs text-muted-foreground text-right">
                          {entry.entry_type === 'directory' ? '--' : formatFileSize(entry.size)}
                        </td>
                        <td className="py-2 px-3 text-xs text-muted-foreground">
                          {formatDate(entry.modified)}
                        </td>
                        {isWritable && (
                          <td className="py-2 px-1" onClick={(e) => e.stopPropagation()}>
                            <EntryContextMenu
                              entry={entry}
                              onDownload={() => {
                                const url = sharedStorageApi.getDownloadUrl(
                                  currentVolume,
                                  joinPath(currentPath, entry.name)
                                );
                                window.open(url, '_blank');
                              }}
                              onRename={() => {
                                setRenameTarget(entry);
                                setRenameValue(entry.name);
                              }}
                              onDelete={() => setDeleteTarget(entry)}
                            />
                          </td>
                        )}
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
            <p className="text-xs text-muted-foreground mt-2 px-1">
              {sortedEntries.length} item{sortedEntries.length !== 1 ? 's' : ''}
            </p>
          </div>
        )}

        {/* Browse - Grid view */}
        {!isSearching && !isLoading && sortedEntries.length > 0 && viewMode === 'grid' && (
          <div className="p-4">
            <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
              {sortedEntries.map((entry) => {
                const Icon = getFileIcon(entry);
                const iconColor = getFileIconColor(entry);
                return (
                  <div
                    key={entry.name}
                    className="group relative border border-border rounded-lg p-3 hover:border-primary/50 hover:bg-muted/30 cursor-pointer transition-all"
                    onClick={() => handleEntryClick(entry)}
                  >
                    {/* Context menu button */}
                    {isWritable && (
                      <div
                        className="absolute top-1.5 right-1.5 opacity-0 group-hover:opacity-100 transition-opacity"
                        onClick={(e) => e.stopPropagation()}
                      >
                        <EntryContextMenu
                          entry={entry}
                          onDownload={() => {
                            const url = sharedStorageApi.getDownloadUrl(
                              currentVolume,
                              joinPath(currentPath, entry.name)
                            );
                            window.open(url, '_blank');
                          }}
                          onRename={() => {
                            setRenameTarget(entry);
                            setRenameValue(entry.name);
                          }}
                          onDelete={() => setDeleteTarget(entry)}
                        />
                      </div>
                    )}

                    <div className="flex flex-col items-center text-center gap-2">
                      <Icon className={`h-10 w-10 ${iconColor}`} />
                      <p className="text-xs font-medium truncate w-full" title={entry.name}>
                        {entry.name}
                      </p>
                      <p className="text-[10px] text-muted-foreground">
                        {entry.entry_type === 'directory'
                          ? 'Folder'
                          : formatFileSize(entry.size)}
                      </p>
                    </div>
                  </div>
                );
              })}
            </div>
            <p className="text-xs text-muted-foreground mt-3 px-1">
              {sortedEntries.length} item{sortedEntries.length !== 1 ? 's' : ''}
            </p>
          </div>
        )}

        {/* Upload progress list */}
        {uploadingFiles.length > 0 && (
          <div className="fixed bottom-4 right-4 bg-background border border-border rounded-lg shadow-lg p-3 w-[280px] z-40">
            <p className="text-xs font-medium mb-2">Uploading</p>
            <div className="space-y-1.5">
              {uploadingFiles.map((name) => (
                <div key={name} className="flex items-center gap-2 text-xs text-muted-foreground">
                  <Loader2 className="h-3 w-3 animate-spin flex-shrink-0" />
                  <span className="truncate">{name}</span>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>

      {/* ── New Folder Dialog ────────────────────────────────────────────── */}
      <Dialog open={showNewFolderDialog} onOpenChange={setShowNewFolderDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>New Folder</DialogTitle>
          </DialogHeader>
          <Input
            value={newFolderName}
            onChange={(e) => setNewFolderName(e.target.value)}
            placeholder="Folder name"
            autoFocus
            onKeyDown={(e) => {
              if (e.key === 'Enter' && newFolderName.trim()) {
                mkdirMutation.mutate(newFolderName.trim());
              }
            }}
          />
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setShowNewFolderDialog(false)}
            >
              Cancel
            </Button>
            <Button
              onClick={() => {
                if (newFolderName.trim()) {
                  mkdirMutation.mutate(newFolderName.trim());
                }
              }}
              disabled={!newFolderName.trim() || mkdirMutation.isPending}
            >
              {mkdirMutation.isPending ? (
                <Loader2 className="h-4 w-4 animate-spin mr-1.5" />
              ) : (
                <FolderPlus className="h-4 w-4 mr-1.5" />
              )}
              Create
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* ── Rename Dialog ────────────────────────────────────────────────── */}
      <Dialog
        open={renameTarget !== null}
        onOpenChange={(open) => {
          if (!open) {
            setRenameTarget(null);
            setRenameValue('');
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              Rename {renameTarget?.entry_type === 'directory' ? 'Folder' : 'File'}
            </DialogTitle>
          </DialogHeader>
          <Input
            value={renameValue}
            onChange={(e) => setRenameValue(e.target.value)}
            placeholder="New name"
            autoFocus
            onKeyDown={(e) => {
              if (e.key === 'Enter' && renameTarget && renameValue.trim()) {
                renameMutation.mutate({
                  entry: renameTarget,
                  newName: renameValue.trim(),
                });
              }
            }}
          />
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setRenameTarget(null);
                setRenameValue('');
              }}
            >
              Cancel
            </Button>
            <Button
              onClick={() => {
                if (renameTarget && renameValue.trim()) {
                  renameMutation.mutate({
                    entry: renameTarget,
                    newName: renameValue.trim(),
                  });
                }
              }}
              disabled={
                !renameValue.trim() ||
                renameValue === renameTarget?.name ||
                renameMutation.isPending
              }
            >
              {renameMutation.isPending && (
                <Loader2 className="h-4 w-4 animate-spin mr-1.5" />
              )}
              Rename
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* ── Delete Confirmation Dialog ───────────────────────────────────── */}
      <Dialog
        open={deleteTarget !== null}
        onOpenChange={(open) => {
          if (!open) setDeleteTarget(null);
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              Delete {deleteTarget?.entry_type === 'directory' ? 'Folder' : 'File'}
            </DialogTitle>
          </DialogHeader>
          <p className="text-sm text-muted-foreground">
            Are you sure you want to delete{' '}
            <span className="font-medium text-foreground">{deleteTarget?.name}</span>?
            {deleteTarget?.entry_type === 'directory' &&
              ' This will delete all contents inside the folder.'}
            {' '}This action cannot be undone.
          </p>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteTarget(null)}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={() => {
                if (deleteTarget) {
                  deleteMutation.mutate(deleteTarget);
                }
              }}
              disabled={deleteMutation.isPending}
            >
              {deleteMutation.isPending && (
                <Loader2 className="h-4 w-4 animate-spin mr-1.5" />
              )}
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

// ── Context Menu Sub-component ───────────────────────────────────────────────

function EntryContextMenu({
  entry,
  onDownload,
  onRename,
  onDelete,
}: {
  entry: StorageEntry;
  onDownload: () => void;
  onRename: () => void;
  onDelete: () => void;
}) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button className="p-1 rounded hover:bg-muted text-muted-foreground hover:text-foreground transition-colors">
          <MoreVertical className="h-4 w-4" />
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        {entry.entry_type === 'file' && (
          <DropdownMenuItem onClick={onDownload}>
            <Download className="h-4 w-4 mr-2" />
            Download
          </DropdownMenuItem>
        )}
        <DropdownMenuItem onClick={onRename}>
          <Pencil className="h-4 w-4 mr-2" />
          Rename
        </DropdownMenuItem>
        <DropdownMenuItem
          onClick={onDelete}
          className="text-destructive focus:text-destructive"
        >
          <Trash2 className="h-4 w-4 mr-2" />
          Delete
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
