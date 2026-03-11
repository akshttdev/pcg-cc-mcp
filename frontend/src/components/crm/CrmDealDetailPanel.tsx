import { useState } from 'react';
import { useNavigate, Link } from 'react-router-dom';
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
  Brain,
  CheckCircle2,
  AlertCircle,
  Loader2,
  RotateCcw,
  ThumbsUp,
  Clock,
} from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import { useDealClient } from '@/hooks/useCrmPipeline';
import { CrmActivityTimeline } from './CrmActivityTimeline';
import { DealConvertDialog } from './DealConvertDialog';
import { reportsApi } from '@/lib/api';
import { toast } from 'sonner';
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
          <TabsTrigger value="intel">
            Intel
            {deal.intelligence_status === 'done' && (
              <span className="ml-1 w-1.5 h-1.5 rounded-full bg-green-500 inline-block" />
            )}
          </TabsTrigger>
          <TabsTrigger value="projects">Projects</TabsTrigger>
          <TabsTrigger value="activity">Activity</TabsTrigger>
        </TabsList>

        <TabsContent value="details" className="flex-1 mt-0 min-h-0">
          <ScrollArea className="h-[calc(100vh-360px)]">
            <DetailsTab deal={deal} onEdit={onEdit} onDelete={onDelete} onConvert={onConvert} orgId={orgId} />
          </ScrollArea>
        </TabsContent>

        <TabsContent value="intel" className="flex-1 mt-0 min-h-0">
          <ScrollArea className="h-[calc(100vh-360px)]">
            <IntelTab deal={deal} />
          </ScrollArea>
        </TabsContent>

        <TabsContent value="projects" className="flex-1 mt-0 min-h-0">
          <ScrollArea className="h-[calc(100vh-360px)]">
            <ProjectsTab
              client={client}
              projects={projects}
              isLoading={isProjectsLoading}
              deal={deal}
            />
          </ScrollArea>
        </TabsContent>

        <TabsContent value="activity" className="flex-1 mt-0 min-h-0">
          <ScrollArea className="h-[calc(100vh-360px)]">
            <ActivityTab deal={deal} projectId={projectId} />
          </ScrollArea>
        </TabsContent>
      </Tabs>
    </>
  );
}

// ── Intel Tab ──

