import { useState, useMemo, useCallback } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  ChevronLeft,
  ChevronRight,
  Calendar as CalendarIcon,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type {
  SocialPost,
  CalendarDay,
  ContentCategory,
  SocialPlatform,
} from '@/types/social';
import { CATEGORY_COLORS, CATEGORY_ICONS, PLATFORM_ICONS } from '@/types/social';

interface ContentCalendarProps {
  posts: SocialPost[];
  onDateClick?: (date: Date) => void;
  onPostClick?: (post: SocialPost) => void;
  onPostDrop?: (postId: string, newDate: Date) => void;
  className?: string;
}

const DAYS_OF_WEEK = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
const MONTHS = [
  'January', 'February', 'March', 'April', 'May', 'June',
  'July', 'August', 'September', 'October', 'November', 'December'
];

function getCalendarDays(year: number, month: number, posts: SocialPost[]): CalendarDay[] {
  const firstDay = new Date(year, month, 1);
  const startDate = new Date(firstDay);
  startDate.setDate(startDate.getDate() - firstDay.getDay());

  const today = new Date();
  today.setHours(0, 0, 0, 0);

  const days: CalendarDay[] = [];

  for (let i = 0; i < 42; i++) { // 6 weeks
    const date = new Date(startDate);
    date.setDate(startDate.getDate() + i);

    const dateStr = date.toISOString().split('T')[0];
    const dayPosts = posts.filter(post => {
      const postDate = post.scheduled_for || post.published_at || post.created_at;
      return postDate?.split('T')[0] === dateStr;
    });

    days.push({
      date: new Date(date),
      posts: dayPosts,
      isToday: date.getTime() === today.getTime(),
      isCurrentMonth: date.getMonth() === month,
    });
  }

  return days;
}

function PostPill({
  post,
  onClick,
  onDragStart,
}: {
  post: SocialPost;
  onClick?: () => void;
  onDragStart?: (e: React.DragEvent) => void;
}) {
  const categoryColor = CATEGORY_COLORS[post.category];
  const platforms = Object.keys(post.platform_adaptations) as SocialPlatform[];

  return (
    <div
      draggable
      onDragStart={onDragStart}
      onClick={(e) => {
        e.stopPropagation();
        onClick?.();
      }}
      className="flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium cursor-pointer hover:opacity-80 transition-opacity truncate"
      style={{ backgroundColor: categoryColor + '20', color: categoryColor }}
    >
      <span>{CATEGORY_ICONS[post.category]}</span>
      <span className="truncate flex-1">
        {post.content_blocks[0]?.content?.slice(0, 20) || 'Untitled'}
      </span>
      <span className="flex gap-0.5">
        {platforms.slice(0, 2).map(p => (
          <span key={p} className="opacity-60">{PLATFORM_ICONS[p]}</span>
        ))}
        {platforms.length > 2 && (
          <span className="opacity-60">+{platforms.length - 2}</span>
        )}
      </span>
    </div>
  );
}

function CalendarCell({
  day,
  onDateClick,
  onPostClick,
  onPostDrop,
}: {
  day: CalendarDay;
  onDateClick?: (date: Date) => void;
  onPostClick?: (post: SocialPost) => void;
  onPostDrop?: (postId: string, date: Date) => void;
}) {
  const [isDragOver, setIsDragOver] = useState(false);

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setIsDragOver(true);
  }, []);

  const handleDragLeave = useCallback(() => {
    setIsDragOver(false);
  }, []);

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setIsDragOver(false);
    const postId = e.dataTransfer.getData('postId');
    if (postId && onPostDrop) {
      onPostDrop(postId, day.date);
    }
  }, [day.date, onPostDrop]);

  const handleDragStart = useCallback((e: React.DragEvent, post: SocialPost) => {
    e.dataTransfer.setData('postId', post.id);
  }, []);

  return (
    <div
      onClick={() => onDateClick?.(day.date)}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      className={cn(
        'min-h-[100px] p-1 border-r border-b cursor-pointer transition-colors',
        !day.isCurrentMonth && 'bg-muted/30 text-muted-foreground',
        day.isToday && 'bg-blue-50',
        isDragOver && 'bg-blue-100 ring-2 ring-blue-400 ring-inset'
      )}
    >
      <div className="flex items-center justify-between mb-1">
        <span className={cn(
          'text-sm font-medium',
          day.isToday && 'text-blue-600'
        )}>
          {day.date.getDate()}
        </span>
        {day.posts.length > 0 && (
          <Badge variant="secondary" className="text-[10px] h-4 px-1">
            {day.posts.length}
          </Badge>
        )}
      </div>
      <div className="space-y-0.5">
        {day.posts.slice(0, 3).map((post) => (
          <PostPill
            key={post.id}
            post={post}
            onClick={() => onPostClick?.(post)}
            onDragStart={(e) => handleDragStart(e, post)}
          />
        ))}
        {day.posts.length > 3 && (
          <div className="text-[10px] text-muted-foreground pl-1">
            +{day.posts.length - 3} more
          </div>
        )}
      </div>
    </div>
  );
}

