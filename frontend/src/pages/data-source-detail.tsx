import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  ArrowLeft,
  Database,
  FileText,
  Upload,
  Play,
  Loader2,
  CheckCircle2,
  Circle,
  ChevronDown,
  ChevronRight,
  Clock,

  History,
  ExternalLink,
} from 'lucide-react';
import { dataSourcesApi, workflowsApi, DATA_TYPE_OPTIONS, SOURCE_TYPE_OPTIONS } from '@/lib/api';
import { StagingReviewPanel } from '@/components/workflows/StagingReviewPanel';
import { RunAndReviewPanel } from '@/components/workflows/RunAndReviewPanel';

interface WorkflowRunStep {
  name: string;
  status: string;
  output?: string;
  error?: string;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  result?: any;
}

interface WorkflowRunResult {
  workflow_run_id: string;
  staged_records: number;
  workflow_name?: string;
  steps?: WorkflowRunStep[];
  reused?: boolean;
}
import { WorkflowRunsPanel } from '@/components/workflows/WorkflowRunsPanel';
import { formatDate } from '@/lib/formatters';

export function DataSourceDetailPage() {
  const { orgId, dataSourceId } = useParams<{ orgId: string; dataSourceId: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const [expandedSteps, setExpandedSteps] = useState<Set<number>>(new Set());
  const [expandedArtifacts, setExpandedArtifacts] = useState<Set<string>>(new Set());
  const [workflowResult, setWorkflowResult] = useState<WorkflowRunResult | null>(null);
  const [selectedWorkflowId, setSelectedWorkflowId] = useState<string>('');
  const [selectedModel, setSelectedModel] = useState<string>('');
  const [reviewRunId, setReviewRunId] = useState<string | null>(null);
  const [showRunHistory, setShowRunHistory] = useState(false);

  const { data: source, isLoading } = useQuery({
    queryKey: ['dataSource', dataSourceId],
    queryFn: () => dataSourcesApi.get(dataSourceId!),
    enabled: !!dataSourceId,
  });

  const { data: workflows } = useQuery({
    queryKey: ['dataSourceWorkflows', dataSourceId],
    queryFn: () => dataSourcesApi.getWorkflows(dataSourceId!),
    enabled: !!dataSourceId,
  });

  const { data: artifacts = [] } = useQuery({
    queryKey: ['dataSourceArtifacts', dataSourceId],
    queryFn: () => dataSourcesApi.getArtifacts(dataSourceId!),
    enabled: !!dataSourceId,
  });

  const { data: availableModels } = useQuery({
    queryKey: ['workflowModels'],
    queryFn: () => workflowsApi.listAvailableModels(),
    staleTime: 60 * 60 * 1000,
  });


  const effectiveModel = selectedModel || availableModels?.find((m) => m.is_default)?.id || '';

  // Clear stale results when switching workflows
  useEffect(() => {
    setWorkflowResult(null);
    setReviewRunId(null);
  }, [selectedWorkflowId]);

  // Auto-select first workflow when loaded
  const effectiveWorkflowId = selectedWorkflowId || (Array.isArray(workflows) && workflows.length > 0 ? workflows[0].id : '');

  const runWorkflowMutation = useMutation({
    mutationFn: (opts?: { force?: boolean }) =>
      dataSourcesApi.runWorkflow(dataSourceId!, effectiveWorkflowId, effectiveModel || undefined, opts?.force),
    onSuccess: (data) => {
      setWorkflowResult(data);
      // Auto-open review panel if there are staged records
      if (data.workflow_run_id && data.staged_records > 0) {
        setReviewRunId(data.workflow_run_id);
      }
      queryClient.invalidateQueries({ queryKey: ['dataSourceArtifacts', dataSourceId] });
    },
  });

  const dataTypeLabel = (dt: string) =>
    DATA_TYPE_OPTIONS.find((o) => o.value === dt)?.label ?? dt;

  const sourceTypeLabel = (st: string) =>
    SOURCE_TYPE_OPTIONS.find((o) => o.value === st)?.label ?? st;

  const formatFileSize = (bytes?: number) => {
    if (!bytes) return '';
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const statusVariants: Record<string, string> = {
    ready: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400',
    pending: 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400',
    processing: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400',
    error: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
  };

  const toggleStep = (index: number) => {
    setExpandedSteps((prev) => {
      const next = new Set(prev);
      if (next.has(index)) next.delete(index);
      else next.add(index);
      return next;
    });
  };

  const toggleArtifact = (id: string) => {
    setExpandedArtifacts((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const parseMetadata = (metadataStr: string) => {
    try {
      return JSON.parse(metadataStr);
    } catch {
      return {};
    }
  };

  const renderJsonContent = (content: any) => {
    if (typeof content === 'string') {
      try {
        const parsed = JSON.parse(content);
        return renderStructuredData(parsed);
      } catch {
        return <p className="text-sm whitespace-pre-wrap">{content}</p>;
      }
    }
    return renderStructuredData(content);
  };

  const renderStructuredData = (data: any, depth = 0): JSX.Element => {
    if (data === null || data === undefined) {
      return <span className="text-muted-foreground italic">null</span>;
    }
    if (typeof data === 'string') {
      return <span className="text-sm">{data}</span>;
    }
    if (typeof data === 'number' || typeof data === 'boolean') {
      return <span className="text-sm font-mono">{String(data)}</span>;
    }
    if (Array.isArray(data)) {
      if (data.length === 0) return <span className="text-muted-foreground italic">empty list</span>;
      return (
        <div className={depth > 0 ? 'ml-4' : ''}>
          {data.map((item, i) => (
            <div key={i} className="py-1 border-b border-border/30 last:border-0">
              {typeof item === 'object' ? renderStructuredData(item, depth + 1) : (
                <span className="text-sm">{String(item)}</span>
              )}
            </div>
          ))}
        </div>
      );
    }
    if (typeof data === 'object') {
      return (
        <div className={depth > 0 ? 'ml-4 space-y-1' : 'space-y-2'}>
          {Object.entries(data).map(([key, value]) => (
            <div key={key}>
              <span className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                {key.replace(/_/g, ' ')}
              </span>
              <div className="mt-0.5">
                {typeof value === 'object' && value !== null
                  ? renderStructuredData(value, depth + 1)
                  : <p className="text-sm">{String(value ?? '')}</p>
                }
              </div>
            </div>
          ))}
        </div>
      );
    }
    return <span>{String(data)}</span>;
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (!source) {
    return (
      <div className="p-6 text-center text-muted-foreground">
        <p>Data source not found.</p>
        <Button
          variant="outline"
          size="sm"
          className="mt-4"
          onClick={() => navigate(`/organizations/${orgId}`)}
        >
          Back to Organization
        </Button>
      </div>
    );
  }

  const metadata = parseMetadata(source.metadata);

  return (
    <div className="p-6 max-w-5xl mx-auto space-y-6">
      {/* Header */}
      <div className="space-y-4">
        <Button
          variant="ghost"
          size="sm"
          className="gap-1.5 -ml-2"
          onClick={() =>
            navigate(`/organizations/${orgId}/intelligence/data-sources`)
          }
        >
          <ArrowLeft className="h-4 w-4" />
          Back
        </Button>

        <div className="flex items-start justify-between gap-4">
          <div className="space-y-2 min-w-0">
            <div className="flex items-center gap-2 flex-wrap">
              <h1 className="text-2xl font-bold tracking-tight">{source.title}</h1>
              <Badge variant="outline">{dataTypeLabel(source.data_type)}</Badge>
              <Badge variant="secondary">{sourceTypeLabel(source.source_type)}</Badge>
              <span
                className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${
                  statusVariants[source.status] ?? 'bg-muted text-muted-foreground'
                }`}
              >
                {source.status}
              </span>
            </div>
            {source.description && (
              <p className="text-muted-foreground">{source.description}</p>
            )}
            <div className="flex items-center gap-4 text-xs text-muted-foreground">
              <span>Created {formatDate(source.created_at)}</span>
              {source.source_type === 'file' && source.file_name && (
                <span className="flex items-center gap-1">
                  <Upload className="h-3 w-3" />
                  {source.file_name}
                  {source.file_size_bytes != null && ` (${formatFileSize(source.file_size_bytes)})`}
                </span>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Content Preview */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base flex items-center gap-2">
            {source.source_type === 'file' ? (
              <Upload className="h-4 w-4" />
            ) : source.source_type === 'text' ? (
              <FileText className="h-4 w-4" />
            ) : (
              <Database className="h-4 w-4" />
            )}
            Content
          </CardTitle>
        </CardHeader>
        <CardContent>
          {source.source_type === 'text' && source.content ? (
            <pre className="bg-muted/50 rounded-md p-4 text-sm font-mono whitespace-pre-wrap max-h-96 overflow-auto border">
              {source.content}
            </pre>
          ) : source.source_type === 'file' ? (
            <div className="space-y-2 text-sm">
              {source.file_name && (
                <div className="flex items-center gap-2">
                  <span className="text-muted-foreground">File:</span>
                  <span className="font-medium">{source.file_name}</span>
                </div>
              )}
              {source.file_size_bytes != null && (
                <div className="flex items-center gap-2">
                  <span className="text-muted-foreground">Size:</span>
                  <span>{formatFileSize(source.file_size_bytes)}</span>
                </div>
              )}
              {metadata.mime_type && (
                <div className="flex items-center gap-2">
                  <span className="text-muted-foreground">Type:</span>
                  <span>{metadata.mime_type}</span>
                </div>
              )}
              {source.file_hash && (
                <div className="flex items-center gap-2">
                  <span className="text-muted-foreground">Hash:</span>
                  <span className="font-mono text-xs">{source.file_hash}</span>
                </div>
              )}
            </div>
          ) : source.source_type === 'integration' ? (
            <div className="space-y-2 text-sm">
              {Object.entries(metadata).map(([key, value]) => (
                <div key={key} className="flex items-center gap-2">
                  <span className="text-muted-foreground">{key.replace(/_/g, ' ')}:</span>
                  <span>{String(value)}</span>
                </div>
              ))}
              {Object.keys(metadata).length === 0 && (
                <p className="text-muted-foreground italic">No integration details available.</p>
              )}
            </div>
          ) : (
            <p className="text-muted-foreground italic">No content preview available.</p>
          )}
        </CardContent>
      </Card>

      {/* Workflows */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <CardTitle className="text-base flex items-center gap-2">
              <Play className="h-4 w-4" />
              Workflows
            </CardTitle>
            <div className="flex items-center gap-2">
              {Array.isArray(workflows) && workflows.length > 1 && (
                <Select
                  value={effectiveWorkflowId}
                  onValueChange={setSelectedWorkflowId}
                >
                  <SelectTrigger className="h-8 w-[200px] text-xs">
                    <SelectValue placeholder="Select workflow" />
                  </SelectTrigger>
                  <SelectContent>
                    {workflows.map((wf: any) => (
                      <SelectItem key={wf.id} value={wf.id}>
                        {wf.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              )}
              {Array.isArray(availableModels) && availableModels.length > 0 && (
                <Select
                  value={effectiveModel}
                  onValueChange={setSelectedModel}
                >
                  <SelectTrigger className="h-8 w-[240px] text-xs">
                    <SelectValue placeholder="Use workflow default" />
                  </SelectTrigger>
                  <SelectContent>
                    {availableModels.map((m) => (
                      <SelectItem key={m.id} value={m.id}>
                        {m.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              )}
              <Button
                size="sm"
                variant="ghost"
                className="gap-1.5 text-xs"
                onClick={() => navigate(`/workflows`)}
                title="Open workflow in the Workflow Builder"
              >
                <ExternalLink className="h-3.5 w-3.5" />
                Builder
              </Button>
              <Button
                size="sm"
                variant="outline"
                className="gap-1.5"
                onClick={() => setShowRunHistory(true)}
              >
                <History className="h-3.5 w-3.5" />
                Run History
              </Button>
              <Button
                size="sm"
                className="gap-1.5"
                onClick={() => runWorkflowMutation.mutate({})}
                disabled={runWorkflowMutation.isPending || !effectiveWorkflowId}
              >
                {runWorkflowMutation.isPending ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <Play className="h-3.5 w-3.5" />
                )}
                {Array.isArray(workflows) && workflows.length > 1
                  ? 'Run'
                  : `Run ${workflows?.find((w: any) => w.id === effectiveWorkflowId)?.name ?? 'Analysis'}`}
              </Button>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          {runWorkflowMutation.isPending && (
            <div className="flex items-center gap-3 p-4 rounded-md bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 mb-4">
              <Loader2 className="h-5 w-5 animate-spin text-blue-600 dark:text-blue-400" />
              <div>
                <p className="text-sm font-medium text-blue-700 dark:text-blue-300">
                  Running data source analysis...
                </p>
                <p className="text-xs text-blue-600 dark:text-blue-400 mt-0.5">
                  This may take a moment.
                </p>
              </div>
            </div>
          )}

          {runWorkflowMutation.isError && (
            <div className="p-4 rounded-md bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 mb-4">
              <p className="text-sm text-red-700 dark:text-red-300">
                Workflow failed: {(runWorkflowMutation.error as Error)?.message ?? 'Unknown error'}
              </p>
            </div>
          )}

          {workflowResult?.reused && (
            <div className="flex items-center gap-3 p-4 rounded-md bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 mb-4">
              <CheckCircle2 className="h-5 w-5 text-amber-600 dark:text-amber-400" />
              <div className="flex-1">
                <p className="text-sm font-medium text-amber-700 dark:text-amber-300">
                  Using results from a previous identical run
                </p>
                <p className="text-xs text-amber-600 dark:text-amber-400 mt-0.5">
                  The same content was already processed by this workflow.
                </p>
              </div>
              <Button
                size="sm"
                variant="outline"
                className="text-xs"
                onClick={() => runWorkflowMutation.mutate({ force: true })}
                disabled={runWorkflowMutation.isPending}
              >
                Force Re-run
              </Button>
            </div>
          )}

          {workflowResult && workflowResult.staged_records > 0 && workflowResult.workflow_run_id && (
            <div className="mb-4">
              <RunAndReviewPanel
                runId={workflowResult.workflow_run_id}
                stagedRecords={workflowResult.staged_records}
                workflowName={workflowResult.workflow_name}
                onRunAnother={() => {
                  setWorkflowResult(null);
                  setReviewRunId(null);
                }}
                onClose={() => {
                  setWorkflowResult(null);
                  setReviewRunId(null);
                }}
              />
            </div>
          )}

          {workflowResult?.steps && workflowResult.steps.length > 0 ? (
            <div className="space-y-1">
              {workflowResult.steps.map((step, index) => {
                const isExpanded = expandedSteps.has(index);
                const StatusIcon =
                  step.status === 'complete'
                    ? CheckCircle2
                    : step.status === 'running'
                      ? Loader2
                      : Circle;
                const statusColor =
                  step.status === 'complete'
                    ? 'text-green-600 dark:text-green-400'
                    : step.status === 'running'
                      ? 'text-blue-600 dark:text-blue-400 animate-spin'
                      : 'text-muted-foreground';

                return (
                  <div key={index} className="relative">
                    {index < (workflowResult.steps?.length ?? 0) - 1 && (
                      <div className="absolute left-[11px] top-8 bottom-0 w-px bg-border" />
                    )}
                    <button
                      className="flex items-start gap-3 w-full text-left p-2 rounded-md hover:bg-muted/50 transition-colors"
                      onClick={() => toggleStep(index)}
                    >
                      <StatusIcon className={`h-5 w-5 mt-0.5 shrink-0 ${statusColor}`} />
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                          <span className="text-sm font-medium">{step.name ?? `Step ${index + 1}`}</span>
                          <Badge variant="outline" className="text-[10px]">
                            {step.status}
                          </Badge>
                        </div>
                      </div>
                      {step.result && (
                        isExpanded ? (
                          <ChevronDown className="h-4 w-4 text-muted-foreground shrink-0 mt-0.5" />
                        ) : (
                          <ChevronRight className="h-4 w-4 text-muted-foreground shrink-0 mt-0.5" />
                        )
                      )}
                    </button>
                    {isExpanded && step.result && (
                      <div className="ml-8 mb-2 p-3 rounded-md bg-muted/30 border">
                        {renderJsonContent(step.result)}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          ) : workflowResult && !workflowResult.steps ? (
            <div className="p-4 rounded-md bg-muted/30 border">
              {renderJsonContent(workflowResult)}
            </div>
          ) : !runWorkflowMutation.isPending && !runWorkflowMutation.isError ? (
            <div className="py-4">
              {(() => {
                const selectedWf = Array.isArray(workflows)
                  ? workflows.find((w: any) => w.id === effectiveWorkflowId)
                  : null;
                if (selectedWf) {
                  return (
                    <div className="space-y-3">
                      <p className="text-sm text-muted-foreground">
                        <span className="font-medium text-foreground">{selectedWf.name}</span>
                        {' '}&mdash; {selectedWf.nodes?.length ?? 0} node{(selectedWf.nodes?.length ?? 0) !== 1 ? 's' : ''}
                      </p>
                      <div className="space-y-1.5">
                        {(selectedWf.nodes ?? []).map((node: any, idx: number) => {
                          const inputs = (selectedWf.connections ?? []).filter((c: any) => c.target === node.id);
                          const inputNames = inputs.map((c: any) => {
                            const src = (selectedWf.nodes ?? []).find((n: any) => n.id === c.source);
                            return src?.name ?? c.source;
                          });
                          return (
                          <div key={node.id} className="flex items-center gap-2">
                            <div className="w-5 h-5 rounded-full bg-muted flex items-center justify-center text-[10px] font-medium text-muted-foreground shrink-0">
                              {idx + 1}
                            </div>
                            <span className="text-sm">{node.name}</span>
                            {inputNames.length > 0 && (
                              <span className="text-xs text-muted-foreground">
                                (from: {inputNames.join(', ')})
                              </span>
                            )}
                          </div>
                          );
                        })}
                      </div>
                    </div>
                  );
                }
                return (
                  <div className="text-center py-4 text-muted-foreground">
                    <Play className="h-8 w-8 mx-auto mb-2 opacity-40" />
                    <p className="text-sm">Select a workflow and click Run to process this data source.</p>
                  </div>
                );
              })()}
            </div>
          ) : null}
        </CardContent>
      </Card>

      {/* Artifacts */}
      {artifacts.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle className="text-base flex items-center gap-2">
              <Database className="h-4 w-4" />
              Artifacts
              <Badge variant="secondary" className="ml-1">{artifacts.length}</Badge>
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            {artifacts.map((artifact: any) => {
              const isExpanded = expandedArtifacts.has(artifact.id);
              return (
                <div key={artifact.id} className="border rounded-md">
                  <button
                    className="flex items-center justify-between w-full p-3 text-left hover:bg-muted/30 transition-colors"
                    onClick={() => toggleArtifact(artifact.id)}
                  >
                    <div className="flex items-center gap-2 min-w-0">
                      {isExpanded ? (
                        <ChevronDown className="h-4 w-4 text-muted-foreground shrink-0" />
                      ) : (
                        <ChevronRight className="h-4 w-4 text-muted-foreground shrink-0" />
                      )}
                      <span className="text-sm font-medium truncate">
                        {artifact.title ?? artifact.name ?? 'Artifact'}
                      </span>
                      {artifact.type && (
                        <Badge variant="outline" className="text-[10px]">
                          {artifact.type}
                        </Badge>
                      )}
                    </div>
                    {artifact.created_at && (
                      <span className="text-xs text-muted-foreground shrink-0 ml-2 flex items-center gap-1">
                        <Clock className="h-3 w-3" />
                        {formatDate(artifact.created_at)}
                      </span>
                    )}
                  </button>
                  {isExpanded && (
                    <div className="px-3 pb-3 border-t">
                      <div className="mt-3">
                        {artifact.content ? (
                          renderJsonContent(artifact.content)
                        ) : (
                          <p className="text-sm text-muted-foreground italic">No content.</p>
                        )}
                      </div>
                    </div>
                  )}
                </div>
              );
            })}
          </CardContent>
        </Card>
      )}

      {reviewRunId && (
        <StagingReviewPanel
          open={!!reviewRunId}
          onOpenChange={(open) => !open && setReviewRunId(null)}
          workflowRunId={reviewRunId}
          workflowName={workflowResult?.workflow_name}
        />
      )}

      <WorkflowRunsPanel
        open={showRunHistory}
        onOpenChange={setShowRunHistory}
        organizationId={orgId}
      />
    </div>
  );
}
