import { useMutation } from '@tanstack/react-query';
import { Phone, Video } from 'lucide-react';
import { useState } from 'react';
import { callScheduling as tid } from 'shared/testids';
import { toast } from 'sonner';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { ListItem } from '@/components/ui/list-item';
import { SectionHeader } from '@/components/ui/section-header';
import { crmDealsApi } from '@/lib/api/crm';
import { cn } from '@/lib/utils';
import type { CrmDealWithContact } from '@/types/crm';

const CALL_METHODS = ['Phone', 'Video', 'In-Person'] as const;
const CALL_STATUSES = ['scheduled', 'completed', 'cancelled'] as const;

interface CallSchedulingSectionProps {
  deal: CrmDealWithContact;
  customFields: Record<string, unknown>;
  invalidateKanban: () => void;
}

export function CallSchedulingSection({
  deal,
  customFields,
  invalidateKanban,
}: CallSchedulingSectionProps) {
  const [editing, setEditing] = useState<'discovery' | 'presentation' | null>(null);

  const saveMutation = useMutation({
    mutationFn: (fields: Record<string, unknown>) => {
      const merged = { ...customFields, ...fields };
      return crmDealsApi.updateDeal(deal.id, {
        custom_fields: merged,
      });
    },
    onSuccess: () => {
      toast.success('Call schedule updated');
      setEditing(null);
      invalidateKanban();
    },
    onError: () => toast.error('Failed to save call schedule'),
  });

  const renderCallRow = (
    type: 'discovery' | 'presentation',
    label: string,
    icon: React.ElementType,
  ) => {
    const Icon = icon;
    const dateKey = `${type}_call_date`;
    const methodKey = `${type}_call_method`;
    const statusKey = `${type}_call_status`;
    const date = customFields[dateKey] as string | undefined;
    const method = (customFields[methodKey] as string) || 'Video';
    const status = (customFields[statusKey] as string) || 'scheduled';
    const isEditing = editing === type;

    if (isEditing) {
      return (
        <Card className="bg-muted/30 border-primary/30">
          <CardContent className="p-3 space-y-2">
            <div className="flex items-center gap-2 text-sm font-medium">
              <Icon className="h-3.5 w-3.5" />
              {label}
            </div>
            <div className="grid grid-cols-2 gap-2">
              <div>
                <label className="text-xs text-muted-foreground uppercase">Date</label>
                <input
                  type="date"
                  defaultValue={date || ''}
                  className="w-full h-8 px-2 text-sm border rounded bg-background"
                  id={`${type}-date`}
                  data-testid={tid.date(type)}
                />
              </div>
              <div>
                <label className="text-xs text-muted-foreground uppercase">Method</label>
                <select
                  defaultValue={method}
                  className="w-full h-8 px-2 text-sm border rounded bg-background"
                  id={`${type}-method`}
                  data-testid={tid.method(type)}
                >
                  {CALL_METHODS.map((m) => (
                    <option key={m} value={m}>{m}</option>
                  ))}
                </select>
              </div>
            </div>
            <div>
              <label className="text-xs text-muted-foreground uppercase">Status</label>
              <select
                defaultValue={status}
                className="w-full h-8 px-2 text-sm border rounded bg-background"
                id={`${type}-status`}
                data-testid={tid.status(type)}
              >
                {CALL_STATUSES.map((s) => (
                  <option key={s} value={s}>{s.charAt(0).toUpperCase() + s.slice(1)}</option>
                ))}
              </select>
            </div>
            <div className="flex gap-1.5">
              <Button
                size="sm"
                className="h-7 text-xs"
                data-testid={tid.save(type)}
                disabled={saveMutation.isPending}
                onClick={() => {
                  const dateEl = document.getElementById(`${type}-date`) as HTMLInputElement;
                  const methodEl = document.getElementById(`${type}-method`) as HTMLSelectElement;
                  const statusEl = document.getElementById(`${type}-status`) as HTMLSelectElement;
                  saveMutation.mutate({
                    [dateKey]: dateEl?.value || null,
                    [methodKey]: methodEl?.value || 'Video',
                    [statusKey]: statusEl?.value || 'scheduled',
                  });
                }}
              >
                Save
              </Button>
              <Button size="sm" variant="ghost" className="h-7 text-xs" onClick={() => setEditing(null)}>
                Cancel
              </Button>
            </div>
          </CardContent>
        </Card>
      );
    }

    return (
      <ListItem
        icon={icon}
        title={label}
        size="sm"
        onClick={() => setEditing(type)}
        className="-mx-2"
        data-testid={tid.row(type)}
        actions={
          <div className="flex items-center gap-2">
            {method && date && (
              <span className="text-xs text-muted-foreground">{method}</span>
            )}
            <Badge
              variant="outline"
              className={cn(
                'text-xs',
                status === 'completed'
                  ? 'text-green-500 border-green-500/30'
                  : status === 'cancelled'
                    ? 'text-red-500 border-red-500/30'
                    : 'text-muted-foreground'
              )}
            >
              {date || 'Not scheduled'}
            </Badge>
          </div>
        }
      />
    );
  };

  return (
    <div>
      <SectionHeader icon={Phone} title="Call Scheduling" />
      <Card className="bg-muted/30 border-border/60">
        <CardContent className="p-3 space-y-1">
          {renderCallRow('discovery', 'Discovery Call', Phone)}
          {renderCallRow('presentation', 'Presentation Call', Video)}
        </CardContent>
      </Card>
    </div>
  );
}
