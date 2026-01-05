import { useMemo, useState } from 'react';
import { ArtifactCard } from './ArtifactCard';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Loader } from '@/components/ui/loader';
import { Grid, List, Search, Calendar, FolderOpen } from 'lucide-react';
import { cn } from '@/lib/utils';
import type {
  ExecutionArtifact,
  ArtifactType,
  ArtifactPhase,
} from 'shared/types';
import {
  parseArtifactMetadata,
  type ArtifactMetadata,
  formatArtifactPhase,
} from './utils';

type ViewMode = 'grid' | 'list' | 'timeline';
type GroupBy = 'none' | 'type' | 'phase' | 'date' | 'agent';

interface ArtifactGalleryProps {
  artifacts: ExecutionArtifact[];
  isLoading?: boolean;
  pinnedIds?: Set<string>;
  onArtifactClick?: (artifact: ExecutionArtifact) => void;
  onArtifactPin?: (artifact: ExecutionArtifact) => void;
  onArtifactDownload?: (artifact: ExecutionArtifact) => void;
  onArtifactPreview?: (artifact: ExecutionArtifact) => void;
  className?: string;
}

export function ArtifactGallery({
  artifacts,
  isLoading,
  pinnedIds = new Set(),
  onArtifactClick,
  onArtifactPin,
  onArtifactDownload,
  onArtifactPreview,
  className,
}: ArtifactGalleryProps) {
  const [viewMode, setViewMode] = useState<ViewMode>('grid');
  const [groupBy, setGroupBy] = useState<GroupBy>('none');
  const [searchQuery, setSearchQuery] = useState('');
  const [filterType, setFilterType] = useState<ArtifactType | 'all'>('all');
  const [filterPhase, setFilterPhase] = useState<ArtifactPhase | 'all'>('all');

  const metadataById = useMemo(() => {
    const map = new Map<string, ArtifactMetadata>();
    artifacts.forEach((artifact) => {
      map.set(artifact.id, parseArtifactMetadata(artifact.metadata));
    });
    return map;
  }, [artifacts]);

  // Get unique artifact types for filter
  const uniqueTypes = useMemo(() => {
    const types = new Set<ArtifactType>();
    artifacts.forEach((a) => types.add(a.artifact_type));
    return Array.from(types);
  }, [artifacts]);

  // Filter artifacts
  const filteredArtifacts = useMemo(() => {
    return artifacts.filter((artifact) => {
      const metadata = metadataById.get(artifact.id);

      // Search filter
      if (searchQuery) {
        const query = searchQuery.toLowerCase();
        const matchesTitle = artifact.title?.toLowerCase().includes(query);
        const matchesType = artifact.artifact_type.toLowerCase().includes(query);
        if (!matchesTitle && !matchesType) return false;
      }

      // Type filter
      if (filterType !== 'all' && artifact.artifact_type !== filterType) {
        return false;
      }

      // Phase filter
      if (filterPhase !== 'all' && metadata?.phase !== filterPhase) {
        return false;
      }

      return true;
    });
  }, [artifacts, metadataById, searchQuery, filterType, filterPhase]);

  // Group artifacts
  const groupedArtifacts = useMemo(() => {
    if (groupBy === 'none') {
      return { All: filteredArtifacts };
    }

    const groups: Record<string, ExecutionArtifact[]> = {};

    filteredArtifacts.forEach((artifact) => {
      const metadata = metadataById.get(artifact.id);
      let key: string;

      switch (groupBy) {
        case 'type':
          key = artifact.artifact_type;
          break;
        case 'phase':
          key = formatArtifactPhase(metadata?.phase) || 'No Phase';
          break;
        case 'date':
          key = new Date(artifact.created_at).toLocaleDateString();
          break;
        case 'agent':
          key =
            metadata?.created_by_agent_name ||
            metadata?.created_by_agent_id ||
            'Manual';
          break;
        default:
          key = 'All';
      }

      if (!groups[key]) {
        groups[key] = [];
      }
      groups[key].push(artifact);
    });

    return groups;
  }, [filteredArtifacts, groupBy, metadataById]);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader />
      </div>
    );
  }

  return (
    <div className={cn('flex flex-col h-full', className)}>
      {/* Toolbar */}
      <div className="flex items-center gap-4 p-4 border-b bg-background">
        {/* Search */}
        <div className="relative flex-1 max-w-md">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search artifacts..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9"
          />
        </div>

        {/* Type Filter */}
        <Select
          value={filterType}
          onValueChange={(value) => setFilterType(value as ArtifactType | 'all')}
        >
          <SelectTrigger className="w-40">
            <SelectValue placeholder="All Types" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Types</SelectItem>
            {uniqueTypes.map((type) => (
              <SelectItem key={type} value={type}>
                {type}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>

        {/* Phase Filter */}
        <Select
          value={filterPhase}
          onValueChange={(value) => setFilterPhase(value as ArtifactPhase | 'all')}
        >
          <SelectTrigger className="w-36">
            <SelectValue placeholder="All Phases" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Phases</SelectItem>
            <SelectItem value="planning">Planning</SelectItem>
            <SelectItem value="execution">Execution</SelectItem>
            <SelectItem value="verification">Verification</SelectItem>
          </SelectContent>
        </Select>

        {/* Group By */}
        <Select
          value={groupBy}
          onValueChange={(value) => setGroupBy(value as GroupBy)}
        >
          <SelectTrigger className="w-36">
            <FolderOpen className="h-4 w-4 mr-2" />
            <SelectValue placeholder="Group by" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="none">No Grouping</SelectItem>
            <SelectItem value="type">By Type</SelectItem>
            <SelectItem value="phase">By Phase</SelectItem>
            <SelectItem value="date">By Date</SelectItem>
            <SelectItem value="agent">By Agent</SelectItem>
          </SelectContent>
        </Select>

        {/* View Mode */}
        <Tabs value={viewMode} onValueChange={(v) => setViewMode(v as ViewMode)}>
          <TabsList>
            <TabsTrigger value="grid">
              <Grid className="h-4 w-4" />
            </TabsTrigger>
            <TabsTrigger value="list">
              <List className="h-4 w-4" />
            </TabsTrigger>
            <TabsTrigger value="timeline">
              <Calendar className="h-4 w-4" />
            </TabsTrigger>
          </TabsList>
        </Tabs>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto p-4">
        {Object.entries(groupedArtifacts).map(([group, groupArtifacts]) => (
          <div key={group} className="mb-6">
            {groupBy !== 'none' && (
              <h3 className="text-lg font-semibold mb-3 flex items-center gap-2">
                {group}
                <span className="text-sm font-normal text-muted-foreground">
                  ({groupArtifacts.length})
                </span>
              </h3>
            )}

            {viewMode === 'grid' && (
              <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-4">
                {groupArtifacts.map((artifact) => (
                  <ArtifactCard
                    key={artifact.id}
                    artifact={artifact}
                    isPinned={pinnedIds.has(artifact.id)}
                    onClick={() => onArtifactClick?.(artifact)}
                    onPin={() => onArtifactPin?.(artifact)}
                    onDownload={() => onArtifactDownload?.(artifact)}
                    onPreview={() => onArtifactPreview?.(artifact)}
                  />
                ))}
              </div>
            )}

            {viewMode === 'list' && (
              <div className="space-y-2">
                {groupArtifacts.map((artifact) => (
                  <div
                    key={artifact.id}
                    className="flex items-center gap-4 p-3 border rounded-lg hover:bg-muted/50 cursor-pointer"
                    onClick={() => onArtifactClick?.(artifact)}
                  >
                    <div className="w-16 h-16 bg-muted rounded flex items-center justify-center flex-shrink-0">
                      {artifact.file_path ? (
                        <img
                          src={`/api/files/${artifact.file_path}`}
                          alt=""
                          className="w-full h-full object-cover rounded"
                        />
                      ) : (
                        <FolderOpen className="h-6 w-6 text-muted-foreground" />
                      )}
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="font-medium truncate">
                        {artifact.title || artifact.artifact_type}
                      </div>
                      <div className="text-sm text-muted-foreground capitalize">
                        {artifact.artifact_type} •{' '}
                        {formatArtifactPhase(
                          metadataById.get(artifact.id)?.phase
                        )}
                      </div>
                    </div>
                    <div className="text-sm text-muted-foreground">
                      {new Date(artifact.created_at).toLocaleDateString()}
                    </div>
                  </div>
                ))}
              </div>
            )}

            {viewMode === 'timeline' && (
              <div className="relative pl-8 border-l-2 border-muted space-y-4">
                {groupArtifacts.map((artifact) => (
                  <div key={artifact.id} className="relative">
                    <div className="absolute -left-[25px] w-4 h-4 bg-primary rounded-full" />
                    <div className="text-xs text-muted-foreground mb-1">
                      {new Date(artifact.created_at).toLocaleString()}
                    </div>
                    <ArtifactCard
                      artifact={artifact}
                      isPinned={pinnedIds.has(artifact.id)}
                      onClick={() => onArtifactClick?.(artifact)}
                      onPin={() => onArtifactPin?.(artifact)}
                      onDownload={() => onArtifactDownload?.(artifact)}
                      onPreview={() => onArtifactPreview?.(artifact)}
                      className="max-w-md"
                    />
                  </div>
                ))}
              </div>
            )}
          </div>
        ))}

        {filteredArtifacts.length === 0 && (
          <div className="flex flex-col items-center justify-center h-64 text-muted-foreground">
            <FolderOpen className="h-12 w-12 mb-4" />
            <p>No artifacts found</p>
            {searchQuery && (
              <Button
                variant="link"
                className="mt-2"
                onClick={() => setSearchQuery('')}
              >
                Clear search
              </Button>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
