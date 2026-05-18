import {
  BarChart2,
  Brain,
  FileText,
  Inbox,
  LayoutGrid,
  Share2,
} from 'lucide-react';
import { useSearchParams } from 'react-router-dom';

import { SocialAccountsView } from './SocialAccountsView';
import { SocialAnalyticsView } from './SocialAnalyticsView';
import { SocialContentView } from './SocialContentView';
import { SocialInboxView } from './SocialInboxView';
import { SocialIntelligenceView } from './SocialIntelligenceView';
import { SocialOverviewView } from './SocialOverviewView';

function SocialTab({
  projectEntries,
  orgId,
  platformFilter,
}: {
  projectEntries: { id: string; name: string }[];
  orgId?: string;
  /** External platform filter passed down from a parent filter bar */
  platformFilter?: string[];
}) {
  const [searchParams, setSearchParams] = useSearchParams();
  const socialView = searchParams.get('sv') || 'overview';

  const views = [
    { key: 'overview', label: 'Overview', icon: LayoutGrid },
    { key: 'accounts', label: 'Accounts', icon: Share2 },
    { key: 'content', label: 'Content', icon: FileText },
    { key: 'inbox', label: 'Inbox', icon: Inbox },
    { key: 'analytics', label: 'Analytics', icon: BarChart2 },
    { key: 'intelligence', label: 'Intelligence', icon: Brain },
  ];

  const setSocialView = (v: string) => {
    const params = new URLSearchParams(searchParams);
    if (v === 'overview') params.delete('sv');
    else params.set('sv', v);
    setSearchParams(params, { replace: true });
  };

  return (
    <div className="space-y-6">
      <div className="flex gap-1 p-1 bg-muted/50 rounded-lg w-fit flex-wrap">
        {views.map(({ key, label, icon: Icon }) => (
          <button
            key={key}
            onClick={() => setSocialView(key)}
            className={`flex items-center gap-1.5 px-4 py-1.5 text-sm font-medium rounded-md transition-all ${
              socialView === key
                ? 'bg-background text-foreground shadow-sm'
                : 'text-muted-foreground hover:text-foreground'
            }`}
          >
            <Icon className="h-3.5 w-3.5" />
            {label}
          </button>
        ))}
      </div>

      {socialView === 'overview' && (
        <SocialOverviewView
          projectEntries={projectEntries}
          orgId={orgId}
          onSwitchView={setSocialView}
        />
      )}
      {socialView === 'accounts' && (
        <SocialAccountsView projectEntries={projectEntries} orgId={orgId} />
      )}
      {socialView === 'content' && (
        <SocialContentView
          projectEntries={projectEntries}
          orgId={orgId}
          externalPlatformFilter={platformFilter}
        />
      )}
      {socialView === 'inbox' && (
        <SocialInboxView projectEntries={projectEntries} />
      )}
      {socialView === 'analytics' && (
        <SocialAnalyticsView projectEntries={projectEntries} orgId={orgId} />
      )}
      {socialView === 'intelligence' && (
        <SocialIntelligenceView projectEntries={projectEntries} orgId={orgId} />
      )}
    </div>
  );
}

export default SocialTab;
