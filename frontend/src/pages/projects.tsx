import { useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { projectsApi } from '@/lib/api';
import { projectKeys } from '@/lib/query-keys';
import { ProjectList } from '@/components/projects/project-list';
import { ProjectDetail } from '@/components/projects/project-detail';
import { Loader } from '@/components/ui/loader';

/**
 * Wrapper that redirects client projects to the Client Overview page.
 * Internal (non-client) projects continue to show ProjectDetail.
 */
function ProjectDetailOrRedirect({ projectId }: { projectId: string }) {
  const navigate = useNavigate();

  const { data: project, isLoading } = useQuery({
    queryKey: projectKeys.detail(projectId),
    queryFn: () => projectsApi.getById(projectId),
    staleTime: 60 * 1000,
  });

  useEffect(() => {
    if (!project) return;
    // If this project belongs to a client, redirect to the client overview page
    const p = project as any;
    if (p.client_id && p.organization_id) {
      navigate(`/organizations/${p.organization_id}/clients/${p.client_id}`, { replace: true });
    }
  }, [project, navigate]);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <Loader size={24} message="Loading project…" />
      </div>
    );
  }

  // Non-client project — show normal detail view
  const p = project as any;
  if (!p || (p.client_id && p.organization_id)) {
    // Still redirecting or no project; show nothing
    return null;
  }

  return <ProjectDetail projectId={projectId} onBack={() => navigate('/projects')} />;
}

export function Projects() {
  const { projectId } = useParams<{ projectId: string }>();

  if (projectId) {
    return <ProjectDetailOrRedirect projectId={projectId} />;
  }

  return <ProjectList />;
}
