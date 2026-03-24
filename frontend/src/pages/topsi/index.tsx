import { useState, useEffect, useCallback } from 'react';
import { topsiApi } from '@/lib/api';
import type { TopsiStatusResponse, TopologyOverview, DetectedIssue, ProjectAccess } from '@/lib/api';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Separator } from '@/components/ui/separator';
import {
  Crown,
  MessageSquare,
  Network,
  Shield,
  AlertTriangle,
  Bot,
  Loader2,
  Users,
  Plug,
} from 'lucide-react';
import { toast } from 'sonner';
import { cn } from '@/lib/utils';
import { AgentIntegrationsTab } from '@/components/email';

import { TopsiChatTab } from './TopsiChatTab';
import { TopsiMeetingsTab } from './TopsiMeetingsTab';
import { TopsiTopologyTab } from './TopsiTopologyTab';
import { TopsiAccessTab } from './TopsiAccessTab';
import { TopsiIssuesTab } from './TopsiIssuesTab';

export function TopsiPage() {

  const [activeTab, setActiveTab] = useState('chat');
  const [status, setStatus] = useState<TopsiStatusResponse | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [topology, setTopology] = useState<TopologyOverview | null>(null);
  const [issues, setIssues] = useState<DetectedIssue[]>([]);
  const [projects, setProjects] = useState<ProjectAccess[]>([]);
  const [sessionId] = useState(() => `topsi-${Date.now()}`);

  // Fetch Topsi status
  const fetchStatus = useCallback(async () => {
    try {
      const data = await topsiApi.getStatus();
      setStatus(data);
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
      await topsiApi.initialize(true);
      toast.success('Topsi initialized successfully');
      await fetchStatus();
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
      const data = await topsiApi.getTopology();
      setTopology(data);
    } catch (error) {
      console.error('Failed to fetch topology:', error);
    }
  }, []);

  // Fetch issues
  const fetchIssues = useCallback(async () => {
    try {
      const data = await topsiApi.getIssues();
      setIssues(data.issues || []);
    } catch (error) {
      console.error('Failed to fetch issues:', error);
    }
  }, []);

  // Fetch accessible projects
  const fetchProjects = useCallback(async () => {
    try {
      const data = await topsiApi.getProjects();
      setProjects(data.projects || []);
    } catch (error) {
      console.error('Failed to fetch projects:', error);
    }
  }, []);

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
      <div className="border-b glass-strong relative overflow-hidden">
        <div className="ambient-glow -top-48 -right-32" />
        <div className="page-header p-4 sm:p-6 relative">
          <div className="flex items-center gap-3">
            <div className="section-header-icon !bg-gradient-to-br !from-cyan-500/10 !to-cyan-500/5 !text-cyan-600">
              <Crown className="w-5 h-5" />
            </div>
            <div>
              <h1 className="page-title">
                Topsi Platform Agent
              </h1>
              <p className="page-description">
                Topological Super Intelligence — your platform orchestrator
              </p>
            </div>
          </div>

          <div className="flex items-center gap-3 sm:gap-4">
            <div className="hidden md:flex items-center gap-2">
              <Shield className="w-4 h-4 text-green-600" />
              <span className="text-sm text-muted-foreground">Data Isolation Active</span>
            </div>
            <Separator orientation="vertical" className="h-8 hidden sm:block" />
            <div className="text-right">
              <div className={cn(
                "text-sm font-medium",
                status?.isActive ? "text-green-600" : "text-muted-foreground"
              )}>
                {status?.isActive ? 'Online' : 'Offline'}
              </div>
              <div className="text-xs text-muted-foreground">
                Uptime: {formatUptime(status?.uptimeMs)}
              </div>
            </div>
            <div className={cn(
              "status-dot",
              status?.isActive ? "status-dot-online" : "status-dot-offline"
            )} />
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="flex-1 p-4 sm:p-6 overflow-hidden">
        {isLoading ? (
          <div className="flex items-center justify-center h-full">
            <Loader2 className="w-8 h-8 animate-spin text-cyan-600" />
          </div>
        ) : !status?.isActive ? (
          <div className="empty-state h-full gap-4">
            <Network className="w-16 h-16 text-muted-foreground/30" />
            <h2 className="empty-state-title">Topsi is not initialized</h2>
            <p className="empty-state-description">Initialize Topsi to start managing your platform</p>
            <Button onClick={initializeTopsi} className="bg-cyan-600 hover:bg-cyan-700 mt-2">
              <Bot className="w-4 h-4 mr-2" />
              Initialize Topsi
            </Button>
          </div>
        ) : (
          <Tabs value={activeTab} onValueChange={setActiveTab} className="h-full flex flex-col">
            <TabsList className="tab-grid-6 mb-4 sm:mb-6">
              <TabsTrigger value="chat" className="flex items-center gap-2">
                <MessageSquare className="w-4 h-4" />
                Chat
              </TabsTrigger>
              <TabsTrigger value="meetings" className="flex items-center gap-2">
                <Users className="w-4 h-4" />
                Meetings
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
              <TabsTrigger value="integrations" className="flex items-center gap-2">
                <Plug className="w-4 h-4" />
                Integrations
              </TabsTrigger>
            </TabsList>

            {/* Chat Tab */}
            <TabsContent value="chat" className="flex-1 overflow-auto">
              <TopsiChatTab
                status={status}
                topology={topology}
                sessionId={sessionId}
                fetchTopology={fetchTopology}
                fetchIssues={fetchIssues}
                fetchProjects={fetchProjects}
              />
            </TabsContent>

            {/* Meetings Tab */}
            <TabsContent value="meetings" className="flex-1 overflow-auto">
              <TopsiMeetingsTab />
            </TabsContent>

            {/* Topology Tab */}
            <TabsContent value="topology" className="flex-1 overflow-auto">
              <TopsiTopologyTab topology={topology} />
            </TabsContent>

            {/* Access Control Tab */}
            <TabsContent value="access" className="flex-1 overflow-auto">
              <TopsiAccessTab status={status} projects={projects} />
            </TabsContent>

            {/* Issues Tab */}
            <TabsContent value="issues" className="flex-1 overflow-auto">
              <TopsiIssuesTab issues={issues} fetchIssues={fetchIssues} />
            </TabsContent>

            {/* Integrations Tab */}
            <TabsContent value="integrations" className="flex-1 overflow-auto">
              <div className="max-w-3xl">
                <AgentIntegrationsTab
                  ownerType="agent"
                  ownerId="f8237e4b6b324fada8192e13b808e96a"
                  agentName="Topsi"
                  channels={[
                    {
                      id: 'email',
                      label: 'Email',
                      address: 'topsi@powerclubglobal.com',
                      provider: 'zoho',
                      description: "Topsi's platform notification email",
                    },
                  ]}
                />
              </div>
            </TabsContent>
          </Tabs>
        )}
      </div>
    </div>
  );
}
