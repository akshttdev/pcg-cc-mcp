import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Network } from 'lucide-react';
import type { TopologyOverview } from '@/lib/api';

interface TopsiTopologyTabProps {
  topology: TopologyOverview | null;
}

export function TopsiTopologyTab({ topology }: TopsiTopologyTabProps) {
  return (
    <>
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 animate-stagger">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">
              Total Nodes
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-3xl font-semibold">{topology?.totalNodes || 0}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">
              Total Edges
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-3xl font-semibold">{topology?.totalEdges || 0}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">
              Active Clusters
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-3xl font-semibold">{topology?.totalClusters || 0}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">
              System Health
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-3xl font-semibold text-green-600">
              {topology?.systemHealth ? `${(topology.systemHealth * 100).toFixed(0)}%` : 'N/A'}
            </div>
          </CardContent>
        </Card>
      </div>

      <Card className="mt-6">
        <CardHeader>
          <CardTitle>Topology Visualization</CardTitle>
          <CardDescription>
            Graph view of your project topology (coming soon)
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="h-64 flex items-center justify-center bg-muted rounded-lg">
            <div className="text-center text-muted-foreground">
              <Network className="w-12 h-12 mx-auto mb-2 opacity-50" />
              <p>Topology visualization will be rendered here</p>
            </div>
          </div>
        </CardContent>
      </Card>
    </>
  );
}
