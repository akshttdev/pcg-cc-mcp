import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetDescription,
} from '@/components/ui/sheet';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import {
  Calendar,
  Mail,
  Building2,
  Edit,
  Trash2,
  ExternalLink,
  FolderKanban,
  Tag,
  BarChart3,
  Rocket,
  ListTodo,
  FileText,
  Workflow,
} from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import { useDealClient } from '@/hooks/useCrmPipeline';
import { CrmActivityTimeline } from './CrmActivityTimeline';
import { DealConvertDialog } from './DealConvertDialog';
import type { CrmDealWithContact } from '@/types/crm';
import type { Project } from 'shared/types';

interface CrmDealDetailPanelProps {
  deal: CrmDealWithContact | null;
  isOpen: boolean;
  onClose: () => void;
  onEdit: (deal: CrmDealWithContact) => void;
  onDelete: (deal: CrmDealWithContact) => void;
  orgId?: string;
  projectId?: string;
}

export function CrmDealDetailPanel({
  deal,
  isOpen,
  onClose,
  onEdit,
  onDelete,
  orgId,
  projectId,
}: CrmDealDetailPanelProps) {
  const [activeTab, setActiveTab] = useState('details');
  const [convertOpen, setConvertOpen] = useState(false);

  if (!deal) return null;

  return (
    <>
      <Sheet open={isOpen} onOpenChange={(open) => !open && onClose()}>
        <SheetContent className="w-full sm:max-w-lg overflow-hidden flex flex-col p-0">
          <PanelContent
            deal={deal}
            activeTab={activeTab}
            onTabChange={setActiveTab}
            onEdit={onEdit}
            onDelete={onDelete}
            onConvert={() => setConvertOpen(true)}
            orgId={orgId}
            projectId={projectId}
          />
        </SheetContent>
      </Sheet>

      <DealConvertDialog
        deal={deal}
        open={convertOpen}
        onOpenChange={setConvertOpen}
        orgId={orgId}
      />
    </>
  );
}

function PanelContent({
  deal,
  activeTab,
  onTabChange,
  onEdit,
  onDelete,
  onConvert,
  orgId,
  projectId,
}: {
  deal: CrmDealWithContact;
  activeTab: string;
  onTabChange: (tab: string) => void;
  onEdit: (deal: CrmDealWithContact) => void;
  onDelete: (deal: CrmDealWithContact) => void;
  onConvert: () => void;
  orgId?: string;
  projectId?: string;
}) {
  const { client, projects, isLoading: isProjectsLoading } = useDealClient(orgId, deal);

  const initials = deal.contact_name
    ? deal.contact_name
        .split(' ')
        .map((n) => n[0])
        .join('')
        .toUpperCase()
        .slice(0, 2)
    : deal.name.slice(0, 2).toUpperCase();

  const formatAmount = (amount: number | undefined | null) => {
    if (!amount) return null;
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: deal.currency || 'USD',
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    }).format(amount);
  };

  const formattedAmount = formatAmount(deal.amount);

  return (
    <>
      {/* Header */}
      <SheetHeader className="p-6 pb-4 border-b">
        <div className="flex items-center gap-3">
          <Avatar className="h-10 w-10 shrink-0">
            {deal.contact_avatar_url && (
              <AvatarImage src={deal.contact_avatar_url} alt={deal.contact_name || deal.name} />
            )}
            <AvatarFallback className="text-sm bg-primary/10 text-primary">
              {initials}
            </AvatarFallback>
          </Avatar>
          <div className="min-w-0 flex-1">
            <SheetTitle className="text-base truncate">{deal.name}</SheetTitle>
            <SheetDescription className="text-sm truncate">
              {[deal.contact_name, deal.contact_company].filter(Boolean).join(' \u00B7 ')}
            </SheetDescription>
          </div>
        </div>
      </SheetHeader>

      {/* Summary stats */}
      <div className="grid grid-cols-4 gap-2 px-6 py-4 border-b">
        <div className="text-center">
          <div className="text-xs text-muted-foreground mb-1">Amount</div>
          <div className="text-sm font-semibold text-green-600">
            {formattedAmount || '\u2014'}
          </div>
        </div>
        <div className="text-center">
          <div className="text-xs text-muted-foreground mb-1">Stage</div>
          <Badge variant="outline" className="text-[10px] px-1">
            {deal.stage}
          </Badge>
        </div>
        <div className="text-center">
          <div className="text-xs text-muted-foreground mb-1">Tasks</div>
          <div className="text-sm font-semibold">
            {(deal.task_total ?? 0) > 0 ? `${deal.task_done ?? 0}/${deal.task_total}` : '\u2014'}
          </div>
        </div>
        <div className="text-center">
          <div className="text-xs text-muted-foreground mb-1">Deliverables</div>
          <div className="text-sm font-semibold">
            {(deal.deliverable_count ?? 0) > 0 ? deal.deliverable_count : '\u2014'}
          </div>
        </div>
      </div>

      {/* Tabs */}
      <Tabs value={activeTab} onValueChange={onTabChange} className="flex-1 flex flex-col min-h-0">
        <TabsList className="mx-6 mt-4">
          <TabsTrigger value="details">Details</TabsTrigger>
          <TabsTrigger value="projects">Projects</TabsTrigger>
          <TabsTrigger value="activity">Activity</TabsTrigger>
        </TabsList>

        <TabsContent value="details" className="flex-1 mt-0 min-h-0">
          <ScrollArea className="h-[calc(100vh-340px)]">
            <DetailsTab deal={deal} onEdit={onEdit} onDelete={onDelete} onConvert={onConvert} orgId={orgId} />
          </ScrollArea>
        </TabsContent>

        <TabsContent value="projects" className="flex-1 mt-0 min-h-0">
          <ScrollArea className="h-[calc(100vh-340px)]">
            <ProjectsTab
              client={client}
              projects={projects}
              isLoading={isProjectsLoading}
              deal={deal}
            />
          </ScrollArea>
        </TabsContent>

        <TabsContent value="activity" className="flex-1 mt-0 min-h-0">
          <ScrollArea className="h-[calc(100vh-340px)]">
            <ActivityTab deal={deal} projectId={projectId} />
          </ScrollArea>
        </TabsContent>
      </Tabs>
    </>
  );
}

