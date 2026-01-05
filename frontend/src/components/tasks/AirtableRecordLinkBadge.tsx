import { useCallback, useEffect, useState } from 'react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { ExternalLink, Loader2, RefreshCw, Table2 } from 'lucide-react';
import { airtableApi } from '@/lib/api';
import { AirtableRecordLink } from 'shared/types';
import { toast } from 'sonner';

interface AirtableRecordLinkBadgeProps {
  taskId: string;
  hasExecutionSummary?: boolean;
}

export function AirtableRecordLinkBadge({
  taskId,
  hasExecutionSummary,
}: AirtableRecordLinkBadgeProps) {
  const [link, setLink] = useState<AirtableRecordLink | null>(null);
  const [loading, setLoading] = useState(true);
  const [syncing, setSyncing] = useState(false);

  const loadLink = useCallback(async () => {
    try {
      setLoading(true);
      const result = await airtableApi.getTaskLink(taskId);
      setLink(result);
    } catch (err) {
      // No link exists, that's fine
      setLink(null);
    } finally {
      setLoading(false);
    }
  }, [taskId]);

  useEffect(() => {
    loadLink();
  }, [loadLink]);

  const handleSync = async () => {
    if (!link) return;

    setSyncing(true);
    try {
      await airtableApi.syncDeliverables(taskId);
      toast.success('Deliverables synced to Airtable');
      loadLink(); // Refresh to update sync status
    } catch (err) {
      console.error('Failed to sync deliverables:', err);
      toast.error('Failed to sync deliverables');
    } finally {
      setSyncing(false);
    }
  };

  if (loading) {
    return null;
  }

  if (!link) {
    return null;
  }

  const canSync = hasExecutionSummary && link.origin === 'airtable';
  const needsSync = link.sync_status === 'pending_push';

  return (
    <div className="flex items-center gap-2">
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>
            <Badge
              variant="outline"
              className="gap-1.5 cursor-pointer hover:bg-blue-50 dark:hover:bg-blue-950"
              onClick={() =>
                link.airtable_record_url &&
                window.open(link.airtable_record_url, '_blank')
              }
            >
              <Table2 className="h-3 w-3 text-blue-600" />
              <span className="text-xs">
                {link.origin === 'airtable' ? 'From Airtable' : 'Pushed to Airtable'}
              </span>
              {link.airtable_record_url && (
                <ExternalLink className="h-3 w-3 ml-0.5" />
              )}
            </Badge>
          </TooltipTrigger>
          <TooltipContent>
            <p>
              {link.origin === 'airtable'
                ? 'This task was imported from Airtable'
                : 'This task was pushed to Airtable'}
            </p>
            {link.last_synced_at && (
              <p className="text-xs text-muted-foreground">
                Last synced: {new Date(link.last_synced_at).toLocaleString()}
              </p>
            )}
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>

      {canSync && (
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant={needsSync ? 'default' : 'outline'}
                size="sm"
                className="h-6 px-2 text-xs"
                onClick={handleSync}
                disabled={syncing}
              >
                {syncing ? (
                  <Loader2 className="h-3 w-3 animate-spin" />
                ) : (
                  <>
                    <RefreshCw className="h-3 w-3 mr-1" />
                    Sync
                  </>
                )}
              </Button>
            </TooltipTrigger>
            <TooltipContent>
              <p>Sync execution results to Airtable as a comment</p>
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
      )}
    </div>
  );
}
