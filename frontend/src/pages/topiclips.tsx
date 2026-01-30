import { useState, useEffect, useCallback } from 'react';
import { useParams } from 'react-router-dom';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Separator } from '@/components/ui/separator';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from '@/components/ui/dialog';
import {
  Flame,
  Calendar,
  Play,
  Grid,
  Clock,
  Sparkles,
  Film,
  Loader2,
  RefreshCw,
  CheckCircle,
  AlertTriangle,
  XCircle,
  Plus,
  Eye,
  Layers,
  Zap,
  Trophy,
} from 'lucide-react';
import { toast } from 'sonner';
import { cn } from '@/lib/utils';

// Types for TopiClips
interface TopiClipSession {
  id: string;
  projectId: string;
  title: string;
  dayNumber: number;
  triggerType: 'daily' | 'event' | 'manual';
  primaryTheme?: string;
  emotionalArc?: string;
  narrativeSummary?: string;
  artisticPrompt?: string;
  status: 'pending' | 'analyzing' | 'interpreting' | 'rendering' | 'delivered' | 'failed' | 'cancelled';
  outputAssetIds?: string[];
  eventsAnalyzed: number;
  significanceScore?: number;
  createdAt: string;
  deliveredAt?: string;
}

interface TopiClipGalleryResponse {
  sessions: TopiClipSession[];
  schedule?: TopiClipDailySchedule;
  currentStreak: number;
  longestStreak: number;
  totalClips: number;
}

interface TopiClipDailySchedule {
  id: string;
  projectId: string;
  scheduledTime: string;
  timezone?: string;
  isEnabled: boolean;
  currentStreak: number;
  longestStreak: number;
  totalClipsGenerated: number;
  lastGenerationDate?: string;
}

interface TopiClipTimelineEntry {
  session: TopiClipSession;
  events: TopiClipCapturedEvent[];
  assetUrls: string[];
}

interface TopiClipCapturedEvent {
  id: string;
  sessionId: string;
  eventType: string;
  narrativeRole?: string;
  significanceScore?: number;
  assignedSymbol?: string;
  symbolPrompt?: string;
  occurredAt: string;
}

interface TopiClipSymbol {
  id: string;
  eventPattern: string;
  symbolName: string;
  symbolDescription?: string;
  promptTemplate: string;
  themeAffinity?: string;
  motionType?: string;
}