// ── Details Tab ──

function DetailsTab({
  deal,
  onEdit,
  onDelete,
  onConvert,
  orgId,
}: {
  deal: CrmDealWithContact;
  onEdit: (deal: CrmDealWithContact) => void;
  onDelete: (deal: CrmDealWithContact) => void;
  onConvert: () => void;
  orgId?: string;
}) {
  const navigate = useNavigate();

  let tags: string[] = [];
  if (deal.tags) {
    try { tags = JSON.parse(deal.tags); } catch { tags = deal.tags.split(',').map(t => t.trim()).filter(Boolean); }
  }

  // Parse source provenance from custom_fields
  let sourceInfo: { dataSourceId?: string; workflowRunId?: string } | null = null;
  if (deal.custom_fields) {
    try {
      const cf = typeof deal.custom_fields === 'string' ? JSON.parse(deal.custom_fields) : deal.custom_fields;
      if (cf.source_data_source_id || cf.source_workflow_run_id) {
        sourceInfo = {
          dataSourceId: cf.source_data_source_id,
          workflowRunId: cf.source_workflow_run_id,
        };
      }
    } catch { /* ignore parse errors */ }
  }

  const taskTotal = deal.task_total ?? 0;
  const taskDone = deal.task_done ?? 0;
  const taskPct = taskTotal > 0 ? Math.round((taskDone / taskTotal) * 100) : 0;

  return (
    <div className="p-6 space-y-5">
      {/* Description */}
      {deal.description && (
        <div>
          <h4 className="text-xs font-medium text-muted-foreground mb-1.5">Description</h4>
          <p className="text-sm whitespace-pre-wrap">{deal.description}</p>
        </div>
      )}

      {/* Linked Project */}
      {deal.project_name && (
        <div>
          <h4 className="text-xs font-medium text-muted-foreground mb-2">Linked Project</h4>
          <Card className="bg-muted/30">
            <CardContent className="p-3 space-y-2">
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2 min-w-0">
                  <FolderKanban className="h-4 w-4 text-primary shrink-0" />
                  <span className="text-sm font-medium truncate">{deal.project_name}</span>
                </div>
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-7 text-xs shrink-0"
                  onClick={() => navigate(`/projects/${deal.project_id}/tasks`)}
                >
                  Open
                  <ExternalLink className="h-3 w-3 ml-1" />
                </Button>
              </div>
              {taskTotal > 0 && (
                <div className="space-y-1">
                  <div className="flex items-center justify-between text-xs text-muted-foreground">
                    <span className="flex items-center gap-1">
                      <ListTodo className="h-3 w-3" />
                      Tasks: {taskDone}/{taskTotal} done
                    </span>
                    {(deal.deliverable_count ?? 0) > 0 && (
                      <span className="flex items-center gap-1">
                        <FileText className="h-3 w-3" />
                        {deal.deliverable_count} deliverable{(deal.deliverable_count ?? 0) !== 1 ? 's' : ''}
                      </span>
                    )}
                  </div>
                  <div className="h-2 bg-muted rounded-full overflow-hidden">
                    <div
                      className={`h-full rounded-full transition-all ${taskPct === 100 ? 'bg-green-500' : 'bg-blue-500'}`}
                      style={{ width: `${taskPct}%` }}
                    />
                  </div>
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      )}

      {/* Probability */}
      {deal.probability > 0 && (
        <div>
          <h4 className="text-xs font-medium text-muted-foreground mb-1.5">Probability</h4>
          <div className="flex items-center gap-3">
            <div className="flex-1 h-2 bg-muted rounded-full overflow-hidden">
              <div
                className="h-full bg-primary rounded-full transition-all"
                style={{ width: `${deal.probability}%` }}
              />
            </div>
            <span className="text-sm font-medium">{deal.probability}%</span>
          </div>
        </div>
      )}

      {/* Currency */}
      <div>
        <h4 className="text-xs font-medium text-muted-foreground mb-1.5">Currency</h4>
        <span className="text-sm">{deal.currency || 'USD'}</span>
      </div>

      {/* Tags */}
      {tags.length > 0 && (
        <div>
          <h4 className="text-xs font-medium text-muted-foreground mb-1.5">Tags</h4>
          <div className="flex flex-wrap gap-1.5">
            {tags.map((tag) => (
              <Badge key={tag} variant="secondary" className="text-xs">
                <Tag className="h-3 w-3 mr-1" />
                {tag}
              </Badge>
            ))}
          </div>
        </div>
      )}

      {/* Contact info */}
      <div>
        <h4 className="text-xs font-medium text-muted-foreground mb-2">Contact</h4>
        <div className="space-y-2">
          {deal.contact_email && (
            <div className="flex items-center gap-2 text-sm">
              <Mail className="h-3.5 w-3.5 text-muted-foreground" />
              <span>{deal.contact_email}</span>
            </div>
          )}
          {deal.contact_company && (
            <div className="flex items-center gap-2 text-sm">
              <Building2 className="h-3.5 w-3.5 text-muted-foreground" />
              <span>{deal.contact_company}</span>
            </div>
          )}
        </div>
      </div>

      {/* Dates */}
      <div>
        <h4 className="text-xs font-medium text-muted-foreground mb-2">Timeline</h4>
        <div className="space-y-2 text-sm">
          <div className="flex items-center gap-2">
            <Calendar className="h-3.5 w-3.5 text-muted-foreground" />
            <span className="text-muted-foreground">Created:</span>
            <span>{new Date(deal.created_at).toLocaleDateString()}</span>
          </div>
          {deal.last_activity_at && (
            <div className="flex items-center gap-2">
              <BarChart3 className="h-3.5 w-3.5 text-muted-foreground" />
              <span className="text-muted-foreground">Last activity:</span>
              <span>
                {formatDistanceToNow(new Date(deal.last_activity_at), { addSuffix: true })}
              </span>
            </div>
          )}
          {deal.actual_close_date && (
            <div className="flex items-center gap-2">
              <Calendar className="h-3.5 w-3.5 text-muted-foreground" />
              <span className="text-muted-foreground">Closed:</span>
              <span>{new Date(deal.actual_close_date).toLocaleDateString()}</span>
            </div>
          )}
        </div>
      </div>

      {/* Win/Lost reason */}
      {deal.win_reason && (
        <div>
          <h4 className="text-xs font-medium text-muted-foreground mb-1.5">Win Reason</h4>
          <p className="text-sm">{deal.win_reason}</p>
        </div>
      )}
      {deal.lost_reason && (
        <div>
          <h4 className="text-xs font-medium text-muted-foreground mb-1.5">Lost Reason</h4>
          <p className="text-sm">{deal.lost_reason}</p>
        </div>
      )}

      {/* Source provenance */}
      {sourceInfo && (
        <div>
          <h4 className="text-xs font-medium text-muted-foreground mb-1.5">Source</h4>
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <Workflow className="h-3.5 w-3.5 shrink-0" />
            {orgId ? (
              <button
                type="button"
                className="text-primary hover:underline text-left"
                onClick={() => navigate(`/organizations/${orgId}/intelligence`)}
              >
                Imported via workflow
              </button>
            ) : (
              <span>Imported via workflow</span>
            )}
          </div>
        </div>
      )}

      {/* Quick actions */}
      <div className="space-y-2 pt-2 border-t">
        <Button size="sm" className="w-full" onClick={onConvert}>
          <Rocket className="h-3.5 w-3.5 mr-1.5" />
          Convert to Project
        </Button>
        <div className="flex gap-2">
          <Button variant="outline" size="sm" onClick={() => onEdit(deal)}>
            <Edit className="h-3.5 w-3.5 mr-1.5" />
            Edit Deal
          </Button>
          <Button
            variant="outline"
            size="sm"
            className="text-destructive hover:text-destructive"
            onClick={() => onDelete(deal)}
          >
            <Trash2 className="h-3.5 w-3.5 mr-1.5" />
            Delete
          </Button>
        </div>
      </div>
    </div>
  );
}

// ── Projects Tab ──

function ProjectsTab({
  client,
  projects,
  isLoading,
  deal,
}: {
  client: { id: string; name: string } | null;
  projects: Project[];
  isLoading: boolean;
  deal: CrmDealWithContact;
}) {
  const navigate = useNavigate();

  if (!deal.contact_company) {
    return (
      <div className="p-6 text-center text-sm text-muted-foreground">
        <FolderKanban className="h-8 w-8 mx-auto mb-2 opacity-50" />
        <p>No company associated with this deal.</p>
        <p className="mt-1 text-xs">Link a contact with a company to see related projects.</p>
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="p-6 space-y-3">
        <Skeleton className="h-4 w-32" />
        <Skeleton className="h-20 rounded-lg" />
        <Skeleton className="h-20 rounded-lg" />
      </div>
    );
  }

  if (!client) {
    return (
      <div className="p-6 text-center text-sm text-muted-foreground">
        <FolderKanban className="h-8 w-8 mx-auto mb-2 opacity-50" />
        <p>No client found matching &ldquo;{deal.contact_company}&rdquo;.</p>
        <p className="mt-1 text-xs">Create a client in your organization settings to link projects.</p>
      </div>
    );
  }

  if (projects.length === 0) {
    return (
      <div className="p-6 text-center text-sm text-muted-foreground">
        <FolderKanban className="h-8 w-8 mx-auto mb-2 opacity-50" />
        <p>No projects for {client.name} yet.</p>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-3">
      <div className="flex items-center gap-2 text-xs text-muted-foreground">
        <Building2 className="h-3.5 w-3.5" />
        <span className="font-medium">{client.name}</span>
        <Badge variant="secondary" className="text-[10px]">
          {projects.length} project{projects.length !== 1 ? 's' : ''}
        </Badge>
      </div>

      <div className="space-y-2">
        {projects.map((project) => (
          <Card key={project.id} className="hover:shadow-sm transition-shadow">
            <CardContent className="p-3">
              <div className="flex items-center justify-between gap-2">
                <div className="min-w-0 flex-1">
                  <p className="text-sm font-medium truncate">{project.name}</p>
                </div>
                <Button
                  variant="ghost"
                  size="sm"
                  className="shrink-0 h-7 text-xs"
                  onClick={() => navigate(`/projects/${project.id}/tasks`)}
                >
                  View Project
                  <ExternalLink className="h-3 w-3 ml-1" />
                </Button>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}

// ── Activity Tab ──

function ActivityTab({
  deal,
  projectId,
}: {
  deal: CrmDealWithContact;
  projectId?: string;
}) {
  const resolvedProjectId = projectId || deal.organization_id;

  return (
    <div className="p-6">
      <CrmActivityTimeline
        projectId={resolvedProjectId}
        dealId={deal.id}
        limit={30}
      />
    </div>
  );
}
