import { useQuery } from '@tanstack/react-query';
import { formatDistanceToNow } from 'date-fns';
import { useState } from 'react';

import {
  type MeetingSegment,
  type MeetingSession,
  meetingSessionsApi,
} from '@/lib/api';

function formatDuration(secs?: number) {
  if (!secs) return '—';
  const m = Math.floor(secs / 60);
  const s = secs % 60;
  return `${m}m ${s}s`;
}

function TranscriptPanel({ sessionId }: { sessionId: string }) {
  const { data: segments = [], isLoading } = useQuery<MeetingSegment[]>({
    queryKey: ['meeting-segments', sessionId],
    queryFn: () => meetingSessionsApi.listSegments(sessionId),
    refetchInterval: 5000,
  });

  if (isLoading)
    return <div className="text-gray-400 text-sm p-4">Loading transcript…</div>;
  if (!segments.length)
    return <div className="text-gray-400 text-sm p-4">No transcript yet.</div>;

  return (
    <div className="divide-y divide-gray-800 max-h-[60vh] overflow-y-auto">
      {segments.map((seg) => (
        <div key={seg.id} className="p-3 flex gap-3">
          <span
            className={`text-xs font-semibold mt-0.5 w-20 shrink-0 ${
              seg.speaker_label === 'Nora' ? 'text-violet-400' : 'text-gray-400'
            }`}
          >
            {seg.speaker_label ?? 'Speaker'}
          </span>
          <p className="text-sm text-gray-200 leading-relaxed">{seg.text}</p>
        </div>
      ))}
    </div>
  );
}

export function MeetingsPage() {
  const [selected, setSelected] = useState<string | null>(null);
  const [filter, setFilter] = useState<'all' | 'active' | 'ended'>('all');

  const { data: sessions = [], isLoading } = useQuery<MeetingSession[]>({
    queryKey: ['meeting-sessions', filter],
    queryFn: () =>
      meetingSessionsApi.listSessions(filter !== 'all' ? filter : undefined),
    refetchInterval: 10000,
  });

  const selectedSession = sessions.find((s) => s.id === selected);

  return (
    <div className="flex h-full min-h-screen bg-gray-950 text-gray-100">
      {/* Left: session list */}
      <div className="w-80 shrink-0 border-r border-gray-800 flex flex-col">
        <div className="p-4 border-b border-gray-800">
          <h1 className="text-lg font-semibold text-white">Meetings</h1>
          <p className="text-xs text-gray-400 mt-0.5">
            Voice sessions with transcripts
          </p>
          <div className="flex gap-2 mt-3">
            {(['all', 'active', 'ended'] as const).map((f) => (
              <button
                key={f}
                onClick={() => setFilter(f)}
                className={`px-3 py-1 rounded text-xs font-medium transition-colors ${
                  filter === f
                    ? 'bg-violet-600 text-white'
                    : 'bg-gray-800 text-gray-400 hover:bg-gray-700'
                }`}
              >
                {f.charAt(0).toUpperCase() + f.slice(1)}
              </button>
            ))}
          </div>
        </div>

        <div className="flex-1 overflow-y-auto">
          {isLoading && (
            <div className="p-4 text-sm text-gray-400">Loading…</div>
          )}
          {!isLoading && sessions.length === 0 && (
            <div className="p-4 text-sm text-gray-500">No sessions found.</div>
          )}
          {sessions.map((s) => (
            <button
              key={s.id}
              onClick={() => setSelected(s.id)}
              className={`w-full text-left p-4 border-b border-gray-800 hover:bg-gray-800 transition-colors ${
                selected === s.id
                  ? 'bg-gray-800 border-l-2 border-l-violet-500'
                  : ''
              }`}
            >
              <div className="flex items-center justify-between mb-1">
                <span className="text-sm font-medium text-white truncate flex-1 mr-2">
                  {s.title || 'Untitled Session'}
                </span>
                <span
                  className={`text-xs px-1.5 py-0.5 rounded shrink-0 ${
                    s.status === 'active'
                      ? 'bg-green-900 text-green-300'
                      : 'bg-gray-800 text-gray-400'
                  }`}
                >
                  {s.status}
                </span>
              </div>
              <div className="text-xs text-gray-400">
                {formatDistanceToNow(new Date(s.started_at), {
                  addSuffix: true,
                })}
                {s.duration_seconds
                  ? ` · ${formatDuration(s.duration_seconds)}`
                  : ''}
                {s.segment_count ? ` · ${s.segment_count} segments` : ''}
              </div>
            </button>
          ))}
        </div>
      </div>

      {/* Right: transcript */}
      <div className="flex-1 flex flex-col">
        {selectedSession ? (
          <>
            <div className="p-5 border-b border-gray-800">
              <h2 className="text-base font-semibold text-white">
                {selectedSession.title || 'Untitled Session'}
              </h2>
              <div className="flex gap-4 mt-1 text-xs text-gray-400">
                <span>
                  Started{' '}
                  {new Date(selectedSession.started_at).toLocaleString()}
                </span>
                {selectedSession.ended_at && (
                  <span>
                    Ended {new Date(selectedSession.ended_at).toLocaleString()}
                  </span>
                )}
                <span>{formatDuration(selectedSession.duration_seconds)}</span>
              </div>
            </div>
            <div className="flex-1 overflow-hidden">
              <TranscriptPanel sessionId={selectedSession.id} />
            </div>
          </>
        ) : (
          <div className="flex-1 flex items-center justify-center text-gray-500 text-sm">
            Select a session to view its transcript
          </div>
        )}
      </div>
    </div>
  );
}
