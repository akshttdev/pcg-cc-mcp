import { useMemo } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import {
  Brain,
  Cog,
  CheckCircle2,
  ArrowRight,
  Clock,
  FileText,
  Image,
  Video,
  File,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type {
  AgentFlow,
  AgentPhase,
  ExecutionArtifact,
  ArtifactType,
} from 'shared/types';

interface AgentFlowVisualizationProps {
  flow: AgentFlow;
  artifacts?: ExecutionArtifact[];
  className?: string;
}

const phaseConfig: Record<
  AgentPhase,
  { icon: React.ReactNode; color: string; bgColor: string; label: string }
> = {
  planning: {
    icon: <Brain className="h-6 w-6" />,
    color: 'text-blue-600',
    bgColor: 'bg-blue-50',
    label: 'Planning',
  },
  execution: {
    icon: <Cog className="h-6 w-6" />,
    color: 'text-yellow-600',
    bgColor: 'bg-yellow-50',
    label: 'Executing',
  },
  verification: {
    icon: <CheckCircle2 className="h-6 w-6" />,
    color: 'text-green-600',
    bgColor: 'bg-green-50',
    label: 'Verifying',
  },
};

const artifactIcons: Partial<Record<ArtifactType, React.ReactNode>> = {
  plan: <FileText className="h-4 w-4" />,
  screenshot: <Image className="h-4 w-4" />,
  walkthrough: <Video className="h-4 w-4" />,
  research_report: <FileText className="h-4 w-4" />,
  strategy_document: <FileText className="h-4 w-4" />,
  content_draft: <FileText className="h-4 w-4" />,
  verification_report: <CheckCircle2 className="h-4 w-4" />,
  browser_recording: <Video className="h-4 w-4" />,
  platform_screenshot: <Image className="h-4 w-4" />,
};

export function AgentFlowVisualization({
  flow,
  artifacts = [],
  className,
}: AgentFlowVisualizationProps) {
  const phases: AgentPhase[] = ['planning', 'execution', 'verification'];

  const currentPhaseIndex = phases.indexOf(flow.current_phase);
  const progressPercent = ((currentPhaseIndex + 1) / phases.length) * 100;

  // Group artifacts by inferred phase based on artifact type
  const artifactsByPhase = useMemo(() => {
    const grouped: Record<string, ExecutionArtifact[]> = {
      planning: [],
      execution: [],
      verification: [],
    };

    artifacts.forEach((artifact) => {
      // Infer phase from artifact type
      let phase: string = 'execution';
      if (artifact.artifact_type === 'plan' || artifact.artifact_type === 'strategy_document') {
        phase = 'planning';
      } else if (artifact.artifact_type === 'verification_report' || artifact.artifact_type === 'compliance_score') {
        phase = 'verification';
      }
      grouped[phase].push(artifact);
    });

    return grouped;
  }, [artifacts]);

  const formatDuration = (start?: string, end?: string) => {
    if (!start) return '-';
    const startDate = new Date(start);
    const endDate = end ? new Date(end) : new Date();
    const diffMs = endDate.getTime() - startDate.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffSecs = Math.floor((diffMs % 60000) / 1000);
    return `${diffMins}m ${diffSecs}s`;
  };

  return (
    <Card className={cn('w-full', className)}>
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-lg">Agent Flow Pipeline</CardTitle>
          <Badge
            variant={flow.status === 'completed' ? 'default' : 'secondary'}
          >
            {flow.status}
          </Badge>
        </div>
        <Progress value={progressPercent} className="h-2 mt-2" />
      </CardHeader>

      <CardContent>
        <div className="flex items-start justify-between gap-4">
          {phases.map((phase, idx) => {
            const isActive = flow.current_phase === phase;
            const isComplete = currentPhaseIndex > idx;
            const config = phaseConfig[phase];
            const phaseArtifacts = artifactsByPhase[phase] || [];

            // Get timing info
            let startTime: string | undefined | null;
            let endTime: string | undefined | null;

            if (phase === 'planning') {
              startTime = flow.planning_started_at;
              endTime = flow.planning_completed_at;
            } else if (phase === 'execution') {
              startTime = flow.execution_started_at;
              endTime = flow.execution_completed_at;
            } else if (phase === 'verification') {
              startTime = flow.verification_started_at;
              endTime = flow.verification_completed_at;
            }

            return (
              <div key={phase} className="flex items-center flex-1">
                <div
                  className={cn(
                    'flex flex-col items-center p-4 rounded-lg border-2 transition-all flex-1',
                    isActive && 'border-primary shadow-md',
                    isComplete && 'border-green-500 bg-green-50/50',
                    !isActive && !isComplete && 'border-gray-200 opacity-50'
                  )}
                >
                  {/* Phase Icon */}
                  <div
                    className={cn(
                      'p-3 rounded-full mb-2',
                      config.bgColor,
                      config.color
                    )}
                  >
                    {config.icon}
                  </div>

                  {/* Phase Name */}
                  <span className="font-medium text-sm mb-1">{config.label}</span>

                  {/* Duration */}
                  <div className="flex items-center gap-1 text-xs text-muted-foreground mb-2">
                    <Clock className="h-3 w-3" />
                    {formatDuration(startTime ?? undefined, endTime ?? undefined)}
                  </div>

                  {/* Artifacts */}
                  {phaseArtifacts.length > 0 && (
                    <div className="flex flex-wrap gap-1 mt-2">
                      {phaseArtifacts.slice(0, 3).map((artifact) => (
                        <Badge
                          key={artifact.id}
                          variant="outline"
                          className="text-xs"
                        >
                          {artifactIcons[artifact.artifact_type] || (
                            <File className="h-3 w-3" />
                          )}
                          <span className="ml-1 truncate max-w-[80px]">
                            {artifact.title || artifact.artifact_type}
                          </span>
                        </Badge>
                      ))}
                      {phaseArtifacts.length > 3 && (
                        <Badge variant="outline" className="text-xs">
                          +{phaseArtifacts.length - 3}
                        </Badge>
                      )}
                    </div>
                  )}
                </div>

                {/* Arrow */}
                {idx < phases.length - 1 && (
                  <ArrowRight
                    className={cn(
                      'h-6 w-6 mx-2 flex-shrink-0',
                      isComplete ? 'text-green-500' : 'text-gray-300'
                    )}
                  />
                )}
              </div>
            );
          })}
        </div>

        {/* Verification Score */}
        {flow.verification_score !== null && (
          <div className="mt-4 p-3 bg-muted rounded-lg">
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium">Verification Score</span>
              <Badge
                variant={flow.verification_score >= 0.8 ? 'default' : 'secondary'}
              >
                {(flow.verification_score * 100).toFixed(0)}%
              </Badge>
            </div>
            <Progress
              value={flow.verification_score * 100}
              className="h-2 mt-2"
            />
          </div>
        )}
      </CardContent>
    </Card>
  );
}
