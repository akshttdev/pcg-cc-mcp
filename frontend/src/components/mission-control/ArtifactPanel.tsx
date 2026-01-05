import { cn } from '@/lib/utils';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  FileText,
  Camera,
  GitCompare,
  TestTube,
  AlertCircle,
  CheckSquare,
  BookOpen,
} from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import type { ExecutionArtifact } from '@/hooks/useMissionControl';

interface ArtifactPanelProps {
  artifacts: ExecutionArtifact[];
  className?: string;
}

function getArtifactIcon(type: ExecutionArtifact['artifact_type']) {
  switch (type) {
    case 'plan':
      return <FileText className="h-4 w-4" />;
    case 'screenshot':
      return <Camera className="h-4 w-4" />;
    case 'diff_summary':
      return <GitCompare className="h-4 w-4" />;
    case 'test_result':
      return <TestTube className="h-4 w-4" />;
    case 'error_report':
      return <AlertCircle className="h-4 w-4" />;
    case 'checkpoint':
      return <CheckSquare className="h-4 w-4" />;
    case 'walkthrough':
      return <BookOpen className="h-4 w-4" />;
    default:
      return <FileText className="h-4 w-4" />;
  }
}

function getArtifactBadge(type: ExecutionArtifact['artifact_type']) {
  switch (type) {
    case 'plan':
      return <Badge variant="secondary">Plan</Badge>;
    case 'screenshot':
      return <Badge variant="outline">Screenshot</Badge>;
    case 'diff_summary':
      return <Badge className="bg-purple-500">Diff</Badge>;
    case 'test_result':
      return <Badge className="bg-green-500">Test</Badge>;
    case 'error_report':
      return <Badge variant="destructive">Error</Badge>;
    case 'checkpoint':
      return <Badge className="bg-blue-500">Checkpoint</Badge>;
    case 'walkthrough':
      return <Badge variant="secondary">Walkthrough</Badge>;
    default:
      return <Badge variant="outline">{type}</Badge>;
  }
}

function ArtifactItem({ artifact }: { artifact: ExecutionArtifact }) {
  const createdAt = new Date(artifact.created_at);

  return (
    <div className="p-3 border rounded-lg hover:bg-muted/50 transition-colors cursor-pointer">
      <div className="flex items-start gap-3">
        <div className="rounded-full bg-muted p-2">
          {getArtifactIcon(artifact.artifact_type)}
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className="font-medium text-sm truncate">{artifact.title}</span>
            {getArtifactBadge(artifact.artifact_type)}
          </div>
          <p className="text-xs text-muted-foreground mt-1">
            {formatDistanceToNow(createdAt, { addSuffix: true })}
          </p>
          {artifact.content && (
            <p className="text-xs text-muted-foreground mt-2 line-clamp-2">
              {artifact.content.slice(0, 200)}
              {artifact.content.length > 200 && '...'}
            </p>
          )}
        </div>
      </div>
    </div>
  );
}

function ArtifactsByType({
  artifacts,
  type,
}: {
  artifacts: ExecutionArtifact[];
  type: ExecutionArtifact['artifact_type'];
}) {
  const filtered = artifacts.filter((a) => a.artifact_type === type);

  if (filtered.length === 0) {
    return (
      <div className="text-center text-muted-foreground py-8 text-sm">
        No {type.replace('_', ' ')}s yet
      </div>
    );
  }

  return (
    <div className="space-y-2">
      {filtered.map((artifact) => (
        <ArtifactItem key={artifact.id} artifact={artifact} />
      ))}
    </div>
  );
}

export function ArtifactPanel({ artifacts, className }: ArtifactPanelProps) {
  const counts = {
    all: artifacts.length,
    plan: artifacts.filter((a) => a.artifact_type === 'plan').length,
    screenshot: artifacts.filter((a) => a.artifact_type === 'screenshot').length,
    test_result: artifacts.filter((a) => a.artifact_type === 'test_result').length,
    error_report: artifacts.filter((a) => a.artifact_type === 'error_report').length,
  };

  return (
    <Card className={cn('h-full flex flex-col', className)}>
      <CardHeader className="pb-2">
        <CardTitle className="text-sm font-medium flex items-center gap-2">
          <FileText className="h-4 w-4" />
          Artifacts
          {counts.all > 0 && (
            <Badge variant="secondary" className="ml-auto">
              {counts.all}
            </Badge>
          )}
        </CardTitle>
      </CardHeader>
      <CardContent className="flex-1 overflow-hidden p-0">
        {artifacts.length === 0 ? (
          <div className="text-center text-muted-foreground py-8 text-sm">
            No artifacts yet
          </div>
        ) : (
          <Tabs defaultValue="all" className="h-full flex flex-col">
            <TabsList className="w-full justify-start rounded-none border-b bg-transparent px-4">
              <TabsTrigger value="all" className="text-xs">
                All ({counts.all})
              </TabsTrigger>
              {counts.plan > 0 && (
                <TabsTrigger value="plan" className="text-xs">
                  Plans ({counts.plan})
                </TabsTrigger>
              )}
              {counts.screenshot > 0 && (
                <TabsTrigger value="screenshot" className="text-xs">
                  Screenshots ({counts.screenshot})
                </TabsTrigger>
              )}
              {counts.test_result > 0 && (
                <TabsTrigger value="test_result" className="text-xs">
                  Tests ({counts.test_result})
                </TabsTrigger>
              )}
              {counts.error_report > 0 && (
                <TabsTrigger value="error_report" className="text-xs">
                  Errors ({counts.error_report})
                </TabsTrigger>
              )}
            </TabsList>
            <ScrollArea className="flex-1 px-4 py-2">
              <TabsContent value="all" className="m-0 space-y-2">
                {artifacts.map((artifact) => (
                  <ArtifactItem key={artifact.id} artifact={artifact} />
                ))}
              </TabsContent>
              <TabsContent value="plan" className="m-0">
                <ArtifactsByType artifacts={artifacts} type="plan" />
              </TabsContent>
              <TabsContent value="screenshot" className="m-0">
                <ArtifactsByType artifacts={artifacts} type="screenshot" />
              </TabsContent>
              <TabsContent value="test_result" className="m-0">
                <ArtifactsByType artifacts={artifacts} type="test_result" />
              </TabsContent>
              <TabsContent value="error_report" className="m-0">
                <ArtifactsByType artifacts={artifacts} type="error_report" />
              </TabsContent>
            </ScrollArea>
          </Tabs>
        )}
      </CardContent>
    </Card>
  );
}
