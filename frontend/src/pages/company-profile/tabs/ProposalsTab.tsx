import { FileText } from 'lucide-react';
import { Card, CardContent } from '@/components/ui/card';
import { EmptyState } from '@/components/ui/empty-state';
import { type ProposalRecord } from '@/lib/api';
import { ProposalStatusBadge } from '../components/helpers';

export function ProposalsTab({ proposals, onNewProposal }: { proposals: ProposalRecord[]; onNewProposal: () => void }) {
  if (proposals.length === 0) {
    return (
      <EmptyState
        icon={FileText}
        title="No proposals yet"
        action={{ label: "Create Proposal", onClick: onNewProposal }}
        className="h-32"
      />
    );
  }
  return (
    <div className="space-y-2">
      {proposals.map((p) => (
        <Card key={p.id} className="hover:shadow-sm transition-shadow">
          <CardContent className="pt-3 pb-3">
            <div className="flex items-start justify-between gap-3">
              <div className="flex-1 min-w-0">
                <p className="font-medium text-sm">{p.title}</p>
                {p.description && (
                  <p className="text-xs text-muted-foreground mt-0.5 line-clamp-1">{p.description}</p>
                )}
                {p.quote_amount_vibe > 0 && (
                  <p className="text-xs text-muted-foreground mt-0.5">
                    ${(p.quote_amount_vibe / 100).toLocaleString()}
                  </p>
                )}
              </div>
              <ProposalStatusBadge status={p.status} />
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}
