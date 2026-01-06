import { useState, useCallback, useMemo } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import {
  ChevronLeft,
  ChevronRight,
  Search,
  Palette,
  Globe,
  Mail,
  Scale,
  Share2,
  CheckCircle2,
  Clock,
  Play,
  Loader2,
} from 'lucide-react';
import { cn } from '@/lib/utils';

// Segment types matching the backend
export type SegmentType = 'research' | 'brand' | 'website' | 'email' | 'legal' | 'social' | 'custom';
export type SegmentStatus = 'pending' | 'in_progress' | 'needs_review' | 'completed' | 'skipped';

export interface OnboardingSegment {
  id: string;
  segment_type: SegmentType;
  name: string;
  assigned_agent_name?: string;
  status: SegmentStatus;
  recommendations?: string;
  user_decisions?: string;
  order_index: number;
}

interface OnboardingCarouselProps {
  segments: OnboardingSegment[];
  currentPhase: string;
  onSegmentClick?: (segment: OnboardingSegment) => void;
  onStartSegment?: (segment: OnboardingSegment) => void;
  className?: string;
}

const SEGMENT_CONFIG: Record<SegmentType, {
  icon: typeof Search;
  color: string;
  bgGradient: string;
  description: string;
}> = {
  research: {
    icon: Search,
    color: 'text-indigo-600',
    bgGradient: 'from-indigo-500/20 to-indigo-600/5',
    description: 'Market research, competitor analysis, and positioning strategy',
  },
  brand: {
    icon: Palette,
    color: 'text-pink-600',
    bgGradient: 'from-pink-500/20 to-pink-600/5',
    description: 'Logo, colors, fonts, brand guide, and visual identity',
  },
  website: {
    icon: Globe,
    color: 'text-sky-600',
    bgGradient: 'from-sky-500/20 to-sky-600/5',
    description: 'Landing page, dashboard, admin panel development',
  },
  email: {
    icon: Mail,
    color: 'text-amber-600',
    bgGradient: 'from-amber-500/20 to-amber-600/5',
    description: 'Gmail master account, Zoho operations, CRM configuration',
  },
  legal: {
    icon: Scale,
    color: 'text-emerald-600',
    bgGradient: 'from-emerald-500/20 to-emerald-600/5',
    description: 'Entity formation, compliance, and regulatory research',
  },
  social: {
    icon: Share2,
    color: 'text-violet-600',
    bgGradient: 'from-violet-500/20 to-violet-600/5',
    description: 'Social account setup and content strategy',
  },
  custom: {
    icon: Search,
    color: 'text-slate-600',
    bgGradient: 'from-slate-500/20 to-slate-600/5',
    description: 'Custom workflow for specialized needs',
  },
};

const STATUS_CONFIG: Record<SegmentStatus, {
  label: string;
  icon: typeof CheckCircle2;
  color: string;
}> = {
  pending: { label: 'Not Started', icon: Clock, color: 'text-muted-foreground' },
  in_progress: { label: 'In Progress', icon: Loader2, color: 'text-blue-600' },
  needs_review: { label: 'Needs Review', icon: Clock, color: 'text-amber-600' },
  completed: { label: 'Completed', icon: CheckCircle2, color: 'text-green-600' },
  skipped: { label: 'Skipped', icon: Clock, color: 'text-muted-foreground' },
};

