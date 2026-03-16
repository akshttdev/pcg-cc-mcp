import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Calendar, Clock } from 'lucide-react';
import type { Project } from 'shared/types';

interface ProjectStatsPanelProps {
  project: Project;
  repoLabel: string;
}

export function ProjectStatsPanel({ project, repoLabel }: ProjectStatsPanelProps) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Operational Snapshot</CardTitle>
        <CardDescription>
          Metadata, repo link, and automation hooks for this workspace.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4 text-sm">
        <div>
          <p className="text-xs uppercase text-muted-foreground">Project ID</p>
          <code className="mt-1 block rounded bg-muted px-2 py-1 font-mono text-xs">
            {project.id}
          </code>
        </div>
        <div className="grid gap-3">
          <div className="flex items-center text-sm">
            <Calendar className="mr-2 h-4 w-4 text-muted-foreground" />
            Created {new Date(project.created_at).toLocaleDateString()}
          </div>
          <div className="flex items-center text-sm">
            <Clock className="mr-2 h-4 w-4 text-muted-foreground" />
            Updated {new Date(project.updated_at).toLocaleDateString()}
          </div>
        </div>
        <div>
          <p className="text-xs uppercase text-muted-foreground">Repository</p>
          <p className="text-sm font-medium text-foreground">{repoLabel}</p>
          <p className="text-xs text-muted-foreground">
            {project.git_repo_path
              ? 'Synced automatically from GitHub.'
              : 'Link a repository to enable automated PRs.'}
          </p>
        </div>
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <span className="text-xs uppercase text-muted-foreground">
              Dev Script
            </span>
            <span className="font-medium text-foreground">
              {project.dev_script || 'pnpm run dev'}
            </span>
          </div>
          {project.setup_script && (
            <div className="flex items-center justify-between">
              <span className="text-xs uppercase text-muted-foreground">
                Setup
              </span>
              <span className="font-medium text-foreground">
                {project.setup_script}
              </span>
            </div>
          )}
          {project.cleanup_script && (
            <div className="flex items-center justify-between">
              <span className="text-xs uppercase text-muted-foreground">
                Cleanup
              </span>
              <span className="font-medium text-foreground">
                {project.cleanup_script}
              </span>
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
