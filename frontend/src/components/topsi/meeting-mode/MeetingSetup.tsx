// ── Meeting Setup (Idle State) ──────────────────────────────────────────────
//
// New meeting form + join active meetings list, shown when meetingState === 'idle'.

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Mic,
  Users,
  Loader2,
  UserPlus,
  X,
  LogIn,
  RefreshCw,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { ActiveMeeting } from './types';

interface Project {
  id: string;
  name: string;
}

interface MeetingSetupProps {
  // Project
  propProjectId?: string;
  selectedProjectId: string;
  onSelectedProjectIdChange: (id: string) => void;
  projects: Project[];
  activeProjectId: string;

  // Title
  meetingTitle: string;
  onMeetingTitleChange: (title: string) => void;

  // Participants
  participants: string[];
  participantInput: string;
  onParticipantInputChange: (value: string) => void;
  onAddParticipant: () => void;
  onRemoveParticipant: (name: string) => void;

  // Actions
  isProcessing: boolean;
  onStartMeeting: () => void;

  // Join tab
  activeMeetings: ActiveMeeting[];
  isLoadingActive: boolean;
  onFetchActiveMeetings: () => void;
  onJoinMeeting: (meeting: ActiveMeeting) => void;
}

export function MeetingSetup({
  propProjectId,
  selectedProjectId,
  onSelectedProjectIdChange,
  projects,
  activeProjectId,
  meetingTitle,
  onMeetingTitleChange,
  participants,
  participantInput,
  onParticipantInputChange,
  onAddParticipant,
  onRemoveParticipant,
  isProcessing,
  onStartMeeting,
  activeMeetings,
  isLoadingActive,
  onFetchActiveMeetings,
  onJoinMeeting,
}: MeetingSetupProps) {
  return (
    <Tabs defaultValue="new" className="flex flex-col flex-1 min-h-0">
      <TabsList className="mx-4 mt-3 grid w-auto grid-cols-2">
        <TabsTrigger value="new">
          <Mic className="h-3.5 w-3.5 mr-1.5" />
          New Meeting
        </TabsTrigger>
        <TabsTrigger value="join" onClick={onFetchActiveMeetings}>
          <LogIn className="h-3.5 w-3.5 mr-1.5" />
          Join Active
        </TabsTrigger>
      </TabsList>

      {/* New meeting tab */}
      <TabsContent value="new" className="flex-1 overflow-auto">
        <ScrollArea className="flex-1">
          <div className="flex flex-col items-center p-6 gap-4 max-w-sm mx-auto">
            <div className="w-16 h-16 rounded-full bg-cyan-100 dark:bg-cyan-900 flex items-center justify-center">
              <Mic className="h-8 w-8 text-cyan-600" />
            </div>
            <div className="text-center">
              <h3 className="font-semibold">Start a Meeting</h3>
              <p className="text-sm text-muted-foreground mt-1">
                Topsi listens silently and takes structured notes.
                <br />Say <strong>"Topsi"</strong> to get its attention.
              </p>
            </div>

            {/* Project selector */}
            <div className="w-full space-y-1.5">
              <Label className="text-xs font-medium">Project</Label>
              {propProjectId ? (
                <p className="text-sm text-muted-foreground px-1">
                  {projects.find((p) => p.id === propProjectId)?.name || propProjectId}
                </p>
              ) : (
                <Select value={selectedProjectId} onValueChange={onSelectedProjectIdChange}>
                  <SelectTrigger className="h-9 text-sm">
                    <SelectValue placeholder="Select a project…" />
                  </SelectTrigger>
                  <SelectContent>
                    {projects.map((p) => (
                      <SelectItem key={p.id} value={p.id}>{p.name}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              )}
            </div>

            {/* Title */}
            <div className="w-full space-y-1.5">
              <Label className="text-xs font-medium">Meeting title <span className="text-muted-foreground">(optional)</span></Label>
              <Input
                placeholder="e.g. Bulgari Perfume kick-off"
                value={meetingTitle}
                onChange={(e) => onMeetingTitleChange(e.target.value)}
                className="h-9 text-sm"
              />
            </div>

            {/* Participants */}
            <div className="w-full space-y-1.5">
              <Label className="text-xs font-medium">Participants <span className="text-muted-foreground">(optional, for speaker attribution)</span></Label>
              <div className="flex gap-1.5">
                <Input
                  placeholder="Name or email"
                  value={participantInput}
                  onChange={(e) => onParticipantInputChange(e.target.value)}
                  onKeyDown={(e) => e.key === 'Enter' && onAddParticipant()}
                  className="h-9 text-sm"
                />
                <Button variant="outline" size="sm" onClick={onAddParticipant} className="h-9 px-2">
                  <UserPlus className="h-4 w-4" />
                </Button>
              </div>
              {participants.length > 0 && (
                <div className="flex flex-wrap gap-1 mt-1">
                  {participants.map((p) => (
                    <Badge key={p} variant="secondary" className="text-xs gap-1">
                      {p}
                      <button onClick={() => onRemoveParticipant(p)} className="hover:text-destructive">
                        <X className="h-2.5 w-2.5" />
                      </button>
                    </Badge>
                  ))}
                </div>
              )}
            </div>

            <Button
              onClick={onStartMeeting}
              disabled={isProcessing || !activeProjectId}
              className="bg-cyan-600 hover:bg-cyan-700 w-full"
            >
              {isProcessing ? (
                <Loader2 className="h-4 w-4 animate-spin mr-2" />
              ) : (
                <Mic className="h-4 w-4 mr-2" />
              )}
              Start Meeting
            </Button>
          </div>
        </ScrollArea>
      </TabsContent>

      {/* Join existing meeting tab */}
      <TabsContent value="join" className="flex-1 overflow-auto">
        <div className="p-4 space-y-3">
          <div className="flex items-center justify-between mb-1">
            <p className="text-sm text-muted-foreground">Active meetings you can join:</p>
            <Button variant="ghost" size="sm" onClick={onFetchActiveMeetings} className="h-7 px-2">
              <RefreshCw className={cn('h-3.5 w-3.5', isLoadingActive && 'animate-spin')} />
            </Button>
          </div>

          {isLoadingActive ? (
            <div className="flex justify-center py-6">
              <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            </div>
          ) : activeMeetings.length === 0 ? (
            <div className="text-center py-8 text-sm text-muted-foreground">
              <Users className="h-8 w-8 mx-auto mb-2 opacity-40" />
              <p>No active meetings right now.</p>
              <p className="text-xs mt-1">Ask a colleague to start one, or start your own.</p>
            </div>
          ) : (
            activeMeetings.map((m) => (
              <div
                key={m.id}
                className="flex items-center justify-between p-3 border rounded-lg hover:bg-muted/50"
              >
                <div className="min-w-0 flex-1 pr-3">
                  <div className="font-medium text-sm truncate">{m.title}</div>
                  <div className="text-xs text-muted-foreground flex items-center gap-2 mt-0.5">
                    <span>
                      {projects.find((p) => p.id === m.projectId)?.name || 'Unknown project'}
                    </span>
                    {m.participantCount != null && (
                      <span className="flex items-center gap-0.5">
                        <Users className="h-3 w-3" />
                        {m.participantCount}
                      </span>
                    )}
                  </div>
                </div>
                <Button
                  size="sm"
                  onClick={() => onJoinMeeting(m)}
                  disabled={isProcessing}
                  className="bg-cyan-600 hover:bg-cyan-700 shrink-0"
                >
                  {isProcessing ? (
                    <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  ) : (
                    <>
                      <LogIn className="h-3.5 w-3.5 mr-1" />
                      Join
                    </>
                  )}
                </Button>
              </div>
            ))
          )}
        </div>
      </TabsContent>
    </Tabs>
  );
}
