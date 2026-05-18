import {
  useMutation,
  useQueries,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query';
import {
  CheckCircle,
  ExternalLink,
  Globe,
  Loader2,
  Plus,
  Share2,
  Trash2,
} from 'lucide-react';
import { useMemo, useState } from 'react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { CardGrid } from '@/components/ui/card-grid';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  organizationsApi,
  type SocialAccountRecord,
  socialApi,
} from '@/lib/api';
import { formatCompactNumber } from '@/lib/formatters';
import { organizationKeys, socialKeys } from '@/lib/query-keys';

import { PLATFORM_BG, PLATFORM_COLORS, PLATFORM_ICONS } from '../../constants';

const PLATFORMS = [
  { key: 'linkedin', label: 'LinkedIn' },
  { key: 'instagram', label: 'Instagram' },
  { key: 'twitter', label: 'X / Twitter' },
  { key: 'tiktok', label: 'TikTok' },
  { key: 'youtube', label: 'YouTube' },
  { key: 'facebook', label: 'Facebook' },
];

function ConnectDialog({
  open,
  onClose,
  orgId,
  projectEntries,
}: {
  open: boolean;
  onClose: () => void;
  orgId?: string;
  projectEntries: { id: string; name: string }[];
}) {
  const [scope, setScope] = useState<'org' | string>('org');

  const { data: platformStatus = [] } = useQuery({
    queryKey: ['social-platform-status'],
    queryFn: () => socialApi.getPlatformStatus(),
    staleTime: 300_000,
    enabled: open,
  });

  const configuredSet = new Set(
    platformStatus.filter((p) => p.configured).map((p) => p.platform)
  );

  const handleConnect = (platform: string) => {
    if (scope === 'org') {
      socialApi.connectAccount(platform, { orgId });
    } else {
      socialApi.connectAccount(platform, { projectId: scope });
    }
    onClose();
  };

  return (
    <Dialog open={open} onOpenChange={(v) => !v && onClose()}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>Connect Social Account</DialogTitle>
        </DialogHeader>

        {/* Scope selector */}
        <div className="space-y-2">
          <p className="text-xs text-muted-foreground font-medium">
            Connect to
          </p>
          <div className="flex flex-wrap gap-2">
            <button
              onClick={() => setScope('org')}
              className={`px-3 py-1.5 text-xs rounded-md border transition-colors ${
                scope === 'org'
                  ? 'bg-primary text-primary-foreground border-primary'
                  : 'border-border hover:bg-muted'
              }`}
            >
              Organization
            </button>
            {projectEntries.map((p) => (
              <button
                key={p.id}
                onClick={() => setScope(p.id)}
                className={`px-3 py-1.5 text-xs rounded-md border transition-colors ${
                  scope === p.id
                    ? 'bg-primary text-primary-foreground border-primary'
                    : 'border-border hover:bg-muted'
                }`}
              >
                {p.name}
              </button>
            ))}
          </div>
        </div>

        {/* Platform grid */}
        <div className="grid grid-cols-2 gap-2 mt-2">
          {PLATFORMS.map(({ key, label }) => {
            const Icon = PLATFORM_ICONS[key] || Globe;
            const configured =
              configuredSet.size === 0 || configuredSet.has(key);
            return (
              <button
                key={key}
                disabled={!configured}
                onClick={() => configured && handleConnect(key)}
                className={`flex items-center gap-3 p-3 rounded-lg border text-left transition-colors ${
                  configured
                    ? 'border-border hover:bg-muted/60 cursor-pointer'
                    : 'border-border/40 opacity-40 cursor-not-allowed'
                }`}
              >
                <div
                  className={`h-8 w-8 rounded-full flex items-center justify-center shrink-0 ${PLATFORM_BG[key] || 'bg-muted'}`}
                >
                  <Icon
                    className={`h-4 w-4 ${PLATFORM_COLORS[key] || 'text-muted-foreground'}`}
                  />
                </div>
                <div className="min-w-0">
                  <p className="text-sm font-medium">{label}</p>
                  <p className="text-[10px] text-muted-foreground">
                    {configured ? 'Click to connect' : 'Not configured'}
                  </p>
                </div>
              </button>
            );
          })}
        </div>
      </DialogContent>
    </Dialog>
  );
}

