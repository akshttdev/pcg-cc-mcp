import { useQuery } from '@tanstack/react-query';
import { CheckCircle2, Globe, Share2 } from 'lucide-react';
import { useNavigate } from 'react-router-dom';

import { Button } from '@/components/ui/button';
import { socialApi } from '@/lib/api';

import { PLATFORM_BG, PLATFORM_COLORS, PLATFORM_ICONS } from '../../constants';

const PLATFORM_META: { key: string; label: string; desc: string }[] = [
  {
    key: 'linkedin',
    label: 'LinkedIn',
    desc: 'Company page, posts, B2B lead tracking.',
  },
  {
    key: 'instagram',
    label: 'Instagram',
    desc: 'Schedule posts, track mentions, monitor engagement.',
  },
  {
    key: 'twitter',
    label: 'X / Twitter',
    desc: 'Post scheduling, mention monitoring, DM management.',
  },
  {
    key: 'tiktok',
    label: 'TikTok',
    desc: 'Video scheduling and performance analytics.',
  },
  {
    key: 'youtube',
    label: 'YouTube',
    desc: 'Channel analytics, comment monitoring, content sync.',
  },
  {
    key: 'facebook',
    label: 'Facebook',
    desc: 'Page management, ads integration, audience insights.',
  },
];

export function SocialSection({ orgId }: { orgId?: string }) {
  const navigate = useNavigate();

  const { data: platformStatus = [] } = useQuery({
    queryKey: ['social-platform-status'],
    queryFn: () => socialApi.getPlatformStatus(),
    staleTime: 300_000,
  });

  const configuredSet = new Set(
    platformStatus.filter((p) => p.configured).map((p) => p.platform)
  );

  const goToAccounts = () => {
    if (orgId) {
      navigate(`/organizations/${orgId}?tab=social&sv=accounts`);
    }
  };

  return (
    <section className="space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Social Media
        </h3>
        <Button
          size="sm"
          variant="outline"
          className="h-7 text-xs"
          onClick={goToAccounts}
        >
          <Share2 className="h-3 w-3 mr-1.5" />
          Manage Accounts
        </Button>
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
        {PLATFORM_META.map(({ key, label, desc }) => {
          const Icon = PLATFORM_ICONS[key] || Globe;
          const configured = configuredSet.has(key);
          return (
            <div
              key={key}
              className={`flex items-start gap-3 p-3 rounded-lg border ${
                configured
                  ? 'border-green-300/60 bg-green-50/40 dark:bg-green-950/10'
                  : 'border-border/60 bg-card/60'
              }`}
            >
              <div
                className={`h-8 w-8 rounded-full flex items-center justify-center shrink-0 mt-0.5 ${PLATFORM_BG[key] || 'bg-muted'}`}
              >
                <Icon
                  className={`h-4 w-4 ${PLATFORM_COLORS[key] || 'text-muted-foreground'}`}
                />
              </div>
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 mb-0.5">
                  <span className="text-sm font-medium">{label}</span>
                  {configured && (
                    <CheckCircle2 className="h-3.5 w-3.5 text-green-500 shrink-0" />
                  )}
                </div>
                <p className="text-xs text-muted-foreground leading-relaxed">
                  {desc}
                </p>
              </div>
              {configured ? (
                <button
                  onClick={goToAccounts}
                  className="text-[10px] text-primary hover:underline shrink-0 mt-0.5 whitespace-nowrap"
                >
                  Connect →
                </button>
              ) : (
                <span className="text-[10px] text-muted-foreground italic shrink-0 mt-0.5 whitespace-nowrap">
                  Not configured
                </span>
              )}
            </div>
          );
        })}
      </div>

      <p className="text-xs text-muted-foreground">
        To enable additional platforms, add their OAuth credentials to the
        server environment.
      </p>
    </section>
  );
}
