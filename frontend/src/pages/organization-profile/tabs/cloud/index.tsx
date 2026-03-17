import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Cloud, Upload, Activity, Settings, RefreshCw, Loader2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { toast } from 'sonner';
import { orgCloudApi } from '@/lib/api/org-cloud';
import { orgCloudKeys } from '@/lib/query-keys';
import { CloudBrowser } from './CloudBrowser';
import { CloudUpload } from './CloudUpload';
import { CloudActivity } from './CloudActivity';
import { CloudSettings } from './CloudSettings';
import { CloudStats } from './CloudStats';

interface CloudTabProps {
  orgId: string;
}

export default function CloudTab({ orgId }: CloudTabProps) {
  const [subTab, setSubTab] = useState('browse');
  const queryClient = useQueryClient();

  const { data: stats } = useQuery({
    queryKey: orgCloudKeys.stats(orgId),
    queryFn: () => orgCloudApi.getStats(orgId),
    enabled: !!orgId,
    staleTime: 60_000,
  });

  const indexMutation = useMutation({
    mutationFn: () => orgCloudApi.triggerIndex(orgId),
    onSuccess: (data) => {
      toast.success(`Indexed ${data.indexed_count} files`);
      queryClient.invalidateQueries({ queryKey: orgCloudKeys.all(orgId) });
      queryClient.invalidateQueries({ queryKey: orgCloudKeys.stats(orgId) });
    },
    onError: () => toast.error('Failed to index files'),
  });

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Cloud className="h-5 w-5 text-blue-500" />
          <h2 className="text-lg font-semibold">Organization Cloud</h2>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={() => indexMutation.mutate()}
          disabled={indexMutation.isPending}
          className="text-xs gap-1.5"
        >
          {indexMutation.isPending ? (
            <><Loader2 className="h-3.5 w-3.5 animate-spin" />Indexing...</>
          ) : (
            <><RefreshCw className="h-3.5 w-3.5" />Index Data</>
          )}
        </Button>
      </div>

      {stats && <CloudStats stats={stats} />}

      <Tabs value={subTab} onValueChange={setSubTab}>
        <TabsList>
          <TabsTrigger value="browse" className="text-xs gap-1.5">
            <Cloud className="h-3.5 w-3.5" />Browse
          </TabsTrigger>
          <TabsTrigger value="upload" className="text-xs gap-1.5">
            <Upload className="h-3.5 w-3.5" />Upload
          </TabsTrigger>
          <TabsTrigger value="activity" className="text-xs gap-1.5">
            <Activity className="h-3.5 w-3.5" />Activity
          </TabsTrigger>
          <TabsTrigger value="settings" className="text-xs gap-1.5">
            <Settings className="h-3.5 w-3.5" />Settings
          </TabsTrigger>
        </TabsList>

        <TabsContent value="browse">
          <CloudBrowser orgId={orgId} />
        </TabsContent>

        <TabsContent value="upload">
          <CloudUpload orgId={orgId} />
        </TabsContent>

        <TabsContent value="activity">
          <CloudActivity orgId={orgId} />
        </TabsContent>

        <TabsContent value="settings">
          <CloudSettings orgId={orgId} />
        </TabsContent>
      </Tabs>
    </div>
  );
}
