import { CrmActivityTimeline } from '../../CrmActivityTimeline';
import type { CrmDealWithContact } from '@/types/crm';

interface ActivityTabProps {
  deal: CrmDealWithContact;
  projectId?: string;
}

export function ActivityTab({ deal, projectId }: ActivityTabProps) {
  const resolvedProjectId = projectId || deal.organization_id;
  return (
    <div className="p-5">
      <CrmActivityTimeline projectId={resolvedProjectId} dealId={deal.id} limit={30} />
    </div>
  );
}
