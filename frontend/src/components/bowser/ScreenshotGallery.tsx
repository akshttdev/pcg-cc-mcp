import { useState } from 'react';
import { cn } from '@/lib/utils';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Camera, ExternalLink, GitCompare, Maximize2 } from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import type { BrowserScreenshot } from '@/hooks/useBowser';
import { useScreenshots } from '@/hooks/useBowser';

interface ScreenshotGalleryProps {
  sessionId: string;
  className?: string;
}

interface ScreenshotCardProps {
  screenshot: BrowserScreenshot;
  onClick?: () => void;
}

function getDiffBadge(diffPercentage: number | null) {
  if (diffPercentage === null) return null;

  if (diffPercentage === 0) {
    return <Badge className="bg-green-500">Identical</Badge>;
  } else if (diffPercentage < 5) {
    return <Badge className="bg-green-500">{diffPercentage.toFixed(1)}% diff</Badge>;
  } else if (diffPercentage < 15) {
    return <Badge className="bg-yellow-500">{diffPercentage.toFixed(1)}% diff</Badge>;
  } else {
    return <Badge variant="destructive">{diffPercentage.toFixed(1)}% diff</Badge>;
  }
}

function ScreenshotCard({ screenshot, onClick }: ScreenshotCardProps) {
  const hasDiff = screenshot.diff_percentage !== null;

  return (
    <div
      className={cn(
        'group relative border rounded-lg overflow-hidden cursor-pointer hover:shadow-md transition-all',
        hasDiff && screenshot.diff_percentage! > 15 && 'border-red-300'
      )}
      onClick={onClick}
    >
      {/* Thumbnail or placeholder */}
      <div className="aspect-video bg-muted flex items-center justify-center">
        {screenshot.thumbnail_path ? (
          <img
            src={`/api/images/${screenshot.thumbnail_path}`}
            alt={screenshot.page_title || screenshot.url}
            className="w-full h-full object-cover"
          />
        ) : (
          <Camera className="h-8 w-8 text-muted-foreground" />
        )}
        {/* Overlay on hover */}
        <div className="absolute inset-0 bg-black/50 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center gap-2">
          <Maximize2 className="h-6 w-6 text-white" />
        </div>
      </div>

      {/* Info */}
      <div className="p-2 space-y-1">
        <p className="text-xs font-medium truncate" title={screenshot.page_title || screenshot.url}>
          {screenshot.page_title || 'Untitled'}
        </p>
        <p className="text-[10px] text-muted-foreground truncate" title={screenshot.url}>
          {screenshot.url}
        </p>
        <div className="flex items-center justify-between">
          <span className="text-[10px] text-muted-foreground">
            {formatDistanceToNow(new Date(screenshot.created_at), { addSuffix: true })}
          </span>
          {hasDiff && getDiffBadge(screenshot.diff_percentage)}
        </div>
      </div>
    </div>
  );
}

function ScreenshotModal({ screenshot }: { screenshot: BrowserScreenshot }) {
  const [showDiff, setShowDiff] = useState(false);
  const hasDiff = screenshot.diff_path !== null;

  return (
    <DialogContent className="max-w-4xl max-h-[90vh] overflow-hidden flex flex-col">
      <DialogHeader>
        <DialogTitle className="flex items-center justify-between">
          <span className="truncate">{screenshot.page_title || 'Screenshot'}</span>
          {hasDiff && (
            <Button
              variant="outline"
              size="sm"
              onClick={() => setShowDiff(!showDiff)}
            >
              <GitCompare className="h-4 w-4 mr-2" />
              {showDiff ? 'Show Current' : 'Show Diff'}
            </Button>
          )}
        </DialogTitle>
      </DialogHeader>
      <div className="flex-1 overflow-auto">
        <div className="relative">
          <img
            src={`/api/images/${showDiff && screenshot.diff_path ? screenshot.diff_path : screenshot.screenshot_path}`}
            alt={screenshot.page_title || screenshot.url}
            className="w-full h-auto"
          />
        </div>
        <div className="mt-4 space-y-2">
          <div className="flex items-center gap-2 text-sm">
            <ExternalLink className="h-4 w-4 text-muted-foreground" />
            <a
              href={screenshot.url}
              target="_blank"
              rel="noopener noreferrer"
              className="text-blue-500 hover:underline truncate"
            >
              {screenshot.url}
            </a>
          </div>
          <div className="flex items-center gap-4 text-xs text-muted-foreground">
            <span>
              {screenshot.viewport_width}x{screenshot.viewport_height}
            </span>
            {screenshot.full_page && <Badge variant="outline">Full Page</Badge>}
            {hasDiff && getDiffBadge(screenshot.diff_percentage)}
          </div>
        </div>
      </div>
    </DialogContent>
  );
}

export function ScreenshotGallery({ sessionId, className }: ScreenshotGalleryProps) {
  const { data: screenshots = [], isLoading } = useScreenshots(sessionId);
  const [selectedScreenshot, setSelectedScreenshot] = useState<BrowserScreenshot | null>(null);

  if (isLoading) {
    return (
      <Card className={cn('h-full', className)}>
        <CardContent className="flex items-center justify-center h-full">
          <div className="animate-pulse text-muted-foreground">Loading...</div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className={cn('h-full flex flex-col', className)}>
      <CardHeader className="pb-2">
        <CardTitle className="text-sm font-medium flex items-center gap-2">
          <Camera className="h-4 w-4" />
          Screenshots
          <Badge variant="secondary" className="ml-auto">
            {screenshots.length}
          </Badge>
        </CardTitle>
      </CardHeader>
      <CardContent className="flex-1 overflow-hidden p-0">
        {screenshots.length === 0 ? (
          <div className="flex items-center justify-center h-full text-muted-foreground text-sm">
            No screenshots captured yet
          </div>
        ) : (
          <ScrollArea className="h-full px-4 py-2">
            <div className="grid grid-cols-2 gap-3">
              {screenshots.map((screenshot) => (
                <ScreenshotCard
                  key={screenshot.id}
                  screenshot={screenshot}
                  onClick={() => setSelectedScreenshot(screenshot)}
                />
              ))}
            </div>
          </ScrollArea>
        )}
      </CardContent>

      {/* Screenshot modal */}
      <Dialog open={!!selectedScreenshot} onOpenChange={(open) => !open && setSelectedScreenshot(null)}>
        {selectedScreenshot && <ScreenshotModal screenshot={selectedScreenshot} />}
      </Dialog>
    </Card>
  );
}
