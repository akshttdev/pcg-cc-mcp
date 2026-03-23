import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  CheckCheck,
  XCircle,
  LayoutGrid,
  TableProperties,
} from 'lucide-react';

interface StagingHeaderProps {
  pendingCount: number;
  totalPending: number;
  totalApproved: number;
  totalDuplicates: number;
  groupedCount: number;
  viewMode: 'cards' | 'table';
  setCardsView: () => void;
  setTableView: () => void;
  pendingNonDuplicateCount: number;
  handleRejectDups: () => void;
  handleApproveAll: () => void;
  isRejectingDups: boolean;
  isApprovingAll: boolean;
}

export function StagingHeader({
  pendingCount,
  totalPending,
  totalApproved,
  totalDuplicates,
  groupedCount,
  viewMode,
  setCardsView,
  setTableView,
  pendingNonDuplicateCount,
  handleRejectDups,
  handleApproveAll,
  isRejectingDups,
  isApprovingAll,
}: StagingHeaderProps) {
  return (
    <>
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold">Staging Review</h2>
          <p className="text-sm text-muted-foreground">
            Review and approve records extracted by workflows before they are committed to the CRM.
          </p>
        </div>
        <div className="flex items-center gap-3">
          {pendingCount > 0 && (
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              <Badge variant="outline">{totalPending} pending</Badge>
              {totalApproved > 0 && <Badge variant="outline" className="text-green-600 border-green-200">{totalApproved} approved</Badge>}
              {totalDuplicates > 0 && <Badge variant="outline" className="text-amber-600 border-amber-200">{totalDuplicates} duplicates</Badge>}
              <span className="text-muted-foreground/50">|</span>
              <span>{groupedCount} run{groupedCount !== 1 ? 's' : ''}</span>
            </div>
          )}
          {/* View toggle */}
          {pendingCount > 0 && (
            <div className="flex items-center border rounded-md">
              <button
                onClick={setCardsView}
                className={`p-1.5 rounded-l-md transition-colors ${viewMode === 'cards' ? 'bg-primary text-primary-foreground' : 'hover:bg-muted text-muted-foreground'}`}
                title="Card view"
              >
                <LayoutGrid className="h-3.5 w-3.5" />
              </button>
              <button
                onClick={setTableView}
                className={`p-1.5 rounded-r-md transition-colors ${viewMode === 'table' ? 'bg-primary text-primary-foreground' : 'hover:bg-muted text-muted-foreground'}`}
                title="Table view"
              >
                <TableProperties className="h-3.5 w-3.5" />
              </button>
            </div>
          )}
        </div>
      </div>

      {/* Global batch action bar */}
      {pendingCount > 0 && (
        <div className="flex items-center gap-2 p-3 rounded-lg border bg-muted/30">
          <span className="text-xs text-muted-foreground mr-auto">Batch actions across all runs:</span>
          {totalDuplicates > 0 && (
            <Button
              size="sm"
              variant="outline"
              className="h-7 text-xs gap-1 text-amber-600 border-amber-200 hover:bg-amber-50"
              onClick={handleRejectDups}
              disabled={isRejectingDups}
            >
              <XCircle className="h-3 w-3" />
              {isRejectingDups ? 'Removing...' : `Reject ${totalDuplicates} duplicates`}
            </Button>
          )}
          {pendingNonDuplicateCount > 0 && (
            <Button
              size="sm"
              variant="outline"
              className="h-7 text-xs gap-1 text-green-600 border-green-200 hover:bg-green-50"
              onClick={handleApproveAll}
              disabled={isApprovingAll}
            >
              <CheckCheck className="h-3 w-3" />
              {isApprovingAll ? 'Approving & committing...' : `Approve & commit ${pendingNonDuplicateCount} valid`}
            </Button>
          )}
        </div>
      )}
    </>
  );
}
