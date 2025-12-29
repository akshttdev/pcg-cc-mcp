import { useEffect, useState, useCallback } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  fetchNoraPlans,
  fetchNoraPlan,
  updateNoraPlanNode,
} from '@/lib/api';
import type { GraphPlan, GraphPlanSummary, GraphNodeStatus } from 'shared/types';
import { toast } from 'sonner';
import { formatDistanceToNow } from 'date-fns';

const STATUS_COLORS: Record<GraphNodeStatus, string> = {
  pending: 'bg-gray-100 text-gray-800',
  running: 'bg-blue-100 text-blue-800',
  completed: 'bg-green-100 text-green-800',
  failed: 'bg-red-100 text-red-800',
};

const PLAN_STATUS_CLASSES: Record<string, string> = {
  pending: 'bg-gray-100 text-gray-800',
  inProgress: 'bg-blue-100 text-blue-800',
  completed: 'bg-green-100 text-green-800',
  failed: 'bg-red-100 text-red-800',
};

const NODE_STATUS_OPTIONS: GraphNodeStatus[] = [
  'pending',
  'running',
  'completed',
  'failed',
];

const formatPlanStatus = (status: string) =>
  status === 'inProgress'
    ? 'In Progress'
    : status.charAt(0).toUpperCase() + status.slice(1);

const formatNodeStatus = (status: GraphNodeStatus) =>
  status.charAt(0).toUpperCase() + status.slice(1);

export function NoraPlansPanel() {
  const [plans, setPlans] = useState<GraphPlanSummary[]>([]);
  const [selectedPlanId, setSelectedPlanId] = useState<string | null>(null);
  const [selectedPlan, setSelectedPlan] = useState<GraphPlan | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isRefreshingNode, setIsRefreshingNode] = useState<string | null>(null);

  const loadPlans = useCallback(async () => {
    setIsLoading(true);
    try {
      const data = await fetchNoraPlans();
      setPlans(data);
      if (!selectedPlanId && data.length > 0) {
        setSelectedPlanId(data[0].id);
      }
    } catch (error) {
      console.error(error);
      toast.error('Failed to load orchestration plans');
    } finally {
      setIsLoading(false);
    }
  }, [selectedPlanId]);

  const loadPlanDetail = useCallback(async (planId: string) => {
    setIsLoading(true);
    try {
      const detail = await fetchNoraPlan(planId);
      setSelectedPlan(detail);
    } catch (error) {
      console.error(error);
      toast.error('Unable to fetch plan detail');
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadPlans();
  }, [loadPlans]);

  useEffect(() => {
    if (selectedPlanId) {
      void loadPlanDetail(selectedPlanId);
    }
  }, [selectedPlanId, loadPlanDetail]);

  const handleNodeStatusChange = async (
    nodeId: string,
    status: GraphNodeStatus
  ) => {
    if (!selectedPlanId) {
      return;
    }
    setIsRefreshingNode(nodeId);
    try {
      const updated = await updateNoraPlanNode(selectedPlanId, nodeId, status);
      setSelectedPlan(updated);
      toast.success(`Node updated to ${status}`);
    } catch (error) {
      console.error(error);
      toast.error('Failed to update node status');
    } finally {
      setIsRefreshingNode(null);
    }
  };

  return (
    <div className="grid gap-6 md:grid-cols-2 h-full">
      <Card className="h-full overflow-hidden">
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle className="text-lg">Orchestration Plans</CardTitle>
          <Button variant="outline" size="sm" onClick={() => void loadPlans()} disabled={isLoading}>
            Refresh
          </Button>
        </CardHeader>
        <CardContent className="space-y-3 overflow-auto" style={{ maxHeight: 'calc(100vh - 320px)' }}>
          {plans.length === 0 && (
            <p className="text-sm text-muted-foreground">
              {isLoading ? 'Loading plans…' : 'No orchestration plans yet.'}
            </p>
          )}
          {plans.map((plan) => (
            <button
              key={plan.id}
              className={`w-full text-left border rounded-lg px-4 py-3 hover:border-purple-400 transition ${
                selectedPlanId === plan.id ? 'border-purple-500 bg-purple-50' : 'border-gray-200'
              }`}
              onClick={() => setSelectedPlanId(plan.id)}
            >
              <div className="flex items-center justify-between">
                <div>
                  <p className="font-medium text-sm line-clamp-1">{plan.title}</p>
                  <p className="text-xs text-muted-foreground">
                    {formatDistanceToNow(new Date(plan.createdAt), { addSuffix: true })}
                  </p>
                </div>
                <Badge variant="secondary" className={PLAN_STATUS_CLASSES[plan.status] ?? ''}>
                  {formatPlanStatus(plan.status)}
                </Badge>
              </div>
              <p className="text-xs text-muted-foreground mt-1">{plan.nodeCount} nodes</p>
            </button>
          ))}
        </CardContent>
      </Card>

      <Card className="h-full overflow-hidden">
        <CardHeader>
          <CardTitle className="text-lg">Plan Detail</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4 overflow-auto" style={{ maxHeight: 'calc(100vh - 320px)' }}>
          {!selectedPlan && (
            <p className="text-sm text-muted-foreground">
              {selectedPlanId ? 'Loading plan…' : 'Select a plan to inspect nodes.'}
            </p>
          )}

          {selectedPlan && (
            <div className="space-y-4">
              <div>
                <p className="font-medium text-sm">Executed from request</p>
                <p className="text-sm text-muted-foreground">{selectedPlan.title}</p>
              </div>

              <div className="space-y-3">
                {selectedPlan.nodes.map((node) => (
                  <div key={node.id} className="border rounded-lg p-3">
                    <div className="flex items-center justify-between">
                      <div>
                        <p className="font-medium text-sm">{node.label}</p>
                        {node.description && (
                          <p className="text-xs text-muted-foreground">{node.description}</p>
                        )}
                        {node.agent && (
                          <p className="text-xs text-muted-foreground">Agent: {node.agent}</p>
                        )}
                      </div>
                      <Badge variant="secondary" className={STATUS_COLORS[node.status]}>
                        {formatNodeStatus(node.status)}
                      </Badge>
                    </div>
                    <div className="mt-3 flex items-center gap-2">
                      <label className="text-xs text-muted-foreground" htmlFor={`status-${node.id}`}>
                        Update status
                      </label>
                      <select
                        id={`status-${node.id}`}
                        className="border rounded-md px-2 py-1 text-sm"
                        value={node.status}
                        onChange={(event) =>
                          void handleNodeStatusChange(
                            node.id,
                            event.target.value as GraphNodeStatus
                          )
                        }
                        disabled={isRefreshingNode === node.id}
                      >
                        {NODE_STATUS_OPTIONS.map((option) => (
                          <option key={option} value={option}>
                            {option}
                          </option>
                        ))}
                      </select>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
