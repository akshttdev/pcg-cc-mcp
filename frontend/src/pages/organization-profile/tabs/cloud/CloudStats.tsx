import { HardDrive, FileText, GitPullRequest } from 'lucide-react';
import type { CloudStats as CloudStatsType } from '@/lib/api/org-cloud';

interface CloudStatsProps {
  stats: CloudStatsType;
}

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(i > 0 ? 1 : 0)} ${units[i]}`;
}

export function CloudStats({ stats }: CloudStatsProps) {
  const usagePct = stats.quota_bytes > 0
    ? Math.round((stats.total_size_bytes / stats.quota_bytes) * 100)
    : 0;

  return (
    <div className="flex items-center gap-4 flex-wrap text-xs">
      <div className="flex items-center gap-1.5 text-muted-foreground">
        <HardDrive className="h-3.5 w-3.5" />
        <span>
          {formatSize(stats.total_size_bytes)} / {formatSize(stats.quota_bytes)} ({usagePct}%)
        </span>
      </div>
      <div className="flex items-center gap-1.5 text-muted-foreground">
        <FileText className="h-3.5 w-3.5" />
        <span>{stats.total_files} files</span>
      </div>
      <div className="flex items-center gap-1.5 text-muted-foreground">
        <GitPullRequest className="h-3.5 w-3.5" />
        <span>{stats.total_contributions} contributions</span>
      </div>

      {/* Usage bar */}
      <div className="flex items-center gap-2 ml-auto">
        <div className="w-32 h-1.5 bg-muted rounded-full overflow-hidden">
          <div
            className={`h-full rounded-full transition-all ${
              usagePct > 90 ? 'bg-red-500' : usagePct > 70 ? 'bg-amber-500' : 'bg-green-500'
            }`}
            style={{ width: `${Math.min(usagePct, 100)}%` }}
          />
        </div>
      </div>
    </div>
  );
}
