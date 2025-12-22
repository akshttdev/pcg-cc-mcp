import { useState, useEffect, useCallback, useRef } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Loader } from '@/components/ui/loader';
import {
  Users,
  AlertTriangle,
  CheckCircle,
  Clock,
  Zap,
  Activity,
  AlertCircle,
  TrendingUp,
  Calendar,
  Bot,
} from 'lucide-react';
import {
  AgentCoordinationState,
  AgentStatus,
  CoordinationEvent,
  CoordinationStats,
} from '@/types/nora';

interface NoraCoordinationPanelProps {
  className?: string;
}

export function NoraCoordinationPanel({ className }: NoraCoordinationPanelProps) {
  const [stats, setStats] = useState<CoordinationStats | null>(null);
  const [agents, setAgents] = useState<AgentCoordinationState[]>([]);
  const [events, setEvents] = useState<CoordinationEvent[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [isSocketConnected, setIsSocketConnected] = useState(false);
  const websocketRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<number | undefined>(undefined);

  const fetchCoordinationData = useCallback(async () => {
    try {
      setIsLoading(true);
      setErrorMessage(null);

      // Fetch coordination stats
      const statsResponse = await fetch('/api/nora/coordination/stats');
      if (statsResponse.ok) {
        const statsData = (await statsResponse.json()) as CoordinationStats;
        setStats(statsData);
      }

      // Fetch agent states
      const agentsResponse = await fetch('/api/nora/coordination/agents');
      if (agentsResponse.ok) {
        const agentsData = (await agentsResponse.json()) as AgentCoordinationState[];
        setAgents(agentsData);
      }
    } catch (error) {
      console.error('Failed to fetch coordination data:', error);
      setErrorMessage('Unable to load coordination data. Please try again later.');
    } finally {
      setIsLoading(false);
    }
  }, []);

  const handleEventMessage = useCallback(
    (event: CoordinationEvent) => {
      setEvents(prev => [event, ...prev.slice(0, 49)]);
      if (event.type === 'AgentStatusUpdate') {
        void fetchCoordinationData();
      }
    },
    [fetchCoordinationData]
  );

  const setupWebSocket = useCallback(() => {
    if (typeof window === 'undefined') {
      return;
    }

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const ws = new WebSocket(`${protocol}//${window.location.host}/api/nora/coordination/events`);

    ws.onopen = () => {
      websocketRef.current = ws;
      setIsSocketConnected(true);
    };

    ws.onmessage = (event) => {
      try {
        const message = JSON.parse(event.data) as CoordinationEvent;
        handleEventMessage(message);
      } catch (parseError) {
        console.error('Failed to parse coordination event:', parseError);
      }
    };

    ws.onclose = () => {
      websocketRef.current = null;
      reconnectTimeoutRef.current = window.setTimeout(setupWebSocket, 5000);
      setIsSocketConnected(false);
    };

    ws.onerror = (evt) => {
      console.error('Coordination WebSocket error:', evt);
    };
  }, [handleEventMessage]);

  useEffect(() => {
    void fetchCoordinationData();
    setupWebSocket();

    return () => {
      if (websocketRef.current) {
        websocketRef.current.close();
        websocketRef.current = null;
      }
      if (reconnectTimeoutRef.current) {
        window.clearTimeout(reconnectTimeoutRef.current);
      }
      setIsSocketConnected(false);
    };
  }, [fetchCoordinationData, setupWebSocket]);

  const getStatusIcon = (status: AgentStatus) => {
    switch (status) {
      case 'active':
        return <CheckCircle className="w-4 h-4 text-green-500" />;
      case 'busy':
        return <Clock className="w-4 h-4 text-yellow-500" />;
      case 'idle':
        return <Clock className="w-4 h-4 text-blue-500" />;
      case 'offline':
        return <AlertCircle className="w-4 h-4 text-gray-500" />;
      case 'error':
        return <AlertTriangle className="w-4 h-4 text-red-500" />;
      case 'maintenance':
        return <Activity className="w-4 h-4 text-orange-500" />;
      default:
        return <AlertCircle className="w-4 h-4 text-gray-500" />;
    }
  };

  const getStatusColor = (status: AgentStatus) => {
    switch (status) {
      case 'active':
        return 'bg-green-100 text-green-800';
      case 'busy':
        return 'bg-yellow-100 text-yellow-800';
      case 'idle':
        return 'bg-blue-100 text-blue-800';
      case 'offline':
        return 'bg-gray-100 text-gray-800';
      case 'error':
        return 'bg-red-100 text-red-800';
      case 'maintenance':
        return 'bg-orange-100 text-orange-800';
      default:
        return 'bg-gray-100 text-gray-800';
    }
  };

  const formatUptime = (percentage: number) => {
    return `${percentage.toFixed(1)}%`;
  };

  const formatResponseTime = (ms: number) => {
    if (ms < 1000) return `${Math.round(ms)}ms`;
    return `${(ms / 1000).toFixed(1)}s`;
  };

  const toTitleCase = (value: string) =>
    value.charAt(0).toUpperCase() + value.slice(1);

  const formatStatusLabel = (status: AgentStatus) => toTitleCase(status);

  const getEventIcon = (eventType: string) => {
    switch (eventType) {
      case 'AgentStatusUpdate': return <Activity className="w-4 h-4 text-blue-500" />;
      case 'TaskHandoff': return <Zap className="w-4 h-4 text-purple-500" />;
      case 'ConflictResolution': return <AlertTriangle className="w-4 h-4 text-orange-500" />;
      case 'ApprovalRequest': return <CheckCircle className="w-4 h-4 text-green-500" />;
      case 'ExecutiveAlert': return <AlertCircle className="w-4 h-4 text-red-500" />;
      case 'HumanAvailabilityUpdate': return <Users className="w-4 h-4 text-emerald-500" />;
      case 'AgentDirective': return <Bot className="w-4 h-4 text-purple-500" />;
      default: return <Clock className="w-4 h-4 text-gray-500" />;
    }
  };

  const formatEventDescription = (event: CoordinationEvent): string => {
    switch (event.type) {
      case 'AgentStatusUpdate':
        return `Agent ${event.agentId} is now ${event.status}.`;
      case 'TaskHandoff':
        return `Task ${event.taskId} handed from ${event.fromAgent} to ${event.toAgent}.`;
      case 'ConflictResolution':
        return `Conflict ${event.conflictId} (${toTitleCase(event.priority)}) - ${event.description}.`;
      case 'ApprovalRequest':
        return `${event.requestingAgent} requested approval from ${event.requiredApprover} (${toTitleCase(event.urgency)}).`;
      case 'ExecutiveAlert':
        return `${event.severity} alert from ${event.source}: ${event.message}`;
      case 'HumanAvailabilityUpdate':
        return `${event.userId} is now ${toTitleCase(event.availability)}${event.availableUntil ? ` until ${new Date(event.availableUntil).toLocaleTimeString()}` : ''}.`;
      case 'AgentDirective':
        return `${event.issuedBy} directed ${event.agentId}: ${event.content}`;
      default:
        return 'Coordination update received.';
    }
  };

  if (isLoading) {
    return (
      <Card className={className}>
        <CardContent className="flex items-center justify-center py-8">
          <Loader message="Loading coordination data..." size={32} />
        </CardContent>
      </Card>
    );
  }

  if (errorMessage) {
    return (
      <Card className={className}>
        <CardContent className="py-6">
          <div className="text-center text-sm text-muted-foreground">
            {errorMessage}
          </div>
          <div className="mt-4 flex justify-center">
            <Button onClick={() => void fetchCoordinationData()}>Retry</Button>
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className={`space-y-6 ${className}`}>
      {/* Coordination Statistics */}
      {stats && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Users className="w-5 h-5 text-blue-600" />
              Coordination Overview
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-2 md:grid-cols-5 gap-4">
              <div className="text-center">
                <div className="text-2xl font-bold text-blue-600">{stats.totalAgents}</div>
                <div className="text-sm text-gray-600">Total Agents</div>
              </div>
              <div className="text-center">
                <div className="text-2xl font-bold text-green-600">{stats.activeAgents}</div>
                <div className="text-sm text-gray-600">Active</div>
              </div>
              <div className="text-center">
                <div className="text-2xl font-bold text-yellow-600">{stats.pendingApprovals}</div>
                <div className="text-sm text-gray-600">Pending Approvals</div>
              </div>
              <div className="text-center">
                <div className="text-2xl font-bold text-red-600">{stats.activeConflicts}</div>
                <div className="text-sm text-gray-600">Active Conflicts</div>
              </div>
              <div className="text-center">
                <div className="text-2xl font-bold text-purple-600">
                  {formatResponseTime(stats.averageResponseTime)}
                </div>
                <div className="text-sm text-gray-600">Avg Response</div>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Agent Status Grid */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Activity className="w-5 h-5 text-green-600" />
            Agent Status
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {agents.map((agent) => (
              <div key={agent.agentId} className="border rounded-lg p-4 space-y-3">
                <div className="flex items-center justify-between">
                  <div className="font-medium">{agent.agentType}</div>
                  <div className="flex items-center gap-2">
                    {getStatusIcon(agent.status)}
                    <Badge className={getStatusColor(agent.status)}>
                      {formatStatusLabel(agent.status)}
                    </Badge>
                  </div>
                </div>

                <div className="text-sm text-gray-600">
                  <div>ID: {agent.agentId.substring(0, 8)}...</div>
                  <div>Last seen: {new Date(agent.lastSeen).toLocaleTimeString()}</div>
                </div>

                {/* Capabilities */}
                <div>
                  <div className="text-sm font-medium mb-1">Capabilities:</div>
                  <div className="flex flex-wrap gap-1">
                    {agent.capabilities.map((capability) => (
                      <Badge key={capability} variant="secondary" className="text-xs">
                        {capability}
                      </Badge>
                    ))}
                  </div>
                </div>

                {/* Performance Metrics */}
                <div className="grid grid-cols-2 gap-2 text-sm">
                  <div>
                    <div className="text-gray-600">Tasks</div>
                    <div className="font-medium">{agent.performanceMetrics.tasksCompleted}</div>
                  </div>
                  <div>
                    <div className="text-gray-600">Success Rate</div>
                    <div className="font-medium">{(agent.performanceMetrics.successRate * 100).toFixed(1)}%</div>
                  </div>
                  <div>
                    <div className="text-gray-600">Response</div>
                    <div className="font-medium">{formatResponseTime(agent.performanceMetrics.averageResponseTimeMs)}</div>
                  </div>
                  <div>
                    <div className="text-gray-600">Uptime</div>
                    <div className="font-medium">{formatUptime(agent.performanceMetrics.uptimePercentage)}</div>
                  </div>
                </div>

                {/* Current Tasks */}
                {agent.currentTasks.length > 0 && (
                  <div>
                    <div className="text-sm font-medium mb-1">Current Tasks:</div>
                    <div className="space-y-1">
                      {agent.currentTasks.slice(0, 3).map((task, index) => (
                        <div key={index} className="text-xs bg-blue-50 px-2 py-1 rounded">
                          {task.length > 30 ? `${task.substring(0, 30)}...` : task}
                        </div>
                      ))}
                      {agent.currentTasks.length > 3 && (
                        <div className="text-xs text-gray-500">
                          +{agent.currentTasks.length - 3} more tasks
                        </div>
                      )}
                    </div>
                  </div>
                )}
              </div>
            ))}
          </div>
        </CardContent>
      </Card>

      {/* Recent Events */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <TrendingUp className="w-5 h-5 text-purple-600" />
            Recent Coordination Events
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-3 max-h-96 overflow-y-auto">
            {events.length === 0 ? (
              <div className="text-center text-gray-500 py-4">
                No recent events
              </div>
            ) : (
              events.map((event, index) => (
                <div key={index} className="flex items-start gap-3 p-3 bg-gray-50 rounded-lg">
                  {getEventIcon(event.type)}
                  <div className="flex-1">
                    <div className="flex items-center justify-between">
                      <div className="font-medium text-sm">{event.type}</div>
                      <div className="text-xs text-gray-500 flex items-center gap-1">
                        <Calendar className="w-3 h-3" />
                        {new Date(event.timestamp).toLocaleTimeString()}
                      </div>
                    </div>
                    <div className="text-sm text-gray-600 mt-1">
                      {formatEventDescription(event)}
                    </div>
                  </div>
                </div>
              ))
            )}
          </div>
        </CardContent>
      </Card>

      {/* Connection Status */}
      <Card>
        <CardContent className="py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <div className={`w-2 h-2 rounded-full ${isSocketConnected ? 'bg-green-500' : 'bg-red-500'}`} />
              <span className="text-sm">Real-time coordination</span>
            </div>
            <div className="text-sm text-gray-600">
              {isSocketConnected ? 'Connected' : 'Disconnected'}
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
