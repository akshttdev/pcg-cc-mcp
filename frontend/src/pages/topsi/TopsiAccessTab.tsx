import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Eye, Lock, FolderOpen } from 'lucide-react';
import type { TopsiStatusResponse, ProjectAccess } from '@/lib/api';

interface TopsiAccessTabProps {
  status: TopsiStatusResponse | null;
  projects: ProjectAccess[];
}

export function TopsiAccessTab({ status, projects }: TopsiAccessTabProps) {
  return (
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Eye className="w-5 h-5" />
            Your Access Level
          </CardTitle>
          <CardDescription>
            Topsi enforces strict data isolation between clients
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="p-4 bg-muted rounded-lg">
            <div className="flex items-center gap-2 mb-2">
              <Badge variant={status?.accessScope === 'admin' ? 'default' : 'secondary'}>
                {status?.accessScope === 'admin' ? 'Admin' : 'User'}
              </Badge>
              <span className="text-sm text-muted-foreground">Access Scope</span>
            </div>
            <p className="text-sm">
              {status?.accessScope === 'admin'
                ? 'You have full platform visibility and can see all projects and data.'
                : `You can access ${status?.projectsVisible || 0} project(s) based on your permissions.`}
            </p>
          </div>

          <div className="flex items-start gap-3 p-3 border rounded-lg">
            <Lock className="w-5 h-5 text-green-600 mt-0.5" />
            <div>
              <div className="font-medium">Client Data Isolation</div>
              <p className="text-sm text-muted-foreground">
                Topsi ensures that client data is never shared between users
                without explicit permission.
              </p>
            </div>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <FolderOpen className="w-5 h-5" />
            Accessible Projects
          </CardTitle>
        </CardHeader>
        <CardContent>
          {projects.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <FolderOpen className="w-8 h-8 mx-auto mb-2 opacity-50" />
              <p>No projects accessible</p>
            </div>
          ) : (
            <ScrollArea className="h-64">
              <div className="space-y-2">
                {projects.map((project) => (
                  <div
                    key={project.projectId}
                    className="flex items-center justify-between p-3 border rounded-lg"
                  >
                    <div>
                      <div className="font-medium">{project.projectName}</div>
                      <div className="text-xs text-muted-foreground">
                        Granted: {new Date(project.grantedAt).toLocaleDateString()}
                      </div>
                    </div>
                    <Badge variant="outline" className="capitalize">
                      {project.role}
                    </Badge>
                  </div>
                ))}
              </div>
            </ScrollArea>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
