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
            <TabsContent value="chat" className="flex-1 overflow-hidden">
              <div className="grid grid-cols-1 lg:grid-cols-3 gap-6 h-full">
                {/* Chat Interface */}
                <div className="lg:col-span-2 flex flex-col h-full min-h-0">
                  <Card className="flex-1 flex flex-col min-h-0 h-full">
                    <CardHeader className="flex-shrink-0">
                      <CardTitle className="text-lg">Chat with Topsi</CardTitle>
                      <CardDescription>
                        Ask questions, get insights, or manage your projects
                      </CardDescription>
                    </CardHeader>
                    <CardContent className="flex-1 flex flex-col min-h-0 overflow-hidden">
                      <ScrollArea className="flex-1 min-h-0 pr-4" style={{ maxHeight: 'calc(100vh - 400px)' }}>
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
                                <p className={cn(
                                  "text-xs mt-1",
                                  msg.role === 'user' ? "text-cyan-100" : "text-muted-foreground"
                                )}>
                                  {msg.timestamp.toLocaleTimeString()}
                                </p>
                              </div>
                            </div>
                          ))}
                          <div ref={messagesEndRef} />
                        </div>
                      </ScrollArea>
                      <div className="flex gap-2 mt-4 flex-shrink-0">
                        <Input
                          placeholder="Ask Topsi anything..."
                          value={inputMessage}
                          onChange={(e) => setInputMessage(e.target.value)}
                          onKeyDown={(e) => e.key === 'Enter' && !e.shiftKey && sendMessage()}
                          disabled={isSending}
                        />
                        <Button onClick={sendMessage} disabled={isSending} className="bg-cyan-600 hover:bg-cyan-700">
                          {isSending ? (
                            <Loader2 className="w-4 h-4 animate-spin" />
                          ) : (
                            <Send className="w-4 h-4" />
                          )}
                        </Button>
                      </div>
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
