import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { RefreshCw, AlertTriangle, CheckCircle } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { DetectedIssue } from '@/lib/api';

interface TopsiIssuesTabProps {
  issues: DetectedIssue[];
  fetchIssues: () => void;
}

export function TopsiIssuesTab({ issues, fetchIssues }: TopsiIssuesTabProps) {
  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle>Detected Issues</CardTitle>
            <CardDescription>
              Topsi automatically detects topology issues and suggests fixes
            </CardDescription>
          </div>
          <Button variant="outline" onClick={fetchIssues}>
            <RefreshCw className="w-4 h-4 mr-2" />
            Refresh
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        {issues.length === 0 ? (
          <div className="text-center py-12">
            <CheckCircle className="w-12 h-12 mx-auto mb-3 text-green-500" />
            <h3 className="text-lg font-medium">No Issues Detected</h3>
            <p className="text-muted-foreground">
              Your topology is healthy
            </p>
          </div>
        ) : (
          <div className="space-y-4">
            {issues.map((issue, index) => (
              <div
                key={index}
                className={cn(
                  "p-4 border rounded-lg",
                  issue.severity === 'critical' && "border-red-500 bg-red-50 dark:bg-red-950/20",
                  issue.severity === 'warning' && "border-yellow-500 bg-yellow-50 dark:bg-yellow-950/20"
                )}
              >
                <div className="flex items-start justify-between mb-2">
                  <div className="flex items-center gap-2">
                    <AlertTriangle className={cn(
                      "w-4 h-4",
                      issue.severity === 'critical' && "text-red-500",
                      issue.severity === 'warning' && "text-yellow-500"
                    )} />
                    <span className="font-medium capitalize">{issue.issueType}</span>
                  </div>
                  <Badge variant={issue.severity === 'critical' ? 'destructive' : 'outline'}>
                    {issue.severity}
                  </Badge>
                </div>
                <p className="text-sm text-muted-foreground mb-2">
                  {issue.description}
                </p>
                {issue.suggestedAction && (
                  <div className="text-sm bg-background p-2 rounded border">
                    <span className="font-medium">Suggested: </span>
                    {issue.suggestedAction}
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
