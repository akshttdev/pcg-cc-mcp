import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { Shield, User, Edit, Trash2, UserPlus, Eye, Pencil } from 'lucide-react';
import type { ProjectMemberItem, UserListItem } from 'shared/types';

interface ProjectMembersDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  projectId: string;
  projectName: string;
}

// API functions
const api = {
  listProjectMembers: async (projectId: string): Promise<ProjectMemberItem[]> => {
    const response = await fetch(`/api/permissions/projects/${projectId}/members`);
    if (!response.ok) throw new Error('Failed to fetch project members');
    const data = await response.json();
    return data.data;
  },

  listUsers: async (): Promise<UserListItem[]> => {
    const response = await fetch('/api/users');
    if (!response.ok) throw new Error('Failed to fetch users');
    const data = await response.json();
    return data.data;
  },

  addProjectMember: async (projectId: string, userId: string, role: string) => {
    const response = await fetch(`/api/permissions/projects/${projectId}/members`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ user_id: userId, role }),
    });
    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.message || 'Failed to add member');
    }
    return response.json();
  },

  updateMemberRole: async (projectId: string, userId: string, role: string) => {
    const response = await fetch(`/api/permissions/projects/${projectId}/members/${userId}/role`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ role }),
    });
    if (!response.ok) throw new Error('Failed to update member role');
    return response.json();
  },

  removeMember: async (projectId: string, userId: string) => {
    const response = await fetch(`/api/permissions/projects/${projectId}/members/${userId}`, {
      method: 'DELETE',
    });
    if (!response.ok) throw new Error('Failed to remove member');
  },
};

const roleIcons = {
  owner: Shield,
  admin: Shield,
  editor: Pencil,
  viewer: Eye,
};

const roleColors = {
  owner: 'bg-purple-500',
  admin: 'bg-red-500',
  editor: 'bg-blue-500',
  viewer: 'bg-gray-500',
};

