import { Badge } from '@/components/ui/badge';
import { GitBranch } from 'lucide-react';
import { LIFECYCLE_STAGE_INFO, type LifecycleStage } from '@/types/crm';
import type { CrmContactRecord } from '@/lib/api';

export function ContactCard({ contact, onClick }: { contact: CrmContactRecord; onClick?: () => void }) {
  const stageInfo = LIFECYCLE_STAGE_INFO[contact.lifecycle_stage as LifecycleStage];

  let importedViaWorkflow = false;
  if (contact.custom_fields) {
    try {
      const cf = typeof contact.custom_fields === 'string' ? JSON.parse(contact.custom_fields) : contact.custom_fields;
      importedViaWorkflow = !!cf.source_workflow_run_id;
    } catch { /* ignore */ }
  }

  return (
    <button
      type="button"
      onClick={onClick}
      className="block w-full text-left p-4 rounded-xl border border-border/50 bg-card/50 hover:bg-accent/30 hover:border-accent/50 transition-all group"
    >
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <p className="text-sm font-medium truncate group-hover:text-foreground">
            {contact.full_name || 'Unnamed'}
          </p>
          {contact.job_title && (
            <p className="text-xs text-muted-foreground truncate mt-0.5">{contact.job_title}</p>
          )}
          {contact.email && (
            <p className="text-xs text-muted-foreground truncate">{contact.email}</p>
          )}
          {contact.company_name && (
            <p className="text-xs text-muted-foreground truncate">{contact.company_name}</p>
          )}
        </div>
        {contact.lead_score > 0 && (
          <span className="text-xs font-medium px-1.5 py-0.5 rounded bg-blue-100 text-blue-700 dark:bg-blue-950 dark:text-blue-300 shrink-0">
            {contact.lead_score}
          </span>
        )}
      </div>
      <div className="flex items-center gap-2 mt-3 flex-wrap">
        {stageInfo && (
          <Badge
            variant="secondary"
            className="text-xs"
            style={{ backgroundColor: stageInfo.color + '20', color: stageInfo.color }}
          >
            {stageInfo.label}
          </Badge>
        )}
        {contact.source && (
          <Badge variant="outline" className="text-xs capitalize">{contact.source}</Badge>
        )}
        {importedViaWorkflow && (
          <Badge variant="outline" className="text-xs gap-0.5" title="Imported via workflow">
            <GitBranch className="h-2.5 w-2.5" />
            Workflow
          </Badge>
        )}
      </div>
    </button>
  );
}