function IntelTab({ deal }: { deal: CrmDealWithContact }) {
  const [revisionNotes, setRevisionNotes] = useState('');
  const [showRevisionInput, setShowRevisionInput] = useState(false);
  const [loading, setLoading] = useState(false);

  const status = deal.intelligence_status;
  const isResearching = status === 'running' || status === 'queued';
  const isDone = status === 'done';
  const confidencePct = isDone && deal.intelligence_confidence != null
    ? Math.round(deal.intelligence_confidence * 100)
    : null;

  const handleApprove = async () => {
    if (!deal.report_id) return;
    setLoading(true);
    try {
      await reportsApi.approve(deal.report_id);
      toast.success('Report approved');
    } catch {
      toast.error('Failed to approve report');
    } finally {
      setLoading(false);
    }
  };

  const handleRequestRevision = async () => {
    if (!deal.report_id) return;
    setLoading(true);
    try {
      await reportsApi.requestRevision(deal.report_id, revisionNotes);
      toast.success('Revision requested');
      setShowRevisionInput(false);
      setRevisionNotes('');
    } catch {
      toast.error('Failed to request revision');
    } finally {
      setLoading(false);
    }
  };

  if (isResearching) {
    return (
      <div className="p-6 flex flex-col items-center justify-center gap-3 text-center">
        <Loader2 className="h-8 w-8 text-blue-500 animate-spin" />
        <p className="text-sm font-medium">Research in progress...</p>
        <p className="text-xs text-muted-foreground">
          Nora is gathering intelligence on this lead. Check back shortly.
        </p>
      </div>
    );
  }

  if (!isDone && !deal.intelligence_summary) {
    return (
      <div className="p-6 flex flex-col items-center justify-center gap-3 text-center">
        <Brain className="h-8 w-8 text-muted-foreground/40" />
        <p className="text-sm text-muted-foreground">No intelligence data yet.</p>
        {deal.person_id && (
          <Button variant="outline" size="sm" asChild>
            <Link to={`/people/${deal.person_id}`}>
              <Brain className="h-3.5 w-3.5 mr-1.5" />
              Research this lead
            </Link>
          </Button>
        )}
      </div>
    );
  }

  return (
    <div className="p-6 space-y-5">
      {/* Intelligence status header */}
      <div className="flex items-center gap-2">
        {isDone ? (
          <CheckCircle2 className="h-4 w-4 text-green-500 shrink-0" />
        ) : (
          <AlertCircle className="h-4 w-4 text-amber-400 shrink-0" />
        )}
        <span className="text-sm font-medium">
          {isDone ? 'Intelligence Complete' : 'Partial Intelligence'}
        </span>
        {confidencePct != null && (
          <Badge variant="outline" className="text-xs ml-auto">
            {confidencePct}% confidence
          </Badge>
        )}
      </div>

      {/* Research passes */}
      {(deal.research_pass_count ?? 0) > 0 && (
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <RotateCcw className="h-3.5 w-3.5" />
          <span>{deal.research_pass_count} research pass{(deal.research_pass_count ?? 0) !== 1 ? 'es' : ''} completed</span>
        </div>
      )}

      {/* Intelligence summary */}
      {deal.intelligence_summary && (
        <div>
          <h4 className="text-xs font-medium text-muted-foreground mb-2">AI Profile Summary</h4>
          <p className="text-sm leading-relaxed whitespace-pre-wrap">{deal.intelligence_summary}</p>
        </div>
      )}

      {/* Business report section */}
      {deal.report_id && (
        <div>
          <div className="flex items-center justify-between mb-3">
            <h4 className="text-xs font-medium text-muted-foreground">Business Report</h4>
            <Button variant="ghost" size="sm" className="h-7 text-xs" asChild>
              <Link to={`/business-reports/${deal.report_id}`}>
                Open Report
                <ExternalLink className="h-3 w-3 ml-1" />
              </Link>
            </Button>
          </div>

          {/* Review status */}
          {deal.report_review_status && (
            <Card className="bg-muted/30">
              <CardContent className="p-3 space-y-3">
                <div className="flex items-center gap-2">
                  {deal.report_review_status === 'approved' ? (
                    <CheckCircle2 className="h-4 w-4 text-green-500" />
                  ) : deal.report_review_status === 'rejected' ? (
                    <AlertCircle className="h-4 w-4 text-red-400" />
                  ) : (
                    <Clock className="h-4 w-4 text-amber-400" />
                  )}
                  <span className="text-sm font-medium">
                    {deal.report_review_status === 'approved' ? 'Report Approved'
                      : deal.report_review_status === 'rejected' ? 'Revision Requested'
                      : 'Pending Review'}
                  </span>
                </div>

                {deal.report_review_status !== 'approved' && (
                  <div className="flex gap-2">
                    <Button
                      size="sm"
                      className="h-7 text-xs flex-1"
                      onClick={handleApprove}
                      disabled={loading}
                    >
                      <ThumbsUp className="h-3 w-3 mr-1.5" />
                      Approve
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      className="h-7 text-xs flex-1"
                      onClick={() => setShowRevisionInput(!showRevisionInput)}
                      disabled={loading}
                    >
                      <RotateCcw className="h-3 w-3 mr-1.5" />
                      Revise
                    </Button>
                  </div>
                )}

                {showRevisionInput && (
                  <div className="space-y-2">
                    <textarea
                      className="w-full text-xs border rounded p-2 min-h-[60px] resize-none focus:outline-none focus:ring-1 focus:ring-primary"
                      placeholder="Describe what needs to be revised..."
                      value={revisionNotes}
                      onChange={(e) => setRevisionNotes(e.target.value)}
                    />
                    <Button
                      size="sm"
                      variant="outline"
                      className="h-7 text-xs w-full"
                      onClick={handleRequestRevision}
                      disabled={loading || !revisionNotes.trim()}
                    >
                      Submit Revision Request
                    </Button>
                  </div>
                )}
              </CardContent>
            </Card>
          )}
        </div>
      )}

      {/* Link to full person profile */}
      {deal.person_id && (
        <div className="pt-2 border-t">
          <Button variant="ghost" size="sm" className="w-full h-8 text-xs" asChild>
            <Link to={`/people/${deal.person_id}`}>
              <Brain className="h-3.5 w-3.5 mr-1.5" />
              View Full Intelligence Profile
              <ExternalLink className="h-3 w-3 ml-1" />
            </Link>
          </Button>
        </div>
      )}
    </div>
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
