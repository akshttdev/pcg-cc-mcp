import { useParams, useNavigate } from 'react-router-dom';
import { CrmPipelineBoard } from '@/components/crm/CrmPipelineBoard';

export function CrmClientsPage() {
  const { projectId } = useParams<{ projectId: string }>();
  const navigate = useNavigate();

  if (!projectId) {
    return (
      <div className="h-full flex items-center justify-center text-muted-foreground">
        Project not found
      </div>
    );
  }

  return (
    <CrmPipelineBoard
      projectId={projectId}
      pipelineType="clients"
      title="Clients Pipeline"
      onSettingsClick={() => navigate(`/projects/${projectId}/crm/settings`)}
    />
  );
}

export function CrmAcquisitionPage() {
  const { orgId } = useParams<{ orgId: string }>();

  return (
    <div className="h-full flex items-center justify-center text-muted-foreground">
      {orgId ? 'Acquisition Pipeline — coming soon' : 'Organization not found'}
    </div>
  );
}

export function CrmLifecyclePage() {
  const { orgId } = useParams<{ orgId: string }>();

  return (
    <div className="h-full flex items-center justify-center text-muted-foreground">
      {orgId ? 'Lifecycle Pipeline — coming soon' : 'Organization not found'}
    </div>
  );
}
