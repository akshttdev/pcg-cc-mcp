import { useParams, useNavigate } from 'react-router-dom';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
  TrendingUp,
  Package,
  Users,
  Calendar,
  ArrowRight,
} from 'lucide-react';
import { CrmPipelineMetrics } from '@/components/crm/CrmPipelineMetrics';
import { CrmActivityTimeline } from '@/components/crm/CrmActivityTimeline';

export function CrmOverviewPage() {
  const { projectId } = useParams<{ projectId: string }>();
  const navigate = useNavigate();

  if (!projectId) {
    return (
      <div className="h-full flex items-center justify-center text-muted-foreground">
        Project not found
      </div>
    );
  }

  const quickLinks = [
    {
      label: 'Sales Pipeline',
      to: `/projects/${projectId}/crm/sales`,
      icon: TrendingUp,
      description: 'Track leads from research to close',
      color: 'text-blue-600 bg-blue-100',
    },
    {
      label: 'Client Delivery',
      to: `/projects/${projectId}/crm/delivery`,
      icon: Package,
      description: 'Manage client onboarding and project delivery',
      color: 'text-purple-600 bg-purple-100',
    },
    {
      label: 'Contacts',
      to: `/projects/${projectId}/crm`,
      icon: Users,
      description: 'View and manage all contacts',
      color: 'text-green-600 bg-green-100',
    },
    {
      label: 'Conferences',
      to: `/projects/${projectId}/crm/conferences`,
      icon: Calendar,
      description: 'Track conference applications',
      color: 'text-amber-600 bg-amber-100',
    },
  ];

  return (
    <div className="p-6 space-y-6">
      <div>
        <h1 className="text-2xl font-bold">CRM Overview</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Pipeline metrics and recent activity at a glance
        </p>
      </div>

      {/* Pipeline Metrics */}
      <CrmPipelineMetrics projectId={projectId} />

      {/* Quick Links */}
      <div className="grid gap-4 md:grid-cols-4">
        {quickLinks.map((link) => {
          const Icon = link.icon;
          return (
            <Card
              key={link.to}
              className="cursor-pointer hover:shadow-md transition-shadow"
              onClick={() => navigate(link.to)}
            >
              <CardContent className="pt-6">
                <div className="flex items-start gap-3">
                  <div className={`p-2 rounded-lg ${link.color}`}>
                    <Icon className="h-5 w-5" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium">{link.label}</p>
                    <p className="text-xs text-muted-foreground mt-0.5">
                      {link.description}
                    </p>
                  </div>
                  <ArrowRight className="h-4 w-4 text-muted-foreground shrink-0 mt-1" />
                </div>
              </CardContent>
            </Card>
          );
        })}
      </div>

      {/* Recent Activity */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">Recent Activity</CardTitle>
        </CardHeader>
        <CardContent>
          <CrmActivityTimeline projectId={projectId} limit={20} />
        </CardContent>
      </Card>
    </div>
  );
}
