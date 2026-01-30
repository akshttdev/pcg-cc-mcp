import { useParams, useNavigate } from 'react-router-dom';
import { CrmPipelineBoard } from '@/components/crm/CrmPipelineBoard';

export function CrmConferencesPage() {
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
      pipelineType="conferences"
      title="Conferences Pipeline"
      onSettingsClick={() => navigate(`/projects/${projectId}/crm/settings`)}
    />
  );
}
