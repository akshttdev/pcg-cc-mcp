import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Users,
  ChevronDown,
  ChevronRight,
  Trash2,
  UserPlus,
  Link2,
  Eye,
  Shield,
  Loader2,
  Copy,
  X,
} from 'lucide-react';
import { organizationsApi, resolveApiUrl } from '@/lib/api';
import type { OrgMember } from '../types';
import { formatDate } from '../helpers';
import { MemberAssignments } from '../components/MemberAssignments';

export function MembersTab({ orgId, orgName }: { orgId: string; orgName: string }) {
  const queryClient = useQueryClient();
  const { data: members = [], isLoading } = useQuery<OrgMember[]>({
    queryKey: ['org-members', orgId],
    queryFn: () => organizationsApi.getMembers(orgId),
    enabled: !!orgId,
  });

  const { data: allUsers = [] } = useQuery<any[]>({
    queryKey: ['all-users'],
    queryFn: async () => {
      const res = await fetch(resolveApiUrl('/api/users'), { credentials: 'include' });
      if (!res.ok) return [];
      const data = await res.json();
      return data.data || [];
    },
  });

  const [expandedMember, setExpandedMember] = useState<string | null>(null);
  const [showAddMember, setShowAddMember] = useState(false);
  const [addUserId, setAddUserId] = useState('');
  const [addRole, setAddRole] = useState('member');
  const [showInviteDialog, setShowInviteDialog] = useState(false);
  const [inviteRole, setInviteRole] = useState('member');
  const [inviteLink, setInviteLink] = useState('');
  const [copied, setCopied] = useState(false);

  const availableUsers = allUsers.filter(
    (u: any) => !members.some((m) => m.user_id === u.id)
  );

  const addMemberMutation = useMutation({
    mutationFn: () => organizationsApi.addMember(orgId, addUserId, addRole),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['org-members', orgId] });
      setAddUserId('');
      setAddRole('member');
      setShowAddMember(false);
      toast.success('Member added');
    },
    onError: () => toast.error('Failed to add member'),
  });

  const removeMemberMutation = useMutation({
    mutationFn: (userId: string) => organizationsApi.removeMember(orgId, userId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['org-members', orgId] });
      toast.success('Member removed');
    },
    onError: () => toast.error('Failed to remove member'),
  });

  const changeRoleMutation = useMutation({
    mutationFn: ({ userId, role }: { userId: string; role: string }) =>
      organizationsApi.changeMemberRole(orgId, userId, role),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['org-members', orgId] });
      toast.success('Role updated');
    },
    onError: () => toast.error('Failed to update role'),
  });

  const createInviteMutation = useMutation({
    mutationFn: () => organizationsApi.createInvitation(orgId, inviteRole),
    onSuccess: (data: any) => {
      setInviteLink(data.invite_url || '');
    },
    onError: () => toast.error('Failed to create invite'),
  });

  const handleCopyLink = () => {
    navigator.clipboard.writeText(inviteLink);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <Card className="bg-card/80 backdrop-blur-sm border-border/50">
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle>Team Members</CardTitle>
            <CardDescription>Manage who has access to {orgName}</CardDescription>
          </div>
          <div className="flex gap-2">
            <Button size="sm" variant="outline" onClick={() => { setShowInviteDialog(true); setInviteLink(''); }}>
              <Link2 className="h-4 w-4 mr-2" />
              Invite Link
            </Button>
            <Button size="sm" onClick={() => setShowAddMember(true)}>
              <UserPlus className="h-4 w-4 mr-2" />
              Add Member
            </Button>
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-3">
        {showAddMember && (
          <div className="flex items-center gap-2 p-3 border rounded-lg bg-muted/50">
            <Select value={addUserId} onValueChange={setAddUserId}>
              <SelectTrigger className="flex-1">
                <SelectValue placeholder="Select user..." />
              </SelectTrigger>
              <SelectContent>
                {availableUsers.map((u: any) => (
                  <SelectItem key={u.id} value={u.id}>
                    {u.full_name || u.username} <span className="text-muted-foreground ml-1">@{u.username}</span>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <Select value={addRole} onValueChange={setAddRole}>
              <SelectTrigger className="w-[120px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="viewer">Viewer</SelectItem>
                <SelectItem value="member">Member</SelectItem>
                <SelectItem value="admin">Admin</SelectItem>
              </SelectContent>
            </Select>
            <Button size="sm" onClick={() => addMemberMutation.mutate()} disabled={!addUserId || addMemberMutation.isPending}>
              Add
            </Button>
            <Button size="sm" variant="ghost" onClick={() => setShowAddMember(false)}>
              <X className="h-4 w-4" />
            </Button>
          </div>
        )}

        {isLoading ? (
          <div className="text-center py-8 text-muted-foreground">
            <Loader2 className="h-5 w-5 mx-auto mb-2 animate-spin" />
            Loading members...
          </div>
        ) : members.length === 0 ? (
          <div className="text-center py-8 text-muted-foreground">
            <Users className="h-8 w-8 mx-auto mb-2 opacity-40" />
            <p>No members yet. Add members or send an invite link.</p>
          </div>
        ) : (
          <div className="space-y-1">
            {members.map((m: OrgMember) => {
              const isExpanded = expandedMember === m.user_id;
              const displayName = m.user?.full_name || m.user?.username || m.user_id;
              return (
                <div key={m.id} className="border rounded-lg">
                  <div
                    className="flex items-center justify-between p-3 cursor-pointer hover:bg-muted/50"
                    onClick={() => setExpandedMember(isExpanded ? null : m.user_id)}
                  >
                    <div className="flex items-center gap-3">
                      {isExpanded ? <ChevronDown className="h-4 w-4 text-muted-foreground" /> : <ChevronRight className="h-4 w-4 text-muted-foreground" />}
                      <div className="h-8 w-8 rounded-full bg-muted flex items-center justify-center text-sm font-medium">
                        {displayName[0].toUpperCase()}
                      </div>
                      <div>
                        <p className="text-sm font-medium">{displayName}</p>
                        {m.user?.email && (
                          <p className="text-xs text-muted-foreground">{m.user.email}</p>
                        )}
                      </div>
                    </div>
                    <div className="flex items-center gap-2" onClick={(e) => e.stopPropagation()}>
                      <Select
                        value={m.role}
                        onValueChange={(role) => changeRoleMutation.mutate({ userId: m.user_id, role })}
                      >
                        <SelectTrigger className="w-[110px] h-7 text-xs">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="viewer"><div className="flex items-center gap-1"><Eye className="h-3 w-3" /> Viewer</div></SelectItem>
                          <SelectItem value="member"><div className="flex items-center gap-1"><Users className="h-3 w-3" /> Member</div></SelectItem>
                          <SelectItem value="admin"><div className="flex items-center gap-1"><Shield className="h-3 w-3" /> Admin</div></SelectItem>
                        </SelectContent>
                      </Select>
                      <span className="text-xs text-muted-foreground hidden sm:inline">
                        {formatDate(m.joined_at)}
                      </span>
                      <Button
                        size="sm"
                        variant="ghost"
                        className="h-7 w-7 p-0"
                        onClick={() => removeMemberMutation.mutate(m.user_id)}
                      >
                        <Trash2 className="h-3.5 w-3.5 text-destructive" />
                      </Button>
                    </div>
                  </div>
                  {isExpanded && (
                    <MemberAssignments orgId={orgId} userId={m.user_id} />
                  )}
                </div>
              );
            })}
          </div>
        )}
      </CardContent>

      {/* Invite Link Dialog */}
      <Dialog open={showInviteDialog} onOpenChange={setShowInviteDialog}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Create Invite Link</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <div>
              <Label>Role for new members</Label>
              <Select value={inviteRole} onValueChange={setInviteRole}>
                <SelectTrigger className="mt-1">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="viewer">Viewer</SelectItem>
                  <SelectItem value="member">Member</SelectItem>
                  <SelectItem value="admin">Admin</SelectItem>
                </SelectContent>
              </Select>
            </div>
            {!inviteLink ? (
              <Button onClick={() => createInviteMutation.mutate()} disabled={createInviteMutation.isPending} className="w-full">
                {createInviteMutation.isPending ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : <Link2 className="h-4 w-4 mr-2" />}
                Generate Link
              </Button>
            ) : (
              <div className="space-y-2">
                <div className="flex items-center gap-2">
                  <Input value={inviteLink} readOnly className="text-xs" />
                  <Button size="sm" variant="outline" onClick={handleCopyLink}>
                    <Copy className="h-4 w-4" />
                  </Button>
                </div>
                {copied && <p className="text-xs text-green-600">Copied to clipboard!</p>}
                <p className="text-xs text-muted-foreground">This link expires in 7 days.</p>
              </div>
            )}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowInviteDialog(false)}>Close</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </Card>
  );
}