export function ProjectMembersDialog({
  open,
  onOpenChange,
  projectId,
  projectName,
}: ProjectMembersDialogProps) {
  const queryClient = useQueryClient();
  const [selectedUserId, setSelectedUserId] = useState<string>('');
  const [selectedRole, setSelectedRole] = useState<string>('viewer');
  const [editingMember, setEditingMember] = useState<string | null>(null);
  const [editRole, setEditRole] = useState<string>('');

  // Fetch project members
  const { data: members = [], isLoading } = useQuery({
    queryKey: ['project-members', projectId],
    queryFn: () => api.listProjectMembers(projectId),
    enabled: open,
  });

  // Fetch all users for adding new members
  const { data: allUsers = [] } = useQuery({
    queryKey: ['users'],
    queryFn: api.listUsers,
    enabled: open,
  });

  // Filter out users who are already members
  const availableUsers = allUsers.filter(
    (user) => !members.some((member) => member.user_id === user.id)
  );

  // Add member mutation
  const addMutation = useMutation({
    mutationFn: ({ userId, role }: { userId: string; role: string }) =>
      api.addProjectMember(projectId, userId, role),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['project-members', projectId] });
      setSelectedUserId('');
      setSelectedRole('viewer');
    },
  });

  // Update role mutation
  const updateRoleMutation = useMutation({
    mutationFn: ({ userId, role }: { userId: string; role: string }) =>
      api.updateMemberRole(projectId, userId, role),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['project-members', projectId] });
      setEditingMember(null);
    },
  });

  // Remove member mutation
  const removeMutation = useMutation({
    mutationFn: (userId: string) => api.removeMember(projectId, userId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['project-members', projectId] });
    },
  });

  const handleAddMember = () => {
    if (selectedUserId && selectedRole) {
      addMutation.mutate({ userId: selectedUserId, role: selectedRole });
    }
  };

  const handleUpdateRole = (userId: string) => {
    if (editRole) {
      updateRoleMutation.mutate({ userId, role: editRole });
    }
  };

  const getInitials = (name: string) => {
    return name
      .split(' ')
      .map((n) => n[0])
      .join('')
      .toUpperCase()
      .slice(0, 2);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[700px] max-h-[80vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>Manage Project Access</DialogTitle>
          <DialogDescription>
            Control who has access to "{projectName}" and their permission levels.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-6 py-4">
          {/* Add New Member Section */}
          <div className="space-y-3">
            <h3 className="text-sm font-medium">Add Member</h3>
            <div className="flex gap-2">
              <Select value={selectedUserId} onValueChange={setSelectedUserId}>
                <SelectTrigger className="flex-1">
                  <SelectValue placeholder="Select user..." />
                </SelectTrigger>
                <SelectContent>
                  {availableUsers.map((user) => (
                    <SelectItem key={user.id} value={user.id}>
                      <div className="flex items-center gap-2">
                        <span>{user.full_name}</span>
                        <span className="text-muted-foreground text-xs">@{user.username}</span>
                      </div>
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>

              <Select value={selectedRole} onValueChange={setSelectedRole}>
                <SelectTrigger className="w-[140px]">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="viewer">
                    <div className="flex items-center gap-2">
                      <Eye className="h-4 w-4" />
                      Viewer
                    </div>
                  </SelectItem>
                  <SelectItem value="editor">
                    <div className="flex items-center gap-2">
                      <Pencil className="h-4 w-4" />
                      Editor
                    </div>
                  </SelectItem>
                  <SelectItem value="admin">
                    <div className="flex items-center gap-2">
                      <Shield className="h-4 w-4" />
                      Admin
                    </div>
                  </SelectItem>
                  <SelectItem value="owner">
                    <div className="flex items-center gap-2">
                      <Shield className="h-4 w-4" />
                      Owner
                    </div>
                  </SelectItem>
                </SelectContent>
              </Select>

              <Button
                onClick={handleAddMember}
                disabled={!selectedUserId || addMutation.isPending}
              >
                <UserPlus className="h-4 w-4 mr-2" />
                Add
              </Button>
            </div>
          </div>

          {/* Current Members List */}
          <div className="space-y-3">
            <h3 className="text-sm font-medium">Current Members ({members.length})</h3>
            
            {isLoading ? (
              <div className="text-center py-8 text-muted-foreground">Loading members...</div>
            ) : members.length === 0 ? (
              <div className="text-center py-8 text-muted-foreground">
                No members yet. Add members to grant them access.
              </div>
            ) : (
              <div className="border rounded-lg">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>User</TableHead>
                      <TableHead>Role</TableHead>
                      <TableHead>Granted</TableHead>
                      <TableHead className="text-right">Actions</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {members.map((member) => {
                      const RoleIcon = roleIcons[member.role as keyof typeof roleIcons] || User;
                      const isEditing = editingMember === member.user_id;

                      return (
                        <TableRow key={member.user_id}>
                          <TableCell>
                            <div className="flex items-center gap-3">
                              <Avatar className="h-8 w-8">
                                <AvatarImage src={member.avatar_url || undefined} />
                                <AvatarFallback>{getInitials(member.full_name)}</AvatarFallback>
                              </Avatar>
                              <div>
                                <div className="font-medium">{member.full_name}</div>
                                <div className="text-xs text-muted-foreground">
                                  @{member.username}
                                </div>
                              </div>
                            </div>
                          </TableCell>
                          <TableCell>
                            {isEditing ? (
                              <Select value={editRole} onValueChange={setEditRole}>
                                <SelectTrigger className="w-[130px]">
                                  <SelectValue />
                                </SelectTrigger>
                                <SelectContent>
                                  <SelectItem value="viewer">Viewer</SelectItem>
                                  <SelectItem value="editor">Editor</SelectItem>
                                  <SelectItem value="admin">Admin</SelectItem>
                                  <SelectItem value="owner">Owner</SelectItem>
                                </SelectContent>
                              </Select>
                            ) : (
                              <Badge
                                variant="secondary"
                                className={`${roleColors[member.role as keyof typeof roleColors]} text-white`}
                              >
                                <RoleIcon className="h-3 w-3 mr-1" />
                                {member.role}
                              </Badge>
                            )}
                          </TableCell>
                          <TableCell>
                            <div className="text-sm text-muted-foreground">
                              {new Date(member.granted_at).toLocaleDateString()}
                              {member.granted_by_username && (
                                <div className="text-xs">by @{member.granted_by_username}</div>
                              )}
                            </div>
                          </TableCell>
                          <TableCell className="text-right">
                            {isEditing ? (
                              <div className="flex gap-1 justify-end">
                                <Button
                                  size="sm"
                                  variant="ghost"
                                  onClick={() => handleUpdateRole(member.user_id)}
                                  disabled={updateRoleMutation.isPending}
                                >
                                  Save
                                </Button>
                                <Button
                                  size="sm"
                                  variant="ghost"
                                  onClick={() => setEditingMember(null)}
                                >
                                  Cancel
                                </Button>
                              </div>
                            ) : (
                              <div className="flex gap-1 justify-end">
                                <Button
                                  size="sm"
                                  variant="ghost"
                                  onClick={() => {
                                    setEditingMember(member.user_id);
                                    setEditRole(member.role);
                                  }}
                                >
                                  <Edit className="h-4 w-4" />
                                </Button>
                                <Button
                                  size="sm"
                                  variant="ghost"
                                  onClick={() => removeMutation.mutate(member.user_id)}
                                  disabled={removeMutation.isPending}
                                >
                                  <Trash2 className="h-4 w-4 text-destructive" />
                                </Button>
                              </div>
                            )}
                          </TableCell>
                        </TableRow>
                      );
                    })}
                  </TableBody>
                </Table>
              </div>
            )}
          </div>

          {/* Role Descriptions */}
          <div className="bg-muted p-4 rounded-lg space-y-2 text-sm">
            <h4 className="font-medium">Role Permissions:</h4>
            <ul className="space-y-1 text-muted-foreground">
              <li>
                <Badge variant="secondary" className="bg-gray-500 text-white mr-2">Viewer</Badge>
                Can view project (read-only)
              </li>
              <li>
                <Badge variant="secondary" className="bg-blue-500 text-white mr-2">Editor</Badge>
                Can view and edit project
              </li>
              <li>
                <Badge variant="secondary" className="bg-red-500 text-white mr-2">Admin</Badge>
                Can edit and manage members
              </li>
              <li>
                <Badge variant="secondary" className="bg-purple-500 text-white mr-2">Owner</Badge>
                Full control including deletion
              </li>
            </ul>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Close
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
