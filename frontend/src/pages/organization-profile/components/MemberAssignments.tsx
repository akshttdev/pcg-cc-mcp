import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  FolderOpen,
  Briefcase,
  Eye,
  X,
  Plus,
} from 'lucide-react';
import {
  organizationsApi,
  resolveApiUrl,
  type ClientData,
} from '@/lib/api';

export function MemberAssignments({ orgId, userId }: { orgId: string; userId: string }) {
  const queryClient = useQueryClient();
  const { data: assignments, isLoading } = useQuery({
    queryKey: ['member-assignments', orgId, userId],
    queryFn: () => organizationsApi.getMemberAssignments(orgId, userId),
  });

  const { data: orgClients = [] } = useQuery<ClientData[]>({
    queryKey: ['orgClients', orgId],
    queryFn: () => organizationsApi.getClients(orgId),
  });

  const [assignType, setAssignType] = useState<string>('');
  const [assignTargetId, setAssignTargetId] = useState('');
  const [assignRole, setAssignRole] = useState('editor');

  const { data: orgProjects = [] } = useQuery<any[]>({
    queryKey: ['org-projects-list', orgId],
    queryFn: async () => {
      const res = await fetch(resolveApiUrl(`/api/projects?organization_id=${orgId}`), { credentials: 'include' });
      if (!res.ok) return [];
      const data = await res.json();
      return data.data || [];
    },
  });

  const assignMutation = useMutation({
    mutationFn: () => organizationsApi.assignMember(orgId, userId, assignType, assignTargetId, assignRole),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['member-assignments', orgId, userId] });
      queryClient.invalidateQueries({ queryKey: ['sidebarTree'] });
      setAssignType('');
      setAssignTargetId('');
    },
  });

  const unassignProjectMutation = useMutation({
    mutationFn: (projectId: string) => organizationsApi.unassignProject(orgId, userId, projectId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['member-assignments', orgId, userId] });
      queryClient.invalidateQueries({ queryKey: ['sidebarTree'] });
    },
  });

  const unassignClientMutation = useMutation({
    mutationFn: (clientId: string) => organizationsApi.unassignClient(orgId, userId, clientId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['member-assignments', orgId, userId] });
      queryClient.invalidateQueries({ queryKey: ['sidebarTree'] });
    },
  });

  if (isLoading) return <div className="text-xs text-muted-foreground py-2">Loading assignments...</div>;

  const projectAssignments = assignments?.projects || [];
  const clientAssignments = assignments?.clients || [];
  const taskAssignments = assignments?.tasks || [];
  const watchedTasks = assignments?.watched_tasks || [];

  return (
    <div className="pl-11 pb-3 space-y-3">
      {projectAssignments.length > 0 && (
        <div>
          <p className="text-xs font-medium text-muted-foreground mb-1">Projects</p>
          <div className="flex flex-wrap gap-1">
            {projectAssignments.map((p: any) => (
              <Badge key={p.project_id} variant="secondary" className="text-xs gap-1">
                <FolderOpen className="h-3 w-3" />
                {p.project_name} ({p.role})
                <button onClick={() => unassignProjectMutation.mutate(p.project_id)} className="ml-1 hover:text-destructive">
                  <X className="h-3 w-3" />
                </button>
              </Badge>
            ))}
          </div>
        </div>
      )}
      {clientAssignments.length > 0 && (
        <div>
          <p className="text-xs font-medium text-muted-foreground mb-1">Clients</p>
          <div className="flex flex-wrap gap-1">
            {clientAssignments.map((c: any) => (
              <Badge key={c.client_id} variant="secondary" className="text-xs gap-1">
                <Briefcase className="h-3 w-3" />
                {c.client_name} ({c.role})
                <button onClick={() => unassignClientMutation.mutate(c.client_id)} className="ml-1 hover:text-destructive">
                  <X className="h-3 w-3" />
                </button>
              </Badge>
            ))}
          </div>
        </div>
      )}
      {taskAssignments.length > 0 && (
        <div>
          <p className="text-xs font-medium text-muted-foreground mb-1">Tasks (assignee)</p>
          <div className="flex flex-wrap gap-1">
            {taskAssignments.map((t: any) => (
              <Badge key={t.task_id} variant="outline" className="text-xs">
                {t.title} <span className="text-muted-foreground ml-1">({t.project_name})</span>
              </Badge>
            ))}
          </div>
        </div>
      )}
      {watchedTasks.length > 0 && (
        <div>
          <p className="text-xs font-medium text-muted-foreground mb-1">Tasks (watching)</p>
          <div className="flex flex-wrap gap-1">
            {watchedTasks.map((t: any) => (
              <Badge key={t.task_id} variant="outline" className="text-xs">
                <Eye className="h-3 w-3 mr-1" />
                {t.title} <span className="text-muted-foreground ml-1">({t.project_name})</span>
              </Badge>
            ))}
          </div>
        </div>
      )}
      {projectAssignments.length === 0 && clientAssignments.length === 0 && taskAssignments.length === 0 && (
        <p className="text-xs text-muted-foreground">No assignments yet</p>
      )}
      <div className="flex items-center gap-2 pt-1">
        <Select value={assignType} onValueChange={(v) => { setAssignType(v); setAssignTargetId(''); }}>
          <SelectTrigger className="w-[120px] h-7 text-xs">
            <SelectValue placeholder="Assign to..." />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="project">Project</SelectItem>
            <SelectItem value="client">Client</SelectItem>
          </SelectContent>
        </Select>
        {assignType === 'project' && (
          <>
            <Select value={assignTargetId} onValueChange={setAssignTargetId}>
              <SelectTrigger className="w-[180px] h-7 text-xs">
                <SelectValue placeholder="Select project..." />
              </SelectTrigger>
              <SelectContent>
                {orgProjects
                  .filter((p: any) => !projectAssignments.some((a: any) => a.project_id === p.id))
                  .map((p: any) => (
                    <SelectItem key={p.id} value={p.id}>{p.name}</SelectItem>
                  ))}
              </SelectContent>
            </Select>
            <Select value={assignRole} onValueChange={setAssignRole}>
              <SelectTrigger className="w-[90px] h-7 text-xs">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="viewer">Viewer</SelectItem>
                <SelectItem value="editor">Editor</SelectItem>
                <SelectItem value="admin">Admin</SelectItem>
              </SelectContent>
            </Select>
          </>
        )}
        {assignType === 'client' && (
          <Select value={assignTargetId} onValueChange={setAssignTargetId}>
            <SelectTrigger className="w-[180px] h-7 text-xs">
              <SelectValue placeholder="Select client..." />
            </SelectTrigger>
            <SelectContent>
              {orgClients
                .filter((c: any) => !clientAssignments.some((a: any) => a.client_id === c.id))
                .map((c: any) => (
                  <SelectItem key={c.id} value={c.id}>{c.name}</SelectItem>
                ))}
            </SelectContent>
          </Select>
        )}
        {assignType && assignTargetId && (
          <Button size="sm" variant="outline" className="h-7 text-xs" onClick={() => assignMutation.mutate()} disabled={assignMutation.isPending}>
            <Plus className="h-3 w-3 mr-1" />
            Assign
          </Button>
        )}
      </div>
    </div>
  );
}