export function TopiClipsPage() {
  const { projectId } = useParams<{ projectId: string }>();

  // All hooks must be called unconditionally (before any early returns)
  const [activeTab, setActiveTab] = useState('gallery');
  const [gallery, setGallery] = useState<TopiClipGalleryResponse | null>(null);
  const [selectedSession, setSelectedSession] = useState<TopiClipSession | null>(null);
  const [timeline, setTimeline] = useState<TopiClipTimelineEntry | null>(null);
  const [symbols, setSymbols] = useState<TopiClipSymbol[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isGenerating, setIsGenerating] = useState(false);
  const [scheduleDialogOpen, setScheduleDialogOpen] = useState(false);
  const [scheduleTime, setScheduleTime] = useState('09:00');

  // Fetch gallery data
  const fetchGallery = useCallback(async () => {
    try {
      const res = await fetch(`/api/topiclips/gallery?projectId=${projectId}`);
      if (res.ok) {
        const data = await res.json();
        setGallery(data.data);
      }
    } catch (error) {
      console.error('Failed to fetch gallery:', error);
    } finally {
      setIsLoading(false);
    }
  }, [projectId]);

  // Fetch symbols
  const fetchSymbols = useCallback(async () => {
    try {
      const res = await fetch('/api/topiclips/symbols');
      if (res.ok) {
        const data = await res.json();
        setSymbols(data.data || []);
      }
    } catch (error) {
      console.error('Failed to fetch symbols:', error);
    }
  }, []);

  // Fetch timeline for a session
  const fetchTimeline = useCallback(async (sessionId: string) => {
    try {
      const res = await fetch(`/api/topiclips/sessions/${sessionId}/timeline`);
      if (res.ok) {
        const data = await res.json();
        setTimeline(data.data);
      }
    } catch (error) {
      console.error('Failed to fetch timeline:', error);
    }
  }, []);

  // Create new clip manually
  const createManualClip = useCallback(async () => {
    setIsGenerating(true);
    try {
      // Create session
      const createRes = await fetch('/api/topiclips/sessions', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          projectId,
          triggerType: 'manual',
        }),
      });

      if (!createRes.ok) {
        throw new Error('Failed to create session');
      }

      const createData = await createRes.json();
      const session = createData.data;

      // Generate the clip
      const generateRes = await fetch(`/api/topiclips/sessions/${session.id}/generate`, {
        method: 'POST',
      });

      if (generateRes.ok) {
        toast.success('TopiClip generated successfully!');
        await fetchGallery();
      } else {
        toast.error('Generation failed');
      }
    } catch (error) {
      console.error('Failed to create clip:', error);
      toast.error('Failed to create TopiClip');
    } finally {
      setIsGenerating(false);
    }
  }, [projectId, fetchGallery]);

  // Force daily generation
  const forceDailyGeneration = useCallback(async () => {
    setIsGenerating(true);
    try {
      const res = await fetch(`/api/topiclips/daily/${projectId}/generate`, {
        method: 'POST',
      });

      if (res.ok) {
        toast.success('Daily TopiClip generated!');
        await fetchGallery();
      } else {
        toast.error('Generation failed');
      }
    } catch (error) {
      console.error('Failed to generate daily clip:', error);
      toast.error('Failed to generate daily TopiClip');
    } finally {
      setIsGenerating(false);
    }
  }, [projectId, fetchGallery]);

  // Create/update schedule
  const saveSchedule = useCallback(async () => {
    try {
      const res = await fetch('/api/topiclips/daily', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          projectId,
          scheduledTime: scheduleTime,
          timezone: 'UTC',
        }),
      });

      if (res.ok) {
        toast.success('Schedule saved!');
        setScheduleDialogOpen(false);
        await fetchGallery();
      } else {
        toast.error('Failed to save schedule');
      }
    } catch (error) {
      console.error('Failed to save schedule:', error);
      toast.error('Failed to save schedule');
    }
  }, [projectId, scheduleTime, fetchGallery]);

  useEffect(() => {
    if (projectId) {
      fetchGallery();
      fetchSymbols();
    }
  }, [projectId, fetchGallery, fetchSymbols]);

  // Early return after all hooks are called
  if (!projectId) {
    return <div className="p-6">No project selected</div>;
  }

  // Status badge helper
  const getStatusBadge = (status: TopiClipSession['status']) => {
    const variants: Record<string, { color: string; icon: React.ReactNode }> = {
      pending: { color: 'bg-gray-500', icon: <Clock className="w-3 h-3" /> },
      analyzing: { color: 'bg-blue-500', icon: <Loader2 className="w-3 h-3 animate-spin" /> },
      interpreting: { color: 'bg-purple-500', icon: <Sparkles className="w-3 h-3" /> },
      rendering: { color: 'bg-yellow-500', icon: <Film className="w-3 h-3" /> },
      delivered: { color: 'bg-green-500', icon: <CheckCircle className="w-3 h-3" /> },
      failed: { color: 'bg-red-500', icon: <XCircle className="w-3 h-3" /> },
      cancelled: { color: 'bg-gray-400', icon: <AlertTriangle className="w-3 h-3" /> },
    };
    const v = variants[status] || variants.pending;
    return (
      <Badge className={cn('gap-1', v.color)}>
        {v.icon}
        {status}
      </Badge>
    );
  };

  // Theme color helper
  const getThemeColor = (theme?: string) => {
    const colors: Record<string, string> = {
      growth: 'text-green-500',
      struggle: 'text-red-500',
      transformation: 'text-purple-500',
      connection: 'text-blue-500',
      loss: 'text-gray-500',
    };
    return colors[theme || ''] || 'text-gray-500';
  };

  return (
    <div className="flex flex-col h-full bg-background">
      {/* Header */}
      <div className="border-b bg-white shadow-sm dark:bg-gray-900">
        <div className="flex items-center justify-between p-6">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-gradient-to-br from-purple-500 to-pink-500 rounded-lg">
              <Sparkles className="w-6 h-6 text-white" />
            </div>
            <div>
              <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100">
                TopiClips
              </h1>
              <p className="text-sm text-gray-600 dark:text-gray-400">
                Beeple Everydays from Topsi - AI-generated artistic clips from your topology
              </p>
            </div>
          </div>

          <div className="flex items-center gap-4">
            {/* Streak Display */}
            <div className="flex items-center gap-2 px-4 py-2 bg-gradient-to-r from-orange-500 to-red-500 rounded-lg text-white">
              <Flame className="w-5 h-5" />
              <div className="text-right">
                <div className="text-lg font-bold">{gallery?.currentStreak || 0}</div>
                <div className="text-xs opacity-80">day streak</div>
              </div>
            </div>

            <Separator orientation="vertical" className="h-10" />

            <Dialog open={scheduleDialogOpen} onOpenChange={setScheduleDialogOpen}>
              <DialogTrigger asChild>
                <Button variant="outline" className="gap-2">
                  <Calendar className="w-4 h-4" />
                  Schedule
                </Button>
              </DialogTrigger>
              <DialogContent>
                <DialogHeader>
                  <DialogTitle>Daily Generation Schedule</DialogTitle>
                </DialogHeader>
                <div className="space-y-4 py-4">
                  <div className="space-y-2">
                    <Label>Generation Time (UTC)</Label>
                    <Input
                      type="time"
                      value={scheduleTime}
                      onChange={(e) => setScheduleTime(e.target.value)}
                    />
                  </div>
                  <p className="text-sm text-muted-foreground">
                    TopiClips will automatically generate a daily artistic interpretation
                    of your topology changes at this time.
                  </p>
                  <Button onClick={saveSchedule} className="w-full">
                    Save Schedule
                  </Button>
                </div>
              </DialogContent>
            </Dialog>

            <Button
              onClick={createManualClip}
              disabled={isGenerating}
              className="bg-gradient-to-r from-purple-600 to-pink-600 hover:from-purple-700 hover:to-pink-700"
            >
              {isGenerating ? (
                <Loader2 className="w-4 h-4 mr-2 animate-spin" />
              ) : (
                <Plus className="w-4 h-4 mr-2" />
              )}
              Create Clip
            </Button>
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="flex-1 p-6 overflow-hidden">
        {isLoading ? (
          <div className="flex items-center justify-center h-full">
            <Loader2 className="w-8 h-8 animate-spin text-purple-600" />
          </div>
        ) : (
          <Tabs value={activeTab} onValueChange={setActiveTab} className="h-full flex flex-col">
            <TabsList className="grid w-full grid-cols-4 mb-6">
              <TabsTrigger value="gallery" className="flex items-center gap-2">
                <Grid className="w-4 h-4" />
                Gallery
              </TabsTrigger>
              <TabsTrigger value="timeline" className="flex items-center gap-2">
                <Clock className="w-4 h-4" />
                Timeline
              </TabsTrigger>
              <TabsTrigger value="symbols" className="flex items-center gap-2">
                <Layers className="w-4 h-4" />
                Symbols
              </TabsTrigger>
              <TabsTrigger value="stats" className="flex items-center gap-2">
                <Trophy className="w-4 h-4" />
                Stats
              </TabsTrigger>
            </TabsList>

            {/* Gallery Tab */}
            <TabsContent value="gallery" className="flex-1 overflow-auto">
              {gallery?.sessions.length === 0 ? (
                <div className="flex flex-col items-center justify-center h-full gap-4">
                  <Film className="w-16 h-16 text-gray-400" />
                  <h2 className="text-xl font-semibold text-gray-600">No clips yet</h2>
                  <p className="text-gray-500">Create your first TopiClip to start your streak!</p>
                  <Button
                    onClick={createManualClip}
                    disabled={isGenerating}
                    className="bg-gradient-to-r from-purple-600 to-pink-600"
                  >
                    <Sparkles className="w-4 h-4 mr-2" />
                    Create First Clip
                  </Button>
                </div>
              ) : (
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
                  {gallery?.sessions.map((session) => (
                    <Card
                      key={session.id}
                      className={cn(
                        'cursor-pointer transition-all hover:shadow-lg',
                        selectedSession?.id === session.id && 'ring-2 ring-purple-500'
                      )}
                      onClick={() => {
                        setSelectedSession(session);
                        fetchTimeline(session.id);
                      }}
                    >
                      <CardHeader className="pb-2">
                        <div className="flex items-center justify-between">
                          <Badge variant="outline" className="gap-1">
                            <Flame className="w-3 h-3" />
                            Day {session.dayNumber}
                          </Badge>
                          {getStatusBadge(session.status)}
                        </div>
                        <CardTitle className="text-sm mt-2">{session.title}</CardTitle>
                      </CardHeader>
                      <CardContent>
                        {/* Placeholder for video thumbnail */}
                        <div className="aspect-video bg-gradient-to-br from-gray-800 to-gray-900 rounded-lg mb-3 flex items-center justify-center">
                          {session.status === 'delivered' ? (
                            <Play className="w-8 h-8 text-white/50" />
                          ) : session.status === 'rendering' ? (
                            <Loader2 className="w-8 h-8 text-white/50 animate-spin" />
                          ) : (
                            <Film className="w-8 h-8 text-white/30" />
                          )}
                        </div>
                        <div className="space-y-1 text-xs">
                          {session.primaryTheme && (
                            <div className="flex items-center gap-2">
                              <span className="text-muted-foreground">Theme:</span>
                              <span className={cn('font-medium capitalize', getThemeColor(session.primaryTheme))}>
                                {session.primaryTheme}
                              </span>
                            </div>
                          )}
                          {session.emotionalArc && (
                            <div className="flex items-center gap-2">
                              <span className="text-muted-foreground">Arc:</span>
                              <span className="font-medium capitalize">{session.emotionalArc}</span>
                            </div>
                          )}
                          <div className="flex items-center gap-2">
                            <span className="text-muted-foreground">Events:</span>
                            <span className="font-medium">{session.eventsAnalyzed}</span>
                          </div>
                        </div>
                      </CardContent>
                    </Card>
                  ))}
                </div>
              )}
            </TabsContent>

            {/* Timeline Tab */}
            <TabsContent value="timeline" className="flex-1 overflow-auto">
              <div className="grid grid-cols-1 lg:grid-cols-3 gap-6 h-full">
                {/* Session List */}
                <div className="lg:col-span-1">
                  <Card className="h-full">
                    <CardHeader>
                      <CardTitle className="text-lg">Clip History</CardTitle>
                    </CardHeader>
                    <CardContent>
                      <ScrollArea className="h-[500px] pr-4">
                        <div className="space-y-2">
                          {gallery?.sessions.map((session) => (
                            <button
                              key={session.id}
                              className={cn(
                                'w-full p-3 rounded-lg text-left transition-colors',
                                selectedSession?.id === session.id
                                  ? 'bg-purple-100 dark:bg-purple-900/30'
                                  : 'hover:bg-muted'
                              )}
                              onClick={() => {
                                setSelectedSession(session);
                                fetchTimeline(session.id);
                              }}
                            >
                              <div className="flex items-center justify-between mb-1">
                                <span className="font-medium">Day {session.dayNumber}</span>
                                {getStatusBadge(session.status)}
                              </div>
                              <p className="text-xs text-muted-foreground truncate">
                                {session.narrativeSummary || session.title}
                              </p>
                            </button>
                          ))}
                        </div>
                      </ScrollArea>
                    </CardContent>
                  </Card>
                </div>

                {/* Timeline Detail */}
                <div className="lg:col-span-2">
                  {timeline ? (
                    <Card className="h-full">
                      <CardHeader>
                        <CardTitle>{timeline.session.title}</CardTitle>
                        <CardDescription>
                          {timeline.session.narrativeSummary}
                        </CardDescription>
                      </CardHeader>
                      <CardContent>
                        <div className="space-y-6">
                          {/* Artistic Prompt */}
                          {timeline.session.artisticPrompt && (
                            <div>
                              <h4 className="text-sm font-medium mb-2">Artistic Prompt</h4>
                              <p className="text-sm text-muted-foreground bg-muted p-3 rounded-lg">
                                {timeline.session.artisticPrompt}
                              </p>
                            </div>
                          )}

                          {/* Events */}
                          <div>
                            <h4 className="text-sm font-medium mb-2">Captured Events</h4>
                            <div className="space-y-2">
                              {timeline.events.map((event) => (
                                <div
                                  key={event.id}
                                  className="flex items-start gap-3 p-3 border rounded-lg"
                                >
                                  <div className="p-2 bg-purple-100 dark:bg-purple-900/30 rounded">
                                    <Zap className="w-4 h-4 text-purple-600" />
                                  </div>
                                  <div className="flex-1">
                                    <div className="flex items-center gap-2 mb-1">
                                      <span className="font-medium text-sm">{event.eventType}</span>
                                      {event.assignedSymbol && (
                                        <Badge variant="outline" className="text-xs">
                                          {event.assignedSymbol}
                                        </Badge>
                                      )}
                                    </div>
                                    {event.symbolPrompt && (
                                      <p className="text-xs text-muted-foreground">
                                        {event.symbolPrompt}
                                      </p>
                                    )}
                                  </div>
                                  {event.significanceScore && (
                                    <Badge
                                      variant={
                                        event.significanceScore >= 0.7
                                          ? 'default'
                                          : 'secondary'
                                      }
                                    >
                                      {(event.significanceScore * 100).toFixed(0)}%
                                    </Badge>
                                  )}
                                </div>
                              ))}
                            </div>
                          </div>
                        </div>
                      </CardContent>
                    </Card>
                  ) : (
                    <Card className="h-full flex items-center justify-center">
                      <div className="text-center text-muted-foreground">
                        <Eye className="w-12 h-12 mx-auto mb-2 opacity-50" />
                        <p>Select a clip to view its timeline</p>
                      </div>
                    </Card>
                  )}
                </div>
              </div>
            </TabsContent>

            {/* Symbols Tab */}
            <TabsContent value="symbols" className="flex-1 overflow-auto">
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                {symbols.map((symbol) => (
                  <Card key={symbol.id}>
                    <CardHeader className="pb-2">
                      <div className="flex items-center justify-between">
                        <CardTitle className="text-lg">{symbol.symbolName}</CardTitle>
                        {symbol.themeAffinity && (
                          <Badge variant="outline" className={getThemeColor(symbol.themeAffinity)}>
                            {symbol.themeAffinity}
                          </Badge>
                        )}
                      </div>
                      <CardDescription>{symbol.eventPattern}</CardDescription>
                    </CardHeader>
                    <CardContent>
                      <p className="text-sm text-muted-foreground mb-3">
                        {symbol.symbolDescription}
                      </p>
                      <div className="bg-muted p-3 rounded-lg text-sm">
                        {symbol.promptTemplate}
                      </div>
                      {symbol.motionType && (
                        <div className="mt-2 flex items-center gap-2">
                          <span className="text-xs text-muted-foreground">Motion:</span>
                          <Badge variant="secondary" className="text-xs">
                            {symbol.motionType}
                          </Badge>
                        </div>
                      )}
                    </CardContent>
                  </Card>
                ))}
              </div>
            </TabsContent>

            {/* Stats Tab */}
            <TabsContent value="stats" className="flex-1 overflow-auto">
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
                <Card>
                  <CardHeader className="pb-2">
                    <CardTitle className="text-sm font-medium text-muted-foreground">
                      Current Streak
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="flex items-center gap-2">
                      <Flame className="w-8 h-8 text-orange-500" />
                      <div className="text-3xl font-bold">{gallery?.currentStreak || 0}</div>
                    </div>
                    <p className="text-xs text-muted-foreground mt-1">consecutive days</p>
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader className="pb-2">
                    <CardTitle className="text-sm font-medium text-muted-foreground">
                      Longest Streak
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="flex items-center gap-2">
                      <Trophy className="w-8 h-8 text-yellow-500" />
                      <div className="text-3xl font-bold">{gallery?.longestStreak || 0}</div>
                    </div>
                    <p className="text-xs text-muted-foreground mt-1">personal best</p>
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader className="pb-2">
                    <CardTitle className="text-sm font-medium text-muted-foreground">
                      Total Clips
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="flex items-center gap-2">
                      <Film className="w-8 h-8 text-purple-500" />
                      <div className="text-3xl font-bold">{gallery?.totalClips || 0}</div>
                    </div>
                    <p className="text-xs text-muted-foreground mt-1">generated</p>
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader className="pb-2">
                    <CardTitle className="text-sm font-medium text-muted-foreground">
                      Schedule
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="flex items-center gap-2">
                      <Calendar className="w-8 h-8 text-blue-500" />
                      <div className="text-xl font-bold">
                        {gallery?.schedule?.scheduledTime || 'Not set'}
                      </div>
                    </div>
                    <p className="text-xs text-muted-foreground mt-1">
                      {gallery?.schedule?.isEnabled ? 'Active' : 'Inactive'}
                    </p>
                  </CardContent>
                </Card>
              </div>

              <Card>
                <CardHeader>
                  <CardTitle>Quick Actions</CardTitle>
                </CardHeader>
                <CardContent className="flex gap-4">
                  <Button
                    onClick={forceDailyGeneration}
                    disabled={isGenerating}
                    variant="outline"
                    className="gap-2"
                  >
                    {isGenerating ? (
                      <Loader2 className="w-4 h-4 animate-spin" />
                    ) : (
                      <Zap className="w-4 h-4" />
                    )}
                    Force Daily Generation
                  </Button>
                  <Button onClick={fetchGallery} variant="outline" className="gap-2">
                    <RefreshCw className="w-4 h-4" />
                    Refresh
                  </Button>
                </CardContent>
              </Card>
            </TabsContent>
          </Tabs>
        )}
      </div>
    </div>
  );
}
