import { useState, useCallback } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  FileText,
  Image,
  Video,
  Upload,
  Link2,
  Download,
  Eye,
  Pin,
  FolderOpen,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { ExecutionArtifact, ArtifactType, ArtifactPhase } from 'shared/types';

interface TaskArtifactsPanelProps {
  taskId: string;
  artifacts: ExecutionArtifact[];
  pinnedArtifacts?: string[];
  onPin?: (artifactId: string) => void;
  onUnpin?: (artifactId: string) => void;
  onDownload?: (artifact: ExecutionArtifact) => void;
  onPreview?: (artifact: ExecutionArtifact) => void;
  onUpload?: (file: File) => Promise<void>;
  onLinkAdd?: (url: string, name: string) => Promise<void>;
  className?: string;
}

const phaseConfig: Record<ArtifactPhase, { label: string; color: string }> = {
  planning: { label: 'Planning', color: 'bg-purple-100 text-purple-700' },
  execution: { label: 'Execution', color: 'bg-yellow-100 text-yellow-700' },
  verification: { label: 'Verification', color: 'bg-green-100 text-green-700' },
};

const artifactTypeIcons: Partial<Record<ArtifactType, React.ReactNode>> = {
  plan: <FileText className="h-4 w-4" />,
  screenshot: <Image className="h-4 w-4" />,
  walkthrough: <Video className="h-4 w-4" />,
  diff_summary: <FileText className="h-4 w-4" />,
  test_result: <FileText className="h-4 w-4" />,
  research_report: <FileText className="h-4 w-4 text-blue-500" />,
  strategy_document: <FileText className="h-4 w-4 text-purple-500" />,
  content_draft: <FileText className="h-4 w-4 text-yellow-500" />,
  visual_brief: <Image className="h-4 w-4 text-pink-500" />,
};

