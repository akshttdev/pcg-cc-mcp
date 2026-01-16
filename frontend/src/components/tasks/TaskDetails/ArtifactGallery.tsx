import { useState, useMemo } from 'react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  Grid3X3,
  List,
  Search,
  Upload,
  Link2,
  SortAsc,
  SortDesc,
  FolderOpen,
  Pin,
  Bot,
  User,
  FileText,
  Image,
  Video,
  File,
  Plus,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { ArtifactPreviewCard } from '../ArtifactPreviewCard';
import type { ExecutionArtifact, ArtifactType, ArtifactPhase } from 'shared/types';

type ViewMode = 'grid' | 'list';
type SortField = 'date' | 'name' | 'type';
type SortDirection = 'asc' | 'desc';
type FilterPhase = 'all' | ArtifactPhase | 'user';

interface ArtifactGalleryProps {
  artifacts: ExecutionArtifact[];
  pinnedIds?: string[];
  onPin?: (id: string) => void;
  onUnpin?: (id: string) => void;
  onDownload?: (artifact: ExecutionArtifact) => void;
  onUpload?: (file: File) => Promise<void>;
  onLinkAdd?: (url: string, name: string) => Promise<void>;
  className?: string;
  defaultView?: ViewMode;
  showHeader?: boolean;
}

// Artifact type icons
const typeIcons: Partial<Record<ArtifactType, React.ReactNode>> = {
  plan: <FileText className="h-4 w-4 text-purple-500" />,
  screenshot: <Image className="h-4 w-4 text-pink-500" />,
  visual_brief: <Image className="h-4 w-4 text-pink-500" />,
  walkthrough: <Video className="h-4 w-4 text-blue-500" />,
  browser_recording: <Video className="h-4 w-4 text-blue-500" />,
  research_report: <FileText className="h-4 w-4 text-emerald-500" />,
  strategy_document: <FileText className="h-4 w-4 text-amber-500" />,
  content_draft: <FileText className="h-4 w-4 text-cyan-500" />,
};

// Phase colors
const phaseColors: Record<ArtifactPhase, string> = {
  planning: 'bg-purple-100 text-purple-700 dark:bg-purple-900 dark:text-purple-300',
  execution: 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900 dark:text-yellow-300',
  verification: 'bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300',
};

