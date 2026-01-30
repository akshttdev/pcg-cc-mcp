import { useQuery } from '@tanstack/react-query';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import { Copy, ExternalLink, Check } from 'lucide-react';
import { useState } from 'react';

interface ArticlePreviewProps {
  workflowId: string;
  artifactId: string;
}

export function ArticlePreview({ workflowId, artifactId }: ArticlePreviewProps) {
  const [copied, setCopied] = useState(false);

  // For now, we'll fetch from a simpler approach
  // In production, you'd want a dedicated artifact detail endpoint
  const { data: artifactsData, isLoading } = useQuery({
    queryKey: ['workflow-artifacts', workflowId],
    queryFn: async () => {
      const response = await fetch(`/api/nora/workflows/${workflowId}/artifacts`);
      if (!response.ok) throw new Error('Failed to fetch artifacts');
      return response.json();
    },
  });

  const artifact = artifactsData?.artifacts?.find(
    (a: { id: string }) => a.id === artifactId
  );

  const handleCopy = async (text: string) => {
    await navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  if (isLoading) {
    return (
      <div className="space-y-4">
        <Skeleton className="h-8 w-2/3" />
        <Skeleton className="h-4 w-1/3" />
        <Skeleton className="h-64 w-full" />
      </div>
    );
  }

  if (!artifact) {
    return (
      <div className="text-center py-8 text-muted-foreground">
        Artifact not found
      </div>
    );
  }

  // Render based on artifact type
  if (artifact.artifactType === 'article') {
    // For articles, we'd ideally have the full content
    // For now, show what we have
    return (
      <div className="space-y-4">
        <div className="flex items-start justify-between">
          <div>
            <h3 className="text-lg font-semibold">{artifact.title}</h3>
            <Badge variant="secondary" className="mt-1 capitalize">
              {artifact.artifactType.replace('_', ' ')}
            </Badge>
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={() => handleCopy(artifact.title)}
          >
            {copied ? (
              <Check className="w-4 h-4 mr-2" />
            ) : (
              <Copy className="w-4 h-4 mr-2" />
            )}
            Copy
          </Button>
        </div>

        <div className="prose prose-sm max-w-none dark:prose-invert">
          <p className="text-muted-foreground italic">
            Article content preview. Full content available via download.
          </p>
          {artifact.fileUrl && (
            <div className="mt-4">
              <Button variant="outline" size="sm" asChild>
                <a href={artifact.fileUrl} target="_blank" rel="noopener noreferrer">
                  <ExternalLink className="w-4 h-4 mr-2" />
                  View Full Article
                </a>
              </Button>
            </div>
          )}
        </div>
      </div>
    );
  }

  if (artifact.artifactType === 'thumbnail' || artifact.artifactType === 'social_graphic') {
    return (
      <div className="space-y-4">
        <div className="flex items-start justify-between">
          <div>
            <h3 className="text-lg font-semibold">{artifact.title}</h3>
            <Badge variant="secondary" className="mt-1 capitalize">
              {artifact.artifactType.replace('_', ' ')}
            </Badge>
          </div>
          {artifact.fileUrl && (
            <Button variant="outline" size="sm" asChild>
              <a href={artifact.fileUrl} target="_blank" rel="noopener noreferrer">
                <ExternalLink className="w-4 h-4 mr-2" />
                Open
              </a>
            </Button>
          )}
        </div>

        {artifact.fileUrl ? (
          <div className="border rounded-lg overflow-hidden bg-muted">
            <img
              src={artifact.fileUrl}
              alt={artifact.title}
              className="w-full h-auto max-h-[400px] object-contain"
              onError={(e) => {
                const target = e.target as HTMLImageElement;
                target.style.display = 'none';
                const parent = target.parentElement;
                if (parent) {
                  parent.innerHTML = `
                    <div class="flex items-center justify-center h-48 text-muted-foreground">
                      <p>Image preview unavailable</p>
                    </div>
                  `;
                }
              }}
            />
          </div>
        ) : (
          <div className="border rounded-lg h-48 flex items-center justify-center bg-muted text-muted-foreground">
            No image URL available
          </div>
        )}
      </div>
    );
  }

  if (artifact.artifactType === 'social_post') {
    return (
      <div className="space-y-4">
        <div className="flex items-start justify-between">
          <div>
            <h3 className="text-lg font-semibold">{artifact.title}</h3>
            <Badge variant="secondary" className="mt-1 capitalize">
              {artifact.artifactType.replace('_', ' ')}
            </Badge>
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={() => handleCopy(artifact.title)}
          >
            {copied ? (
              <Check className="w-4 h-4 mr-2" />
            ) : (
              <Copy className="w-4 h-4 mr-2" />
            )}
            Copy
          </Button>
        </div>

        <div className="p-4 border rounded-lg bg-muted/50">
          <p className="whitespace-pre-wrap text-sm">
            Social caption content. Full content available via download.
          </p>
        </div>

        <p className="text-xs text-muted-foreground">
          Created: {new Date(artifact.createdAt).toLocaleString()}
        </p>
      </div>
    );
  }

  // Default fallback
  return (
    <div className="space-y-4">
      <div>
        <h3 className="text-lg font-semibold">{artifact.title}</h3>
        <Badge variant="secondary" className="mt-1 capitalize">
          {artifact.artifactType.replace('_', ' ')}
        </Badge>
      </div>
      <p className="text-sm text-muted-foreground">
        Preview not available for this artifact type.
      </p>
      {artifact.fileUrl && (
        <Button variant="outline" size="sm" asChild>
          <a href={artifact.fileUrl} target="_blank" rel="noopener noreferrer">
            <ExternalLink className="w-4 h-4 mr-2" />
            Open
          </a>
        </Button>
      )}
    </div>
  );
}
