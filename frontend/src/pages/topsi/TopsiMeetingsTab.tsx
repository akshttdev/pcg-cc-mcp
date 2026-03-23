import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { MeetingMode } from '@/components/topsi/meeting-mode';
import { MeetingHistory } from '@/components/topsi/MeetingHistory';

export function TopsiMeetingsTab() {
  return (
    <div className="grid grid-cols-1 lg:grid-cols-3 gap-6 min-h-[600px]">
      {/* Left: New Meeting + History */}
      <div className="lg:col-span-2 space-y-6">
        <Card>
          <CardContent className="p-0">
            <MeetingMode className="min-h-[400px]" />
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle className="text-lg">Past Meetings</CardTitle>
            <CardDescription>
              Browse transcripts, notes, and share meeting artifacts
            </CardDescription>
          </CardHeader>
          <CardContent>
            <MeetingHistory className="min-h-[200px]" />
          </CardContent>
        </Card>
      </div>
      {/* Right: Info sidebar */}
      <div className="space-y-4">
        <Card>
          <CardHeader>
            <CardTitle className="text-sm">Meeting Mode</CardTitle>
            <CardDescription className="text-xs">
              Topsi acts as a silent AI observer during meetings,
              documenting conversations and only engaging when addressed.
            </CardDescription>
          </CardHeader>
          <CardContent className="text-xs text-muted-foreground space-y-2">
            <p>Say "Topsi" during a meeting to ask questions or get summaries.</p>
            <p>Meeting notes are auto-generated when the session ends.</p>
            <p>Transcripts are stored with admin-only access by default.</p>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
