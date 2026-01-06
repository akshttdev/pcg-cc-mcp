import { useState } from 'react';
import { cn } from '@/lib/utils';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Shield, Plus, Trash2, Globe, Lock, Loader2 } from 'lucide-react';
import type { BrowserAllowlist, PatternType } from '@/hooks/useBowser';
import {
  useAllowlist,
  useAddToAllowlist,
  useRemoveFromAllowlist,
  useCheckUrl,
} from '@/hooks/useBowser';

interface AllowlistManagerProps {
  projectId: string;
  className?: string;
}

function PatternTypeBadge({ type }: { type: PatternType }) {
  switch (type) {
    case 'glob':
      return <Badge variant="secondary">Glob</Badge>;
    case 'regex':
      return <Badge variant="outline">Regex</Badge>;
    case 'exact':
      return <Badge>Exact</Badge>;
    default:
      return <Badge variant="outline">{type}</Badge>;
  }
}

function AllowlistEntry({
  entry,
  onRemove,
  isRemoving,
  showConfirm,
  onShowConfirm,
}: {
  entry: BrowserAllowlist;
  onRemove: () => void;
  isRemoving: boolean;
  showConfirm: boolean;
  onShowConfirm: (show: boolean) => void;
}) {
  return (
    <div className="flex items-center gap-3 p-3 border rounded-lg">
      <div className="rounded-full bg-muted p-2">
        {entry.is_global ? (
          <Globe className="h-4 w-4 text-blue-500" />
        ) : (
          <Lock className="h-4 w-4 text-muted-foreground" />
        )}
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <code className="text-sm font-mono truncate">{entry.pattern}</code>
          <PatternTypeBadge type={entry.pattern_type} />
          {entry.is_global && (
            <Badge variant="outline" className="text-[10px]">
              Global
            </Badge>
          )}
        </div>
        {entry.description && (
          <p className="text-xs text-muted-foreground mt-1">{entry.description}</p>
        )}
      </div>
      {!entry.is_global && (
        <>
          <Button
            variant="ghost"
            size="sm"
            disabled={isRemoving}
            onClick={() => onShowConfirm(true)}
          >
            {isRemoving ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <Trash2 className="h-4 w-4 text-muted-foreground hover:text-destructive" />
            )}
          </Button>
          <Dialog open={showConfirm} onOpenChange={onShowConfirm}>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>Remove from allowlist?</DialogTitle>
                <DialogDescription>
                  This will remove <code>{entry.pattern}</code> from the allowlist. URLs matching this
                  pattern will be blocked.
                </DialogDescription>
              </DialogHeader>
              <DialogFooter>
                <Button variant="outline" onClick={() => onShowConfirm(false)}>
                  Cancel
                </Button>
                <Button
                  variant="destructive"
                  onClick={() => {
                    onRemove();
                    onShowConfirm(false);
                  }}
                >
                  Remove
                </Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>
        </>
      )}
    </div>
  );
}

function AddPatternForm({
  projectId,
  onSuccess,
}: {
  projectId: string;
  onSuccess: () => void;
}) {
  const [pattern, setPattern] = useState('');
  const [patternType, setPatternType] = useState<PatternType>('glob');
  const [description, setDescription] = useState('');
  const { mutate: addPattern, isPending } = useAddToAllowlist();

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!pattern.trim()) return;

    addPattern(
      {
        project_id: projectId,
        pattern: pattern.trim(),
        pattern_type: patternType,
        description: description.trim() || undefined,
      },
      {
        onSuccess: () => {
          setPattern('');
          setDescription('');
          onSuccess();
        },
      }
    );
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-4 p-4 border rounded-lg bg-muted/30">
      <div className="space-y-2">
        <Label htmlFor="pattern">URL Pattern</Label>
        <Input
          id="pattern"
          value={pattern}
          onChange={(e) => setPattern(e.target.value)}
          placeholder="e.g., *.example.com or localhost:*"
          disabled={isPending}
        />
      </div>

      <div className="grid grid-cols-2 gap-4">
        <div className="space-y-2">
          <Label>Pattern Type</Label>
          <Select
            value={patternType}
            onValueChange={(v) => setPatternType(v as PatternType)}
            disabled={isPending}
          >
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="glob">Glob (*.example.com)</SelectItem>
              <SelectItem value="regex">Regex</SelectItem>
              <SelectItem value="exact">Exact Match</SelectItem>
            </SelectContent>
          </Select>
        </div>

        <div className="space-y-2">
          <Label htmlFor="description">Description (optional)</Label>
          <Input
            id="description"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder="What is this for?"
            disabled={isPending}
          />
        </div>
      </div>

      <Button type="submit" disabled={!pattern.trim() || isPending} className="w-full">
        {isPending ? (
          <Loader2 className="h-4 w-4 mr-2 animate-spin" />
        ) : (
          <Plus className="h-4 w-4 mr-2" />
        )}
        Add to Allowlist
      </Button>
    </form>
  );
}

