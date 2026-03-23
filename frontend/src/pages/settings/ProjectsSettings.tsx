import { useQuery } from '@tanstack/react-query';
import {
  Building2,
  Calendar,
  Coins,
  FolderKanban,
  Loader2,
  MoreVertical,
  Search,
  Shield,
  Users,
} from 'lucide-react';
import { useState } from 'react';
import type { Project } from 'shared/types';
import { toast } from 'sonner';

import { ProjectMembersDialog } from '@/components/dialogs/project-members-dialog';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { EmptyState } from '@/components/ui/empty-state';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { useMutationWithToast } from '@/hooks/useMutationWithToast';
import { type ClientData, organizationsApi, permissionsApi,projectsApi } from '@/lib/api';
import { formatDate } from '@/lib/formatters';
import { organizationKeys,projectKeys, sidebarKeys } from '@/lib/query-keys';

// Project already includes organization_id and client_id
type ProjectWithOrg = Project;

export function ProjectsSettings() {
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedProject, setSelectedProject] = useState<ProjectWithOrg | null>(null);
  const [membersDialogOpen, setMembersDialogOpen] = useState(false);
  const [budgetDialogOpen, setBudgetDialogOpen] = useState(false);
  const [budgetProject, setBudgetProject] = useState<ProjectWithOrg | null>(null);
  const [budgetLimit, setBudgetLimit] = useState<string>('');
  const [unlimitedBudget, setUnlimitedBudget] = useState(true);
  const [clientDialogOpen, setClientDialogOpen] = useState(false);
  const [clientProject, setClientProject] = useState<ProjectWithOrg | null>(null);
  const [selectedClientId, setSelectedClientId] = useState<string>('none');

  // Fetch projects
  const { data: projects = [], isLoading } = useQuery({
    queryKey: projectKeys.list(searchQuery),
    queryFn: () => permissionsApi.listProjects({ search: searchQuery }),
  });

  // Fetch clients for the client assignment dialog
  const { data: availableClients = [] } = useQuery({
    queryKey: organizationKeys.clientsSettings(clientProject?.organization_id),
    queryFn: () =>
      clientProject?.organization_id
        ? organizationsApi.getClients(clientProject.organization_id)
        : Promise.resolve([] as ClientData[]),
    enabled: clientDialogOpen && !!clientProject?.organization_id,
  });

  // Budget mutation
  const budgetMutation = useMutationWithToast({
    mutationFn: ({ projectId, budgetLimit }: { projectId: string; budgetLimit: number | null }) =>
      permissionsApi.setProjectBudget(projectId, budgetLimit),
    successMessage: 'VIBE budget updated successfully',
    errorMessage: (error: Error) => `Failed to update budget: ${error.message}`,
    invalidateKeys: [projectKeys.all],
    onSuccess: () => setBudgetDialogOpen(false),
  });

  // Client assignment mutation
  const clientMutation = useMutationWithToast({
    mutationFn: ({ projectId, clientId }: { projectId: string; clientId: string | null }) =>
      projectsApi.setClient(projectId, clientId),
    successMessage: 'Client assignment updated',
    errorMessage: (error: Error) => `Failed to update client: ${error.message}`,
    invalidateKeys: [projectKeys.all, sidebarKeys.treeLegacy()],
    onSuccess: () => setClientDialogOpen(false),
  });

  const handleAssignClient = (project: ProjectWithOrg) => {
    setClientProject(project);
    setSelectedClientId(project.client_id ?? 'none');
    setClientDialogOpen(true);
  };

  const handleSaveClient = () => {
    if (!clientProject) return;
    const clientId = selectedClientId === 'none' ? null : selectedClientId;
    clientMutation.mutate({ projectId: clientProject.id, clientId });
  };

  const handleManageAccess = (project: ProjectWithOrg) => {
    setSelectedProject(project);
    setMembersDialogOpen(true);
  };

  const handleManageBudget = (project: ProjectWithOrg) => {
    setBudgetProject(project);
    if (project.vibe_budget_limit !== null) {
      setBudgetLimit(project.vibe_budget_limit.toString());
      setUnlimitedBudget(false);
    } else {
      setBudgetLimit('');
      setUnlimitedBudget(true);
    }
    setBudgetDialogOpen(true);
  };

  const handleSaveBudget = () => {
    if (!budgetProject) return;
    const limit = unlimitedBudget ? null : parseInt(budgetLimit, 10);
    if (!unlimitedBudget && (isNaN(limit!) || limit! < 0)) {
      toast.error('Please enter a valid budget amount');
      return;
    }
    budgetMutation.mutate({ projectId: budgetProject.id, budgetLimit: limit });
  };

  const formatVibe = (amount: number | null) => {
    if (amount === null) return 'Unlimited';
    return `${amount.toLocaleString()} VIBE`;
  };

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-3xl font-bold tracking-tight">Project Access Management</h2>
        <p className="text-muted-foreground mt-2">
          Manage user access and permissions for each project.
        </p>
      </div>

      {/* Stats Cards */}
      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Total Projects</CardTitle>
            <FolderKanban className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{projects.length}</div>
          </CardContent>
        </Card>
        
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">With Members</CardTitle>
            <Users className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">-</div>
            <p className="text-xs text-muted-foreground">
              Projects with access controls
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Admin Access</CardTitle>
            <Shield className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">All</div>
            <p className="text-xs text-muted-foreground">
              Admins have full access
            </p>
          </CardContent>
        </Card>
      </div>

      {/* Projects Table */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <CardTitle>Projects</CardTitle>
            <div className="flex items-center gap-2">
              <div className="relative">
                <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                <Input
                  placeholder="Search projects..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="pl-9 w-[300px]"
                />
              </div>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="text-center py-12 text-muted-foreground">Loading projects...</div>
          ) : projects.length === 0 ? (
            <EmptyState
              icon={FolderKanban}
              title="No projects found"
              description={searchQuery ? 'Try adjusting your search' : 'No projects available yet'}
            />
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Project Name</TableHead>
                  <TableHead>Repository Path</TableHead>
                  <TableHead>Created</TableHead>
                  <TableHead>VIBE Budget</TableHead>
                  <TableHead>Members</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {projects.map((project) => (
                  <TableRow key={project.id}>
                    <TableCell>
                      <div className="font-medium">{project.name}</div>
                    </TableCell>
                    <TableCell>
                      <code className="text-xs bg-muted px-2 py-1 rounded">
                        {project.git_repo_path}
                      </code>
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-2 text-sm text-muted-foreground">
                        <Calendar className="h-4 w-4" />
                        {formatDate(project.created_at)}
                      </div>
                    </TableCell>
                    <TableCell>
                      <div className="space-y-1">
                        <Badge
                          variant={project.vibe_budget_limit === null ? 'secondary' : 'outline'}
                          className="cursor-pointer hover:bg-muted"
                          onClick={() => handleManageBudget(project)}
                        >
                          <Coins className="h-3 w-3 mr-1" />
                          {formatVibe(project.vibe_budget_limit)}
                        </Badge>
                        {project.vibe_budget_limit !== null && (
                          <div className="text-xs text-muted-foreground">
                            Used: {project.vibe_spent_amount.toLocaleString()} VIBE
                          </div>
                        )}
                      </div>
                    </TableCell>
                    <TableCell>
                      <Badge variant="secondary">
                        <Users className="h-3 w-3 mr-1" />
                        -
                      </Badge>
                    </TableCell>
                    <TableCell className="text-right">
                      <div className="flex gap-2 justify-end">
                        <Button
                          size="sm"
                          variant="outline"
                          onClick={() => handleManageAccess(project)}
                        >
                          <Users className="h-4 w-4 mr-2" />
                          Manage Access
                        </Button>
                        <DropdownMenu>
                          <DropdownMenuTrigger asChild>
                            <Button variant="ghost" size="sm">
                              <MoreVertical className="h-4 w-4" />
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent align="end">
                            <DropdownMenuItem onClick={() => handleManageBudget(project)}>
                              <Coins className="h-4 w-4 mr-2" />
                              Manage Budget
                            </DropdownMenuItem>
                            <DropdownMenuItem onClick={() => handleManageAccess(project)}>
                              <Users className="h-4 w-4 mr-2" />
                              Manage Members
                            </DropdownMenuItem>
                            {project.organization_id && (
                              <DropdownMenuItem onClick={() => handleAssignClient(project)}>
                                <Building2 className="h-4 w-4 mr-2" />
                                Assign to Client
                              </DropdownMenuItem>
                            )}
                          </DropdownMenuContent>
                        </DropdownMenu>
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Project Members Dialog */}
      {selectedProject && (
        <ProjectMembersDialog
          open={membersDialogOpen}
          onOpenChange={setMembersDialogOpen}
          projectId={selectedProject.id}
          projectName={selectedProject.name}
        />
      )}

      {/* Assign to Client Dialog */}
      <Dialog open={clientDialogOpen} onOpenChange={setClientDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Assign to Client</DialogTitle>
            <DialogDescription>
              Move <strong>{clientProject?.name}</strong> under a client in its organization. Select "No client" to make it an internal project.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label>Client</Label>
              {!clientProject?.organization_id ? (
                <p className="text-sm text-muted-foreground">
                  This project is not linked to an organization and cannot be assigned to a client.
                </p>
              ) : (
                <select
                  className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
                  value={selectedClientId}
                  onChange={(e) => setSelectedClientId(e.target.value)}
                >
                  <option value="none">No client (internal project)</option>
                  {availableClients.map((client) => (
                    <option key={client.id} value={client.id}>
                      {client.name}
                    </option>
                  ))}
                </select>
              )}
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setClientDialogOpen(false)}>
              Cancel
            </Button>
            <Button
              onClick={handleSaveClient}
              disabled={clientMutation.isPending || !clientProject?.organization_id}
            >
              {clientMutation.isPending ? (
                <>
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                  Saving...
                </>
              ) : (
                'Save'
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* VIBE Budget Dialog */}
      <Dialog open={budgetDialogOpen} onOpenChange={setBudgetDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Manage VIBE Budget</DialogTitle>
            <DialogDescription>
              Set a VIBE budget limit for {budgetProject?.name}. This controls how much VIBE can be spent on AI operations for this project.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="flex items-center justify-between">
              <div className="space-y-0.5">
                <Label>Unlimited Budget</Label>
                <p className="text-xs text-muted-foreground">
                  Allow unlimited VIBE usage for this project
                </p>
              </div>
              <Switch
                checked={unlimitedBudget}
                onCheckedChange={(checked) => {
                  setUnlimitedBudget(checked);
                  if (checked) setBudgetLimit('');
                }}
              />
            </div>

            {!unlimitedBudget && (
              <div className="space-y-2">
                <Label>Budget Limit (VIBE)</Label>
                <Input
                  type="number"
                  placeholder="Enter budget limit"
                  value={budgetLimit}
                  onChange={(e) => setBudgetLimit(e.target.value)}
                  min="0"
                />
                <p className="text-xs text-muted-foreground">
                  1 VIBE = $0.01 USD. Set the maximum VIBE this project can spend on AI operations.
                </p>
              </div>
            )}

            {budgetProject && budgetProject.vibe_spent_amount > 0 && (
              <div className="rounded-lg bg-muted p-3">
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Current Usage</span>
                  <span className="font-medium">{budgetProject.vibe_spent_amount.toLocaleString()} VIBE</span>
                </div>
                {budgetProject.vibe_budget_limit !== null && (
                  <div className="flex justify-between text-sm mt-1">
                    <span className="text-muted-foreground">Remaining</span>
                    <span className="font-medium">
                      {Math.max(0, budgetProject.vibe_budget_limit - budgetProject.vibe_spent_amount).toLocaleString()} VIBE
                    </span>
                  </div>
                )}
              </div>
            )}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setBudgetDialogOpen(false)}>
              Cancel
            </Button>
            <Button onClick={handleSaveBudget} disabled={budgetMutation.isPending}>
              {budgetMutation.isPending ? (
                <>
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                  Saving...
                </>
              ) : (
                'Save Budget'
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
