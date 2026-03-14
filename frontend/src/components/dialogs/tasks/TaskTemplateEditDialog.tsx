import { useState, useEffect } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Alert } from '@/components/ui/alert';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Loader2 } from 'lucide-react';
import { FormDialogBody } from '@/components/ui/form-dialog-body';
import { templatesApi, agentsApi } from '@/lib/api';
import type { AgentWithParsedFields } from 'shared/types';
import type {
  TaskTemplate,
  CreateTaskTemplate,
  UpdateTaskTemplate,
} from 'shared/types';
import NiceModal, { useModal } from '@ebay/nice-modal-react';

export interface TaskTemplateEditDialogProps {
  template?: TaskTemplate | null; // null for create mode
  projectId?: string;
  isGlobal?: boolean;
}

export type TaskTemplateEditResult = 'saved' | 'canceled';

export const TaskTemplateEditDialog =
  NiceModal.create<TaskTemplateEditDialogProps>(
    ({ template, projectId, isGlobal = false }) => {
      const modal = useModal();
      const [formData, setFormData] = useState({
        template_name: '',
        title: '',
        description: '',
        priority: '' as string,
        completion_criteria: '',
        output_format: '',
        assigned_agent: '',
        tags: '',
      });
      const [saving, setSaving] = useState(false);
      const [error, setError] = useState<string | null>(null);
      const [agents, setAgents] = useState<AgentWithParsedFields[]>([]);

      useEffect(() => {
        agentsApi.list().then(setAgents).catch(() => {});
      }, []);

      const isEditMode = Boolean(template);

      useEffect(() => {
        if (template) {
          setFormData({
            template_name: template.template_name,
            title: template.title,
            description: template.description || '',
            priority: template.priority || '',
            completion_criteria: template.completion_criteria || '',
            output_format: template.output_format || '',
            assigned_agent: template.assigned_agent || '',
            tags: template.tags || '',
          });
        } else {
          setFormData({
            template_name: '',
            title: '',
            description: '',
            priority: '',
            completion_criteria: '',
            output_format: '',
            assigned_agent: '',
            tags: '',
          });
        }
        setError(null);
      }, [template]);

      const handleSave = async () => {
        if (!formData.template_name.trim() || !formData.title.trim()) {
          setError('Template name and title are required');
          return;
        }

        setSaving(true);
        setError(null);

        try {
          if (isEditMode && template) {
            const updateData: UpdateTaskTemplate = {
              template_name: formData.template_name,
              title: formData.title,
              description: formData.description || null,
              priority: formData.priority || null,
              completion_criteria: formData.completion_criteria || null,
              output_format: formData.output_format || null,
              assigned_agent: formData.assigned_agent || null,
              tags: formData.tags ? formData.tags.split(',').map((t) => t.trim()).filter(Boolean) : null,
            };
            await templatesApi.update(template.id, updateData);
          } else {
            const createData: CreateTaskTemplate = {
              project_id: isGlobal ? null : projectId || null,
              template_name: formData.template_name,
              title: formData.title,
              description: formData.description || null,
              priority: formData.priority || null,
              completion_criteria: formData.completion_criteria || null,
              output_format: formData.output_format || null,
              assigned_agent: formData.assigned_agent || null,
              tags: formData.tags ? formData.tags.split(',').map((t) => t.trim()).filter(Boolean) : null,
              organization_id: null,
            };
            await templatesApi.create(createData);
          }

          modal.resolve('saved' as TaskTemplateEditResult);
          modal.hide();
        } catch (err: any) {
          setError(err.message || 'Failed to save template');
        } finally {
          setSaving(false);
        }
      };

      const handleCancel = () => {
        modal.resolve('canceled' as TaskTemplateEditResult);
        modal.hide();
      };

      const handleOpenChange = (open: boolean) => {
        if (!open) {
          handleCancel();
        }
      };

      return (
        <Dialog open={modal.visible} onOpenChange={handleOpenChange}>
          <DialogContent className="sm:max-w-[560px] max-h-[85vh] flex flex-col">
            <DialogHeader>
              <DialogTitle>
                {isEditMode ? 'Edit Template' : 'Create Template'}
              </DialogTitle>
            </DialogHeader>
            <FormDialogBody
              footer={
                <>
                  <Button
                    variant="outline"
                    onClick={handleCancel}
                    disabled={saving}
                  >
                    Cancel
                  </Button>
                  <Button onClick={handleSave} disabled={saving}>
                    {saving && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                    {isEditMode ? 'Update' : 'Create'}
                  </Button>
                </>
              }
            >
              <div className="space-y-4 py-4">
                <div>
                  <Label htmlFor="template-name">Template Name</Label>
                  <Input
                    id="template-name"
                    value={formData.template_name}
                    onChange={(e) =>
                      setFormData({ ...formData, template_name: e.target.value })
                    }
                    placeholder="e.g., Bug Fix, Feature Request"
                    disabled={saving}
                    autoFocus
                  />
                </div>
                <div>
                  <Label htmlFor="template-title">Default Title</Label>
                  <Input
                    id="template-title"
                    value={formData.title}
                    onChange={(e) =>
                      setFormData({ ...formData, title: e.target.value })
                    }
                    placeholder="e.g., Fix bug in..."
                    disabled={saving}
                  />
                </div>
                <div>
                  <Label htmlFor="template-description">
                    Default Description
                  </Label>
                  <Textarea
                    id="template-description"
                    value={formData.description}
                    onChange={(e) =>
                      setFormData({ ...formData, description: e.target.value })
                    }
                    placeholder="Enter a default description for tasks created with this template"
                    rows={3}
                    disabled={saving}
                  />
                </div>

                <div>
                  <Label htmlFor="template-priority">Priority</Label>
                  <Select
                    value={formData.priority}
                    onValueChange={(value) =>
                      setFormData({ ...formData, priority: value })
                    }
                    disabled={saving}
                  >
                    <SelectTrigger id="template-priority">
                      <SelectValue placeholder="Select priority" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="critical">Critical</SelectItem>
                      <SelectItem value="high">High</SelectItem>
                      <SelectItem value="medium">Medium</SelectItem>
                      <SelectItem value="low">Low</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div>
                  <Label htmlFor="template-completion-criteria">
                    Completion Criteria
                  </Label>
                  <Textarea
                    id="template-completion-criteria"
                    value={formData.completion_criteria}
                    onChange={(e) =>
                      setFormData({ ...formData, completion_criteria: e.target.value })
                    }
                    placeholder="What must be true for tasks using this template to be considered done?"
                    rows={3}
                    disabled={saving}
                  />
                  <p className="text-xs text-muted-foreground mt-1">
                    Structured success criteria for agents to self-evaluate completion
                  </p>
                </div>

                <div>
                  <Label htmlFor="template-output-format">Output Format</Label>
                  <Input
                    id="template-output-format"
                    value={formData.output_format}
                    onChange={(e) =>
                      setFormData({ ...formData, output_format: e.target.value })
                    }
                    placeholder="e.g., markdown report, code PR, JSON API response"
                    disabled={saving}
                  />
                  <p className="text-xs text-muted-foreground mt-1">
                    Expected deliverable format for agent output
                  </p>
                </div>

                <div>
                  <Label htmlFor="template-assigned-agent">
                    Assigned Agent
                  </Label>
                  <Select
                    value={formData.assigned_agent}
                    onValueChange={(value) =>
                      setFormData({ ...formData, assigned_agent: value === '__none__' ? '' : value })
                    }
                    disabled={saving}
                  >
                    <SelectTrigger id="template-assigned-agent">
                      <SelectValue placeholder="Select an agent" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="__none__">None</SelectItem>
                      {agents.map((agent) => (
                        <SelectItem key={agent.id} value={agent.short_name}>
                          <span>{agent.short_name}</span>
                          {agent.designation && (
                            <span className="text-muted-foreground ml-1">({agent.designation})</span>
                          )}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>

                <div>
                  <Label htmlFor="template-tags">Tags</Label>
                  <Input
                    id="template-tags"
                    value={formData.tags}
                    onChange={(e) =>
                      setFormData({ ...formData, tags: e.target.value })
                    }
                    placeholder="Comma-separated tags, e.g., frontend, bugfix, urgent"
                    disabled={saving}
                  />
                  <p className="text-xs text-muted-foreground mt-1">
                    Comma-separated list of tags to apply to created tasks
                  </p>
                </div>

                {error && <Alert variant="destructive">{error}</Alert>}
              </div>
            </FormDialogBody>
          </DialogContent>
        </Dialog>
      );
    }
  );
