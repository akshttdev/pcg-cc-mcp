import { useState, useEffect, useCallback, useRef } from 'react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Separator } from '@/components/ui/separator';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Crown,
  MessageSquare,
  RefreshCw,
  Network,
  Shield,
  AlertTriangle,
  CheckCircle,
  Send,
  Bot,
  Loader2,
  Eye,
  Lock,
  FolderOpen,
  Mic,
  MicOff,
  Volume2,
} from 'lucide-react';
import { toast } from 'sonner';
import { cn } from '@/lib/utils';

// Types for Topsi responses
interface TopsiStatusResponse {
  isActive: boolean;
  topsiId?: string;
  uptimeMs?: number;
  accessScope: string;
  projectsVisible: number;
  systemHealth?: number;
}

interface TopsiChatMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
  hasAudio?: boolean;
}

interface TopologyOverview {
  totalNodes: number;
  totalEdges: number;
  totalClusters: number;
  systemHealth?: number;
}

interface DetectedIssue {
  issueType: string;
  severity: string;
  description: string;
  affectedNodes: string[];
  suggestedAction?: string;
}

interface ProjectAccess {
  projectId: string;
  projectName: string;
  role: string;
  grantedAt: string;
}

export function TopsiPage() {
  const [activeTab, setActiveTab] = useState('chat');
  const [status, setStatus] = useState<TopsiStatusResponse | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [messages, setMessages] = useState<TopsiChatMessage[]>([
    {
      id: '1',
      role: 'assistant',
      content: "Hello! I'm Topsi, the PCG Platform Intelligence Agent. I manage projects, coordinate agents, and ensure data isolation between clients. How can I help you today?",
      timestamp: new Date(),
    },
  ]);
  const [inputMessage, setInputMessage] = useState('');
  const [isSending, setIsSending] = useState(false);
  const [topology, setTopology] = useState<TopologyOverview | null>(null);
  const [issues, setIssues] = useState<DetectedIssue[]>([]);
  const [projects, setProjects] = useState<ProjectAccess[]>([]);
  const [sessionId] = useState(() => `topsi-${Date.now()}`);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Voice state
  const [isRecording, setIsRecording] = useState(false);
  const [isProcessingVoice, setIsProcessingVoice] = useState(false);
  const [audioLevel, setAudioLevel] = useState(0);
  const [isSpeakerOn, setIsSpeakerOn] = useState(true);

  // Voice refs
  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const audioContextRef = useRef<AudioContext | null>(null);
  const analyserRef = useRef<AnalyserNode | null>(null);
  const audioChunksRef = useRef<Blob[]>([]);
  const animationRef = useRef<number | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const currentAudioRef = useRef<HTMLAudioElement | null>(null);

  // Auto-scroll to bottom when messages change
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  // Fetch Topsi status
  const fetchStatus = useCallback(async () => {
    try {
      const res = await fetch('/api/topsi/status');
      if (res.ok) {
        const data = await res.json();
        setStatus(data);
      }
    } catch (error) {
      console.error('Failed to fetch Topsi status:', error);
    } finally {
      setIsLoading(false);
    }
  }, []);

  // Initialize Topsi
  const initializeTopsi = useCallback(async () => {
    setIsLoading(true);
    try {
      const res = await fetch('/api/topsi/initialize', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ activateImmediately: true }),
      });
      if (res.ok) {
        await res.json(); // Consume response
        toast.success('Topsi initialized successfully');
        await fetchStatus();
      } else {
        toast.error('Failed to initialize Topsi');
      }
    } catch (error) {
      console.error('Failed to initialize Topsi:', error);
      toast.error('Failed to initialize Topsi');
    } finally {
      setIsLoading(false);
    }
  }, [fetchStatus]);

  // Fetch topology overview
  const fetchTopology = useCallback(async () => {
    try {
      const res = await fetch('/api/topsi/topology');
      if (res.ok) {
        const data = await res.json();
        setTopology(data);
      }
    } catch (error) {
      console.error('Failed to fetch topology:', error);
    }
  }, []);

  // Fetch issues
  const fetchIssues = useCallback(async () => {
    try {
      const res = await fetch('/api/topsi/issues');
      if (res.ok) {
        const data = await res.json();
        setIssues(data.issues || []);
      }
    } catch (error) {
      console.error('Failed to fetch issues:', error);
    }
  }, []);

  // Fetch accessible projects
  const fetchProjects = useCallback(async () => {
    try {
      const res = await fetch('/api/topsi/projects');
      if (res.ok) {
        const data = await res.json();
        setProjects(data.projects || []);
      }
    } catch (error) {
      console.error('Failed to fetch projects:', error);
    }
  }, []);

  // Send message to Topsi
  const sendMessage = useCallback(async () => {
    if (!inputMessage.trim() || isSending) return;

    const userMessage: TopsiChatMessage = {
      id: `user-${Date.now()}`,
      role: 'user',
      content: inputMessage,
      timestamp: new Date(),
    };

    setMessages((prev) => [...prev, userMessage]);
    setInputMessage('');
    setIsSending(true);

    // Create abort controller with 2 minute timeout (Ollama fallback can be slow)
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 120000);

    try {
      const res = await fetch('/api/topsi/chat', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          message: inputMessage,
          sessionId,
        }),
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      if (res.ok) {
        const data = await res.json();
        const assistantMessage: TopsiChatMessage = {
          id: `assistant-${Date.now()}`,
          role: 'assistant',
          content: data.message,
          timestamp: new Date(),
        };
        setMessages((prev) => [...prev, assistantMessage]);
      } else {
        const errorText = await res.text();
        console.error('Topsi error:', errorText);
        toast.error(`Topsi error: ${res.status}`);
      }
    } catch (error) {
      clearTimeout(timeoutId);
      console.error('Failed to send message:', error);
      if (error instanceof Error && error.name === 'AbortError') {
        toast.error('Request timed out - Topsi may be busy');
      } else {
        toast.error('Failed to send message');
      }
    } finally {
      setIsSending(false);
    }
  }, [inputMessage, isSending, sessionId]);

  // Voice recording functions
  const startRecording = useCallback(async () => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      streamRef.current = stream;

      // Audio level monitoring
      audioContextRef.current = new AudioContext();
      const source = audioContextRef.current.createMediaStreamSource(stream);
      analyserRef.current = audioContextRef.current.createAnalyser();
      analyserRef.current.fftSize = 256;
      source.connect(analyserRef.current);

      const bufferLength = analyserRef.current.frequencyBinCount;
      const dataArray = new Uint8Array(bufferLength);

      const updateLevel = () => {
        if (!analyserRef.current || !isRecording) return;
        analyserRef.current.getByteFrequencyData(dataArray);
        const average = dataArray.reduce((a, b) => a + b, 0) / bufferLength;
        setAudioLevel(average / 255);
        animationRef.current = requestAnimationFrame(updateLevel);
      };

      // Media recorder
      mediaRecorderRef.current = new MediaRecorder(stream);
      audioChunksRef.current = [];

      mediaRecorderRef.current.ondataavailable = (event) => {
        audioChunksRef.current.push(event.data);
      };

      mediaRecorderRef.current.onstop = async () => {
        const audioBlob = new Blob(audioChunksRef.current, { type: 'audio/wav' });
        await processVoiceInput(audioBlob);
      };

      mediaRecorderRef.current.start();
      setIsRecording(true);
      updateLevel();
    } catch (error) {
      console.error('Failed to start recording:', error);
      toast.error('Could not access microphone');
    }
  }, []);

  const stopRecording = useCallback(() => {
    if (mediaRecorderRef.current && isRecording) {
      mediaRecorderRef.current.stop();
      setIsRecording(false);
      setAudioLevel(0);
    }

    if (streamRef.current) {
      streamRef.current.getTracks().forEach(track => track.stop());
      streamRef.current = null;
    }

    if (audioContextRef.current) {
      audioContextRef.current.close();
      audioContextRef.current = null;
    }

    if (animationRef.current) {
      cancelAnimationFrame(animationRef.current);
      animationRef.current = null;
    }
  }, [isRecording]);

  const blobToBase64 = (blob: Blob): Promise<string> => {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => {
        const result = reader.result as string;
        resolve(result.split(',')[1]);
      };
      reader.onerror = reject;
      reader.readAsDataURL(blob);
    });
  };

  const playAudio = async (base64Audio: string) => {
    if (!isSpeakerOn) return;
    try {
      const audioData = atob(base64Audio);
      const audioBuffer = new Uint8Array(audioData.length);
      for (let i = 0; i < audioData.length; i++) {
        audioBuffer[i] = audioData.charCodeAt(i);
      }

      const audioBlob = new Blob([audioBuffer], { type: 'audio/wav' });
      const audioUrl = URL.createObjectURL(audioBlob);

      if (currentAudioRef.current) {
        currentAudioRef.current.pause();
      }

      const audio = new Audio(audioUrl);
      currentAudioRef.current = audio;

      await audio.play();

      audio.onended = () => {
        URL.revokeObjectURL(audioUrl);
      };
    } catch (error) {
      console.error('Failed to play audio:', error);
    }
  };

  const processVoiceInput = async (audioBlob: Blob) => {
    setIsProcessingVoice(true);

    try {
      const base64Audio = await blobToBase64(audioBlob);

      const res = await fetch('/api/topsi/voice/interaction', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          sessionId,
          audioInput: base64Audio,
        }),
      });

      if (res.ok) {
        const data = await res.json();
        const responseData = data.data || data;

        // Add user's transcribed message
        if (responseData.transcription) {
          const userMessage: TopsiChatMessage = {
            id: `user-${Date.now()}`,
            role: 'user',
            content: responseData.transcription,
            timestamp: new Date(),
          };
          setMessages((prev) => [...prev, userMessage]);
        }

        // Add Topsi's response
        const responseText = responseData.responseText || 'I received your message.';
        const hasAudio = responseData.audioResponse && responseData.audioResponse.length > 100;
        const assistantMessage: TopsiChatMessage = {
          id: `assistant-${Date.now()}`,
          role: 'assistant',
          content: responseText,
          timestamp: new Date(),
          hasAudio,
        };
        setMessages((prev) => [...prev, assistantMessage]);

        // Play audio response
        if (hasAudio) {
          await playAudio(responseData.audioResponse);
        }
      } else {
        toast.error('Voice processing failed');
      }
    } catch (error) {
      console.error('Failed to process voice:', error);
      toast.error('Voice processing failed');
    } finally {
      setIsProcessingVoice(false);
    }
  };

  // Push-to-talk handlers
  const handlePushToTalkStart = useCallback(() => {
    if (!isProcessingVoice && !isSending) {
      startRecording();
    }
  }, [isProcessingVoice, isSending, startRecording]);

  const handlePushToTalkEnd = useCallback(() => {
    if (isRecording) {
      stopRecording();
    }
  }, [isRecording, stopRecording]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      stopRecording();
      if (currentAudioRef.current) {
        currentAudioRef.current.pause();
      }
    };
  }, [stopRecording]);

  useEffect(() => {
    fetchStatus();
  }, [fetchStatus]);

  useEffect(() => {
    if (status?.isActive) {
      fetchTopology();
      fetchIssues();
      fetchProjects();
    }
  }, [status?.isActive, fetchTopology, fetchIssues, fetchProjects]);

  // Listen for Topsi action completion events to auto-refresh data
  useEffect(() => {
    const handleTopsiAction = () => {
      // Refresh projects and topology when Topsi completes an action
      if (status?.isActive) {
        fetchProjects();
        fetchTopology();
        fetchIssues();
      }
    };

    window.addEventListener('topsi-action-complete', handleTopsiAction);
    return () => window.removeEventListener('topsi-action-complete', handleTopsiAction);
  }, [status?.isActive, fetchProjects, fetchTopology, fetchIssues]);

  const formatUptime = (ms?: number) => {
    if (!ms) return 'N/A';
    const seconds = Math.floor(ms / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);
    if (hours > 0) return `${hours}h ${minutes % 60}m`;
    if (minutes > 0) return `${minutes}m ${seconds % 60}s`;
    return `${seconds}s`;
  };

  return (
    <div className="flex flex-col h-full bg-background">
      {/* Header */}
      <div className="border-b bg-white shadow-sm dark:bg-gray-900">
        <div className="flex items-center justify-between p-6">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-cyan-100 dark:bg-cyan-950 rounded-lg">
              <Crown className="w-6 h-6 text-cyan-600" />
            </div>
            <div>
              <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100">
                Topsi Platform Agent
              </h1>
              <p className="text-sm text-gray-600 dark:text-gray-400">
                Topological Super Intelligence - Your platform orchestrator with secure, containerized access
              </p>
            </div>
          </div>

          <div className="flex items-center gap-4">
            <div className="flex items-center gap-2">
              <Shield className="w-4 h-4 text-green-600" />
              <span className="text-sm text-gray-600 dark:text-gray-400">Data Isolation Active</span>
            </div>
            <Separator orientation="vertical" className="h-8" />
            <div className="text-right">
              <div className={cn(
                "text-sm font-medium",
                status?.isActive ? "text-green-600" : "text-gray-400"
              )}>
                {status?.isActive ? 'Online' : 'Offline'}
              </div>
              <div className="text-xs text-gray-500">
                Uptime: {formatUptime(status?.uptimeMs)}
              </div>
            </div>
            <div className={cn(
              "w-3 h-3 rounded-full",
              status?.isActive ? "bg-green-500 animate-pulse" : "bg-gray-400"
            )} />
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="flex-1 p-6 overflow-hidden">
        {isLoading ? (
          <div className="flex items-center justify-center h-full">
            <Loader2 className="w-8 h-8 animate-spin text-cyan-600" />
          </div>
        ) : !status?.isActive ? (
          <div className="flex flex-col items-center justify-center h-full gap-4">
            <Network className="w-16 h-16 text-gray-400" />
            <h2 className="text-xl font-semibold text-gray-600">Topsi is not initialized</h2>
            <p className="text-gray-500">Initialize Topsi to start managing your platform</p>
            <Button onClick={initializeTopsi} className="bg-cyan-600 hover:bg-cyan-700">
              <Bot className="w-4 h-4 mr-2" />
              Initialize Topsi
            </Button>
          </div>
        ) : (
          <Tabs value={activeTab} onValueChange={setActiveTab} className="h-full flex flex-col">
            <TabsList className="grid w-full grid-cols-4 mb-6">
              <TabsTrigger value="chat" className="flex items-center gap-2">
                <MessageSquare className="w-4 h-4" />
                Chat
              </TabsTrigger>
              <TabsTrigger value="topology" className="flex items-center gap-2">
                <Network className="w-4 h-4" />
                Topology
              </TabsTrigger>
              <TabsTrigger value="access" className="flex items-center gap-2">
                <Shield className="w-4 h-4" />
                Access Control
              </TabsTrigger>
              <TabsTrigger value="issues" className="flex items-center gap-2">
                <AlertTriangle className="w-4 h-4" />
                Issues
                {issues.length > 0 && (
                  <Badge variant="destructive" className="ml-1">
                    {issues.length}
                  </Badge>
                )}
              </TabsTrigger>
            </TabsList>

            {/* Chat Tab */}
            <TabsContent value="chat" className="flex-1 overflow-auto">
              <div className="grid grid-cols-1 lg:grid-cols-3 gap-6 h-full min-h-[600px]">
                {/* Chat Interface */}
                <div className="lg:col-span-2 flex flex-col h-full min-h-0">
                  <Card className="flex-1 flex flex-col min-h-0 h-full">
                    <CardHeader className="flex-shrink-0">
                      <CardTitle className="text-lg">Chat with Topsi</CardTitle>
                      <CardDescription>
                        Ask questions, get insights, or manage your projects
                      </CardDescription>
                    </CardHeader>
                    <CardContent className="flex-1 flex flex-col min-h-0 overflow-hidden p-4">
                      <ScrollArea className="flex-1 min-h-0 h-[400px] lg:h-[500px] pr-4">
                        <div className="space-y-4 pb-4">
                          {messages.map((msg) => (
                            <div
                              key={msg.id}
                              className={cn(
                                "flex",
                                msg.role === 'user' ? "justify-end" : "justify-start"
                              )}
                            >
                              <div
                                className={cn(
                                  "max-w-[80%] rounded-lg px-4 py-2",
                                  msg.role === 'user'
                                    ? "bg-cyan-600 text-white"
                                    : "bg-muted"
                                )}
                              >
                                <p className="text-sm whitespace-pre-wrap">{msg.content}</p>
                                <div className={cn(
                                  "flex items-center gap-2 text-xs mt-1",
                                  msg.role === 'user' ? "text-cyan-100" : "text-muted-foreground"
                                )}>
                                  <span>{msg.timestamp.toLocaleTimeString()}</span>
                                  {msg.hasAudio && msg.role === 'assistant' && (
                                    <Volume2 className="w-3 h-3" />
                                  )}
                                </div>
                              </div>
                            </div>
                          ))}
                          <div ref={messagesEndRef} />
                        </div>
                      </ScrollArea>
                      {/* Audio level indicator when recording */}
                      {isRecording && (
                        <div className="mb-2">
                          <div className="h-1 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden">
                            <div
                              className="h-full bg-cyan-500 transition-all duration-100"
                              style={{ width: `${audioLevel * 100}%` }}
                            />
                          </div>
                          <p className="text-xs text-center text-muted-foreground mt-1">Listening...</p>
                        </div>
                      )}
                      {isProcessingVoice && (
                        <div className="mb-2 flex items-center justify-center gap-2 text-sm text-muted-foreground">
                          <Loader2 className="w-4 h-4 animate-spin" />
                          Processing voice...
                        </div>
                      )}
                      <div className="flex gap-2 mt-4 flex-shrink-0">
                        {/* Push-to-talk button */}
                        <Button
                          variant={isRecording ? "destructive" : "outline"}
                          size="icon"
                          className="shrink-0"
                          onMouseDown={handlePushToTalkStart}
                          onMouseUp={handlePushToTalkEnd}
                          onMouseLeave={handlePushToTalkEnd}
                          onTouchStart={handlePushToTalkStart}
                          onTouchEnd={handlePushToTalkEnd}
                          disabled={isProcessingVoice || isSending}
                          title="Hold to talk"
                        >
                          {isRecording ? <MicOff className="w-4 h-4" /> : <Mic className="w-4 h-4" />}
                        </Button>
                        <Input
                          placeholder="Ask Topsi anything..."
                          value={inputMessage}
                          onChange={(e) => setInputMessage(e.target.value)}
                          onKeyDown={(e) => e.key === 'Enter' && !e.shiftKey && sendMessage()}
                          disabled={isSending || isRecording}
                        />
                        <Button onClick={sendMessage} disabled={isSending || !inputMessage.trim()} className="bg-cyan-600 hover:bg-cyan-700">
                          {isSending ? (
                            <Loader2 className="w-4 h-4 animate-spin" />
                          ) : (
                            <Send className="w-4 h-4" />
                          )}
                        </Button>
                      </div>
                      <p className="text-xs text-muted-foreground mt-2 text-center">
                        Hold the mic button to speak, or type your message
                      </p>
                    </CardContent>
                  </Card>
                </div>

                {/* Quick Stats */}
                <div className="space-y-4">
                  <Card>
                    <CardHeader>
                      <CardTitle className="text-lg">Platform Status</CardTitle>
                    </CardHeader>
                    <CardContent className="space-y-4">
                      <div className="flex items-center justify-between">
                        <span className="text-sm text-muted-foreground">Access Level</span>
                        <Badge variant="outline" className="capitalize">
                          {status?.accessScope || 'unknown'}
                        </Badge>
                      </div>
                      <div className="flex items-center justify-between">
                        <span className="text-sm text-muted-foreground">Projects Visible</span>
                        <span className="font-medium">{status?.projectsVisible || 0}</span>
                      </div>
                      {topology && (
                        <>
                          <Separator />
                          <div className="flex items-center justify-between">
                            <span className="text-sm text-muted-foreground">Total Nodes</span>
                            <span className="font-medium">{topology.totalNodes}</span>
                          </div>
                          <div className="flex items-center justify-between">
                            <span className="text-sm text-muted-foreground">Total Edges</span>
                            <span className="font-medium">{topology.totalEdges}</span>
                          </div>
                          <div className="flex items-center justify-between">
                            <span className="text-sm text-muted-foreground">Clusters</span>
                            <span className="font-medium">{topology.totalClusters}</span>
                          </div>
                        </>
                      )}
                    </CardContent>
                  </Card>

                  <Card>
                    <CardHeader>
                      <CardTitle className="text-lg">Quick Actions</CardTitle>
                    </CardHeader>
                    <CardContent className="space-y-2">
                      <Button variant="outline" className="w-full justify-start" onClick={fetchTopology}>
                        <RefreshCw className="w-4 h-4 mr-2" />
                        Refresh Topology
                      </Button>
                      <Button variant="outline" className="w-full justify-start" onClick={fetchIssues}>
                        <AlertTriangle className="w-4 h-4 mr-2" />
                        Detect Issues
                      </Button>
                      <Button variant="outline" className="w-full justify-start" onClick={fetchProjects}>
                        <FolderOpen className="w-4 h-4 mr-2" />
                        Refresh Projects
                      </Button>
                    </CardContent>
                  </Card>
                </div>
              </div>
            </TabsContent>

            {/* Topology Tab */}
            <TabsContent value="topology" className="flex-1 overflow-auto">
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
                <Card>
                  <CardHeader className="pb-2">
                    <CardTitle className="text-sm font-medium text-muted-foreground">
                      Total Nodes
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="text-3xl font-bold">{topology?.totalNodes || 0}</div>
                  </CardContent>
                </Card>
                <Card>
                  <CardHeader className="pb-2">
                    <CardTitle className="text-sm font-medium text-muted-foreground">
                      Total Edges
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="text-3xl font-bold">{topology?.totalEdges || 0}</div>
                  </CardContent>
                </Card>
                <Card>
                  <CardHeader className="pb-2">
                    <CardTitle className="text-sm font-medium text-muted-foreground">
                      Active Clusters
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="text-3xl font-bold">{topology?.totalClusters || 0}</div>
                  </CardContent>
                </Card>
                <Card>
                  <CardHeader className="pb-2">
                    <CardTitle className="text-sm font-medium text-muted-foreground">
                      System Health
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="text-3xl font-bold text-green-600">
                      {topology?.systemHealth ? `${(topology.systemHealth * 100).toFixed(0)}%` : 'N/A'}
                    </div>
                  </CardContent>
                </Card>
              </div>

              <Card className="mt-6">
                <CardHeader>
                  <CardTitle>Topology Visualization</CardTitle>
                  <CardDescription>
                    Graph view of your project topology (coming soon)
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  <div className="h-64 flex items-center justify-center bg-muted rounded-lg">
                    <div className="text-center text-muted-foreground">
                      <Network className="w-12 h-12 mx-auto mb-2 opacity-50" />
                      <p>Topology visualization will be rendered here</p>
                    </div>
                  </div>
                </CardContent>
              </Card>
            </TabsContent>

            {/* Access Control Tab */}
            <TabsContent value="access" className="flex-1 overflow-auto">
              <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
                <Card>
                  <CardHeader>
                    <CardTitle className="flex items-center gap-2">
                      <Eye className="w-5 h-5" />
                      Your Access Level
                    </CardTitle>
                    <CardDescription>
                      Topsi enforces strict data isolation between clients
                    </CardDescription>
                  </CardHeader>
                  <CardContent className="space-y-4">
                    <div className="p-4 bg-muted rounded-lg">
                      <div className="flex items-center gap-2 mb-2">
                        <Badge variant={status?.accessScope === 'admin' ? 'default' : 'secondary'}>
                          {status?.accessScope === 'admin' ? 'Admin' : 'User'}
                        </Badge>
                        <span className="text-sm text-muted-foreground">Access Scope</span>
                      </div>
                      <p className="text-sm">
                        {status?.accessScope === 'admin'
                          ? 'You have full platform visibility and can see all projects and data.'
                          : `You can access ${status?.projectsVisible || 0} project(s) based on your permissions.`}
                      </p>
                    </div>

                    <div className="flex items-start gap-3 p-3 border rounded-lg">
                      <Lock className="w-5 h-5 text-green-600 mt-0.5" />
                      <div>
                        <div className="font-medium">Client Data Isolation</div>
                        <p className="text-sm text-muted-foreground">
                          Topsi ensures that client data is never shared between users
                          without explicit permission.
                        </p>
                      </div>
                    </div>
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader>
                    <CardTitle className="flex items-center gap-2">
                      <FolderOpen className="w-5 h-5" />
                      Accessible Projects
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    {projects.length === 0 ? (
                      <div className="text-center py-8 text-muted-foreground">
                        <FolderOpen className="w-8 h-8 mx-auto mb-2 opacity-50" />
                        <p>No projects accessible</p>
                      </div>
                    ) : (
                      <ScrollArea className="h-64">
                        <div className="space-y-2">
                          {projects.map((project) => (
                            <div
                              key={project.projectId}
                              className="flex items-center justify-between p-3 border rounded-lg"
                            >
                              <div>
                                <div className="font-medium">{project.projectName}</div>
                                <div className="text-xs text-muted-foreground">
                                  Granted: {new Date(project.grantedAt).toLocaleDateString()}
                                </div>
                              </div>
                              <Badge variant="outline" className="capitalize">
                                {project.role}
                              </Badge>
                            </div>
                          ))}
                        </div>
                      </ScrollArea>
                    )}
                  </CardContent>
                </Card>
              </div>
            </TabsContent>

            {/* Issues Tab */}
            <TabsContent value="issues" className="flex-1 overflow-auto">
              <Card>
                <CardHeader>
                  <div className="flex items-center justify-between">
                    <div>
                      <CardTitle>Detected Issues</CardTitle>
                      <CardDescription>
                        Topsi automatically detects topology issues and suggests fixes
                      </CardDescription>
                    </div>
                    <Button variant="outline" onClick={fetchIssues}>
                      <RefreshCw className="w-4 h-4 mr-2" />
                      Refresh
                    </Button>
                  </div>
                </CardHeader>
                <CardContent>
                  {issues.length === 0 ? (
                    <div className="text-center py-12">
                      <CheckCircle className="w-12 h-12 mx-auto mb-3 text-green-500" />
                      <h3 className="text-lg font-medium">No Issues Detected</h3>
                      <p className="text-muted-foreground">
                        Your topology is healthy
                      </p>
                    </div>
                  ) : (
                    <div className="space-y-4">
                      {issues.map((issue, index) => (
                        <div
                          key={index}
                          className={cn(
                            "p-4 border rounded-lg",
                            issue.severity === 'critical' && "border-red-500 bg-red-50 dark:bg-red-950/20",
                            issue.severity === 'warning' && "border-yellow-500 bg-yellow-50 dark:bg-yellow-950/20"
                          )}
                        >
                          <div className="flex items-start justify-between mb-2">
                            <div className="flex items-center gap-2">
                              <AlertTriangle className={cn(
                                "w-4 h-4",
                                issue.severity === 'critical' && "text-red-500",
                                issue.severity === 'warning' && "text-yellow-500"
                              )} />
                              <span className="font-medium capitalize">{issue.issueType}</span>
                            </div>
                            <Badge variant={issue.severity === 'critical' ? 'destructive' : 'outline'}>
                              {issue.severity}
                            </Badge>
                          </div>
                          <p className="text-sm text-muted-foreground mb-2">
                            {issue.description}
                          </p>
                          {issue.suggestedAction && (
                            <div className="text-sm bg-background p-2 rounded border">
                              <span className="font-medium">Suggested: </span>
                              {issue.suggestedAction}
                            </div>
                          )}
                        </div>
                      ))}
                    </div>
                  )}
                </CardContent>
              </Card>
            </TabsContent>
          </Tabs>
        )}
      </div>
    </div>
  );
}