export function SocialAccountsView({
  projectEntries,
  orgId,
}: {
  projectEntries: { id: string; name: string }[];
  orgId?: string;
}) {
  const [connectOpen, setConnectOpen] = useState(false);

  const { data: brandProfile } = useQuery({
    queryKey: organizationKeys.brandProfile(orgId ?? ''),
    queryFn: () => organizationsApi.getBrandProfile(orgId!),
    staleTime: 300_000,
    enabled: !!orgId,
  });

  const accountQueries = useQueries({
    queries: projectEntries.map((e) => ({
      queryKey: socialKeys.accounts(e.id),
      queryFn: () => socialApi.listAccounts({ projectId: e.id }),
      staleTime: 60_000,
    })),
  });

  // Also fetch org-level accounts
  const { data: orgAccounts = [] } = useQuery({
    queryKey: socialKeys.accounts(orgId ?? ''),
    queryFn: () => socialApi.listAccounts({ organizationId: orgId }),
    staleTime: 60_000,
    enabled: !!orgId,
  });

  const allAccounts = useMemo(() => {
    const list: (SocialAccountRecord & {
      _project: string;
      _projectId: string;
    })[] = [];
    orgAccounts.forEach((a) =>
      list.push({ ...a, _project: 'Organization', _projectId: orgId ?? '' })
    );
    accountQueries.forEach((q, i) => {
      q.data?.forEach((a) =>
        list.push({
          ...a,
          _project: projectEntries[i].name,
          _projectId: projectEntries[i].id,
        })
      );
    });
    return list.sort((a, b) => a.platform.localeCompare(b.platform));
  }, [accountQueries, projectEntries, orgAccounts, orgId]);

  const queryClient = useQueryClient();
  const deleteMut = useMutation({
    mutationFn: (id: string) => socialApi.deleteAccount(id),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: socialKeys.accounts(orgId ?? ''),
      });
      accountQueries.forEach((_, i) => {
        queryClient.invalidateQueries({
          queryKey: socialKeys.accounts(projectEntries[i].id),
        });
      });
    },
  });

  const brandHandles: { platform: string; handle: string }[] = [];
  if (brandProfile) {
    if (brandProfile.socialInstagram)
      brandHandles.push({
        platform: 'instagram',
        handle: brandProfile.socialInstagram,
      });
    if (brandProfile.socialLinkedin)
      brandHandles.push({
        platform: 'linkedin',
        handle: brandProfile.socialLinkedin,
      });
    if (brandProfile.socialTwitter)
      brandHandles.push({
        platform: 'twitter',
        handle: brandProfile.socialTwitter,
      });
    if (brandProfile.socialTiktok)
      brandHandles.push({
        platform: 'tiktok',
        handle: brandProfile.socialTiktok,
      });
    if (brandProfile.socialYoutube)
      brandHandles.push({
        platform: 'youtube',
        handle: brandProfile.socialYoutube,
      });
    if (brandProfile.socialFacebook)
      brandHandles.push({
        platform: 'facebook',
        handle: brandProfile.socialFacebook,
      });
  }

  return (
    <div className="space-y-6">
      <ConnectDialog
        open={connectOpen}
        onClose={() => setConnectOpen(false)}
        orgId={orgId}
        projectEntries={projectEntries}
      />

      {/* Brand profile handles */}
      {brandHandles.length > 0 && (
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium flex items-center gap-2">
              <Share2 className="h-4 w-4 text-pink-500" />
              Brand Profile Handles
              <Badge variant="outline" className="text-xs ml-1">
                From brand research
              </Badge>
            </CardTitle>
          </CardHeader>
          <CardContent>
            <CardGrid columns={{ sm: 2, lg: 3 }} gap={3}>
              {brandHandles.map(({ platform, handle }) => {
                const Icon = PLATFORM_ICONS[platform] || Globe;
                const isConnected = allAccounts.some(
                  (a) => a.platform === platform
                );
                return (
                  <div
                    key={platform}
                    className={`flex items-center gap-3 p-3 rounded-lg border ${isConnected ? 'border-green-300 bg-green-50 dark:bg-green-950/20' : 'border-border/50 bg-muted/30'}`}
                  >
                    <div
                      className={`h-9 w-9 rounded-full flex items-center justify-center ${PLATFORM_BG[platform] || 'bg-muted'}`}
                    >
                      <Icon
                        className={`h-4.5 w-4.5 ${PLATFORM_COLORS[platform] || 'text-muted-foreground'}`}
                      />
                    </div>
                    <div className="min-w-0 flex-1">
                      <p className="text-xs font-medium truncate">{handle}</p>
                      <p className="text-xs text-muted-foreground capitalize">
                        {platform}
                      </p>
                    </div>
                    {isConnected ? (
                      <Badge
                        variant="outline"
                        className="text-[9px] text-green-600 border-green-300"
                      >
                        connected
                      </Badge>
                    ) : (
                      <button
                        onClick={() => setConnectOpen(true)}
                        className="text-[9px] text-primary hover:underline"
                      >
                        connect →
                      </button>
                    )}
                  </div>
                );
              })}
            </CardGrid>
          </CardContent>
        </Card>
      )}

      {/* Connected accounts */}
      <Card className="bg-card/80 backdrop-blur-sm border-border/50">
        <CardHeader className="pb-3">
          <div className="flex items-center justify-between">
            <CardTitle className="text-sm font-medium flex items-center gap-2">
              <CheckCircle className="h-4 w-4 text-green-500" />
              Connected Accounts
              <Badge variant="secondary" className="text-xs">
                {allAccounts.length}
              </Badge>
            </CardTitle>
            <Button
              size="sm"
              variant="outline"
              className="h-7 text-xs gap-1.5"
              onClick={() => setConnectOpen(true)}
            >
              <Plus className="h-3.5 w-3.5" />
              Connect Account
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {allAccounts.length === 0 ? (
            <div className="text-center py-10 text-muted-foreground">
              <Share2 className="h-8 w-8 mx-auto mb-3 opacity-40" />
              <p className="text-sm font-medium mb-1">
                No accounts connected yet
              </p>
              <p className="text-xs mb-4">
                Connect social accounts to track performance and publish content
              </p>
              <Button
                size="sm"
                variant="outline"
                className="gap-1.5"
                onClick={() => setConnectOpen(true)}
              >
                <Plus className="h-3.5 w-3.5" />
                Connect Account
              </Button>
            </div>
          ) : (
            <div className="space-y-3">
              {allAccounts.map((account) => {
                const Icon = PLATFORM_ICONS[account.platform] || Globe;
                return (
                  <div
                    key={account.id}
                    className="flex items-center gap-4 p-3 rounded-lg border border-border/50 hover:bg-muted/30 transition-colors"
                  >
                    <div
                      className={`h-10 w-10 rounded-full flex items-center justify-center shrink-0 ${PLATFORM_BG[account.platform] || 'bg-muted'}`}
                    >
                      {account.avatar_url ? (
                        <img
                          src={account.avatar_url}
                          className="h-10 w-10 rounded-full object-cover"
                          alt=""
                        />
                      ) : (
                        <Icon
                          className={`h-5 w-5 ${PLATFORM_COLORS[account.platform] || 'text-muted-foreground'}`}
                        />
                      )}
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-2">
                        <p className="text-sm font-medium truncate">
                          {account.display_name ||
                            account.username ||
                            account.platform}
                        </p>
                        <Badge
                          variant={
                            account.status === 'active'
                              ? 'default'
                              : account.status === 'error'
                                ? 'destructive'
                                : 'secondary'
                          }
                          className="text-xs"
                        >
                          {account.status}
                        </Badge>
                      </div>
                      <div className="flex items-center gap-3 text-xs text-muted-foreground mt-0.5">
                        <span className="capitalize">{account.platform}</span>
                        <span>&middot;</span>
                        <span>
                          {formatCompactNumber(account.follower_count)}{' '}
                          followers
                        </span>
                        {account.post_count != null && (
                          <>
                            <span>&middot;</span>
                            <span>{account.post_count} posts</span>
                          </>
                        )}
                        <span>&middot;</span>
                        <span>{account._project}</span>
                      </div>
                    </div>
                    <div className="flex items-center gap-1 shrink-0">
                      {account.profile_url && (
                        <a
                          href={account.profile_url}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="p-1.5 rounded hover:bg-muted transition-colors"
                          title="View profile"
                        >
                          <ExternalLink className="h-3.5 w-3.5 text-muted-foreground" />
                        </a>
                      )}
                      <button
                        onClick={() => {
                          if (confirm('Disconnect this account?'))
                            deleteMut.mutate(account.id);
                        }}
                        className="p-1.5 rounded hover:bg-destructive/10 transition-colors"
                        title="Disconnect"
                      >
                        {deleteMut.isPending ? (
                          <Loader2 className="h-3.5 w-3.5 animate-spin text-muted-foreground" />
                        ) : (
                          <Trash2 className="h-3.5 w-3.5 text-muted-foreground hover:text-destructive" />
                        )}
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
