import NiceModal, { useModal } from '@ebay/nice-modal-react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { entityConversionApi, organizationsApi } from '@/lib/api';
import type { EntityType } from '@/lib/api';
import { useNavigate } from 'react-router-dom';
import { ArrowRight, AlertTriangle } from 'lucide-react';

interface ConvertEntityDialogProps {
  sourceType: EntityType;
  sourceId: string;
  sourceName: string;
}

const CONVERSION_OPTIONS: Record<EntityType, EntityType[]> = {
  organization: ['client', 'project'],
  client: ['organization', 'project'],
  project: ['client', 'organization'],
};

const TYPE_LABELS: Record<EntityType, string> = {
  organization: 'Organization',
  client: 'Client',
  project: 'Project',
};

export const ConvertEntityDialog = NiceModal.create(
  ({ sourceType, sourceId, sourceName }: ConvertEntityDialogProps) => {
    const modal = useModal();
    const navigate = useNavigate();
    const queryClient = useQueryClient();
    const [targetType, setTargetType] = useState<EntityType | ''>('');
    const [targetParentId, setTargetParentId] = useState<string>('');
    const [isConverting, setIsConverting] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const needsParent =
      (sourceType === 'organization' && (targetType === 'client' || targetType === 'project'));

    const { data: organizations = [] } = useQuery({
      queryKey: ['organizations'],
      queryFn: () => organizationsApi.getAll(),
      enabled: needsParent,
    });

    const handleConvert = async () => {
      if (!targetType) return;
      setIsConverting(true);
      setError(null);

      try {
        const result = await entityConversionApi.convert({
          source_type: sourceType,
          source_id: sourceId,
          target_type: targetType,
          target_parent_id: needsParent ? targetParentId || undefined : undefined,
        });

        queryClient.invalidateQueries({ queryKey: ['sidebarTree'] });
        queryClient.invalidateQueries({ queryKey: ['organizations'] });

        modal.resolve(result);
        modal.hide();

        // Navigate to the new entity
        if (result.new_type === 'organization') {
          navigate(`/organizations/${result.new_id}`);
        } else if (result.new_type === 'project') {
          navigate(`/projects/${result.new_id}`);
        }
      } catch (err: any) {
        setError(err?.message || 'Conversion failed');
      } finally {
        setIsConverting(false);
      }
    };

    return (
      <Dialog open={modal.visible} onOpenChange={(open) => !open && modal.hide()}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Convert Entity</DialogTitle>
            <DialogDescription>
              Convert <strong>{sourceName}</strong> ({TYPE_LABELS[sourceType]}) to a different entity type.
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-2">
            <div className="flex items-center gap-3 text-sm">
              <span className="px-2 py-1 bg-muted rounded font-medium">
                {TYPE_LABELS[sourceType]}
              </span>
              <ArrowRight className="h-4 w-4 text-muted-foreground" />
              <Select value={targetType} onValueChange={(v) => setTargetType(v as EntityType)}>
                <SelectTrigger className="w-[180px]">
                  <SelectValue placeholder="Select type..." />
                </SelectTrigger>
                <SelectContent>
                  {CONVERSION_OPTIONS[sourceType].map((type) => (
                    <SelectItem key={type} value={type}>
                      {TYPE_LABELS[type]}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            {needsParent && (
              <div className="space-y-2">
                <Label>Destination Organization</Label>
                <Select value={targetParentId} onValueChange={setTargetParentId}>
                  <SelectTrigger>
                    <SelectValue placeholder="Select organization..." />
                  </SelectTrigger>
                  <SelectContent>
                    {organizations
                      .filter((org) => org.id !== sourceId)
                      .map((org) => (
                        <SelectItem key={org.id} value={org.id}>
                          {org.name}
                        </SelectItem>
                      ))}
                  </SelectContent>
                </Select>
              </div>
            )}

            {targetType && (
              <div className="flex items-start gap-2 p-3 bg-amber-50 dark:bg-amber-950/30 rounded-md text-sm">
                <AlertTriangle className="h-4 w-4 text-amber-500 mt-0.5 shrink-0" />
                <div>
                  <p className="font-medium text-amber-700 dark:text-amber-300">
                    This action will:
                  </p>
                  <ul className="mt-1 text-amber-600 dark:text-amber-400 list-disc list-inside space-y-0.5">
                    <li>Create a new {TYPE_LABELS[targetType].toLowerCase()}</li>
                    <li>Move all child entities to the new {TYPE_LABELS[targetType].toLowerCase()}</li>
                    <li>Migrate members where possible</li>
                    <li>Soft-delete the original {TYPE_LABELS[sourceType].toLowerCase()}</li>
                  </ul>
                </div>
              </div>
            )}

            {error && (
              <p className="text-sm text-destructive">{error}</p>
            )}
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={() => modal.hide()}>
              Cancel
            </Button>
            <Button
              onClick={handleConvert}
              disabled={!targetType || isConverting || (needsParent && !targetParentId)}
            >
              {isConverting ? 'Converting...' : 'Convert'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    );
  }
);

NiceModal.register('convert-entity', ConvertEntityDialog);
