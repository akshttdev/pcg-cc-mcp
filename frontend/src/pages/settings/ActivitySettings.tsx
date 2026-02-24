import { useState } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Button } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Activity,
  CheckCircle2,
  XCircle,
  AlertCircle,
  FileText,
  Users,
  Settings,
  LogIn,
  LogOut,
  Trash2,
  Download,
  Clock,
} from 'lucide-react';
import { cn } from '@/lib/utils';

type ActivityType = 'login' | 'logout' | 'create' | 'update' | 'delete' | 'invite' | 'settings';
type ActivityStatus = 'success' | 'warning' | 'error';

interface ActivityLog {
  id: string;
  type: ActivityType;
  action: string;
  description: string;
  timestamp: string;
  status: ActivityStatus;
  ipAddress?: string;
  device?: string;
}

// Mock activity logs
const mockActivityLogs: ActivityLog[] = [
  {
    id: '1',
    type: 'login',
    action: 'Signed In',
    description: 'Successfully signed in to your account',
    timestamp: '2 minutes ago',
    status: 'success',
    ipAddress: '192.168.1.1',
    device: 'Chrome on macOS',
  },
  {
    id: '2',
    type: 'create',
    action: 'Created Task',
    description: 'Created new task "Implement authentication"',
    timestamp: '15 minutes ago',
    status: 'success',
  },
  {
    id: '3',
    type: 'invite',
    action: 'Invited Team Member',
    description: 'Sent invitation to jane@example.com',
    timestamp: '1 hour ago',
    status: 'success',
  },
  {
    id: '4',
    type: 'update',
    action: 'Updated Settings',
    description: 'Changed privacy settings',
    timestamp: '2 hours ago',
    status: 'success',
  },
  {
    id: '5',
    type: 'delete',
    action: 'Deleted Project',
    description: 'Removed project "Old Demo"',
    timestamp: '3 hours ago',
    status: 'warning',
  },
  {
    id: '6',
    type: 'login',
    action: 'Failed Login Attempt',
    description: 'Incorrect password entered',
    timestamp: '1 day ago',
    status: 'error',
    ipAddress: '203.0.113.0',
    device: 'Firefox on Windows',
  },
  {
    id: '7',
    type: 'settings',
    action: 'Updated Profile',
    description: 'Changed profile information',
    timestamp: '2 days ago',
    status: 'success',
  },
  {
    id: '8',
    type: 'logout',
    action: 'Signed Out',
    description: 'Logged out from all devices',
    timestamp: '3 days ago',
    status: 'success',
  },
];

const ACTIVITY_ICONS: Record<ActivityType, typeof Activity> = {
  login: LogIn,
  logout: LogOut,
  create: FileText,
  update: Settings,
  delete: Trash2,
  invite: Users,
  settings: Settings,
};

const STATUS_ICONS = {
  success: CheckCircle2,
  warning: AlertCircle,
  error: XCircle,
};

export function ActivitySettings() {
  const [filter, setFilter] = useState<'all' | ActivityType>('all');
  const [timeRange, setTimeRange] = useState('7');

  const filteredLogs = mockActivityLogs.filter(
    (log) => filter === 'all' || log.type === filter
  );

  const getStatusColor = (status: ActivityStatus) => {
    switch (status) {
      case 'success':
        return 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200';
      case 'warning':
        return 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200';
      case 'error':
        return 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200';
    }
  };

  const handleExportLogs = () => {
    alert('Activity logs will be downloaded as CSV');
  };

  const handleClearLogs = () => {
    if (confirm('Are you sure you want to clear all activity logs?')) {
      alert('Activity logs cleared');
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold">Activity Log</h1>
        <p className="text-muted-foreground mt-2">
          View your recent account activity and security events
        </p>
      </div>

      {/* Filters */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">Filters</CardTitle>
        </CardHeader>
        <CardContent className="flex flex-col sm:flex-row gap-4">
          <div className="flex-1">
            <label className="text-sm font-medium mb-1.5 block">Activity Type</label>
            <Select value={filter} onValueChange={(v: any) => setFilter(v)}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">All Activities</SelectItem>
                <SelectItem value="login">Sign In/Out</SelectItem>
                <SelectItem value="create">Created Items</SelectItem>
                <SelectItem value="update">Updates</SelectItem>
                <SelectItem value="delete">Deletions</SelectItem>
                <SelectItem value="invite">Invitations</SelectItem>
                <SelectItem value="settings">Settings Changes</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="flex-1">
            <label className="text-sm font-medium mb-1.5 block">Time Range</label>
            <Select value={timeRange} onValueChange={setTimeRange}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="1">Last 24 hours</SelectItem>
                <SelectItem value="7">Last 7 days</SelectItem>
                <SelectItem value="30">Last 30 days</SelectItem>
                <SelectItem value="90">Last 90 days</SelectItem>
                <SelectItem value="all">All time</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </CardContent>
      </Card>

      {/* Activity List */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <div>
            <CardTitle>Recent Activity</CardTitle>
            <CardDescription>{filteredLogs.length} events found</CardDescription>
          </div>
          <div className="flex gap-2">
            <Button variant="outline" size="sm" onClick={handleExportLogs}>
              <Download className="mr-2 h-4 w-4" />
              Export
            </Button>
            <Button variant="outline" size="sm" onClick={handleClearLogs}>
              <Trash2 className="mr-2 h-4 w-4" />
              Clear
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          <ScrollArea className="h-[600px] pr-4">
            <div className="space-y-3">
              {filteredLogs.map((log) => {
                const ActivityIcon = ACTIVITY_ICONS[log.type];
                const StatusIcon = STATUS_ICONS[log.status];

                return (
                  <div
                    key={log.id}
                    className="border rounded-lg p-4 hover:bg-accent/50 transition-colors"
                  >
                    <div className="flex items-start gap-3">
                      {/* Icon */}
                      <div className="w-10 h-10 rounded-full bg-primary/10 flex items-center justify-center shrink-0">
                        <ActivityIcon className="h-5 w-5 text-primary" />
                      </div>

                      {/* Content */}
                      <div className="flex-1 min-w-0">
                        <div className="flex items-start justify-between gap-2">
                          <div className="flex-1">
                            <div className="font-medium">{log.action}</div>
                            <div className="text-sm text-muted-foreground mt-0.5">
                              {log.description}
                            </div>
                          </div>
                          <Badge className={cn('shrink-0', getStatusColor(log.status))}>
                            <StatusIcon className="h-3 w-3 mr-1" />
                            {log.status}
                          </Badge>
                        </div>

                        {/* Metadata */}
                        <div className="flex items-center gap-4 mt-2 text-xs text-muted-foreground">
                          <div className="flex items-center gap-1">
                            <Clock className="h-3 w-3" />
                            {log.timestamp}
                          </div>
                          {log.ipAddress && (
                            <div className="flex items-center gap-1">
                              <span>IP: {log.ipAddress}</span>
                            </div>
                          )}
                          {log.device && (
                            <div className="flex items-center gap-1">
                              <span>{log.device}</span>
                            </div>
                          )}
                        </div>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </ScrollArea>
        </CardContent>
      </Card>

      {/* Security Notice */}
      <Card className="border-yellow-200 dark:border-yellow-800">
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-sm">
            <AlertCircle className="h-4 w-4 text-yellow-600" />
            Security Notice
          </CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">
            If you notice any suspicious activity, please change your password immediately and
            enable two-factor authentication in Privacy & Security settings.
          </p>
        </CardContent>
      </Card>
    </div>
  );
}
