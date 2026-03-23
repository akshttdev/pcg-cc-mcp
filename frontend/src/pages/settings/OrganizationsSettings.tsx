import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useMutationWithToast } from '@/hooks/useMutationWithToast';
import { organizationKeys, sidebarKeys } from '@/lib/query-keys';
import { Building2, Search, MoreVertical, Power, PowerOff, Trash2, Calendar } from 'lucide-react';
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
import { organizationsApi, type OrganizationData } from '@/lib/api';
import { showConfirm } from '@/lib/modals';

export function OrganizationsSettings() {
  const [searchQuery, setSearchQuery] = useState('');
  const [statusFilter, setStatusFilter] = useState<string>('all');

  const { data: orgs = [], isLoading, error } = useQuery({
    queryKey: organizationKeys.admin(),
    queryFn: () => organizationsApi.getAll(),
  });

  const activateMutation = useMutationWithToast({
    mutationFn: organizationsApi.activate,
    successMessage: 'Organization activated',
    errorMessage: 'Failed to activate organization',
    invalidateKeys: [organizationKeys.admin(), sidebarKeys.tree()],
  });

  const deactivateMutation = useMutationWithToast({
    mutationFn: organizationsApi.deactivate,
    successMessage: 'Organization deactivated',
    errorMessage: 'Failed to deactivate organization',
    invalidateKeys: [organizationKeys.admin(), sidebarKeys.tree()],
  });

  const deleteMutation = useMutationWithToast({
    mutationFn: organizationsApi.delete,
    successMessage: 'Organization deleted',
    errorMessage: 'Failed to delete organization',
    invalidateKeys: [organizationKeys.admin(), sidebarKeys.tree()],
  });

  const filtered = orgs.filter((org: OrganizationData) => {
    if (searchQuery && !org.name.toLowerCase().includes(searchQuery.toLowerCase())) return false;
    if (statusFilter === 'active' && !org.is_active) return false;
    if (statusFilter === 'inactive' && org.is_active) return false;
    return true;
  });

  const activeCount = orgs.filter((o: OrganizationData) => o.is_active).length;
  const inactiveCount = orgs.length - activeCount;

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold">Organizations</h1>
        <p className="text-muted-foreground mt-2">
          Manage all organizations — activate, deactivate, or delete
        </p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Total</CardTitle>
            <Building2 className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-semibold">{orgs.length}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Active</CardTitle>
            <Power className="h-4 w-4 text-green-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-semibold">{activeCount}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Inactive</CardTitle>
            <PowerOff className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-semibold">{inactiveCount}</div>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <div className="flex flex-col md:flex-row gap-4 items-start md:items-center justify-between">
            <div className="flex-1 w-full md:w-auto">
              <div className="relative">
                <Search className="absolute left-3 top-3 h-4 w-4 text-muted-foreground" />
                <Input
                  placeholder="Search organizations..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="pl-9"
                />
              </div>
            </div>
            <Select value={statusFilter} onValueChange={setStatusFilter}>
              <SelectTrigger className="w-[140px]">
                <SelectValue placeholder="Status" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">All Status</SelectItem>
                <SelectItem value="active">Active</SelectItem>
                <SelectItem value="inactive">Inactive</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="text-center py-12 text-muted-foreground">Loading organizations...</div>
          ) : error ? (
            <div className="text-center py-12 text-destructive">
              Failed to load organizations.
            </div>
          ) : filtered.length === 0 ? (
            <div className="text-center py-12 text-muted-foreground">
              No organizations found.
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Organization</TableHead>
                  <TableHead>Slug</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Created</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {filtered.map((org: OrganizationData) => (
                  <TableRow key={org.id} className={!org.is_active ? 'opacity-60' : ''}>
                    <TableCell>
                      <div className="font-medium">{org.name}</div>
                      {org.description && (
                        <div className="text-sm text-muted-foreground truncate max-w-[300px]">
                          {org.description}
                        </div>
                      )}
                    </TableCell>
                    <TableCell className="text-muted-foreground text-sm">{org.slug}</TableCell>
                    <TableCell>
                      {org.is_active ? (
                        <Badge variant="outline" className="bg-green-50 text-green-700 border-green-200 dark:bg-green-950 dark:text-green-400 dark:border-green-800">
                          Active
                        </Badge>
                      ) : (
                        <Badge variant="outline" className="bg-red-50 text-red-700 border-red-200 dark:bg-red-950 dark:text-red-400 dark:border-red-800">
                          Inactive
                        </Badge>
                      )}
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-2 text-sm text-muted-foreground">
                        <Calendar className="h-4 w-4" />
                        {new Date(org.created_at).toLocaleDateString()}
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
                          {org.is_active ? (
                            <DropdownMenuItem
                              onClick={async () => {
                                if (await showConfirm({ title: 'Deactivate Organization', message: `Deactivate "${org.name}"? It will be hidden from the sidebar.`, variant: 'destructive', confirmText: 'Deactivate' })) {
                                  deactivateMutation.mutate(org.id);
                                }
                              }}
                            >
                              <PowerOff className="h-4 w-4 mr-2" />
                              Deactivate
                            </DropdownMenuItem>
                          ) : (
                            <DropdownMenuItem
                              onClick={() => activateMutation.mutate(org.id)}
                            >
                              <Power className="h-4 w-4 mr-2" />
                              Activate
                            </DropdownMenuItem>
                          )}
                          <DropdownMenuSeparator />
                          <DropdownMenuItem
                            className="text-destructive focus:text-destructive"
                            onClick={async () => {
                              if (await showConfirm({ title: 'Delete Organization', message: `Delete "${org.name}"? This will permanently deactivate the organization.`, variant: 'destructive', confirmText: 'Delete' })) {
                                deleteMutation.mutate(org.id);
                              }
                            }}
                          >
                            <Trash2 className="h-4 w-4 mr-2" />
                            Delete
                          </DropdownMenuItem>
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
    </div>
  );
}
