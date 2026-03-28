import { Skeleton } from '@/components/ui/skeleton';
import { Alert, AlertDescription } from '@/components/ui/alert';
import type { ExecutionArtifact } from 'shared/types';
import { ArtifactGallery } from '../../ArtifactGallery';

interface ArtifactsTabProps {
  artifacts: ExecutionArtifact[];
  artifactsLoading: boolean;
  artifactsError: string | null;
  onArtifactDownload: (artifact: ExecutionArtifact) => void;
  onArtifactUploadComplete?: () => void;
}

export function ArtifactsTab({
  artifacts,
  artifactsLoading,
  artifactsError,
  onArtifactDownload,
  onArtifactUploadComplete,
}: ArtifactsTabProps) {
  if (artifactsLoading) {
    return (
      <div className="p-4 space-y-4">
        <Skeleton className="h-10 w-full" />
        <div className="grid grid-cols-3 gap-4">
          <Skeleton className="h-32" />
          <Skeleton className="h-32" />
          <Skeleton className="h-32" />
          <Skeleton className="h-32" />
          <Skeleton className="h-32" />
          <Skeleton className="h-32" />
        </div>
      </div>
    );
  }

  if (artifactsError) {
    return (
      <div className="p-4">
        <Alert variant="destructive">
          <AlertDescription>{artifactsError}</AlertDescription>
        </Alert>
      </div>
    );
  }

  return (
    <ArtifactGallery
      artifacts={artifacts}
      className="h-full"
      onDownload={onArtifactDownload}
      onArtifactUploadComplete={onArtifactUploadComplete}
      onUpload={async () => {
        // TODO: Implement file upload
      }}
      onLinkAdd={async () => {
        // TODO: Implement link addition
      }}
    />
  );
}
