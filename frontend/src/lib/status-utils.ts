import type { StatusVariant } from '@/components/ui/status-badge';
import {
  CheckCircle2, AlertTriangle, Loader2, Clock, Circle,
  AlertCircle, Activity, Pause, HelpCircle, Skull,
  FileText, Eye, Send, Search,
} from 'lucide-react';
import type { LucideIcon } from 'lucide-react';

export type StatusContext = 'task' | 'flow' | 'workflow' | 'intelligence' | 'proposal' | 'review' | 'execution';

export interface StatusInfo {
  variant: StatusVariant;
  icon: LucideIcon;
  label: string;
}

export function getStatusInfo(status: string, context: StatusContext): StatusInfo {
  const normalized = status?.toLowerCase().replace(/[\s-]/g, '_') ?? '';

  if (context === 'task') {
    switch (normalized) {
      case 'done': return { variant: 'success', icon: CheckCircle2, label: 'Done' };
      case 'inreview': return { variant: 'warning', icon: Eye, label: 'In Review' };
      case 'inprogress': return { variant: 'info', icon: Activity, label: 'In Progress' };
      case 'cancelled': return { variant: 'error', icon: AlertCircle, label: 'Cancelled' };
      case 'todo': return { variant: 'muted', icon: Circle, label: 'To Do' };
      default: return { variant: 'muted', icon: Circle, label: status || 'Unknown' };
    }
  }

  if (context === 'flow') {
    switch (normalized) {
      case 'completed': return { variant: 'success', icon: CheckCircle2, label: 'Completed' };
      case 'failed': return { variant: 'error', icon: AlertTriangle, label: 'Failed' };
      case 'executing': return { variant: 'info', icon: Loader2, label: 'Executing' };
      case 'planning': return { variant: 'pending', icon: Clock, label: 'Planning' };
      case 'paused': return { variant: 'warning', icon: Pause, label: 'Paused' };
      case 'awaiting_approval': return { variant: 'warning', icon: Clock, label: 'Awaiting Approval' };
      case 'needs_clarification': return { variant: 'warning', icon: HelpCircle, label: 'Needs Clarification' };
      case 'verifying': return { variant: 'info', icon: Search, label: 'Verifying' };
      default: return { variant: 'muted', icon: Circle, label: status || 'Unknown' };
    }
  }

  if (context === 'workflow') {
    switch (normalized) {
      case 'completed': return { variant: 'success', icon: CheckCircle2, label: 'Completed' };
      case 'failed': return { variant: 'error', icon: AlertCircle, label: 'Failed' };
      case 'running': return { variant: 'pending', icon: Loader2, label: 'Running' };
      default: return { variant: 'muted', icon: Circle, label: status || 'Unknown' };
    }
  }

  if (context === 'intelligence') {
    switch (normalized) {
      case 'done': case 'complete': return { variant: 'success', icon: CheckCircle2, label: 'Done' };
      case 'running': return { variant: 'info', icon: Loader2, label: 'Running' };
      case 'queued': return { variant: 'pending', icon: Clock, label: 'Queued' };
      case 'idle': return { variant: 'muted', icon: Circle, label: 'Idle' };
      default: return { variant: 'muted', icon: Circle, label: status || 'Unknown' };
    }
  }

  if (context === 'proposal') {
    switch (normalized) {
      case 'approved': return { variant: 'success', icon: CheckCircle2, label: 'Approved' };
      case 'sent': return { variant: 'info', icon: Send, label: 'Sent' };
      case 'draft': return { variant: 'warning', icon: FileText, label: 'Draft' };
      default: return { variant: 'muted', icon: Circle, label: status || 'Unknown' };
    }
  }

  if (context === 'review') {
    switch (normalized) {
      case 'approved': return { variant: 'success', icon: CheckCircle2, label: 'Approved' };
      case 'rejected': return { variant: 'error', icon: AlertTriangle, label: 'Rejected' };
      case 'revision_requested': return { variant: 'warning', icon: AlertCircle, label: 'Changes Requested' };
      case 'pending': return { variant: 'pending', icon: Clock, label: 'Pending' };
      case 'acknowledged': return { variant: 'success', icon: CheckCircle2, label: 'Acknowledged' };
      default: return { variant: 'muted', icon: Circle, label: status || 'Unknown' };
    }
  }

  if (context === 'execution') {
    switch (normalized) {
      case 'completed': return { variant: 'success', icon: CheckCircle2, label: 'Completed' };
      case 'failed': return { variant: 'error', icon: AlertTriangle, label: 'Failed' };
      case 'killed': return { variant: 'error', icon: Skull, label: 'Killed' };
      case 'running': return { variant: 'info', icon: Loader2, label: 'Running' };
      default: return { variant: 'muted', icon: Circle, label: status || 'Unknown' };
    }
  }

  return { variant: 'muted', icon: Circle, label: status || 'Unknown' };
}
