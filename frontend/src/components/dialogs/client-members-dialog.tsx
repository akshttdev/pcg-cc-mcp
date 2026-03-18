import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useMutationWithToast } from '@/hooks/useMutationWithToast';
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
import { Shield, Trash2, UserPlus, Eye, Pencil } from 'lucide-react';
import type { UserListItem } from 'shared/types';
import { makeRequest, handleApiResponse } from '@/lib/api/client';
import { userKeys } from '@/lib/query-keys';

interface ClientMembersDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  clientId: string;
  clientName: string;
}

interface ClientMemberItem {
  id: string;
  client_id: string;
  user_id: string;
  role: string;
  granted_by: string | null;
  granted_at: string;
  username: string;
  full_name: string;
  avatar_url: string | null;
  granted_by_username: string | null;
}

const api = {
  listClientMembers: async (clientId: string): Promise<ClientMemberItem[]> => {
    const response = await makeRequest(`/api/clients/${clientId}/members`);
    return handleApiResponse<ClientMemberItem[]>(response);
  },

  listUsers: async (): Promise<UserListItem[]> => {
    const response = await makeRequest('/api/users');
    return handleApiResponse<UserListItem[]>(response);
  },

  addClientMember: async (clientId: string, userId: string, role: string) => {
    const response = await makeRequest(`/api/clients/${clientId}/members`, {
      method: 'POST',
      body: JSON.stringify({ user_id: userId, role }),
    });
    return handleApiResponse(response);
  },

  removeClientMember: async (clientId: string, userId: string) => {
    const response = await makeRequest(`/api/clients/${clientId}/members/${userId}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },
};

const roleIcons = {
  admin: Shield,
  editor: Pencil,
  viewer: Eye,
};

const roleColors = {
  admin: 'bg-red-500',
  editor: 'bg-blue-500',
  viewer: 'bg-gray-500',
};

export function ClientMembersDialog({
  open,
  onOpenChange,
  clientId,
  clientName,
}: ClientMembersDialogProps) {
  const [selectedUserId, setSelectedUserId] = useState<string>('');
  const [selectedRole, setSelectedRole] = useState<string>('viewer');

  const { data: members = [], isLoading } = useQuery({
    queryKey: userKeys.clientMembers(clientId),
    queryFn: () => api.listClientMembers(clientId),
    enabled: open,
  });

  const { data: allUsers = [] } = useQuery({
    queryKey: userKeys.all,
    queryFn: api.listUsers,
    enabled: open,
  });

  const availableUsers = allUsers.filter(
    (user) => !members.some((member) => member.user_id === user.id)
  );

  const addMutation = useMutationWithToast({
    mutationFn: ({ userId, role }: { userId: string; role: string }) =>
      api.addClientMember(clientId, userId, role),
    successMessage: 'Member added',
    errorMessage: 'Failed to add member',
    invalidateKeys: [userKeys.clientMembers(clientId)],
    onSuccess: () => {
      setSelectedUserId('');
      setSelectedRole('viewer');
    },
  });

  const removeMutation = useMutationWithToast({
    mutationFn: (userId: string) => api.removeClientMember(clientId, userId),
    successMessage: 'Member removed',
    errorMessage: 'Failed to remove member',
    invalidateKeys: [userKeys.clientMembers(clientId)],
  });

  const handleAddMember = () => {
    if (selectedUserId && selectedRole) {
      addMutation.mutate({ userId: selectedUserId, role: selectedRole });
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
          <DialogTitle>Manage Client Members</DialogTitle>
          <DialogDescription>
            Control who has access to "{clientName}" and their permission levels.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-6 py-4">
          {/* Add New Member */}
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

          {/* Current Members */}
          <div className="space-y-3">
            <h3 className="text-sm font-medium">Current Members ({members.length})</h3>

            {isLoading ? (
              <div className="text-center py-8 text-muted-foreground">Loading members...</div>
            ) : members.length === 0 ? (
              <div className="text-center py-8 text-muted-foreground">
                No members yet. Add members to grant them access to this client.
              </div>
            ) : (
              <div className="border rounded-lg">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>User</TableHead>
                      <TableHead>Role</TableHead>
                      <TableHead>Added</TableHead>
                      <TableHead className="text-right">Actions</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {members.map((member) => {
                      const RoleIcon = roleIcons[member.role as keyof typeof roleIcons] || Eye;
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
                            <Badge
                              variant="secondary"
                              className={`${roleColors[member.role as keyof typeof roleColors] || 'bg-gray-500'} text-white`}
                            >
                              <RoleIcon className="h-3 w-3 mr-1" />
                              {member.role}
                            </Badge>
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
                            <Button
                              size="sm"
                              variant="ghost"
                              onClick={() => removeMutation.mutate(member.user_id)}
                              disabled={removeMutation.isPending}
                            >
                              <Trash2 className="h-4 w-4 text-destructive" />
                            </Button>
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
                Can view client data (read-only)
              </li>
              <li>
                <Badge variant="secondary" className="bg-blue-500 text-white mr-2">Editor</Badge>
                Can view and edit client data
              </li>
              <li>
                <Badge variant="secondary" className="bg-red-500 text-white mr-2">Admin</Badge>
                Full client access and member management
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
