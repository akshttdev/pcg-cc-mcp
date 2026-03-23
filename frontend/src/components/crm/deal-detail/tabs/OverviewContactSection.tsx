import { useQuery } from '@tanstack/react-query';
import {
  Building2,
  ExternalLink,
  FileText,
  FolderKanban,
  ListTodo,
  Mail,
  User,
  Users,
} from 'lucide-react';
import { Link, useNavigate } from 'react-router-dom';

import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { organizationsApi } from '@/lib/api';
import { organizationKeys } from '@/lib/query-keys';
import { cn } from '@/lib/utils';
import type { CrmDealWithContact } from '@/types/crm';

interface OverviewContactSectionProps {
  deal: CrmDealWithContact;
  orgId?: string;
}

export function OverviewContactSection({ deal, orgId }: OverviewContactSectionProps) {
  const navigate = useNavigate();
  const effectiveOrgId = orgId || deal.organization_id;
  const taskTotal = deal.task_total ?? 0;
  const taskDone = deal.task_done ?? 0;
  const taskPct = taskTotal > 0 ? Math.round((taskDone / taskTotal) * 100) : 0;

  const { data: orgData } = useQuery({
    queryKey: organizationKeys.orgData(effectiveOrgId!),
    queryFn: () => organizationsApi.getById(effectiveOrgId!),
    enabled: !!effectiveOrgId,
    staleTime: 5 * 60 * 1000,
  });

  return (
    <>
      {/* Linked Project */}
      {deal.project_name && (
        <div>
          <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">
            Linked Project
          </h4>
          <Card className="bg-muted/30 border-border/60">
            <CardContent className="p-3 space-y-2.5">
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2 min-w-0">
                  <FolderKanban className="h-4 w-4 text-primary shrink-0" />
                  <span className="text-sm font-medium truncate">
                    {deal.project_name}
                  </span>
                </div>
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-7 text-xs shrink-0 gap-1"
                  onClick={() => navigate(`/projects/${deal.project_id}/tasks`)}
                >
                  Open <ExternalLink className="h-3 w-3" />
                </Button>
              </div>
              {taskTotal > 0 && (
                <div className="space-y-1.5">
                  <div className="flex items-center justify-between text-xs text-muted-foreground">
                    <span className="flex items-center gap-1">
                      <ListTodo className="h-3 w-3" />
                      {taskDone}/{taskTotal} tasks done
                    </span>
                    <span
                      className={cn(
                        'font-medium',
                        taskPct === 100
                          ? 'text-green-600'
                          : 'text-muted-foreground'
                      )}
                    >
                      {taskPct}%
                    </span>
                  </div>
                  <div className="h-2 bg-muted rounded-full overflow-hidden">
                    <div
                      className={cn(
                        'h-full rounded-full transition-all',
                        taskPct === 100 ? 'bg-green-500' : 'bg-blue-500'
                      )}
                      style={{ width: `${taskPct}%` }}
                    />
                  </div>
                  {(deal.deliverable_count ?? 0) > 0 && (
                    <p className="text-[11px] text-muted-foreground flex items-center gap-1">
                      <FileText className="h-3 w-3" />
                      {deal.deliverable_count} deliverable
                      {(deal.deliverable_count ?? 0) !== 1 ? 's' : ''}
                    </p>
                  )}
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      )}

      {/* Contact */}
      <div>
        <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">
          Contact
        </h4>
        <Card className="bg-muted/30 border-border/60">
          <CardContent className="p-3 space-y-1.5">
            {deal.contact_name && (
              <div className="flex items-center gap-2 text-sm">
                <User className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                <span className="font-medium">{deal.contact_name}</span>
                {deal.person_id && (
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-5 text-[10px] px-1 ml-auto gap-0.5"
                    asChild
                  >
                    <Link to={`/people/${deal.person_id}`}>
                      View profile <ExternalLink className="h-2.5 w-2.5" />
                    </Link>
                  </Button>
                )}
              </div>
            )}
            {deal.contact_email && (
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <Mail className="h-3.5 w-3.5 shrink-0" />
                <span className="truncate">{deal.contact_email}</span>
              </div>
            )}
            {deal.contact_company && (
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <Building2 className="h-3.5 w-3.5 shrink-0" />
                <span className="truncate">{deal.contact_company}</span>
                {deal.company_id && (
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-5 text-[10px] px-1 ml-auto gap-0.5"
                    asChild
                  >
                    <Link to={`/companies/${deal.company_id}`}>
                      Co. Profile <ExternalLink className="h-2.5 w-2.5" />
                    </Link>
                  </Button>
                )}
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      {/* Organization */}
      {effectiveOrgId && (
        <div>
          <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">
            Organization
          </h4>
          <Card className="bg-muted/30 border-border/60">
            <CardContent className="p-3 space-y-2">
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2 min-w-0">
                  <Users className="h-3.5 w-3.5 text-primary shrink-0" />
                  <span className="text-sm font-medium truncate">
                    {orgData?.name ?? 'Loading...'}
                  </span>
                </div>
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-6 text-[10px] px-1.5 shrink-0 gap-0.5"
                  asChild
                >
                  <Link to={`/organizations/${effectiveOrgId}`}>
                    Open <ExternalLink className="h-2.5 w-2.5" />
                  </Link>
                </Button>
              </div>
              {deal.contact_company && (
                <div className="flex items-center gap-2 text-xs text-muted-foreground">
                  <Building2 className="h-3 w-3 shrink-0" />
                  {deal.company_id ? (
                    <Link
                      to={`/companies/${deal.company_id}`}
                      className="truncate hover:text-primary transition-colors"
                    >
                      {deal.contact_company}
                    </Link>
                  ) : (
                    <span className="truncate">{deal.contact_company}</span>
                  )}
                  {deal.company_id && (
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-5 text-[10px] px-1 ml-auto gap-0.5 shrink-0"
                      asChild
                    >
                      <Link to={`/companies/${deal.company_id}`}>
                        Profile <ExternalLink className="h-2.5 w-2.5" />
                      </Link>
                    </Button>
                  )}
                </div>
              )}
              {deal.project_id && (
                <div className="flex items-center gap-2 text-xs text-muted-foreground">
                  <FolderKanban className="h-3 w-3 shrink-0" />
                  <span className="truncate">
                    {deal.project_name ?? 'Linked project'}
                  </span>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-5 text-[10px] px-1 ml-auto gap-0.5 shrink-0"
                    asChild
                  >
                    <Link to={`/organizations/${effectiveOrgId}`}>
                      CRM <ExternalLink className="h-2.5 w-2.5" />
                    </Link>
                  </Button>
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      )}
    </>
  );
}
