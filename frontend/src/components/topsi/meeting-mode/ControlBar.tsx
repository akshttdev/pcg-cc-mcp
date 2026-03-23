import {
  Clock,
  Monitor,
  Network,
  Users,
} from 'lucide-react';

import { Badge } from '@/components/ui/badge';

import type { MeetingRole, MeetingState } from './types';
import { formatDuration } from './types';

interface ControlBarProps {
  meetingState: MeetingState;
  meetingRole: MeetingRole;
  meetingTitle: string;
  isScreenSharing: boolean;
  duration: number;
  participantCount: number;
  participants: string[];
}

export function ControlBar({
  meetingState,
  meetingRole,
  meetingTitle,
  isScreenSharing,
  duration,
  participantCount,
  participants,
}: ControlBarProps) {
  return (
    <div className="flex items-center justify-between p-3 border-b bg-cyan-50 dark:bg-cyan-950">
      <div className="flex items-center gap-2">
        <Network className="h-4 w-4 text-cyan-600" />
        <span className="text-sm font-medium">
          {meetingState === 'idle' ? 'Meetings' : meetingTitle || 'Meeting'}
        </span>
        {meetingState === 'active' && (
          <Badge className="bg-red-500 text-white text-xs animate-pulse">REC</Badge>
        )}
        {meetingState === 'paused' && (
          <Badge variant="secondary" className="text-xs">Paused</Badge>
        )}
        {meetingRole === 'observer' && meetingState !== 'idle' && meetingState !== 'ended' && (
          <Badge variant="outline" className="text-xs text-blue-600 border-blue-300">Joined</Badge>
        )}
        {isScreenSharing && (
          <Badge className="bg-blue-500 text-white text-xs">
            <Monitor className="h-3 w-3 mr-1" />
            Screen
          </Badge>
        )}
      </div>
      <div className="flex items-center gap-2">
        {(meetingState === 'active' || meetingState === 'paused') && (
          <>
            <div className="flex items-center gap-1 text-xs text-muted-foreground">
              <Clock className="h-3 w-3" />
              {formatDuration(duration)}
            </div>
            <div className="flex items-center gap-1 text-xs text-muted-foreground">
              <Users className="h-3 w-3" />
              {Math.max(participantCount, participants.length)}
            </div>
          </>
        )}
      </div>
    </div>
  );
}