function UrlTester({ projectId }: { projectId: string }) {
  const [url, setUrl] = useState('');
  const { mutate: checkUrl, data: result, isPending } = useCheckUrl(projectId);

  const handleCheck = (e: React.FormEvent) => {
    e.preventDefault();
    if (!url.trim()) return;
    checkUrl(url.trim());
  };

  return (
    <form onSubmit={handleCheck} className="space-y-2">
      <Label>Test URL</Label>
      <div className="flex gap-2">
        <Input
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          placeholder="https://example.com"
          disabled={isPending}
        />
        <Button type="submit" variant="outline" disabled={!url.trim() || isPending}>
          {isPending ? <Loader2 className="h-4 w-4 animate-spin" /> : 'Test'}
        </Button>
      </div>
      {result && (
        <div
          className={cn(
            'p-2 rounded text-sm',
            result.allowed
              ? 'bg-green-50 dark:bg-green-900/20 text-green-700 dark:text-green-300'
              : 'bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-300'
          )}
        >
          {result.allowed ? '✓ URL is allowed' : '✗ URL is blocked'}
        </div>
      )}
    </form>
  );
}

export function AllowlistManager({ projectId, className }: AllowlistManagerProps) {
  const { data: entries = [], isLoading, refetch } = useAllowlist(projectId);
  const { mutate: removeEntry } = useRemoveFromAllowlist();
  const [showAddForm, setShowAddForm] = useState(false);
  const [removingId, setRemovingId] = useState<string | null>(null);
  const [confirmingId, setConfirmingId] = useState<string | null>(null);

  const globalEntries = entries.filter((e) => e.is_global);
  const projectEntries = entries.filter((e) => !e.is_global);

  const handleRemove = (id: string) => {
    setRemovingId(id);
    removeEntry(id, {
      onSettled: () => setRemovingId(null),
    });
  };

  return (
    <Card className={cn('h-full flex flex-col', className)}>
      <CardHeader className="pb-2">
        <CardTitle className="text-sm font-medium flex items-center gap-2">
          <Shield className="h-4 w-4" />
          URL Allowlist
          <Badge variant="secondary" className="ml-auto">
            {entries.length}
          </Badge>
        </CardTitle>
      </CardHeader>
      <CardContent className="flex-1 overflow-hidden flex flex-col gap-4">
        {/* URL Tester */}
        <UrlTester projectId={projectId} />

        {/* Add form toggle */}
        {!showAddForm ? (
          <Button
            variant="outline"
            className="w-full"
            onClick={() => setShowAddForm(true)}
          >
            <Plus className="h-4 w-4 mr-2" />
            Add Pattern
          </Button>
        ) : (
          <AddPatternForm
            projectId={projectId}
            onSuccess={() => {
              setShowAddForm(false);
              refetch();
            }}
          />
        )}

        {/* Entries list */}
        <ScrollArea className="flex-1">
          <div className="space-y-4">
            {/* Project-specific entries */}
            {projectEntries.length > 0 && (
              <div className="space-y-2">
                <h4 className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
                  Project Patterns
                </h4>
                {projectEntries.map((entry) => (
                  <AllowlistEntry
                    key={entry.id}
                    entry={entry}
                    onRemove={() => handleRemove(entry.id)}
                    isRemoving={removingId === entry.id}
                    showConfirm={confirmingId === entry.id}
                    onShowConfirm={(show) => setConfirmingId(show ? entry.id : null)}
                  />
                ))}
              </div>
            )}

            {/* Global entries */}
            {globalEntries.length > 0 && (
              <div className="space-y-2">
                <h4 className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
                  Global Patterns (Read-only)
                </h4>
                {globalEntries.map((entry) => (
                  <AllowlistEntry
                    key={entry.id}
                    entry={entry}
                    onRemove={() => {}}
                    isRemoving={false}
                    showConfirm={false}
                    onShowConfirm={() => {}}
                  />
                ))}
              </div>
            )}

            {entries.length === 0 && !isLoading && (
              <div className="text-center text-muted-foreground py-8 text-sm">
                No allowlist patterns configured
              </div>
            )}
          </div>
        </ScrollArea>
      </CardContent>
    </Card>
  );
}
