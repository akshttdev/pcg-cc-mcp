import { Badge } from '@/components/ui/badge';
import { parseJsonArray } from '@/lib/formatters';

const TAG_COLORS = [
  'bg-blue-100 text-blue-800 dark:bg-blue-950/40 dark:text-blue-300',
  'bg-purple-100 text-purple-800 dark:bg-purple-950/40 dark:text-purple-300',
  'bg-emerald-100 text-emerald-800 dark:bg-emerald-950/40 dark:text-emerald-300',
  'bg-amber-100 text-amber-800 dark:bg-amber-950/40 dark:text-amber-300',
  'bg-rose-100 text-rose-800 dark:bg-rose-950/40 dark:text-rose-300',
  'bg-cyan-100 text-cyan-800 dark:bg-cyan-950/40 dark:text-cyan-300',
  'bg-indigo-100 text-indigo-800 dark:bg-indigo-950/40 dark:text-indigo-300',
  'bg-orange-100 text-orange-800 dark:bg-orange-950/40 dark:text-orange-300',
] as const;

function hashTagColor(tag: string): string {
  let hash = 0;
  for (let i = 0; i < tag.length; i++) {
    hash = ((hash << 5) - hash + tag.charCodeAt(i)) | 0;
  }
  return TAG_COLORS[Math.abs(hash) % TAG_COLORS.length];
}

interface TagChipsProps {
  tags: string | null | undefined;
  maxVisible?: number;
  size?: 'xs' | 'sm';
}

export function TagChips({ tags, maxVisible = 2, size = 'xs' }: TagChipsProps) {
  const parsed = parseJsonArray(tags);
  if (parsed.length === 0) return null;

  const visible = parsed.slice(0, maxVisible);
  const overflow = parsed.length - maxVisible;
  const sizeClass = size === 'xs' ? 'text-xs px-1.5 py-0' : 'text-xs px-2 py-0.5';

  return (
    <span className="inline-flex items-center gap-1 flex-wrap">
      {visible.map((tag) => (
        <Badge
          key={tag}
          variant="outline"
          className={`${hashTagColor(tag)} border-0 ${sizeClass}`}
        >
          {tag}
        </Badge>
      ))}
      {overflow > 0 && (
        <span className={`text-muted-foreground ${size === 'xs' ? 'text-xs' : 'text-xs'}`}>
          +{overflow}
        </span>
      )}
    </span>
  );
}
