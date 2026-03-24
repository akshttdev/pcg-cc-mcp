import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Rocket,
  TrendingUp,
  CheckCircle2,
  Users,
  Bot,
  Eye,
  Loader2,
  ArrowRight,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';
import { useWorkflowTemplates, useConvertDeal } from '@/hooks/useWorkflowTemplates';
import type { CrmDealWithContact } from '@/types/crm';
import type { WorkflowTemplate } from '@/lib/api';

interface DealConvertDialogProps {
  deal: CrmDealWithContact;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  orgId?: string;
}

export function DealConvertDialog({
  deal,
  open,
  onOpenChange,
  orgId,
}: DealConvertDialogProps) {
  const navigate = useNavigate();
  const { data: templates = [] } = useWorkflowTemplates();
  const convertDeal = useConvertDeal();
  const [selectedTemplate, setSelectedTemplate] = useState<string | null>(null);
  const [projectName, setProjectName] = useState(
    `${deal.name}${deal.contact_company ? ` — ${deal.contact_company}` : ''}`
  );

  const template = templates.find((t) => t.id === selectedTemplate);

  const handleConvert = async () => {
    if (!selectedTemplate) return;

    try {
      const result = await convertDeal.mutateAsync({
        dealId: deal.id,
        data: {
          template_id: selectedTemplate,
          project_name: projectName || undefined,
          organization_id: orgId,
        },
      });

      toast.success(
        `Project created with ${result.tasks_created} tasks across ${result.boards_created} boards.`
      );
      onOpenChange(false);
      navigate(`/projects/${result.project_id}/tasks`);
    } catch {
      toast.error('Failed to convert deal to project.');
    }
  };

  const totalTasks = template?.phases.reduce((sum, p) => sum + p.tasks.length, 0) ?? 0;
  const agentTasks =
    template?.phases
      .flatMap((p) => p.tasks)
      .filter((t) => t.task_type === 'agent' || t.task_type === 'hybrid').length ?? 0;
  const reviewGates =
    template?.phases.flatMap((p) => p.tasks).filter((t) => t.requires_approval).length ?? 0;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>Convert Deal to Project</DialogTitle>
          <DialogDescription>
            Select a workflow template to scaffold a project with boards, tasks, agent assignments,
            and review gates.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-2">
          {/* Project name */}
          <div className="space-y-2">
            <Label htmlFor="project-name">Project Name</Label>
            <Input
              id="project-name"
              value={projectName}
              onChange={(e) => setProjectName(e.target.value)}
              placeholder="Project name..."
            />
          </div>

          {/* Template selection */}
          <div className="space-y-2">
            <Label>Workflow Template</Label>
            <div className="grid gap-3">
              {templates.map((tmpl) => (
                <TemplateCard
                  key={tmpl.id}
                  template={tmpl}
                  selected={selectedTemplate === tmpl.id}
                  onSelect={() => setSelectedTemplate(tmpl.id)}
                />
              ))}
            </div>
          </div>

          {/* Template preview */}
          {template && (
            <div className="space-y-3 pt-2 border-t">
              <div className="flex items-center gap-4 text-sm">
                <span className="flex items-center gap-1.5 text-muted-foreground">
                  <CheckCircle2 className="h-3.5 w-3.5" />
                  {totalTasks} tasks
                </span>
                <span className="flex items-center gap-1.5 text-muted-foreground">
                  <Bot className="h-3.5 w-3.5" />
                  {agentTasks} agent-executed
                </span>
                <span className="flex items-center gap-1.5 text-muted-foreground">
                  <Eye className="h-3.5 w-3.5" />
                  {reviewGates} review gates
                </span>
              </div>

              <ScrollArea className="h-48">
                <div className="space-y-3 pr-4">
                  {template.phases.map((phase) => (
                    <div key={phase.position}>
                      <div className="flex items-center gap-2 mb-1.5">
                        <div className="h-2 w-2 rounded-full bg-primary" />
                        <span className="text-sm font-medium">{phase.name}</span>
                        {phase.is_recurring && (
                          <Badge variant="outline" className="text-xs px-1.5 py-0">
                            recurring
                          </Badge>
                        )}
                      </div>
                      <div className="ml-3 border-l pl-3 space-y-1">
                        {phase.tasks.map((task) => (
                          <div
                            key={task.position}
                            className="flex items-center gap-2 text-xs text-muted-foreground"
                          >
                            {task.task_type === 'human_review' ? (
                              <Users className="h-3 w-3 text-amber-500 shrink-0" />
                            ) : (
                              <Bot className="h-3 w-3 text-blue-500 shrink-0" />
                            )}
                            <span className="truncate">{task.title}</span>
                            {task.requires_approval && (
                              <Badge
                                variant="outline"
                                className="text-[9px] px-1 py-0 shrink-0 text-amber-600 border-amber-300"
                              >
                                gate
                              </Badge>
                            )}
                            {task.agent_role && (
                              <Badge
                                variant="secondary"
                                className="text-[9px] px-1 py-0 shrink-0"
                              >
                                {task.agent_role}
                              </Badge>
                            )}
                          </div>
                        ))}
                      </div>
                    </div>
                  ))}
                </div>
              </ScrollArea>
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button
            onClick={handleConvert}
            disabled={!selectedTemplate || !projectName.trim() || convertDeal.isPending}
          >
            {convertDeal.isPending ? (
              <>
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                Converting...
              </>
            ) : (
              <>
                Convert to Project
                <ArrowRight className="h-4 w-4 ml-2" />
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function TemplateCard({
  template,
  selected,
  onSelect,
}: {
  template: WorkflowTemplate;
  selected: boolean;
  onSelect: () => void;
}) {
  const Icon = template.client_type === 'foundation_build' ? Rocket : TrendingUp;
  const phaseCount = template.phases.length;
  const taskCount = template.phases.reduce((sum, p) => sum + p.tasks.length, 0);

  return (
    <Card
      className={cn(
        'cursor-pointer transition-all hover:shadow-sm',
        selected && 'ring-2 ring-primary shadow-sm'
      )}
      onClick={onSelect}
    >
      <CardContent className="p-3">
        <div className="flex items-start gap-3">
          <div
            className={cn(
              'p-2 rounded-lg shrink-0',
              template.client_type === 'foundation_build'
                ? 'bg-blue-100 text-blue-600'
                : 'bg-green-100 text-green-600'
            )}
          >
            <Icon className="h-4 w-4" />
          </div>
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2">
              <p className="text-sm font-medium">{template.name}</p>
              {template.is_recurring && (
                <Badge variant="outline" className="text-xs px-1.5 py-0">
                  retainer
                </Badge>
              )}
            </div>
            <p className="text-xs text-muted-foreground mt-0.5 line-clamp-2">
              {template.description}
            </p>
            <div className="flex items-center gap-3 mt-1.5 text-xs text-muted-foreground">
              <span>{phaseCount} phases</span>
              <span>{taskCount} tasks</span>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