export function OnboardingCarousel({
  segments,
  currentPhase,
  onSegmentClick,
  onStartSegment,
  className,
}: OnboardingCarouselProps) {
  const [activeIndex, setActiveIndex] = useState(0);

  const sortedSegments = useMemo(
    () => [...segments].sort((a, b) => a.order_index - b.order_index),
    [segments]
  );

  const completedCount = useMemo(
    () => sortedSegments.filter((s) => s.status === 'completed').length,
    [sortedSegments]
  );

  const progressPercent = useMemo(
    () => (sortedSegments.length > 0 ? (completedCount / sortedSegments.length) * 100 : 0),
    [completedCount, sortedSegments.length]
  );

  const handlePrev = useCallback(() => {
    setActiveIndex((prev) => (prev > 0 ? prev - 1 : sortedSegments.length - 1));
  }, [sortedSegments.length]);

  const handleNext = useCallback(() => {
    setActiveIndex((prev) => (prev < sortedSegments.length - 1 ? prev + 1 : 0));
  }, [sortedSegments.length]);

  if (sortedSegments.length === 0) {
    return null;
  }

  const activeSegment = sortedSegments[activeIndex];
  const config = SEGMENT_CONFIG[activeSegment.segment_type] || SEGMENT_CONFIG.custom;
  const statusConfig = STATUS_CONFIG[activeSegment.status];
  const Icon = config.icon;
  const StatusIcon = statusConfig.icon;

  return (
    <div className={cn('space-y-4', className)}>
      {/* Progress Header */}
      <div className="flex items-center justify-between">
        <div className="space-y-1">
          <h3 className="text-lg font-semibold">Project Setup</h3>
          <p className="text-sm text-muted-foreground">
            {completedCount} of {sortedSegments.length} workflows completed
          </p>
        </div>
        <Badge variant="outline" className="text-xs">
          {currentPhase.replace('_', ' ')}
        </Badge>
      </div>

      <Progress value={progressPercent} className="h-2" />

      {/* Segment Indicators */}
      <div className="flex justify-center gap-2">
        {sortedSegments.map((segment, idx) => {
          const isActive = idx === activeIndex;
          const isCompleted = segment.status === 'completed';

          return (
            <button
              key={segment.id}
              onClick={() => setActiveIndex(idx)}
              className={cn(
                'h-2 rounded-full transition-all',
                isActive ? 'w-8' : 'w-2',
                isCompleted
                  ? 'bg-green-500'
                  : segment.status === 'in_progress'
                  ? 'bg-blue-500'
                  : 'bg-muted-foreground/30'
              )}
              aria-label={`Go to ${segment.name}`}
            />
          );
        })}
      </div>

      {/* Main Carousel Card */}
      <Card
        className={cn(
          'relative overflow-hidden cursor-pointer transition-all hover:shadow-lg',
          `bg-gradient-to-br ${config.bgGradient}`
        )}
        onClick={() => onSegmentClick?.(activeSegment)}
      >
        {/* Navigation Arrows */}
        <Button
          variant="ghost"
          size="icon"
          className="absolute left-2 top-1/2 -translate-y-1/2 z-10 h-8 w-8 rounded-full bg-background/80 hover:bg-background"
          onClick={(e) => {
            e.stopPropagation();
            handlePrev();
          }}
        >
          <ChevronLeft className="h-4 w-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="absolute right-2 top-1/2 -translate-y-1/2 z-10 h-8 w-8 rounded-full bg-background/80 hover:bg-background"
          onClick={(e) => {
            e.stopPropagation();
            handleNext();
          }}
        >
          <ChevronRight className="h-4 w-4" />
        </Button>

        <CardHeader className="pb-2">
          <div className="flex items-start justify-between">
            <div className="flex items-center gap-3">
              <div className={cn('p-2 rounded-lg bg-background/50', config.color)}>
                <Icon className="h-6 w-6" />
              </div>
              <div>
                <CardTitle className="text-xl">{activeSegment.name}</CardTitle>
                {activeSegment.assigned_agent_name && (
                  <CardDescription className="text-sm">
                    Powered by {activeSegment.assigned_agent_name}
                  </CardDescription>
                )}
              </div>
            </div>
            <Badge
              variant="secondary"
              className={cn('flex items-center gap-1', statusConfig.color)}
            >
              <StatusIcon className={cn('h-3 w-3', activeSegment.status === 'in_progress' && 'animate-spin')} />
              {statusConfig.label}
            </Badge>
          </div>
        </CardHeader>

        <CardContent className="space-y-4">
          <p className="text-sm text-muted-foreground">{config.description}</p>

          {/* Recommendations Preview */}
          {activeSegment.recommendations && (
            <div className="p-3 bg-background/50 rounded-lg">
              <p className="text-xs font-medium text-muted-foreground mb-1">AI Recommendations</p>
              <p className="text-sm line-clamp-2">
                {JSON.parse(activeSegment.recommendations).summary || 'Recommendations available'}
              </p>
            </div>
          )}

          {/* Action Button */}
          <div className="flex justify-end pt-2">
            {activeSegment.status === 'pending' && (
              <Button
                size="sm"
                onClick={(e) => {
                  e.stopPropagation();
                  onStartSegment?.(activeSegment);
                }}
              >
                <Play className="h-4 w-4 mr-2" />
                Start Workflow
              </Button>
            )}
            {activeSegment.status === 'in_progress' && (
              <Button size="sm" variant="outline">
                View Progress
              </Button>
            )}
            {activeSegment.status === 'needs_review' && (
              <Button size="sm" variant="default">
                Review & Approve
              </Button>
            )}
            {activeSegment.status === 'completed' && (
              <Button size="sm" variant="ghost">
                View Details
              </Button>
            )}
          </div>
        </CardContent>
      </Card>

      {/* Segment Quick Links */}
      <div className="grid grid-cols-3 sm:grid-cols-6 gap-2">
        {sortedSegments.map((segment, idx) => {
          const segConfig = SEGMENT_CONFIG[segment.segment_type] || SEGMENT_CONFIG.custom;
          const SegIcon = segConfig.icon;
          const isActive = idx === activeIndex;
          const isCompleted = segment.status === 'completed';

          return (
            <button
              key={segment.id}
              onClick={() => setActiveIndex(idx)}
              className={cn(
                'flex flex-col items-center gap-1 p-2 rounded-lg transition-all',
                'hover:bg-muted/50',
                isActive && 'bg-muted ring-2 ring-primary/20'
              )}
            >
              <div
                className={cn(
                  'p-1.5 rounded-md',
                  isCompleted ? 'bg-green-100 text-green-600' : 'bg-muted',
                  isActive && !isCompleted && segConfig.color
                )}
              >
                <SegIcon className="h-4 w-4" />
              </div>
              <span className="text-[10px] text-muted-foreground truncate w-full text-center">
                {segment.segment_type.charAt(0).toUpperCase() + segment.segment_type.slice(1)}
              </span>
            </button>
          );
        })}
      </div>
    </div>
  );
}

export default OnboardingCarousel;
