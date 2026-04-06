import { Layers, Loader2, Sparkles, Users } from 'lucide-react';
import { useState } from 'react';
import { Link } from 'react-router-dom';
const LayersIcon = Layers;
import { toast } from 'sonner';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { EmptyState } from '@/components/ui/empty-state';
import { type CrmContactRecord } from '@/lib/api';

export function ContactsTab({
  contacts,
  onResearch,
}: {
  contacts: CrmContactRecord[];
  onResearch: (id: string) => void;
}) {
  const [runningDeep, setRunningDeep] = useState<Set<string>>(new Set());

  const handleDeepResearch = async (personId: string) => {
    setRunningDeep((prev) => new Set([...prev, personId]));
    try {
      const { intelligenceApi } = await import('@/lib/api');
      await intelligenceApi.triggerNextPass(personId);
      toast.success('Deep research pass queued');
    } catch {
      toast.error('Failed to queue deep research');
    } finally {
      setRunningDeep((prev) => {
        const s = new Set(prev);
        s.delete(personId);
        return s;
      });
    }
  };

  if (contacts.length === 0) {
    return (
      <EmptyState
        icon={Users}
        title="No contacts linked"
        description="Research this company to auto-discover contacts"
        className="h-32"
      />
    );
  }
  return (
    <div className="grid gap-2 sm:grid-cols-2">
      {contacts.map((c) => (
        <Card key={c.id} className="hover:shadow-sm transition-shadow">
          <CardContent className="pt-3 pb-3">
            <div className="flex items-center gap-3">
              <div className="h-9 w-9 rounded-full bg-primary/10 flex items-center justify-center text-sm font-semibold text-primary shrink-0">
                {(c.full_name ?? '').slice(0, 2).toUpperCase()}
              </div>
              <div className="flex-1 min-w-0">
                <Link
                  to={`/contacts/${c.id}`}
                  className="font-medium text-sm hover:text-primary transition-colors truncate block"
                >
                  {c.full_name ?? 'Unnamed'}
                </Link>
                <p className="text-xs text-muted-foreground truncate">
                  {c.job_title ?? c.lifecycle_stage}
                  {c.email ? ` \u00b7 ${c.email}` : ''}
                </p>
                {(c.research_pass_count ?? 0) > 0 && (
                  <p className="text-xs text-blue-500/70 mt-0.5">
                    {c.research_pass_count} research{' '}
                    {(c.research_pass_count ?? 0) === 1 ? 'pass' : 'passes'}{' '}
                    \u00b7 {c.research_depth ?? 'shallow'}
                  </p>
                )}
              </div>
              <div className="flex items-center gap-1 shrink-0">
                {c.intelligence_status === 'done' && (
                  <Badge className="text-xs border-0 bg-green-100 text-green-700 px-1.5 py-0">
                    Intel \u2713
                  </Badge>
                )}
                <Button
                  size="sm"
                  variant="ghost"
                  className="h-7 w-7 p-0"
                  onClick={() => onResearch(c.id)}
                  title="Quick Research"
                >
                  <Sparkles className="h-3.5 w-3.5" />
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  className="h-7 w-7 p-0 text-blue-500 hover:text-blue-400"
                  disabled={runningDeep.has(c.id)}
                  onClick={() => handleDeepResearch(c.id)}
                  title="Run next deep research pass"
                >
                  {runningDeep.has(c.id) ? (
                    <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  ) : (
                    <LayersIcon className="h-3.5 w-3.5" />
                  )}
                </Button>
              </div>
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}
