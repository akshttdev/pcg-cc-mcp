import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  FileText,
  Clock,
  Zap,
  LayoutGrid,
  RefreshCw,
  Coins,
  Minimize2,
  Maximize2,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { TaskWithAttemptStatus } from 'shared/types';

import { TaskHeader } from './TaskHeader';
import { OverviewTab } from './tabs/OverviewTab';
import { ArtifactsTab } from './tabs/ArtifactsTab';
import { WorkflowTab } from './tabs/WorkflowTab';
import { ActivityTab } from './tabs/ActivityTab';
import { VibeBudgetTab } from './tabs/VibeBudgetTab';
import { useTaskPanelData } from './useTaskPanelData';

type DetailTab = 'overview' | 'artifacts' | 'workflow' | 'activity' | 'vibe';

interface EnhancedTaskDetailsPanelProps {
  task: TaskWithAttemptStatus;
  projectId: string;
  onClose: () => void;
  onEdit?: () => void;
  onDelete?: () => void;
  onDuplicate?: () => void;
  onToggleFullscreen?: () => void;
  isFullscreen?: boolean;
  onToggleExpand?: () => void;
  isExpanded?: boolean;
  hideClose?: boolean;
  className?: string;
}

export function EnhancedTaskDetailsPanel({
  task,
  projectId,
  onClose,
  onEdit,
  onDelete,
  onDuplicate,
  onToggleFullscreen,
  isFullscreen,
  onToggleExpand,
  isExpanded,
  hideClose,
  className,
}: EnhancedTaskDetailsPanelProps) {
  const {
    activeTab,
    setActiveTab,
    compactMode,
    toggleCompactMode,
    mode,
    artifacts,
    artifactsLoading,
    artifactsError,
    workflowEvents,
    workflowLoading,
    workflowError,
    executingAgentId,
    chatMessages,
    vibeBalance,
    vibeTransactions,
    vibeLoading,
    initialPrompt,
    handleSendMessage,
    handleRefresh,
    handleArtifactDownload,
  } = useTaskPanelData({ task, projectId });

  const artifactCount = artifacts.length;
  const eventCount = workflowEvents.length;

  return (
    <div
      className={cn(
        'flex flex-col h-full bg-background border-l',
        className
      )}
    >
      <TaskHeader
        task={task}
        mode={mode}
        onEdit={onEdit}
        onDelete={onDelete}
        onDuplicate={onDuplicate}
        onClose={onClose}
        onToggleFullscreen={onToggleFullscreen}
        isFullscreen={isFullscreen}
        onToggleExpand={onToggleExpand}
        isExpanded={isExpanded}
        hideClose={hideClose}
      />

      <Tabs
        value={activeTab}
        onValueChange={(v) => setActiveTab(v as DetailTab)}
        className="flex-1 flex flex-col min-h-0"
      >
        <div className="border-b px-4 flex items-center justify-between">
          <TabsList className="h-10 bg-transparent p-0">
            <TabsTrigger
              value="overview"
              className="data-[state=active]:bg-transparent data-[state=active]:shadow-none data-[state=active]:border-b-2 data-[state=active]:border-primary rounded-none px-4"
            >
              <LayoutGrid className="h-4 w-4 mr-2" />
              Overview
            </TabsTrigger>
            {!compactMode && (
              <TabsTrigger
                value="artifacts"
                className="data-[state=active]:bg-transparent data-[state=active]:shadow-none data-[state=active]:border-b-2 data-[state=active]:border-primary rounded-none px-4"
              >
                <FileText className="h-4 w-4 mr-2" />
                Artifacts
                {artifactCount > 0 && (
                  <Badge variant="secondary" className="ml-2 h-5">
                    {artifactCount}
                  </Badge>
                )}
              </TabsTrigger>
            )}
            {!compactMode && (
              <TabsTrigger
                value="workflow"
                className="data-[state=active]:bg-transparent data-[state=active]:shadow-none data-[state=active]:border-b-2 data-[state=active]:border-primary rounded-none px-4"
              >
                <Zap className="h-4 w-4 mr-2" />
                Workflow
              </TabsTrigger>
            )}
            {!compactMode && (
              <TabsTrigger
                value="activity"
                className="data-[state=active]:bg-transparent data-[state=active]:shadow-none data-[state=active]:border-b-2 data-[state=active]:border-primary rounded-none px-4"
              >
                <Clock className="h-4 w-4 mr-2" />
                Logs
                {eventCount > 0 && (
                  <Badge variant="secondary" className="ml-2 h-5">
                    {eventCount}
                  </Badge>
                )}
              </TabsTrigger>
            )}
            {!compactMode && (
              <TabsTrigger
                value="vibe"
                className="data-[state=active]:bg-transparent data-[state=active]:shadow-none data-[state=active]:border-b-2 data-[state=active]:border-primary rounded-none px-4"
              >
                <Coins className="h-4 w-4 mr-2" />
                Vibe
                {task.vibe_cost && Number(task.vibe_cost) > 0 ? (
                  <Badge variant="secondary" className="ml-2 h-5">
                    {Number(task.vibe_cost).toLocaleString()}
                  </Badge>
                ) : null}
              </TabsTrigger>
            )}
          </TabsList>

          <div className="flex items-center gap-1">
            <Button
              variant="ghost"
              size="icon"
              onClick={toggleCompactMode}
              className="h-8 w-8"
              title={compactMode ? 'Show all tabs' : 'Compact view'}
            >
              {compactMode ? <Maximize2 className="h-4 w-4" /> : <Minimize2 className="h-4 w-4" />}
            </Button>
            <Button variant="ghost" size="icon" onClick={handleRefresh} className="h-8 w-8">
              <RefreshCw className={cn('h-4 w-4', (artifactsLoading || workflowLoading) && 'animate-spin')} />
            </Button>
          </div>
        </div>

        <TabsContent value="overview" className="flex-1 m-0 overflow-hidden">
          <OverviewTab
            task={task}
            artifacts={artifacts}
            artifactsLoading={artifactsLoading}
            artifactsError={artifactsError}
            workflowEvents={workflowEvents}
            workflowLoading={workflowLoading}
            workflowError={workflowError}
            chatMessages={chatMessages}
            executingAgentId={executingAgentId}
            initialPrompt={initialPrompt}
            onArtifactDownload={handleArtifactDownload}
            onSendMessage={handleSendMessage}
            onSwitchTab={setActiveTab}
          />
        </TabsContent>

        <TabsContent value="artifacts" className="flex-1 m-0 overflow-hidden">
          <ArtifactsTab
            artifacts={artifacts}
            artifactsLoading={artifactsLoading}
            artifactsError={artifactsError}
            onArtifactDownload={handleArtifactDownload}
          />
        </TabsContent>

        <TabsContent value="workflow" className="flex-1 m-0 overflow-hidden">
          <WorkflowTab
            workflowEvents={workflowEvents}
            workflowLoading={workflowLoading}
            workflowError={workflowError}
            taskId={task.id}
            taskTitle={task.title}
            executingAgentId={executingAgentId}
            initialPrompt={initialPrompt}
            onSendMessage={handleSendMessage}
          />
        </TabsContent>

        <TabsContent value="activity" className="flex-1 m-0 overflow-hidden">
          <ActivityTab
            taskId={task.id}
            workflowEvents={workflowEvents}
            collaborators={task.collaborators}
            chatMessages={chatMessages}
          />
        </TabsContent>

        <TabsContent value="vibe" className="flex-1 m-0 overflow-hidden">
          <VibeBudgetTab
            task={task}
            vibeBalance={vibeBalance}
            vibeTransactions={vibeTransactions}
            vibeLoading={vibeLoading}
          />
        </TabsContent>
      </Tabs>
    </div>
  );
}
