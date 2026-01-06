import { useState, useCallback } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Layers,
  Plus,
  Settings2,
  Clock,
  Calendar,
  RefreshCw,
  Trash2,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { CategoryQueue, ContentCategory, SocialAccount } from '@/types/social';
import { CATEGORY_COLORS, CATEGORY_ICONS } from '@/types/social';

interface CategoryQueueManagerProps {
  queues: CategoryQueue[];
  accounts: SocialAccount[];
  onQueueCreate?: (queue: Omit<CategoryQueue, 'id'>) => void;
  onQueueUpdate?: (id: string, updates: Partial<CategoryQueue>) => void;
  onQueueDelete?: (id: string) => void;
  className?: string;
}

const DAYS_OF_WEEK = [
  { value: 0, label: 'Sun' },
  { value: 1, label: 'Mon' },
  { value: 2, label: 'Tue' },
  { value: 3, label: 'Wed' },
  { value: 4, label: 'Thu' },
  { value: 5, label: 'Fri' },
  { value: 6, label: 'Sat' },
];

const DEFAULT_OPTIMAL_TIMES: Record<ContentCategory, string[]> = {
  events: ['18:00', '20:00'],
  drinks: ['17:00', '19:00'],
  community: ['12:00', '18:00'],
  vibe: ['21:00', '22:00'],
  behind_scenes: ['10:00', '14:00'],
  promo: ['09:00', '17:00'],
  education: ['08:00', '12:00'],
  entertainment: ['19:00', '21:00'],
};

