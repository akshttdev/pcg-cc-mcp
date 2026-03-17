import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Settings } from 'lucide-react';
import { Switch } from '@/components/ui/switch';
import { Skeleton } from '@/components/ui/skeleton';
import { toast } from 'sonner';
import { orgCloudApi, type OrgCloudSettings as SettingsType } from '@/lib/api/org-cloud';

interface CloudSettingsProps {
  orgId: string;
}

export function CloudSettings({ orgId }: CloudSettingsProps) {
  const queryClient = useQueryClient();

  const { data: settings, isLoading } = useQuery({
    queryKey: ['org-cloud-settings', orgId],
    queryFn: () => orgCloudApi.getSettings(orgId),
    enabled: !!orgId,
    staleTime: 60_000,
  });

  const updateMutation = useMutation({
    mutationFn: (data: Partial<SettingsType>) => orgCloudApi.updateSettings(orgId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['org-cloud-settings', orgId] });
      toast.success('Settings updated');
    },
    onError: () => toast.error('Failed to update settings'),
  });

  const toggle = (key: keyof SettingsType, current: boolean) => {
    updateMutation.mutate({ [key]: !current });
  };

  if (isLoading) {
    return (
      <div className="space-y-4 mt-4">
        {Array.from({ length: 4 }).map((_, i) => (
          <Skeleton key={i} className="h-10 rounded-lg" />
        ))}
      </div>
    );
  }

  if (!settings) {
    return (
      <div className="flex flex-col items-center justify-center py-16 text-muted-foreground mt-4">
        <Settings className="h-10 w-10 mb-3 opacity-40" />
        <p className="text-sm">Admin access required to view settings</p>
      </div>
    );
  }

  return (
    <div className="space-y-6 mt-4 max-w-lg">
      <div>
        <h3 className="text-sm font-semibold mb-3">Access Permissions</h3>
        <div className="space-y-3">
          <SettingRow
            label="Viewers can download files"
            description="Allow viewer-role members to download files from the cloud"
            checked={settings.viewer_can_download}
            onChange={() => toggle('viewer_can_download', settings.viewer_can_download)}
            disabled={updateMutation.isPending}
          />
          <SettingRow
            label="Members can upload files"
            description="Allow member-role users to contribute files"
            checked={settings.member_can_upload}
            onChange={() => toggle('member_can_upload', settings.member_can_upload)}
            disabled={updateMutation.isPending}
          />
        </div>
      </div>

      <div>
        <h3 className="text-sm font-semibold mb-3">Auto-Indexing</h3>
        <div className="space-y-3">
          <SettingRow
            label="Index new data sources"
            description="Automatically add uploaded data sources to the cloud index"
            checked={settings.auto_index_data_sources}
            onChange={() => toggle('auto_index_data_sources', settings.auto_index_data_sources)}
            disabled={updateMutation.isPending}
          />
          <SettingRow
            label="Index execution artifacts"
            description="Automatically add task execution artifacts to the cloud"
            checked={settings.auto_index_artifacts}
            onChange={() => toggle('auto_index_artifacts', settings.auto_index_artifacts)}
            disabled={updateMutation.isPending}
          />
          <SettingRow
            label="Index media assets"
            description="Automatically add media pipeline outputs to the cloud"
            checked={settings.auto_index_media}
            onChange={() => toggle('auto_index_media', settings.auto_index_media)}
            disabled={updateMutation.isPending}
          />
        </div>
      </div>

      <div>
        <h3 className="text-sm font-semibold mb-3">APN Integration</h3>
        <div className="space-y-3">
          <SettingRow
            label="Accept NATS contributions"
            description="Allow peer nodes to contribute files via the APN mesh"
            checked={settings.nats_contribution_enabled}
            onChange={() => toggle('nats_contribution_enabled', settings.nats_contribution_enabled)}
            disabled={updateMutation.isPending}
          />
        </div>
      </div>
    </div>
  );
}

function SettingRow({
  label,
  description,
  checked,
  onChange,
  disabled,
}: {
  label: string;
  description: string;
  checked: boolean;
  onChange: () => void;
  disabled: boolean;
}) {
  return (
    <div className="flex items-center justify-between p-3 bg-muted/30 rounded-lg">
      <div>
        <p className="text-xs font-medium">{label}</p>
        <p className="text-[10px] text-muted-foreground">{description}</p>
      </div>
      <Switch checked={checked} onCheckedChange={onChange} disabled={disabled} />
    </div>
  );
}