export function ContentCalendar({
  posts,
  onDateClick,
  onPostClick,
  onPostDrop,
  className,
}: ContentCalendarProps) {
  const [currentDate, setCurrentDate] = useState(new Date());
  const year = currentDate.getFullYear();
  const month = currentDate.getMonth();

  const calendarDays = useMemo(
    () => getCalendarDays(year, month, posts),
    [year, month, posts]
  );

  const goToPrevMonth = useCallback(() => {
    setCurrentDate(new Date(year, month - 1, 1));
  }, [year, month]);

  const goToNextMonth = useCallback(() => {
    setCurrentDate(new Date(year, month + 1, 1));
  }, [year, month]);

  const goToToday = useCallback(() => {
    setCurrentDate(new Date());
  }, []);

  // Category summary for the month
  const categoryCounts = useMemo(() => {
    const counts: Partial<Record<ContentCategory, number>> = {};
    const monthPosts = posts.filter(post => {
      const postDate = new Date(post.scheduled_for || post.published_at || post.created_at);
      return postDate.getMonth() === month && postDate.getFullYear() === year;
    });
    for (const post of monthPosts) {
      counts[post.category] = (counts[post.category] || 0) + 1;
    }
    return counts;
  }, [posts, month, year]);

  return (
    <Card className={className}>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <CalendarIcon className="h-5 w-5 text-muted-foreground" />
            <CardTitle className="text-lg">
              {MONTHS[month]} {year}
            </CardTitle>
          </div>
          <div className="flex items-center gap-1">
            <Button variant="outline" size="sm" onClick={goToPrevMonth}>
              <ChevronLeft className="h-4 w-4" />
            </Button>
            <Button variant="outline" size="sm" onClick={goToToday}>
              Today
            </Button>
            <Button variant="outline" size="sm" onClick={goToNextMonth}>
              <ChevronRight className="h-4 w-4" />
            </Button>
          </div>
        </div>

        {/* Category legend */}
        <div className="flex flex-wrap gap-2 mt-2">
          {(Object.entries(categoryCounts) as [ContentCategory, number][]).map(([cat, count]) => (
            <Badge
              key={cat}
              variant="outline"
              className="gap-1"
              style={{
                backgroundColor: CATEGORY_COLORS[cat] + '20',
                borderColor: CATEGORY_COLORS[cat],
                color: CATEGORY_COLORS[cat],
              }}
            >
              {CATEGORY_ICONS[cat]} {cat}: {count}
            </Badge>
          ))}
        </div>
      </CardHeader>

      <CardContent className="p-0">
        {/* Day headers */}
        <div className="grid grid-cols-7 border-b">
          {DAYS_OF_WEEK.map((day) => (
            <div
              key={day}
              className="p-2 text-center text-sm font-medium text-muted-foreground border-r last:border-r-0"
            >
              {day}
            </div>
          ))}
        </div>

        {/* Calendar grid */}
        <div className="grid grid-cols-7">
          {calendarDays.map((day, index) => (
            <CalendarCell
              key={index}
              day={day}
              onDateClick={onDateClick}
              onPostClick={onPostClick}
              onPostDrop={onPostDrop}
            />
          ))}
        </div>
      </CardContent>
    </Card>
  );
}