function QueueCard({
  queue,
  account,
  onUpdate,
  onDelete,
}: {
  queue: CategoryQueue;
  account?: SocialAccount;
  onUpdate?: (updates: Partial<CategoryQueue>) => void;
  onDelete?: () => void;
}) {
  const [isEditing, setIsEditing] = useState(false);
  const categoryColor = CATEGORY_COLORS[queue.category];

  const toggleDay = useCallback((day: number) => {
    const newDays = queue.preferred_days.includes(day)
      ? queue.preferred_days.filter(d => d !== day)
      : [...queue.preferred_days, day].sort();
    onUpdate?.({ preferred_days: newDays });
  }, [queue.preferred_days, onUpdate]);

  return (
    <Card className="overflow-hidden">
      <div
        className="h-1"
        style={{ backgroundColor: categoryColor }}
      />
      <CardContent className="p-4 space-y-4">
        {/* Header */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <span className="text-2xl">{CATEGORY_ICONS[queue.category]}</span>
            <div>
              <h4 className="font-medium capitalize">{queue.category}</h4>
              {account && (
                <p className="text-xs text-muted-foreground">
                  {account.account_name} ({account.platform})
                </p>
              )}
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Switch
              checked={queue.is_active}
              onCheckedChange={(is_active) => onUpdate?.({ is_active })}
            />
            <Button
              variant="ghost"
              size="icon"
              onClick={() => setIsEditing(!isEditing)}
            >
              <Settings2 className="h-4 w-4" />
            </Button>
            {onDelete && (
              <Button
                variant="ghost"
                size="icon"
                className="text-destructive hover:text-destructive"
                onClick={onDelete}
              >
                <Trash2 className="h-4 w-4" />
              </Button>
            )}
          </div>
        </div>

        {/* Stats */}
        <div className="flex items-center gap-4 text-sm">
          <div className="flex items-center gap-1 text-muted-foreground">
            <RefreshCw className="h-4 w-4" />
            <span>{queue.posts_per_week}/week</span>
          </div>
          <div className="flex items-center gap-1 text-muted-foreground">
            <Clock className="h-4 w-4" />
            <span>{queue.optimal_times.join(', ')}</span>
          </div>
        </div>

        {/* Days */}
        <div className="flex gap-1">
          {DAYS_OF_WEEK.map(({ value, label }) => {
            const isActive = queue.preferred_days.includes(value);
            return (
              <button
                key={value}
                onClick={() => toggleDay(value)}
                disabled={!isEditing && !queue.is_active}
                className={cn(
                  'flex-1 py-1 text-xs font-medium rounded transition-colors',
                  isActive
                    ? 'text-white'
                    : 'bg-muted text-muted-foreground',
                  !isEditing && !queue.is_active && 'opacity-50 cursor-not-allowed'
                )}
                style={isActive ? { backgroundColor: categoryColor } : undefined}
              >
                {label}
              </button>
            );
          })}
        </div>

        {/* Edit panel */}
        {isEditing && (
          <div className="space-y-3 pt-2 border-t">
            <div className="grid grid-cols-2 gap-3">
              <div className="space-y-1">
                <Label className="text-xs">Posts per week</Label>
                <Input
                  type="number"
                  min={1}
                  max={21}
                  value={queue.posts_per_week}
                  onChange={(e) => onUpdate?.({ posts_per_week: parseInt(e.target.value) || 1 })}
                  className="h-8"
                />
              </div>
              <div className="space-y-1">
                <Label className="text-xs">Optimal times</Label>
                <Input
                  value={queue.optimal_times.join(', ')}
                  onChange={(e) => onUpdate?.({
                    optimal_times: e.target.value.split(',').map(t => t.trim()).filter(Boolean)
                  })}
                  placeholder="HH:MM, HH:MM"
                  className="h-8"
                />
              </div>
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function CreateQueueForm({
  accounts,
  existingCategories,
  onSubmit,
  onCancel,
}: {
  accounts: SocialAccount[];
  existingCategories: ContentCategory[];
  onSubmit: (queue: Omit<CategoryQueue, 'id'>) => void;
  onCancel: () => void;
}) {
  const [accountId, setAccountId] = useState(accounts[0]?.id || '');
  const [category, setCategory] = useState<ContentCategory>('events');
  const [postsPerWeek, setPostsPerWeek] = useState(3);

  const availableCategories = (Object.keys(CATEGORY_ICONS) as ContentCategory[])
    .filter(cat => !existingCategories.includes(cat));

  const handleSubmit = useCallback(() => {
    if (!accountId) return;

    onSubmit({
      account_id: accountId,
      category,
      posts_per_week: postsPerWeek,
      optimal_times: DEFAULT_OPTIMAL_TIMES[category],
      preferred_days: [1, 2, 3, 4, 5], // Mon-Fri by default
      is_active: true,
    });
  }, [accountId, category, postsPerWeek, onSubmit]);

  return (
    <Card>
      <CardContent className="p-4 space-y-4">
        <h4 className="font-medium">Add Category Queue</h4>

        <div className="space-y-3">
          <div className="space-y-1">
            <Label className="text-xs">Account</Label>
            <Select value={accountId} onValueChange={setAccountId}>
              <SelectTrigger className="h-9">
                <SelectValue placeholder="Select account" />
              </SelectTrigger>
              <SelectContent>
                {accounts.map(account => (
                  <SelectItem key={account.id} value={account.id}>
                    {account.account_name} ({account.platform})
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-1">
            <Label className="text-xs">Category</Label>
            <Select value={category} onValueChange={(v) => setCategory(v as ContentCategory)}>
              <SelectTrigger className="h-9">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {availableCategories.map(cat => (
                  <SelectItem key={cat} value={cat}>
                    <span className="flex items-center gap-2">
                      <span>{CATEGORY_ICONS[cat]}</span>
                      <span className="capitalize">{cat}</span>
                    </span>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-1">
            <Label className="text-xs">Posts per week</Label>
            <Input
              type="number"
              min={1}
              max={21}
              value={postsPerWeek}
              onChange={(e) => setPostsPerWeek(parseInt(e.target.value) || 1)}
              className="h-9"
            />
          </div>
        </div>

        <div className="flex gap-2 pt-2">
          <Button variant="outline" size="sm" onClick={onCancel} className="flex-1">
            Cancel
          </Button>
          <Button size="sm" onClick={handleSubmit} className="flex-1">
            Add Queue
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

export function CategoryQueueManager({
  queues,
  accounts,
  onQueueCreate,
  onQueueUpdate,
  onQueueDelete,
  className,
}: CategoryQueueManagerProps) {
  const [isCreating, setIsCreating] = useState(false);

  const existingCategories = queues.map(q => q.category);
  const activeQueues = queues.filter(q => q.is_active);
  const totalPostsPerWeek = activeQueues.reduce((sum, q) => sum + q.posts_per_week, 0);

  return (
    <Card className={className}>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Layers className="h-5 w-5 text-muted-foreground" />
            <CardTitle className="text-lg">Category Queues</CardTitle>
          </div>
          <div className="flex items-center gap-2">
            <Badge variant="secondary">
              <Calendar className="h-3 w-3 mr-1" />
              {totalPostsPerWeek} posts/week
            </Badge>
            {!isCreating && onQueueCreate && (
              <Button size="sm" onClick={() => setIsCreating(true)}>
                <Plus className="h-4 w-4 mr-1" />
                Add Queue
              </Button>
            )}
          </div>
        </div>
      </CardHeader>

      <CardContent className="space-y-3">
        {isCreating && (
          <CreateQueueForm
            accounts={accounts}
            existingCategories={existingCategories}
            onSubmit={(queue) => {
              onQueueCreate?.(queue);
              setIsCreating(false);
            }}
            onCancel={() => setIsCreating(false)}
          />
        )}

        {queues.length === 0 && !isCreating ? (
          <div className="text-center py-8 text-muted-foreground">
            <Layers className="h-12 w-12 mx-auto mb-2 opacity-50" />
            <p>No category queues configured</p>
            <p className="text-sm">Add queues to automate content scheduling</p>
          </div>
        ) : (
          <div className="grid gap-3 md:grid-cols-2">
            {queues.map((queue) => (
              <QueueCard
                key={queue.id}
                queue={queue}
                account={accounts.find(a => a.id === queue.account_id)}
                onUpdate={(updates) => onQueueUpdate?.(queue.id, updates)}
                onDelete={onQueueDelete ? () => onQueueDelete(queue.id) : undefined}
              />
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
