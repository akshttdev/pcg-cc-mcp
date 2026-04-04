import { CheckCircle2, Copy, HardDrive, Monitor, Terminal } from 'lucide-react';
import { useEffect, useState } from 'react';

import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';

const DAV_URL = `${window.location.origin}/dav/`;
const VOLUME_NAME = 'PCG APN';
const MOUNT_PATH = `/Volumes/${VOLUME_NAME}`;

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <button
      onClick={() => {
        navigator.clipboard.writeText(text);
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
      }}
      className="ml-2 p-1 rounded hover:bg-white/10 transition-colors shrink-0"
      title="Copy"
    >
      {copied ? (
        <CheckCircle2 className="h-3.5 w-3.5 text-green-400" />
      ) : (
        <Copy className="h-3.5 w-3.5 text-muted-foreground" />
      )}
    </button>
  );
}

function CodeBlock({
  children,
  className = '',
}: {
  children: string;
  className?: string;
}) {
  return (
    <div
      className={`relative flex items-start gap-2 bg-zinc-950 text-zinc-100 font-mono text-xs rounded-lg px-3 py-2.5 ${className}`}
    >
      <span className="flex-1 whitespace-pre-wrap break-all">{children}</span>
      <CopyButton text={children} />
    </div>
  );
}

export default function ApnDriveSettings() {
  const [mounted, setMounted] = useState<boolean | null>(null);

  // Ping the DAV root to check if it's reachable
  useEffect(() => {
    fetch('/dav/', { method: 'OPTIONS', credentials: 'include' })
      .then((r) => setMounted(r.ok || r.status === 401))
      .catch(() => setMounted(false));
  }, []);

  const apiKey =
    localStorage.getItem('pcg_admin_key') ?? '<YOUR_ADMIN_API_KEY>';

  const mountCmd = `osascript -e 'mount volume "${DAV_URL}" as user name "pcg" with password "${apiKey}"'`;

  const launchAgentPlist = `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>          <string>com.pcg.apn-drive</string>
  <key>ProgramArguments</key>
  <array>
    <string>/usr/bin/osascript</string>
    <string>-e</string>
    <string>mount volume "${DAV_URL}" as user name "pcg" with password "${apiKey}"</string>
  </array>
  <key>RunAtLoad</key>      <true/>
  <key>StartInterval</key>  <integer>300</integer>
</dict>
</plist>`;

  const installLaunchAgent = `cat > ~/Library/LaunchAgents/com.pcg.apn-drive.plist << 'EOF'\n${launchAgentPlist}\nEOF\nlaunchctl load ~/Library/LaunchAgents/com.pcg.apn-drive.plist`;

  return (
    <div className="max-w-2xl space-y-6">
      <div>
        <h2 className="text-xl font-semibold flex items-center gap-2">
          <HardDrive className="h-5 w-5 text-blue-500" />
          APN Drive
        </h2>
        <p className="text-muted-foreground mt-1 text-sm">
          Mount PCG media files directly in Finder. Premiere Pro, DaVinci
          Resolve, and Final Cut will find source footage automatically — no
          relinking required.
        </p>
      </div>

      {/* Status */}
      <Card>
        <CardContent className="pt-4 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div
              className={`h-2.5 w-2.5 rounded-full ${mounted === null ? 'bg-zinc-400' : mounted ? 'bg-green-500' : 'bg-red-500'}`}
            />
            <span className="text-sm font-medium">
              {mounted === null
                ? 'Checking…'
                : mounted
                  ? 'APN Drive server is reachable'
                  : 'Server unreachable'}
            </span>
          </div>
          <Badge variant="outline" className="text-xs font-mono">
            {DAV_URL}
          </Badge>
        </CardContent>
      </Card>

      {/* Step 1 — One-click mount */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-base flex items-center gap-2">
            <Monitor className="h-4 w-4 text-blue-500" />
            Step 1 — Connect in Finder
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-3 text-sm">
          <ol className="space-y-2 text-muted-foreground">
            <li>
              <span className="font-medium text-foreground">a.</span> In Finder,
              press{' '}
              <kbd className="bg-muted px-1.5 py-0.5 rounded text-xs">⌘K</kbd>{' '}
              (Go → Connect to Server)
            </li>
            <li>
              <span className="font-medium text-foreground">b.</span> Enter this
              URL:
            </li>
          </ol>
          <CodeBlock>{DAV_URL}</CodeBlock>
          <ol className="space-y-1 text-muted-foreground" start={3}>
            <li>
              <span className="font-medium text-foreground">c.</span> Username:{' '}
              <code className="bg-muted px-1 py-0.5 rounded text-xs">pcg</code>
            </li>
            <li>
              <span className="font-medium text-foreground">d.</span> Password:
              your Admin API Key (copy from Developer settings)
            </li>
            <li>
              <span className="font-medium text-foreground">e.</span> Check
              "Remember this password in my keychain"
            </li>
          </ol>
          <p className="text-muted-foreground text-xs pt-1">
            The drive mounts at{' '}
            <code className="bg-muted px-1 py-0.5 rounded">{MOUNT_PATH}</code>{' '}
            and appears in Finder's sidebar.
          </p>
        </CardContent>
      </Card>

      {/* Step 2 — Terminal one-liner */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-base flex items-center gap-2">
            <Terminal className="h-4 w-4 text-green-500" />
            Step 1 (alternative) — Mount via Terminal
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-3 text-sm">
          <p className="text-muted-foreground">
            Paste this in Terminal to mount immediately:
          </p>
          <CodeBlock>{mountCmd}</CodeBlock>
        </CardContent>
      </Card>

      {/* Step 3 — Auto-mount on login */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-base flex items-center gap-2">
            <HardDrive className="h-4 w-4 text-purple-500" />
            Step 2 — Auto-mount on login (recommended)
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-3 text-sm">
          <p className="text-muted-foreground">
            Run this once to install a LaunchAgent that mounts APN Drive every
            time you log in:
          </p>
          <CodeBlock>{installLaunchAgent}</CodeBlock>
          <p className="text-muted-foreground text-xs">
            To uninstall:{' '}
            <code className="bg-muted px-1 rounded">
              launchctl unload ~/Library/LaunchAgents/com.pcg.apn-drive.plist &&
              rm ~/Library/LaunchAgents/com.pcg.apn-drive.plist
            </code>
          </p>
        </CardContent>
      </Card>

      {/* Step 4 — Using in Premiere */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-base">
            Opening sequences in Premiere Pro
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-3 text-sm text-muted-foreground">
          <p>
            Once APN Drive is mounted, click{' '}
            <strong className="text-foreground">
              Download Premiere Package
            </strong>{' '}
            on any video artifact and select{' '}
            <strong className="text-foreground">APN Drive XML</strong>. The
            downloaded XML will reference media at{' '}
            <code className="bg-muted px-1 rounded">{MOUNT_PATH}/…</code> —
            Premiere opens it with all footage linked automatically.
          </p>
          <p>
            For existing XML files with offline media: in Premiere, right-click
            an offline clip →
            <strong className="text-foreground"> Link Media</strong> → navigate
            to{' '}
            <code className="bg-muted px-1 rounded">
              {MOUNT_PATH}/media_pipeline2/source/
            </code>
            .
          </p>
        </CardContent>
      </Card>
    </div>
  );
}
