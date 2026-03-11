import { GitCompare, MessageSquare, Cog, Workflow, Activity, PackageOpen } from 'lucide-react';
import { useAttemptExecution } from '@/hooks/useAttemptExecution';
import type { TabType } from '@/types/tabs';
import type { TaskAttempt } from 'shared/types';

type Props = {
  activeTab: TabType;
  setActiveTab: (tab: TabType) => void;
  rightContent?: React.ReactNode;
  selectedAttempt: TaskAttempt | null;
  artifactCount?: number;
  workflowEventCount?: number;
};

function TabNavigation({
  activeTab,
  setActiveTab,
  rightContent,
  selectedAttempt,
  artifactCount,
  workflowEventCount,
}: Props) {
  const { attemptData } = useAttemptExecution(selectedAttempt?.id);

  const tabs = [
    { id: 'logs' as TabType, label: 'Logs', icon: MessageSquare },
    { id: 'diffs' as TabType, label: 'Diffs', icon: GitCompare },
    { id: 'processes' as TabType, label: 'Processes', icon: Cog },
    { id: 'workflows' as TabType, label: 'Workflows', icon: Workflow, count: workflowEventCount },
    { id: 'activity' as TabType, label: 'Activity', icon: Activity },
    { id: 'artifacts' as TabType, label: 'Artifacts', icon: PackageOpen, count: artifactCount },
  ];

  const getTabClassName = (tabId: TabType) => {
    const baseClasses = 'flex items-center py-2 px-2 text-sm font-medium';
    const activeClasses = 'text-primary-foreground';
    const inactiveClasses =
      'text-secondary-foreground hover:text-primary-foreground';

    return `${baseClasses} ${activeTab === tabId ? activeClasses : inactiveClasses}`;
  };

  return (
    <div className="border-b border-dashed bg-background sticky top-0 z-10">
      <div className="flex items-center px-3 space-x-1 overflow-x-auto">
        {tabs.map(({ id, label, icon: Icon, count }) => (
          <button
            key={id}
            onClick={() => setActiveTab(id)}
            className={getTabClassName(id)}
          >
            <Icon className="h-4 w-4 mr-1.5" />
            {label}
            {id === 'processes' &&
              attemptData.processes &&
              attemptData.processes.length > 0 && (
                <span className="ml-1.5 px-1.5 py-0.5 text-xs bg-primary/10 text-primary rounded-full">
                  {attemptData.processes.length}
                </span>
              )}
            {count != null && count > 0 && id !== 'processes' && (
              <span className="ml-1.5 px-1.5 py-0.5 text-xs bg-primary/10 text-primary rounded-full">
                {count}
              </span>
            )}
          </button>
        ))}
        <div className="ml-auto flex items-center shrink-0">{rightContent}</div>
      </div>
    </div>
  );
}

export default TabNavigation;
