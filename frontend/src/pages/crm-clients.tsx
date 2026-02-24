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

  if (!orgId) {
    return (
      <div className="h-full flex items-center justify-center text-muted-foreground">
        Organization not found
      </div>
    );
  }

  return (
    <CrmPipelineBoard
      orgId={orgId}
      pipelineType="sales"
      title="Acquisition Pipeline"
    />
  );
}

export function CrmLifecyclePage() {
  const { orgId } = useParams<{ orgId: string }>();

  if (!orgId) {
    return (
      <div className="h-full flex items-center justify-center text-muted-foreground">
        Organization not found
      </div>
    );
  }

  return (
    <CrmPipelineBoard
      orgId={orgId}
      pipelineType="delivery"
      title="Client Lifecycle"
    />
  );
}
