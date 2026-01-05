import { useState, useRef, useEffect } from 'react';
import { cn } from '@/lib/utils';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { GitCompare, ChevronLeft, ChevronRight } from 'lucide-react';
import type { BrowserScreenshot } from '@/hooks/useBowser';

interface VisualDiffSliderProps {
  currentScreenshot: BrowserScreenshot;
  baselineScreenshot: BrowserScreenshot;
  className?: string;
}

interface CompareSliderProps {
  beforeSrc: string;
  afterSrc: string;
  className?: string;
}

function CompareSlider({ beforeSrc, afterSrc, className }: CompareSliderProps) {
  const [position, setPosition] = useState(50);
  const [isDragging, setIsDragging] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const handleMouseMove = (e: MouseEvent) => {
    if (!isDragging || !containerRef.current) return;

    const rect = containerRef.current.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const percentage = Math.min(Math.max((x / rect.width) * 100, 0), 100);
    setPosition(percentage);
  };

  const handleMouseUp = () => {
    setIsDragging(false);
  };

  useEffect(() => {
    if (isDragging) {
      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
      return () => {
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mouseup', handleMouseUp);
      };
    }
  }, [isDragging]);

  return (
    <div
      ref={containerRef}
      className={cn('relative overflow-hidden select-none', className)}
      onMouseDown={() => setIsDragging(true)}
    >
      {/* After image (full width) */}
      <img
        src={afterSrc}
        alt="After"
        className="w-full h-full object-contain"
        draggable={false}
      />

      {/* Before image (clipped) */}
      <div
        className="absolute inset-0 overflow-hidden"
        style={{ width: `${position}%` }}
      >
        <img
          src={beforeSrc}
          alt="Before"
          className="w-full h-full object-contain"
          style={{ width: `${100 / (position / 100)}%`, maxWidth: 'none' }}
          draggable={false}
        />
      </div>

      {/* Slider handle */}
      <div
        className="absolute top-0 bottom-0 w-1 bg-white shadow-lg cursor-ew-resize"
        style={{ left: `${position}%`, transform: 'translateX(-50%)' }}
      >
        <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-8 h-8 rounded-full bg-white shadow-lg flex items-center justify-center">
          <div className="flex items-center gap-0.5">
            <ChevronLeft className="h-3 w-3 text-muted-foreground" />
            <ChevronRight className="h-3 w-3 text-muted-foreground" />
          </div>
        </div>
      </div>

      {/* Labels */}
      <div className="absolute top-2 left-2">
        <Badge variant="secondary" className="bg-background/80">
          Before
        </Badge>
      </div>
      <div className="absolute top-2 right-2">
        <Badge variant="secondary" className="bg-background/80">
          After
        </Badge>
      </div>
    </div>
  );
}

export function VisualDiffSlider({
  currentScreenshot,
  baselineScreenshot,
  className,
}: VisualDiffSliderProps) {
  const [mode, setMode] = useState<'slider' | 'diff' | 'side-by-side'>('slider');
  const diffPercentage = currentScreenshot.diff_percentage ?? 0;

  const beforeSrc = `/api/images/${baselineScreenshot.screenshot_path}`;
  const afterSrc = `/api/images/${currentScreenshot.screenshot_path}`;
  const diffSrc = currentScreenshot.diff_path
    ? `/api/images/${currentScreenshot.diff_path}`
    : null;

  return (
    <Card className={cn('h-full flex flex-col', className)}>
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-sm font-medium flex items-center gap-2">
            <GitCompare className="h-4 w-4" />
            Visual Diff
            <Badge
              variant={
                diffPercentage === 0
                  ? 'default'
                  : diffPercentage < 5
                  ? 'secondary'
                  : diffPercentage < 15
                  ? 'outline'
                  : 'destructive'
              }
            >
              {diffPercentage.toFixed(1)}% changed
            </Badge>
          </CardTitle>
          <div className="flex items-center gap-1">
            <Button
              variant={mode === 'slider' ? 'default' : 'ghost'}
              size="sm"
              onClick={() => setMode('slider')}
            >
              Slider
            </Button>
            {diffSrc && (
              <Button
                variant={mode === 'diff' ? 'default' : 'ghost'}
                size="sm"
                onClick={() => setMode('diff')}
              >
                Diff
              </Button>
            )}
            <Button
              variant={mode === 'side-by-side' ? 'default' : 'ghost'}
              size="sm"
              onClick={() => setMode('side-by-side')}
            >
              Side by Side
            </Button>
          </div>
        </div>
      </CardHeader>
      <CardContent className="flex-1 overflow-hidden p-2">
        {mode === 'slider' && (
          <CompareSlider
            beforeSrc={beforeSrc}
            afterSrc={afterSrc}
            className="h-full rounded-lg"
          />
        )}

        {mode === 'diff' && diffSrc && (
          <div className="h-full flex items-center justify-center">
            <img
              src={diffSrc}
              alt="Visual diff"
              className="max-w-full max-h-full object-contain rounded-lg"
            />
          </div>
        )}

        {mode === 'side-by-side' && (
          <div className="h-full grid grid-cols-2 gap-2">
            <div className="relative">
              <img
                src={beforeSrc}
                alt="Before"
                className="w-full h-full object-contain rounded-lg"
              />
              <Badge className="absolute top-2 left-2 bg-background/80">
                Baseline
              </Badge>
            </div>
            <div className="relative">
              <img
                src={afterSrc}
                alt="After"
                className="w-full h-full object-contain rounded-lg"
              />
              <Badge className="absolute top-2 left-2 bg-background/80">
                Current
              </Badge>
            </div>
          </div>
        )}
      </CardContent>

      {/* Info footer */}
      <div className="px-4 pb-4">
        <div className="flex items-center justify-between text-xs text-muted-foreground">
          <span>
            Baseline: {baselineScreenshot.viewport_width}x{baselineScreenshot.viewport_height}
          </span>
          <span>
            Current: {currentScreenshot.viewport_width}x{currentScreenshot.viewport_height}
          </span>
        </div>
      </div>
    </Card>
  );
}

// Simple diff viewer for when only current screenshot is available
export function SimpleDiffViewer({
  screenshot,
  className,
}: {
  screenshot: BrowserScreenshot;
  className?: string;
}) {
  if (!screenshot.diff_path || !screenshot.baseline_screenshot_id) {
    return (
      <Card className={cn('h-full', className)}>
        <CardContent className="h-full flex items-center justify-center text-muted-foreground text-sm">
          No visual diff available
        </CardContent>
      </Card>
    );
  }

  const diffPercentage = screenshot.diff_percentage ?? 0;

  return (
    <Card className={cn('h-full flex flex-col', className)}>
      <CardHeader className="pb-2">
        <CardTitle className="text-sm font-medium flex items-center gap-2">
          <GitCompare className="h-4 w-4" />
          Visual Changes
          <Badge
            variant={
              diffPercentage === 0
                ? 'default'
                : diffPercentage < 5
                ? 'secondary'
                : diffPercentage < 15
                ? 'outline'
                : 'destructive'
            }
          >
            {diffPercentage.toFixed(1)}% changed
          </Badge>
        </CardTitle>
      </CardHeader>
      <CardContent className="flex-1 overflow-hidden p-2">
        <img
          src={`/api/images/${screenshot.diff_path}`}
          alt="Visual diff"
          className="w-full h-full object-contain rounded-lg"
        />
      </CardContent>
    </Card>
  );
}