function ArtifactCard({
  artifact,
  isPinned,
  onPin,
  onDownload,
  onPreview,
}: {
  artifact: ExecutionArtifact;
  isPinned?: boolean;
  onPin?: () => void;
  onDownload?: () => void;
  onPreview?: () => void;
}) {
  const metadata = artifact.metadata ? JSON.parse(artifact.metadata) : {};
  const phase = metadata.phase as ArtifactPhase | undefined;

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  const isPreviewable = ['screenshot', 'visual_brief', 'walkthrough', 'browser_recording'].includes(
    artifact.artifact_type
  );

  return (
    <Card className={cn('transition-all hover:shadow-md', isPinned && 'ring-2 ring-primary')}>
      <CardContent className="p-3">
        <div className="flex items-start gap-3">
          {/* Icon */}
          <div className="p-2 rounded-lg bg-muted">
            {artifactTypeIcons[artifact.artifact_type] || <FileText className="h-4 w-4" />}
          </div>

          {/* Content */}
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-1">
              <h4 className="font-medium text-sm truncate">
                {artifact.title || artifact.artifact_type}
              </h4>
              {isPinned && <Pin className="h-3 w-3 text-primary" />}
            </div>

            <div className="flex items-center gap-2 mb-2">
              <Badge variant="outline" className="text-xs">
                {artifact.artifact_type}
              </Badge>
              {phase && (
                <Badge variant="outline" className={cn('text-xs', phaseConfig[phase].color)}>
                  {phaseConfig[phase].label}
                </Badge>
              )}
            </div>

            <p className="text-xs text-muted-foreground">
              {formatDate(artifact.created_at)}
            </p>
          </div>

          {/* Actions */}
          <div className="flex items-center gap-1">
            {isPreviewable && onPreview && (
              <Button variant="ghost" size="icon" onClick={onPreview} className="h-7 w-7">
                <Eye className="h-3.5 w-3.5" />
              </Button>
            )}
            {onDownload && artifact.file_path && (
              <Button variant="ghost" size="icon" onClick={onDownload} className="h-7 w-7">
                <Download className="h-3.5 w-3.5" />
              </Button>
            )}
            {onPin && (
              <Button
                variant="ghost"
                size="icon"
                onClick={onPin}
                className={cn('h-7 w-7', isPinned && 'text-primary')}
              >
                <Pin className="h-3.5 w-3.5" />
              </Button>
            )}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

function UploadSection({
  onUpload,
  onLinkAdd,
}: {
  onUpload?: (file: File) => Promise<void>;
  onLinkAdd?: (url: string, name: string) => Promise<void>;
}) {
  const [isUploading, setIsUploading] = useState(false);
  const [linkUrl, setLinkUrl] = useState('');
  const [linkName, setLinkName] = useState('');
  const [showLinkForm, setShowLinkForm] = useState(false);

  const handleFileChange = useCallback(
    async (e: React.ChangeEvent<HTMLInputElement>) => {
      const file = e.target.files?.[0];
      if (!file || !onUpload) return;

      setIsUploading(true);
      try {
        await onUpload(file);
      } finally {
        setIsUploading(false);
        e.target.value = '';
      }
    },
    [onUpload]
  );

  const handleLinkSubmit = useCallback(async () => {
    if (!linkUrl || !onLinkAdd) return;

    await onLinkAdd(linkUrl, linkName || 'External Link');
    setLinkUrl('');
    setLinkName('');
    setShowLinkForm(false);
  }, [linkUrl, linkName, onLinkAdd]);

  return (
    <div className="space-y-3">
      <div className="flex items-center gap-2">
        {onUpload && (
          <Button variant="outline" size="sm" className="relative" disabled={isUploading}>
            <Upload className="h-4 w-4 mr-1" />
            {isUploading ? 'Uploading...' : 'Upload File'}
            <input
              type="file"
              className="absolute inset-0 opacity-0 cursor-pointer"
              onChange={handleFileChange}
              disabled={isUploading}
            />
          </Button>
        )}
        {onLinkAdd && (
          <Button
            variant="outline"
            size="sm"
            onClick={() => setShowLinkForm(!showLinkForm)}
          >
            <Link2 className="h-4 w-4 mr-1" />
            Add Link
          </Button>
        )}
      </div>

      {showLinkForm && (
        <Card>
          <CardContent className="p-3 space-y-3">
            <div className="space-y-1">
              <Label className="text-xs">URL</Label>
              <Input
                value={linkUrl}
                onChange={(e) => setLinkUrl(e.target.value)}
                placeholder="https://dropbox.com/... or any URL"
                className="h-8"
              />
            </div>
            <div className="space-y-1">
              <Label className="text-xs">Name (optional)</Label>
              <Input
                value={linkName}
                onChange={(e) => setLinkName(e.target.value)}
                placeholder="Link name"
                className="h-8"
              />
            </div>
            <div className="flex gap-2">
              <Button size="sm" onClick={handleLinkSubmit} disabled={!linkUrl}>
                Add
              </Button>
              <Button size="sm" variant="outline" onClick={() => setShowLinkForm(false)}>
                Cancel
              </Button>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}

export function TaskArtifactsPanel({
  taskId,
  artifacts,
  pinnedArtifacts = [],
  onPin,
  onUnpin,
  onDownload,
  onPreview,
  onUpload,
  onLinkAdd,
  className,
}: TaskArtifactsPanelProps) {
  // Group artifacts by phase
  const groupedArtifacts = {
    planning: artifacts.filter((a) => {
      const metadata = a.metadata ? JSON.parse(a.metadata) : {};
      return metadata.phase === 'planning';
    }),
    execution: artifacts.filter((a) => {
      const metadata = a.metadata ? JSON.parse(a.metadata) : {};
      return metadata.phase === 'execution';
    }),
    verification: artifacts.filter((a) => {
      const metadata = a.metadata ? JSON.parse(a.metadata) : {};
      return metadata.phase === 'verification';
    }),
    user: artifacts.filter((a) => {
      const metadata = a.metadata ? JSON.parse(a.metadata) : {};
      return metadata.uploaded_by === 'user';
    }),
  };

  const pinned = artifacts.filter((a) => pinnedArtifacts.includes(a.id));

  return (
    <Card className={className} data-task-id={taskId}>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <CardTitle className="text-lg flex items-center gap-2">
            <FolderOpen className="h-5 w-5 text-muted-foreground" />
            Artifacts
          </CardTitle>
          <Badge variant="secondary">{artifacts.length} total</Badge>
        </div>
      </CardHeader>

      <CardContent className="space-y-4">
        {/* Upload section */}
        {(onUpload || onLinkAdd) && (
          <UploadSection onUpload={onUpload} onLinkAdd={onLinkAdd} />
        )}

        {/* Pinned artifacts */}
        {pinned.length > 0 && (
          <div className="space-y-2">
            <h4 className="text-sm font-medium flex items-center gap-1">
              <Pin className="h-3 w-3" />
              Pinned
            </h4>
            <div className="grid gap-2">
              {pinned.map((artifact) => (
                <ArtifactCard
                  key={artifact.id}
                  artifact={artifact}
                  isPinned
                  onPin={onUnpin ? () => onUnpin(artifact.id) : undefined}
                  onDownload={onDownload ? () => onDownload(artifact) : undefined}
                  onPreview={onPreview ? () => onPreview(artifact) : undefined}
                />
              ))}
            </div>
          </div>
        )}

        {/* Tabs for phases */}
        <Tabs defaultValue="all" className="w-full">
          <TabsList className="w-full grid grid-cols-4">
            <TabsTrigger value="all">All</TabsTrigger>
            <TabsTrigger value="planning">Planning</TabsTrigger>
            <TabsTrigger value="execution">Execution</TabsTrigger>
            <TabsTrigger value="user">Uploads</TabsTrigger>
          </TabsList>

          <TabsContent value="all" className="mt-3 space-y-2">
            {artifacts.length === 0 ? (
              <div className="text-center py-8 text-muted-foreground">
                <FolderOpen className="h-12 w-12 mx-auto mb-2 opacity-50" />
                <p>No artifacts yet</p>
              </div>
            ) : (
              artifacts.map((artifact) => (
                <ArtifactCard
                  key={artifact.id}
                  artifact={artifact}
                  isPinned={pinnedArtifacts.includes(artifact.id)}
                  onPin={
                    pinnedArtifacts.includes(artifact.id)
                      ? onUnpin
                        ? () => onUnpin(artifact.id)
                        : undefined
                      : onPin
                        ? () => onPin(artifact.id)
                        : undefined
                  }
                  onDownload={onDownload ? () => onDownload(artifact) : undefined}
                  onPreview={onPreview ? () => onPreview(artifact) : undefined}
                />
              ))
            )}
          </TabsContent>

          <TabsContent value="planning" className="mt-3 space-y-2">
            {groupedArtifacts.planning.map((artifact) => (
              <ArtifactCard
                key={artifact.id}
                artifact={artifact}
                isPinned={pinnedArtifacts.includes(artifact.id)}
                onPin={onPin ? () => onPin(artifact.id) : undefined}
                onDownload={onDownload ? () => onDownload(artifact) : undefined}
                onPreview={onPreview ? () => onPreview(artifact) : undefined}
              />
            ))}
            {groupedArtifacts.planning.length === 0 && (
              <p className="text-center py-4 text-muted-foreground text-sm">
                No planning artifacts
              </p>
            )}
          </TabsContent>

          <TabsContent value="execution" className="mt-3 space-y-2">
            {groupedArtifacts.execution.map((artifact) => (
              <ArtifactCard
                key={artifact.id}
                artifact={artifact}
                isPinned={pinnedArtifacts.includes(artifact.id)}
                onPin={onPin ? () => onPin(artifact.id) : undefined}
                onDownload={onDownload ? () => onDownload(artifact) : undefined}
                onPreview={onPreview ? () => onPreview(artifact) : undefined}
              />
            ))}
            {groupedArtifacts.execution.length === 0 && (
              <p className="text-center py-4 text-muted-foreground text-sm">
                No execution artifacts
              </p>
            )}
          </TabsContent>

          <TabsContent value="user" className="mt-3 space-y-2">
            {groupedArtifacts.user.map((artifact) => (
              <ArtifactCard
                key={artifact.id}
                artifact={artifact}
                isPinned={pinnedArtifacts.includes(artifact.id)}
                onPin={onPin ? () => onPin(artifact.id) : undefined}
                onDownload={onDownload ? () => onDownload(artifact) : undefined}
                onPreview={onPreview ? () => onPreview(artifact) : undefined}
              />
            ))}
            {groupedArtifacts.user.length === 0 && (
              <p className="text-center py-4 text-muted-foreground text-sm">
                No user uploads yet
              </p>
            )}
          </TabsContent>
        </Tabs>
      </CardContent>
    </Card>
  );
}
