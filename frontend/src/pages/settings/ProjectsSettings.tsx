import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  FolderKanban,
  Users,
  Search,
  MoreVertical,
  Calendar,
  Shield,
  Coins,
  Loader2
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { toast } from 'sonner';
import { ProjectMembersDialog } from '@/components/dialogs/project-members-dialog';
import type { Project } from 'shared/types';

interface VibeBudgetResponse {
  vibe_budget_limit: number | null;
  vibe_spent_amount: number;
  vibe_remaining: number | null;
}

// API functions
const api = {
  listProjects: async (filters?: {
    search?: string;
  }): Promise<Project[]> => {
    const params = new URLSearchParams();
    if (filters?.search) params.append('search', filters.search);

    const response = await fetch(`/api/projects?${params}`);
    if (!response.ok) throw new Error('Failed to fetch projects');
    const data = await response.json();
    return data.data;
  },

  getProjectMemberCount: async (projectId: string): Promise<number> => {
    try {
      const response = await fetch(`/api/permissions/projects/${projectId}/members`);
      if (!response.ok) return 0;
      const data = await response.json();
      return data.data?.length || 0;
    } catch {
      return 0;
    }
  },

  getProjectBudget: async (projectId: string): Promise<VibeBudgetResponse> => {
    const response = await fetch(`/api/projects/${projectId}/budget`);
    if (!response.ok) throw new Error('Failed to fetch budget');
    const data = await response.json();
    return data.data;
  },

  setProjectBudget: async (projectId: string, budgetLimit: number | null): Promise<VibeBudgetResponse> => {
    const response = await fetch(`/api/projects/${projectId}/budget`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ vibe_budget_limit: budgetLimit }),
    });
    if (!response.ok) throw new Error('Failed to set budget');
    const data = await response.json();
    return data.data;
  },
};

export function ProjectsSettings() {
  const queryClient = useQueryClient();
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedProject, setSelectedProject] = useState<Project | null>(null);
  const [membersDialogOpen, setMembersDialogOpen] = useState(false);
  const [budgetDialogOpen, setBudgetDialogOpen] = useState(false);
  const [budgetProject, setBudgetProject] = useState<Project | null>(null);
  const [budgetLimit, setBudgetLimit] = useState<string>('');
  const [unlimitedBudget, setUnlimitedBudget] = useState(true);

  // Fetch projects
  const { data: projects = [], isLoading } = useQuery({
    queryKey: ['projects', searchQuery],
    queryFn: () => api.listProjects({ search: searchQuery }),
  });

  // Budget mutation
  const budgetMutation = useMutation({
    mutationFn: ({ projectId, budgetLimit }: { projectId: string; budgetLimit: number | null }) =>
      api.setProjectBudget(projectId, budgetLimit),
    onSuccess: () => {
      toast.success('VIBE budget updated successfully');
      setBudgetDialogOpen(false);
      queryClient.invalidateQueries({ queryKey: ['projects'] });
    },
    onError: (error) => {
      toast.error(`Failed to update budget: ${error.message}`);
    },
  });

  const handleManageAccess = (project: Project) => {
    setSelectedProject(project);
    setMembersDialogOpen(true);
  };

  const handleManageBudget = (project: Project) => {
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

  const formatDate = (date: Date | string) => {
    const dateObj = typeof date === 'string' ? new Date(date) : date;
    return dateObj.toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
    });
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
            <div className="text-center py-12">
              <FolderKanban className="h-12 w-12 mx-auto text-muted-foreground mb-4" />
              <h3 className="text-lg font-semibold mb-2">No projects found</h3>
              <p className="text-muted-foreground">
                {searchQuery ? 'Try adjusting your search' : 'No projects available yet'}
              </p>
            </div>
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
                  1 VIBE ≈ $0.001 USD. Set the maximum VIBE this project can spend on AI operations.
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
