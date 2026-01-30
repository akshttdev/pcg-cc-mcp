import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';
import {
  FileText,
  Image,
  Share2,
  Download,
  RefreshCw,
  Calendar,
  MapPin,
  Users,
  Building2,
  PartyPopper,
  ChevronRight,
  CheckCircle2,
  Clock,
  AlertCircle,
  Loader2,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { ArticlePreview } from './ArticlePreview';

interface WorkflowListItem {
  id: string;
  conferenceName: string;
  status: string;
  startDate: string;
  endDate: string;
  location: string | null;
  createdAt: string;
}

interface WorkflowStatus {
  workflowId: string;
  conferenceName: string;
  status: string;
  currentStage: string | null;
  startDate: string;
  endDate: string;
  location: string | null;
  speakersDiscovered: number | null;
  sponsorsDiscovered: number | null;
  sideEventsDiscovered: number | null;
  postsScheduled: number;
  qaScore: number | null;
  errorLog: string | null;
  createdAt: string;
  completedAt: string | null;
}

interface WorkflowArtifact {
  id: string;
  artifactType: string;
  title: string;
  fileUrl: string | null;
  createdAt: string;
}

interface WorkflowArtifactsResponse {
  workflowId: string;
  conferenceName: string;
  artifacts: WorkflowArtifact[];
  counts: Record<string, number>;
}

// API functions
async function fetchWorkflows(): Promise<WorkflowListItem[]> {
  const response = await fetch('/api/nora/workflows');
  if (!response.ok) throw new Error('Failed to fetch workflows');
  return response.json();
}

async function fetchWorkflowStatus(workflowId: string): Promise<WorkflowStatus> {
  const response = await fetch(`/api/nora/workflows/${workflowId}/status`);
  if (!response.ok) throw new Error('Failed to fetch workflow status');
  return response.json();
}

async function fetchWorkflowArtifacts(workflowId: string): Promise<WorkflowArtifactsResponse> {
  const response = await fetch(`/api/nora/workflows/${workflowId}/artifacts`);
  if (!response.ok) throw new Error('Failed to fetch artifacts');
  return response.json();
}

function getStatusBadge(status: string) {
  const normalized = status.toLowerCase();
  if (normalized.includes('completed') || normalized.includes('complete')) {
    return <Badge variant="default" className="bg-green-500"><CheckCircle2 className="w-3 h-3 mr-1" />Completed</Badge>;
  }
  if (normalized.includes('running') || normalized.includes('progress')) {
    return <Badge variant="default" className="bg-blue-500"><Loader2 className="w-3 h-3 mr-1 animate-spin" />Running</Badge>;
  }
  if (normalized.includes('failed') || normalized.includes('error')) {
    return <Badge variant="destructive"><AlertCircle className="w-3 h-3 mr-1" />Failed</Badge>;
  }
  if (normalized.includes('paused')) {
    return <Badge variant="secondary"><Clock className="w-3 h-3 mr-1" />Paused</Badge>;
  }
  return <Badge variant="outline">{status}</Badge>;
}

function WorkflowCard({
  workflow,
  isSelected,
  onClick,
}: {
  workflow: WorkflowListItem;
  isSelected: boolean;
  onClick: () => void;
}) {
  return (
    <Card
      className={cn(
        'cursor-pointer transition-all hover:shadow-md',
        isSelected && 'ring-2 ring-primary'
      )}
      onClick={onClick}
    >
      <CardHeader className="pb-2">
        <div className="flex items-start justify-between">
          <div>
            <CardTitle className="text-base">{workflow.conferenceName}</CardTitle>
            <CardDescription className="flex items-center gap-2 mt-1">
              <Calendar className="w-3 h-3" />
              {workflow.startDate} - {workflow.endDate}
            </CardDescription>
          </div>
          <ChevronRight className={cn('w-5 h-5 transition-transform', isSelected && 'rotate-90')} />
        </div>
      </CardHeader>
      <CardContent className="pt-0">
        <div className="flex items-center justify-between">
          {getStatusBadge(workflow.status)}
          {workflow.location && (
            <span className="text-xs text-muted-foreground flex items-center gap-1">
              <MapPin className="w-3 h-3" />
              {workflow.location}
            </span>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

function ArtifactsList({
  artifacts,
  selectedId,
  onSelect,
}: {
  artifacts: WorkflowArtifact[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}) {
  const getIcon = (type: string) => {
    switch (type) {
      case 'article':
        return <FileText className="w-4 h-4" />;
      case 'thumbnail':
      case 'social_graphic':
        return <Image className="w-4 h-4" />;
      case 'social_post':
        return <Share2 className="w-4 h-4" />;
      default:
        return <FileText className="w-4 h-4" />;
    }
  };

  return (
    <div className="space-y-2">
      {artifacts.map((artifact) => (
        <div
          key={artifact.id}
          className={cn(
            'flex items-center gap-3 p-3 rounded-lg cursor-pointer transition-colors',
            selectedId === artifact.id
              ? 'bg-primary/10 border border-primary/20'
              : 'hover:bg-muted'
          )}
          onClick={() => onSelect(artifact.id)}
        >
          {getIcon(artifact.artifactType)}
          <div className="flex-1 min-w-0">
            <p className="font-medium truncate">{artifact.title}</p>
            <p className="text-xs text-muted-foreground capitalize">
              {artifact.artifactType.replace('_', ' ')}
            </p>
          </div>
        </div>
      ))}
      {artifacts.length === 0 && (
        <p className="text-sm text-muted-foreground text-center py-4">
          No artifacts yet
        </p>
      )}
    </div>
  );
}

function WorkflowDetails({
  workflowId,
  onDownload,
}: {
  workflowId: string;
  onDownload: () => void;
}) {
  const [selectedArtifactId, setSelectedArtifactId] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState('articles');

  const { data: status, isLoading: statusLoading } = useQuery({
    queryKey: ['workflow-status', workflowId],
    queryFn: () => fetchWorkflowStatus(workflowId),
  });

  // Separate query for polling when workflow is running
  useQuery({
    queryKey: ['workflow-status-poll', workflowId],
    queryFn: () => fetchWorkflowStatus(workflowId),
    refetchInterval: status?.status.toLowerCase().includes('running') ? 5000 : false,
    enabled: !!status?.status.toLowerCase().includes('running'),
  });

  const { data: artifactsData } = useQuery({
    queryKey: ['workflow-artifacts', workflowId],
    queryFn: () => fetchWorkflowArtifacts(workflowId),
  });

  if (statusLoading) {
    return (
      <div className="space-y-4">
        <Skeleton className="h-8 w-48" />
        <Skeleton className="h-32 w-full" />
        <Skeleton className="h-64 w-full" />
      </div>
    );
  }

  if (!status) {
    return <p className="text-muted-foreground">Failed to load workflow details</p>;
  }

  const articles = artifactsData?.artifacts.filter((a) => a.artifactType === 'article') || [];
  const graphics = artifactsData?.artifacts.filter(
    (a) => a.artifactType === 'thumbnail' || a.artifactType === 'social_graphic'
  ) || [];
  const social = artifactsData?.artifacts.filter((a) => a.artifactType === 'social_post') || [];

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-start justify-between">
        <div>
          <h2 className="text-2xl font-bold">{status.conferenceName}</h2>
          <div className="flex items-center gap-4 mt-2 text-sm text-muted-foreground">
            <span className="flex items-center gap-1">
              <Calendar className="w-4 h-4" />
              {status.startDate} - {status.endDate}
            </span>
            {status.location && (
              <span className="flex items-center gap-1">
                <MapPin className="w-4 h-4" />
                {status.location}
              </span>
            )}
          </div>
        </div>
        <div className="flex items-center gap-2">
          {getStatusBadge(status.status)}
          <Button variant="outline" size="sm" onClick={onDownload}>
            <Download className="w-4 h-4 mr-2" />
            Download All
          </Button>
        </div>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-4 gap-4">
        <Card>
          <CardContent className="pt-4">
            <div className="flex items-center gap-2">
              <Users className="w-5 h-5 text-blue-500" />
              <div>
                <p className="text-2xl font-bold">{status.speakersDiscovered ?? 0}</p>
                <p className="text-xs text-muted-foreground">Speakers</p>
              </div>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-4">
            <div className="flex items-center gap-2">
              <Building2 className="w-5 h-5 text-purple-500" />
              <div>
                <p className="text-2xl font-bold">{status.sponsorsDiscovered ?? 0}</p>
                <p className="text-xs text-muted-foreground">Sponsors</p>
              </div>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-4">
            <div className="flex items-center gap-2">
              <PartyPopper className="w-5 h-5 text-amber-500" />
              <div>
                <p className="text-2xl font-bold">{status.sideEventsDiscovered ?? 0}</p>
                <p className="text-xs text-muted-foreground">Side Events</p>
              </div>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-4">
            <div className="flex items-center gap-2">
              <Share2 className="w-5 h-5 text-green-500" />
              <div>
                <p className="text-2xl font-bold">{status.postsScheduled}</p>
                <p className="text-xs text-muted-foreground">Posts Scheduled</p>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Current Stage */}
      {status.currentStage && (
        <Card>
          <CardContent className="pt-4">
            <div className="flex items-center gap-2">
              <RefreshCw className="w-4 h-4 animate-spin text-blue-500" />
              <span className="text-sm">
                Current Stage: <strong className="capitalize">{status.currentStage.replace('_', ' ')}</strong>
              </span>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Error Log */}
      {status.errorLog && (
        <Card className="border-destructive">
          <CardContent className="pt-4">
            <div className="flex items-start gap-2">
              <AlertCircle className="w-4 h-4 text-destructive mt-0.5" />
              <div>
                <p className="font-medium text-destructive">Error</p>
                <p className="text-sm text-muted-foreground">{status.errorLog}</p>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Artifacts Tabs */}
      <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
        <TabsList className="w-full justify-start">
          <TabsTrigger value="articles" className="flex items-center gap-2">
            <FileText className="w-4 h-4" />
            Articles ({articles.length})
          </TabsTrigger>
          <TabsTrigger value="graphics" className="flex items-center gap-2">
            <Image className="w-4 h-4" />
            Graphics ({graphics.length})
          </TabsTrigger>
          <TabsTrigger value="social" className="flex items-center gap-2">
            <Share2 className="w-4 h-4" />
            Social Posts ({social.length})
          </TabsTrigger>
        </TabsList>

        <div className="mt-4 grid grid-cols-3 gap-4">
          {/* Artifact List */}
          <Card className="col-span-1">
            <CardHeader className="pb-2">
              <CardTitle className="text-sm">
                {activeTab === 'articles' && 'Generated Articles'}
                {activeTab === 'graphics' && 'Generated Graphics'}
                {activeTab === 'social' && 'Social Captions'}
              </CardTitle>
            </CardHeader>
            <CardContent>
              <ScrollArea className="h-[400px]">
                <TabsContent value="articles" className="mt-0">
                  <ArtifactsList
                    artifacts={articles}
                    selectedId={selectedArtifactId}
                    onSelect={setSelectedArtifactId}
                  />
                </TabsContent>
                <TabsContent value="graphics" className="mt-0">
                  <ArtifactsList
                    artifacts={graphics}
                    selectedId={selectedArtifactId}
                    onSelect={setSelectedArtifactId}
                  />
                </TabsContent>
                <TabsContent value="social" className="mt-0">
                  <ArtifactsList
                    artifacts={social}
                    selectedId={selectedArtifactId}
                    onSelect={setSelectedArtifactId}
                  />
                </TabsContent>
              </ScrollArea>
            </CardContent>
          </Card>

          {/* Preview Panel */}
          <Card className="col-span-2">
            <CardHeader className="pb-2">
              <CardTitle className="text-sm">Preview</CardTitle>
            </CardHeader>
            <CardContent>
              <ScrollArea className="h-[400px]">
                {selectedArtifactId ? (
                  <ArticlePreview
                    workflowId={workflowId}
                    artifactId={selectedArtifactId}
                  />
                ) : (
                  <div className="h-full flex items-center justify-center text-muted-foreground">
                    Select an artifact to preview
                  </div>
                )}
              </ScrollArea>
            </CardContent>
          </Card>
        </div>
      </Tabs>
    </div>
  );
}

export function ConferenceWorkflowDashboard() {
  const [selectedWorkflowId, setSelectedWorkflowId] = useState<string | null>(null);

  const { data: workflows, isLoading, refetch } = useQuery({
    queryKey: ['workflows'],
    queryFn: fetchWorkflows,
  });

  const handleDownload = () => {
    if (selectedWorkflowId) {
      window.open(`/api/nora/workflows/${selectedWorkflowId}/artifacts/download`, '_blank');
    }
  };

  return (
    <div className="h-full flex gap-4 p-4">
      {/* Workflows List */}
      <div className="w-80 flex-shrink-0">
        <Card className="h-full">
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle>Conference Workflows</CardTitle>
              <Button variant="ghost" size="icon" onClick={() => refetch()}>
                <RefreshCw className="w-4 h-4" />
              </Button>
            </div>
          </CardHeader>
          <CardContent>
            <ScrollArea className="h-[calc(100vh-200px)]">
              <div className="space-y-3">
                {isLoading ? (
                  <>
                    <Skeleton className="h-24 w-full" />
                    <Skeleton className="h-24 w-full" />
                    <Skeleton className="h-24 w-full" />
                  </>
                ) : workflows && workflows.length > 0 ? (
                  workflows.map((workflow) => (
                    <WorkflowCard
                      key={workflow.id}
                      workflow={workflow}
                      isSelected={selectedWorkflowId === workflow.id}
                      onClick={() => setSelectedWorkflowId(workflow.id)}
                    />
                  ))
                ) : (
                  <p className="text-sm text-muted-foreground text-center py-8">
                    No conference workflows yet.
                    <br />
                    Create one via the Nora intake form.
                  </p>
                )}
              </div>
            </ScrollArea>
          </CardContent>
        </Card>
      </div>

      {/* Details Panel */}
      <div className="flex-1 min-w-0">
        {selectedWorkflowId ? (
          <WorkflowDetails
            workflowId={selectedWorkflowId}
            onDownload={handleDownload}
          />
        ) : (
          <Card className="h-full flex items-center justify-center">
            <div className="text-center text-muted-foreground">
              <FileText className="w-12 h-12 mx-auto mb-4 opacity-50" />
              <p>Select a workflow to view details and artifacts</p>
            </div>
          </Card>
        )}
      </div>
    </div>
  );
}