function ArtifactListItem({
  artifact,
  isPinned,
  onPin,
  onDownload: _onDownload,
}: {
  artifact: ExecutionArtifact;
  isPinned?: boolean;
  onPin?: () => void;
  onDownload?: () => void;
}) {
  const metadata = useMemo(() => {
    try {
      return artifact.metadata ? JSON.parse(artifact.metadata) : {};
    } catch {
      return {};
    }
  }, [artifact.metadata]);

  const phase = metadata.phase as ArtifactPhase | undefined;
  const createdBy = metadata.created_by as 'agent' | 'human' | undefined;

  const formatDate = (dateString: string) => {
    const date = new Date(dateString);
    return date.toLocaleDateString(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  return (
    <div
      className={cn(
        'flex items-center gap-3 p-3 rounded-lg border hover:bg-muted/50 transition-colors cursor-pointer',
        isPinned && 'ring-2 ring-primary bg-primary/5'
      )}
    >
      {/* Icon */}
      <div className="shrink-0 p-2 rounded-md bg-muted">
        {typeIcons[artifact.artifact_type] || <File className="h-4 w-4" />}
      </div>

      {/* Content */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="font-medium text-sm truncate">{artifact.title}</span>
          {isPinned && <Pin className="h-3 w-3 text-primary" />}
        </div>
        <div className="flex items-center gap-2 mt-0.5">
          <Badge variant="outline" className="text-[10px] h-4">
            {artifact.artifact_type.replace(/_/g, ' ')}
          </Badge>
          {phase && (
            <Badge variant="outline" className={cn('text-[10px] h-4', phaseColors[phase])}>
              {phase}
            </Badge>
          )}
          {createdBy && (
            <span className="flex items-center gap-0.5 text-[10px] text-muted-foreground">
              {createdBy === 'agent' ? <Bot className="h-3 w-3" /> : <User className="h-3 w-3" />}
            </span>
          )}
        </div>
      </div>

      {/* Timestamp */}
      <span className="text-xs text-muted-foreground shrink-0">
        {formatDate(artifact.created_at)}
      </span>

      {/* Actions */}
      <div className="flex items-center gap-1 shrink-0">
        {onPin && (
          <Button
            variant="ghost"
            size="icon"
            className={cn('h-7 w-7', isPinned && 'text-primary')}
            onClick={(e) => {
              e.stopPropagation();
              onPin();
            }}
          >
            <Pin className="h-3.5 w-3.5" />
          </Button>
        )}
      </div>
    </div>
  );
}

export function ArtifactGallery({
  artifacts,
  pinnedIds = [],
  onPin,
  onUnpin,
  onDownload,
  onUpload,
  onLinkAdd,
  className,
  defaultView = 'grid',
  showHeader = true,
}: ArtifactGalleryProps) {
  const [viewMode, setViewMode] = useState<ViewMode>(defaultView);
  const [searchQuery, setSearchQuery] = useState('');
  const [sortField, setSortField] = useState<SortField>('date');
  const [sortDirection, setSortDirection] = useState<SortDirection>('desc');
  const [filterPhase, setFilterPhase] = useState<FilterPhase>('all');
  const [showUploadForm, setShowUploadForm] = useState(false);
  const [linkUrl, setLinkUrl] = useState('');
  const [linkName, setLinkName] = useState('');
  const [isUploading, setIsUploading] = useState(false);

  // Filter and sort artifacts
  const filteredArtifacts = useMemo(() => {
    let result = [...artifacts];

    // Filter by search
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      result = result.filter(
        (a) =>
          a.title.toLowerCase().includes(query) ||
          a.artifact_type.toLowerCase().includes(query)
      );
    }

    // Filter by phase
    if (filterPhase !== 'all') {
      result = result.filter((a) => {
        try {
          const metadata = a.metadata ? JSON.parse(a.metadata) : {};
          if (filterPhase === 'user') {
            return metadata.uploaded_by === 'user';
          }
          return metadata.phase === filterPhase;
        } catch {
          return false;
        }
      });
    }

    // Sort
    result.sort((a, b) => {
      let comparison = 0;
      switch (sortField) {
        case 'date':
          comparison = new Date(a.created_at).getTime() - new Date(b.created_at).getTime();
          break;
        case 'name':
          comparison = a.title.localeCompare(b.title);
          break;
        case 'type':
          comparison = a.artifact_type.localeCompare(b.artifact_type);
          break;
      }
      return sortDirection === 'asc' ? comparison : -comparison;
    });

    return result;
  }, [artifacts, searchQuery, filterPhase, sortField, sortDirection]);

  // Separate pinned artifacts
  const pinnedArtifacts = filteredArtifacts.filter((a) => pinnedIds.includes(a.id));
  const unpinnedArtifacts = filteredArtifacts.filter((a) => !pinnedIds.includes(a.id));

  const handleFileUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file || !onUpload) return;

    setIsUploading(true);
    try {
      await onUpload(file);
    } finally {
      setIsUploading(false);
      e.target.value = '';
    }
  };

  const handleLinkSubmit = async () => {
    if (!linkUrl.trim() || !onLinkAdd) return;

    setIsUploading(true);
    try {
      await onLinkAdd(linkUrl, linkName || 'External Link');
      setLinkUrl('');
      setLinkName('');
      setShowUploadForm(false);
    } finally {
      setIsUploading(false);
    }
  };

  const toggleSort = (field: SortField) => {
    if (sortField === field) {
      setSortDirection((d) => (d === 'asc' ? 'desc' : 'asc'));
    } else {
      setSortField(field);
      setSortDirection('desc');
    }
  };

  return (
    <div className={cn('flex flex-col h-full', className)}>
      {/* Header */}
      {showHeader && (
        <div className="flex items-center justify-between gap-3 p-3 border-b">
          <div className="flex items-center gap-2">
            <FolderOpen className="h-5 w-5 text-muted-foreground" />
            <span className="font-medium">Artifacts</span>
            <Badge variant="secondary">{artifacts.length}</Badge>
          </div>

          <div className="flex items-center gap-2">
            {/* Upload buttons */}
            {(onUpload || onLinkAdd) && (
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button variant="outline" size="sm" className="gap-1">
                    <Plus className="h-3.5 w-3.5" />
                    Add
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end">
                  {onUpload && (
                    <DropdownMenuItem asChild>
                      <label className="cursor-pointer">
                        <Upload className="h-4 w-4 mr-2" />
                        Upload File
                        <input
                          type="file"
                          className="hidden"
                          onChange={handleFileUpload}
                          disabled={isUploading}
                        />
                      </label>
                    </DropdownMenuItem>
                  )}
                  {onLinkAdd && (
                    <DropdownMenuItem onClick={() => setShowUploadForm(true)}>
                      <Link2 className="h-4 w-4 mr-2" />
                      Add Link
                    </DropdownMenuItem>
                  )}
                </DropdownMenuContent>
              </DropdownMenu>
            )}

            {/* View toggle */}
            <div className="flex items-center border rounded-md">
            <Button
              variant={viewMode === 'grid' ? 'secondary' : 'ghost'}
              size="sm"
              className="h-8 w-8 p-0 rounded-r-none"
              onClick={() => setViewMode('grid')}
            >
              <Grid3X3 className="h-4 w-4" />
            </Button>
            <Button
              variant={viewMode === 'list' ? 'secondary' : 'ghost'}
              size="sm"
              className="h-8 w-8 p-0 rounded-l-none"
              onClick={() => setViewMode('list')}
            >
              <List className="h-4 w-4" />
            </Button>
            </div>
          </div>
        </div>
      )}

      {/* Toolbar */}
      {showHeader && (
        <div className="flex items-center gap-2 p-3 border-b">
        {/* Search */}
        <div className="relative flex-1 max-w-xs">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search artifacts..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-8 h-8"
          />
        </div>

        {/* Phase filter */}
        <Tabs value={filterPhase} onValueChange={(v) => setFilterPhase(v as FilterPhase)}>
          <TabsList className="h-8">
            <TabsTrigger value="all" className="text-xs h-6 px-2">All</TabsTrigger>
            <TabsTrigger value="planning" className="text-xs h-6 px-2">Planning</TabsTrigger>
            <TabsTrigger value="execution" className="text-xs h-6 px-2">Execution</TabsTrigger>
            <TabsTrigger value="user" className="text-xs h-6 px-2">Uploads</TabsTrigger>
          </TabsList>
        </Tabs>

        {/* Sort */}
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="outline" size="sm" className="gap-1 h-8">
              {sortDirection === 'asc' ? <SortAsc className="h-3.5 w-3.5" /> : <SortDesc className="h-3.5 w-3.5" />}
              {sortField === 'date' ? 'Date' : sortField === 'name' ? 'Name' : 'Type'}
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onClick={() => toggleSort('date')}>
              Sort by Date {sortField === 'date' && (sortDirection === 'asc' ? '↑' : '↓')}
            </DropdownMenuItem>
            <DropdownMenuItem onClick={() => toggleSort('name')}>
              Sort by Name {sortField === 'name' && (sortDirection === 'asc' ? '↑' : '↓')}
            </DropdownMenuItem>
            <DropdownMenuItem onClick={() => toggleSort('type')}>
              Sort by Type {sortField === 'type' && (sortDirection === 'asc' ? '↑' : '↓')}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
        </div>
      )}

      {/* Link form */}
      {showUploadForm && (
        <div className="p-3 border-b bg-muted/30 space-y-2">
          <Input
            placeholder="URL (e.g., https://dropbox.com/...)"
            value={linkUrl}
            onChange={(e) => setLinkUrl(e.target.value)}
            className="h-8"
          />
          <div className="flex gap-2">
            <Input
              placeholder="Name (optional)"
              value={linkName}
              onChange={(e) => setLinkName(e.target.value)}
              className="h-8 flex-1"
            />
            <Button size="sm" onClick={handleLinkSubmit} disabled={!linkUrl.trim() || isUploading}>
              Add
            </Button>
            <Button size="sm" variant="outline" onClick={() => setShowUploadForm(false)}>
              Cancel
            </Button>
          </div>
        </div>
      )}

      {/* Content */}
      <ScrollArea className="flex-1">
        <div className="p-3 space-y-4">
          {/* Pinned section */}
          {pinnedArtifacts.length > 0 && (
            <div>
              <div className="flex items-center gap-2 mb-2">
                <Pin className="h-4 w-4 text-primary" />
                <span className="text-sm font-medium">Pinned</span>
              </div>
              {viewMode === 'grid' ? (
                <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-3">
                  {pinnedArtifacts.map((artifact) => (
                    <ArtifactPreviewCard
                      key={artifact.id}
                      artifact={artifact}
                      size="md"
                      showCreator
                      onDownload={onDownload ? () => onDownload(artifact) : undefined}
                    />
                  ))}
                </div>
              ) : (
                <div className="space-y-2">
                  {pinnedArtifacts.map((artifact) => (
                    <ArtifactListItem
                      key={artifact.id}
                      artifact={artifact}
                      isPinned
                      onPin={onUnpin ? () => onUnpin(artifact.id) : undefined}
                      onDownload={onDownload ? () => onDownload(artifact) : undefined}
                    />
                  ))}
                </div>
              )}
            </div>
          )}

          {/* All artifacts */}
          {unpinnedArtifacts.length > 0 ? (
            viewMode === 'grid' ? (
              <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-3">
                {unpinnedArtifacts.map((artifact) => (
                  <ArtifactPreviewCard
                    key={artifact.id}
                    artifact={artifact}
                    size="md"
                    showCreator
                    onDownload={onDownload ? () => onDownload(artifact) : undefined}
                  />
                ))}
              </div>
            ) : (
              <div className="space-y-2">
                {unpinnedArtifacts.map((artifact) => (
                  <ArtifactListItem
                    key={artifact.id}
                    artifact={artifact}
                    onPin={onPin ? () => onPin(artifact.id) : undefined}
                    onDownload={onDownload ? () => onDownload(artifact) : undefined}
                  />
                ))}
              </div>
            )
          ) : filteredArtifacts.length === 0 && artifacts.length > 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <Search className="h-12 w-12 mx-auto mb-2 opacity-50" />
              <p>No artifacts match your search</p>
            </div>
          ) : artifacts.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <FolderOpen className="h-12 w-12 mx-auto mb-2 opacity-50" />
              <p>No artifacts yet</p>
              <p className="text-xs mt-1">Artifacts will appear here as work progresses</p>
            </div>
          ) : null}
        </div>
      </ScrollArea>
    </div>
  );
}
