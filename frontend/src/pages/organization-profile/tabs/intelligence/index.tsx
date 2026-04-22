import {
  Brain,
  Database,
  FileText,
  GitBranch,
  Network,
  Radio,
} from 'lucide-react';
import { useLocation, useNavigate, useSearchParams } from 'react-router-dom';

import { ArtifactsIntelView, DataSourcesIntelView } from './ArtifactsView';
import { DataSourcesView } from './DataSourcesView';
import { KnowledgeTab } from './KnowledgeTab';
import { PulseSection } from './PulseSection';
import { TopologyIntel } from './TopologyIntel';
import { WorkflowsIntelView } from './WorkflowsView';

// ── Intelligence Tab ─────────────────────────────────────────────────────────

export function IntelligenceTab({
  projectEntries,
  orgId,
}: {
  projectEntries: { id: string; name: string }[];
  orgId: string;
}) {
  const [searchParams] = useSearchParams();
  const location = useLocation();

  // Derive view from pathname (sidebar links) or query param (tab clicks)
  const pathSegment = location.pathname.match(/\/intelligence\/([^/]+)/)?.[1];
  const PATH_TO_VIEW: Record<string, string> = {
    'data-sources': 'datasources',
    artifacts: 'artifacts',
    workflows: 'workflows',
    pulse: 'pulse',
    topology: 'topology',
  };
  const viewFromUrl =
    searchParams.get('view') ||
    (pathSegment ? PATH_TO_VIEW[pathSegment] : null) ||
    'overview';

  const views = [
    { key: 'overview', label: 'Overview', icon: Brain },
    { key: 'datasources', label: 'Data Sources', icon: Database },
    { key: 'artifacts', label: 'Artifacts', icon: FileText },
    { key: 'workflows', label: 'Workflows', icon: GitBranch },
    { key: 'pulse', label: 'Pulse', icon: Radio },
    { key: 'topology', label: 'Topology', icon: Network },
  ];

  const navigate = useNavigate();
  const VIEW_PATHS: Record<string, string> = {
    overview: '',
    datasources: '/data-sources',
    artifacts: '/artifacts',
    workflows: '/workflows',
    pulse: '/pulse',
    topology: '/topology',
  };

  const setView = (view: string) => {
    const basePath = `/organizations/${orgId}/intelligence`;
    const viewPath = VIEW_PATHS[view] ?? '';
    navigate(`${basePath}${viewPath}`, { replace: true });
  };

  return (
    <div className="space-y-6">
      <div className="flex gap-1 p-1 bg-muted/50 rounded-lg w-fit flex-wrap">
        {views.map(({ key, label, icon: Icon }) => (
          <button
            key={key}
            onClick={() => setView(key)}
            className={`flex items-center gap-1.5 px-4 py-1.5 text-sm font-medium rounded-md transition-all ${
              viewFromUrl === key
                ? 'bg-background text-foreground shadow-sm'
                : 'text-muted-foreground hover:text-foreground'
            }`}
          >
            <Icon className="h-3.5 w-3.5" />
            {label}
          </button>
        ))}
      </div>

      {viewFromUrl === 'overview' && (
        <KnowledgeTab orgId={orgId} projectEntries={projectEntries} />
      )}
      {viewFromUrl === 'datasources' && (
        <div className="space-y-6">
          <DataSourcesIntelView orgId={orgId} projectEntries={projectEntries} />
          <DataSourcesView orgId={orgId} projectEntries={projectEntries} />
        </div>
      )}
      {viewFromUrl === 'artifacts' && (
        <ArtifactsIntelView projectEntries={projectEntries} />
      )}
      {viewFromUrl === 'workflows' && <WorkflowsIntelView orgId={orgId} />}
      {viewFromUrl === 'pulse' && (
        <PulseSection projectEntries={projectEntries} />
      )}
      {viewFromUrl === 'topology' && (
        <TopologyIntel orgId={orgId} projectEntries={projectEntries} />
      )}
    </div>
  );
}

export default IntelligenceTab;
