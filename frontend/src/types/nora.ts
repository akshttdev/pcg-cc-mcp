export type AgentStatus = 'active' | 'busy' | 'idle' | 'offline' | 'error' | 'maintenance';

export interface PerformanceMetrics {
  tasksCompleted: number;
  averageResponseTimeMs: number;
  successRate: number;
  uptimePercentage: number;
}

export interface AgentCoordinationState {
  agentId: string;
  agentType: string;
  status: AgentStatus;
  capabilities: string[];
  currentTasks: string[];
  lastSeen: string;
  performanceMetrics: PerformanceMetrics;
}

export interface CoordinationStats {
  totalAgents: number;
  activeAgents: number;
  pendingApprovals: number;
  activeConflicts: number;
  averageResponseTime: number;
}

export type CoordinationEvent =
  | {
      type: 'AgentStatusUpdate';
      agentId: string;
      status: AgentStatus;
      capabilities: string[];
      timestamp: string;
    }
  | {
      type: 'TaskHandoff';
      fromAgent: string;
      toAgent: string;
      taskId: string;
      context: unknown;
      timestamp: string;
    }
  | {
      type: 'ConflictResolution';
      conflictId: string;
      involvedAgents: string[];
      description: string;
      priority: 'low' | 'medium' | 'high' | 'critical' | 'Low' | 'Medium' | 'High' | 'Critical';
      timestamp: string;
    }
  | {
      type: 'HumanAvailabilityUpdate';
      userId: string;
      availability: 'available' | 'busy' | 'inMeeting' | 'doNotDisturb' | 'away' | 'offline' | string;
      availableUntil: string | null;
      timestamp: string;
    }
  | {
      type: 'ApprovalRequest';
      requestId: string;
      requestingAgent: string;
      actionDescription: string;
      requiredApprover: string;
      urgency: 'low' | 'normal' | 'high' | 'urgent' | 'emergency' | 'Low' | 'Normal' | 'High' | 'Urgent' | 'Emergency';
      timestamp: string;
    }
  | {
      type: 'ExecutiveAlert';
      alertId: string;
      source: string;
      message: string;
      severity: 'info' | 'warning' | 'error' | 'critical' | 'Info' | 'Warning' | 'Error' | 'Critical';
      requiresAction: boolean;
      timestamp: string;
    }
  | {
      type: 'AgentDirective';
      agentId: string;
      issuedBy: string;
      content: string;
      priority: string | null;
      timestamp: string;
    };
