import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useMutationWithToast } from '@/hooks/useMutationWithToast';
import { userKeys } from '@/lib/query-keys';
import { 
  Users, 
  UserPlus, 
  Shield, 
  UserX, 
  UserCheck, 
  Search, 
  Filter,
  MoreVertical,
  Mail,
  Calendar
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
  DropdownMenuSeparator,
} from '@/components/ui/dropdown-menu';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { 
  CreateUserDialog,
  EditUserRoleDialog,
  ConfirmActionDialog
} from '@/components/dialogs/user-management-dialogs';
import type { UserListItem } from 'shared/types';
import { usersApi } from '@/lib/api';

export function UsersSettings() {
  const [searchQuery, setSearchQuery] = useState('');
  const [statusFilter, setStatusFilter] = useState<string>('all');
  const [roleFilter, setRoleFilter] = useState<string>('all');
  const [inviteDialogOpen, setInviteDialogOpen] = useState(false);
  const [editRoleDialogOpen, setEditRoleDialogOpen] = useState(false);
  const [confirmDialogOpen, setConfirmDialogOpen] = useState(false);
  const [selectedUser, setSelectedUser] = useState<UserListItem | null>(null);
  const [pendingAction, setPendingAction] = useState<'suspend' | 'activate' | null>(null);

  // Build filters
  const filters = {
    search: searchQuery || undefined,
    is_active: statusFilter === 'active' ? true : statusFilter === 'inactive' ? false : undefined,
    is_admin: roleFilter === 'admin' ? true : roleFilter === 'member' ? false : undefined,
  };

  const { data: users = [], isLoading, error } = useQuery({
    queryKey: userKeys.list(filters),
    queryFn: () => usersApi.list(filters),
  });

  const updateRoleMutation = useMutationWithToast({
    mutationFn: ({ userId, isAdmin }: { userId: string; isAdmin: boolean }) =>
      usersApi.updateRole(userId, isAdmin),
    successMessage: 'Role updated',
    errorMessage: 'Failed to update role',
    invalidateKeys: [userKeys.all],
    onSuccess: () => setEditRoleDialogOpen(false),
  });

  const suspendMutation = useMutationWithToast({
    mutationFn: usersApi.suspend,
    successMessage: 'User suspended',
    errorMessage: 'Failed to suspend user',
    invalidateKeys: [userKeys.all],
    onSuccess: () => setConfirmDialogOpen(false),
  });

  const activateMutation = useMutationWithToast({
    mutationFn: usersApi.activate,
    successMessage: 'User activated',
    errorMessage: 'Failed to activate user',
    invalidateKeys: [userKeys.all],
    onSuccess: () => setConfirmDialogOpen(false),
  });

  const createUserMutation = useMutationWithToast({
    mutationFn: usersApi.create,
    successMessage: 'User invited',
    errorMessage: 'Failed to create user',
    invalidateKeys: [userKeys.all],
    onSuccess: () => setInviteDialogOpen(false),
  });

  const handleSuspendUser = (user: UserListItem) => {
    setSelectedUser(user);
    setPendingAction('suspend');
    setConfirmDialogOpen(true);
  };

  const handleActivateUser = (user: UserListItem) => {
    setSelectedUser(user);
    setPendingAction('activate');
    setConfirmDialogOpen(true);
  };

  const handleEditRole = (user: UserListItem) => {
    setSelectedUser(user);
    setEditRoleDialogOpen(true);
  };

  const handleConfirmAction = () => {
    if (!selectedUser) return;

    if (pendingAction === 'suspend') {
      suspendMutation.mutate(selectedUser.id);
    } else if (pendingAction === 'activate') {
      activateMutation.mutate(selectedUser.id);
    }
  };

  const getInitials = (name: string) => {
    return name
      .split(' ')
      .map(n => n[0])
      .join('')
      .toUpperCase()
      .slice(0, 2);
  };

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold">User Management</h1>
        <p className="text-muted-foreground mt-2">
          Manage team members, roles, and access permissions
        </p>
      </div>

      {/* Stats Cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Total Users</CardTitle>
            <Users className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-semibold">{users.length}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Active Users</CardTitle>
            <UserCheck className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-semibold">
              {users.filter(u => u.is_active === 1).length}
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Admins</CardTitle>
            <Shield className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-semibold">
              {users.filter(u => u.is_admin === 1).length}
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Filters and Actions */}
      <Card>
        <CardHeader>
          <div className="flex flex-col md:flex-row gap-4 items-start md:items-center justify-between">
            <div className="flex-1 w-full md:w-auto">
              <div className="relative">
                <Search className="absolute left-3 top-3 h-4 w-4 text-muted-foreground" />
                <Input
                  placeholder="Search users..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="pl-9"
                />
              </div>
            </div>
            <div className="flex gap-2 w-full md:w-auto">
              <Select value={statusFilter} onValueChange={setStatusFilter}>
                <SelectTrigger className="w-[140px]">
                  <Filter className="h-4 w-4 mr-2" />
                  <SelectValue placeholder="Status" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">All Status</SelectItem>
                  <SelectItem value="active">Active</SelectItem>
                  <SelectItem value="inactive">Inactive</SelectItem>
                </SelectContent>
              </Select>
              <Select value={roleFilter} onValueChange={setRoleFilter}>
                <SelectTrigger className="w-[140px]">
                  <Shield className="h-4 w-4 mr-2" />
                  <SelectValue placeholder="Role" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">All Roles</SelectItem>
                  <SelectItem value="admin">Admins</SelectItem>
                  <SelectItem value="member">Members</SelectItem>
                </SelectContent>
              </Select>
              <Button onClick={() => setInviteDialogOpen(true)}>
                <UserPlus className="h-4 w-4 mr-2" />
                Create User
              </Button>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="text-center py-12 text-muted-foreground">Loading users...</div>
          ) : error ? (
            <div className="text-center py-12 text-destructive">
              Failed to load users. Please try again.
            </div>
          ) : users.length === 0 ? (
            <div className="text-center py-12 text-muted-foreground">
              No users found. Try adjusting your filters.
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>User</TableHead>
                  <TableHead>Email</TableHead>
                  <TableHead>Role</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Last Login</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {users.map((user) => (
                  <TableRow key={user.id}>
                    <TableCell>
                      <div className="flex items-center gap-3">
                        <Avatar className="h-8 w-8">
                          <AvatarImage src={user.avatar_url || undefined} alt={user.full_name} />
                          <AvatarFallback>{getInitials(user.full_name)}</AvatarFallback>
                        </Avatar>
                        <div>
                          <div className="font-medium">{user.full_name}</div>
                          <div className="text-sm text-muted-foreground">@{user.username}</div>
                        </div>
                      </div>
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-2">
                        <Mail className="h-4 w-4 text-muted-foreground" />
                        {user.email}
                      </div>
                    </TableCell>
                    <TableCell>
                      {user.is_admin === 1 ? (
                        <Badge variant="default">
                          <Shield className="h-3 w-3 mr-1" />
                          Admin
                        </Badge>
                      ) : (
                        <Badge variant="secondary">Member</Badge>
                      )}
                    </TableCell>
                    <TableCell>
                      {user.is_active === 1 ? (
                        <Badge variant="outline" className="bg-green-50 text-green-700 border-green-200">
                          Active
                        </Badge>
                      ) : (
                        <Badge variant="outline" className="bg-red-50 text-red-700 border-red-200">
                          Suspended
                        </Badge>
                      )}
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-2 text-sm text-muted-foreground">
                        <Calendar className="h-4 w-4" />
                        {user.last_login_at 
                          ? new Date(user.last_login_at).toLocaleDateString()
                          : 'Never'}
                      </div>
                    </TableCell>
                    <TableCell className="text-right">
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button variant="ghost" size="sm">
                            <MoreVertical className="h-4 w-4" />
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                          <DropdownMenuItem onClick={() => handleEditRole(user)}>
                            <Shield className="h-4 w-4 mr-2" />
                            Change Role
                          </DropdownMenuItem>
                          <DropdownMenuSeparator />
                          {user.is_active === 1 ? (
                            <DropdownMenuItem 
                              onClick={() => handleSuspendUser(user)}
                              className="text-destructive"
                            >
                              <UserX className="h-4 w-4 mr-2" />
                              Suspend User
                            </DropdownMenuItem>
                          ) : (
                            <DropdownMenuItem onClick={() => handleActivateUser(user)}>
                              <UserCheck className="h-4 w-4 mr-2" />
                              Activate User
                            </DropdownMenuItem>
                          )}
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Dialogs */}
      <CreateUserDialog
        open={inviteDialogOpen}
        onOpenChange={setInviteDialogOpen}
        onCreate={(data) => createUserMutation.mutate(data)}
        isLoading={createUserMutation.isPending}
      />

      <EditUserRoleDialog
        open={editRoleDialogOpen}
        onOpenChange={setEditRoleDialogOpen}
        user={selectedUser}
        onSave={(isAdmin: boolean) => {
          if (selectedUser) {
            updateRoleMutation.mutate({ userId: selectedUser.id, isAdmin });
          }
        }}
        isLoading={updateRoleMutation.isPending}
      />

      <ConfirmActionDialog
        open={confirmDialogOpen}
        onOpenChange={setConfirmDialogOpen}
        title={pendingAction === 'suspend' ? 'Suspend User' : 'Activate User'}
        description={
          pendingAction === 'suspend'
            ? `Are you sure you want to suspend ${selectedUser?.full_name}? They will no longer be able to access the system.`
            : `Are you sure you want to activate ${selectedUser?.full_name}? They will regain access to the system.`
        }
        onConfirm={handleConfirmAction}
        isLoading={suspendMutation.isPending || activateMutation.isPending}
        variant={pendingAction === 'suspend' ? 'destructive' : 'default'}
      />
    </div>
  );
}
