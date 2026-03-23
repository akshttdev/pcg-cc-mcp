import {
  Link as LinkIcon,
  Loader2,
  Monitor,
  MonitorOff,
  Network,
  Pause,
  Play,
  Send,
  Square,
} from 'lucide-react';
import React, { type RefObject } from 'react';

import { Button } from '@/components/ui/button';
import { IconButton } from '@/components/ui/icon-button';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import { cn } from '@/lib/utils';

import type { MeetingRole, MeetingState, TranscriptEntry } from './types';
import { isUrl } from './types';

interface ActiveSessionProps {
  meetingState: MeetingState;
  meetingRole: MeetingRole;
  transcript: TranscriptEntry[];
  isScreenSharing: boolean;
  topsiOverlay: string | null;
  chatInput: string;
  isSendingChat: boolean;
  screenPreviewRef: RefObject<HTMLVideoElement | null>;
  transcriptEndRef: RefObject<HTMLDivElement | null>;
  onChatInputChange: (value: string) => void;
  onSendChatMessage: () => void;
  onStartScreenShare: () => void;
  onStopScreenShare: () => void;
  onPauseMeeting: () => void;
  onResumeMeeting: () => void;
  onEndMeeting: () => void;
}

export function ActiveSession({
  meetingState,
  meetingRole,
  transcript,
  isScreenSharing,
  topsiOverlay,
  chatInput,
  isSendingChat,
  screenPreviewRef,
  transcriptEndRef,
  onChatInputChange,
  onSendChatMessage,
  onStartScreenShare,
  onStopScreenShare,
  onPauseMeeting,
  onResumeMeeting,
  onEndMeeting,
}: ActiveSessionProps) {
  return (
    <>
      {/* Screen share preview */}
      {isScreenSharing && (
        <div className="mx-3 mt-2 relative">
          <video
            ref={screenPreviewRef as React.RefObject<HTMLVideoElement>}
            autoPlay
            muted
            playsInline
            className="w-full max-h-32 rounded-lg border bg-black object-contain"
          />
          <button
            onClick={onStopScreenShare}
            className="absolute top-1 right-1 bg-black/60 text-white rounded-full p-1 hover:bg-black/80"
            title="Stop screen share"
          >
            <MonitorOff className="h-3.5 w-3.5" />
          </button>
        </div>
      )}

      {/* Topsi Response Overlay */}
      {topsiOverlay && (
        <div className="mx-3 mt-2 p-3 bg-cyan-50 dark:bg-cyan-950 border border-cyan-200 dark:border-cyan-800 rounded-lg">
          <div className="flex items-center gap-2 mb-1">
            <Network className="h-4 w-4 text-cyan-600" />
            <span className="text-xs font-semibold text-cyan-600">Topsi</span>
          </div>
          <p className="text-sm">{topsiOverlay}</p>
        </div>
      )}

      {/* Transcript */}
      <ScrollArea className="flex-1 p-3">
        <div className="space-y-2">
          {transcript.length === 0 && (
            <div className="text-center text-sm text-muted-foreground py-8">
              {meetingState === 'active' ? 'Listening… speak to see the transcript.' : 'Recording paused.'}
            </div>
          )}
          {transcript.map((entry, i) => (
            <div
              key={i}
              className={cn(
                'text-sm rounded-lg p-2',
                entry.isTopsiAddressed
                  ? 'bg-cyan-50 dark:bg-cyan-950 border border-cyan-200 dark:border-cyan-800'
                  : entry.text.startsWith('[SHARED LINK]')
                    ? 'bg-blue-50 dark:bg-blue-950 border border-blue-200 dark:border-blue-800'
                    : 'bg-muted'
              )}
            >
              {entry.speakerLabel && (
                <span className="text-xs font-semibold text-muted-foreground">{entry.speakerLabel}: </span>
              )}
              {entry.text.startsWith('[SHARED LINK] ') ? (
                <>
                  <LinkIcon className="h-3 w-3 inline mr-1 text-blue-500" />
                  <a
                    href={entry.text.replace('[SHARED LINK] ', '')}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-blue-600 underline break-all"
                  >
                    {entry.text.replace('[SHARED LINK] ', '')}
                  </a>
                </>
              ) : (
                <span>{entry.text}</span>
              )}
              {entry.topsiResponse && (
                <div className="mt-1 pt-1 border-t border-cyan-200 dark:border-cyan-800">
                  <span className="text-xs font-semibold text-cyan-600">Topsi: </span>
                  <span className="text-xs">{entry.topsiResponse}</span>
                </div>
              )}
            </div>
          ))}
          <div ref={transcriptEndRef as React.RefObject<HTMLDivElement>} />
        </div>
      </ScrollArea>

      {/* Collaborative chat input (send text or link to meeting) */}
      <div className="px-3 pb-1 pt-1 border-t border-muted">
        <div className="flex gap-1.5 items-center">
          <Input
            placeholder="Share a message or paste a link…"
            value={chatInput}
            onChange={(e) => onChatInputChange(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && !e.shiftKey && onSendChatMessage()}
            className="h-8 text-sm"
            disabled={isSendingChat}
          />
          <Button
            variant={isUrl(chatInput) ? 'default' : 'outline'}
            size="icon"
            className={cn('h-8 w-8 shrink-0', isUrl(chatInput) && 'bg-blue-600 hover:bg-blue-700')}
            onClick={onSendChatMessage}
            disabled={!chatInput.trim() || isSendingChat}
            title={isUrl(chatInput) ? 'Share link' : 'Send message'}
          >
            {isSendingChat ? (
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
            ) : isUrl(chatInput) ? (
              <LinkIcon className="h-3.5 w-3.5" />
            ) : (
              <Send className="h-3.5 w-3.5" />
            )}
          </Button>
        </div>
      </div>

      {/* Controls */}
      <div className="p-3 border-t flex items-center justify-between">
        {/* Screen share toggle */}
        <Button
          variant={isScreenSharing ? 'default' : 'outline'}
          size="sm"
          onClick={isScreenSharing ? onStopScreenShare : onStartScreenShare}
          className={cn(isScreenSharing && 'bg-blue-600 hover:bg-blue-700')}
          title={isScreenSharing ? 'Stop screen share' : 'Share screen'}
        >
          {isScreenSharing ? (
            <MonitorOff className="h-4 w-4 mr-1" />
          ) : (
            <Monitor className="h-4 w-4 mr-1" />
          )}
          {isScreenSharing ? 'Stop sharing' : 'Share screen'}
        </Button>

        <div className="flex items-center gap-2">
          {meetingState === 'active' ? (
            <IconButton
              variant="outline" onClick={onPauseMeeting}
              icon={Pause}
              label="Pause"
            />
          ) : (
            <IconButton
              variant="outline" onClick={onResumeMeeting}
              icon={Play}
              label="Resume"
            />
          )}
          <IconButton
            variant="destructive" onClick={onEndMeeting}
            className="h-10 w-10"
            icon={Square}
            label={meetingRole === 'observer' ? 'Leave meeting' : 'End meeting'}
          />
        </div>
      </div>
    </>
  );
}
