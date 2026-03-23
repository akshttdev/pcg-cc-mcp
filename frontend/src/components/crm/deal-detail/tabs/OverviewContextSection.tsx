import { useMutation } from '@tanstack/react-query';
import { MessageSquare, Zap } from 'lucide-react';
import { useState } from 'react';
import { toast } from 'sonner';

import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { SectionHeader } from '@/components/ui/section-header';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import { crmDealsApi } from '@/lib/api/crm';
import type { CrmDealWithContact } from '@/types/crm';

interface OverviewContextSectionProps {
  deal: CrmDealWithContact;
  invalidateKanban: () => void;
}

export function OverviewContextSection({ deal, invalidateKanban }: OverviewContextSectionProps) {
  const [contextText, setContextText] = useState(deal.description ?? '');
  const [editingContext, setEditingContext] = useState(false);

  const saveContext = useMutation({
    mutationFn: () =>
      crmDealsApi.updateDeal(deal.id, { description: contextText }),
    onSuccess: () => {
      toast.success('Context saved');
      setEditingContext(false);
      invalidateKanban();
    },
    onError: () => toast.error('Failed to save context — please try again.'),
  });

  const toggleExpedite = useMutation({
    mutationFn: () =>
      crmDealsApi.updateDeal(deal.id, { expedited: deal.expedited ? 0 : 1 }),
    onSuccess: () => {
      invalidateKanban();
    },
  });

  return (
    <>
      {/* Operator Context */}
      <div>
        <SectionHeader icon={MessageSquare} title="Operator Context" />
        {editingContext ? (
          <Card className="bg-muted/30 border-border/60">
            <CardContent className="p-3 space-y-2">
              <Textarea
                value={contextText}
                onChange={(e) => setContextText(e.target.value)}
                placeholder="Who is this person? What does their company do? What's the opportunity? Add any background, relationship context, or budget signals..."
                className="min-h-[100px] text-sm"
              />
              <div className="flex gap-1.5">
                <Button
                  size="sm"
                  className="h-7 text-xs gap-1"
                  onClick={() => saveContext.mutate()}
                  disabled={saveContext.isPending}
                >
                  Save Context
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  className="h-7 text-xs"
                  onClick={() => {
                    setEditingContext(false);
                    setContextText(deal.description ?? '');
                  }}
                >
                  Cancel
                </Button>
              </div>
            </CardContent>
          </Card>
        ) : deal.description ? (
          <Card
            className="bg-muted/30 border-border/60 cursor-pointer hover:border-primary/40 transition-colors"
            onClick={() => setEditingContext(true)}
          >
            <CardContent className="p-3">
              <p className="text-sm whitespace-pre-wrap leading-relaxed">
                {deal.description}
              </p>
              <p className="text-xs text-muted-foreground mt-2">
                Click to edit
              </p>
            </CardContent>
          </Card>
        ) : (
          <Card
            className="bg-muted/30 border-dashed border-amber-500/30 cursor-pointer hover:border-amber-500/60 transition-colors"
            onClick={() => setEditingContext(true)}
          >
            <CardContent className="p-3 text-center">
              <p className="text-sm text-muted-foreground">
                No context yet — add notes about this lead
              </p>
              <p className="text-xs text-amber-500 mt-1">
                Required before advancing from Intel
              </p>
            </CardContent>
          </Card>
        )}
      </div>

      {/* Expedite Toggle */}
      <div className="flex items-center justify-between px-1">
        <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <Zap className="h-3 w-3" />
          <span>Expedite</span>
        </div>
        <Switch
          checked={!!deal.expedited}
          onCheckedChange={() => toggleExpedite.mutate()}
        />
      </div>
    </>
  );
}
