import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { ArrowRight } from 'lucide-react';
import type { WorkflowStagingRecord } from '@/lib/api';

interface RunGroup {
  runId: string;
  records: WorkflowStagingRecord[];
  workflowName?: string;
}

interface StagingCardGridProps {
  grouped: RunGroup[];
  selectRun: (runId: string) => void;
}

export function StagingCardGrid({ grouped, selectRun }: StagingCardGridProps) {
  const typeLabels: Record<string, string> = {
    crm_contact: 'contacts',
    company: 'companies',
    crm_deal: 'deals',
    task: 'tasks',
  };

  return (
    <div className="grid gap-3 grid-cols-1 sm:grid-cols-2 lg:grid-cols-3">
      {grouped.map((group) => {
        const statuses = group.records.reduce<Record<string, number>>((acc, r) => {
          acc[r.status] = (acc[r.status] || 0) + 1;
          return acc;
        }, {});
        const groupTargetTypes = group.records.reduce<Record<string, number>>((acc, r) => {
          acc[r.target_type] = (acc[r.target_type] || 0) + 1;
          return acc;
        }, {});
        return (
          <Card
            key={group.runId}
            className="card-interactive cursor-pointer"
            onClick={() => selectRun(group.runId)}
          >
            <CardHeader className="pb-2">
              <div className="flex items-center justify-between">
                <CardTitle className="text-sm">
                  {group.workflowName || 'Workflow Run'}
                </CardTitle>
                <Badge variant="outline" className="text-xs">
                  {group.records.length} pending
                </Badge>
              </div>
              <div className="flex flex-wrap gap-1 mt-1">
                {Object.entries(groupTargetTypes).map(([type, count]) => (
                  <Badge key={type} variant="secondary" className="text-[9px]">
                    {count} {typeLabels[type] || type}
                  </Badge>
                ))}
              </div>
            </CardHeader>
            <CardContent>
              <div className="flex items-center justify-between">
                <div className="flex flex-wrap gap-1">
                  {Object.entries(statuses).map(([status, count]) => {
                    const statusColors: Record<string, string> = {
                      pending: 'text-amber-600',
                      approved: 'text-green-600',
                      rejected: 'text-red-600',
                    };
                    return (
                      <span key={status} className={`text-xs ${statusColors[status] || 'text-muted-foreground'}`}>
                        {count} {status}
                      </span>
                    );
                  })}
                </div>
                <ArrowRight className="h-3.5 w-3.5 text-muted-foreground" />
              </div>
            </CardContent>
          </Card>
        );
      })}
    </div>
  );
}
