import { useEffect, useState } from 'react';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { TaskTemplateManager } from '@/components/TaskTemplateManager';
import { ProjectFormFields } from '@/components/projects/project-form-fields';
import { CreateProject, Project, UpdateProject } from 'shared/types';
import { projectsApi, tasksApi } from '@/lib/api';
import { generateProjectNameFromPath } from '@/utils/string';
import NiceModal, { useModal } from '@ebay/nice-modal-react';
import {
  Globe,
  Smartphone,
  Server,
  Package,
  Terminal,
  Palette,
  BookOpen,
  Box,
  FolderOpen,
  Sparkles,
  type LucideIcon,
} from 'lucide-react';

export const PROJECT_TYPES: readonly { value: string; label: string; icon: LucideIcon }[] = [
  { value: 'web-app', label: 'Web Application', icon: Globe },
  { value: 'mobile-app', label: 'Mobile Application', icon: Smartphone },
  { value: 'api', label: 'API / Backend', icon: Server },
  { value: 'library', label: 'Library / Package', icon: Package },
  { value: 'cli', label: 'CLI Tool', icon: Terminal },
  { value: 'design', label: 'Design System', icon: Palette },
  { value: 'docs', label: 'Documentation', icon: BookOpen },
  { value: 'other', label: 'Other', icon: Box },
  { value: 'folder', label: 'Folder / Group', icon: FolderOpen },
] as const;

export type ProjectType = (typeof PROJECT_TYPES)[number]['value'];

export const PROJECT_TEMPLATES = [
  {
    value: 'none',
    label: 'No Template',
    description: 'Start with an empty project',
  },
  {
    value: 'software',
    label: 'Software Development',
    description: '5 tasks: setup, implementation, testing, review, deployment',
  },
  {
    value: 'research',
    label: 'Research & Intelligence',
    description: '5 tasks: define scope, gather sources, analyze, synthesize, present',
  },
  {
    value: 'marketing',
    label: 'Marketing Campaign',
    description: '5 tasks: audience research, strategy, content creation, launch, metrics',
  },
  {
    value: 'client_onboarding',
    label: 'Client Onboarding',
    description: '5 tasks: discovery, setup, training, go-live, follow-up',
  },
] as const;

export type ProjectTemplate = (typeof PROJECT_TEMPLATES)[number]['value'];

export interface ProjectFormDialogProps {
  project?: Project | null;
  organization_id?: string | null;
  client_id?: string | null;
  parent_project_id?: string | null;
}

export type ProjectFormDialogResult = 'saved' | 'canceled';

export const ProjectFormDialog = NiceModal.create<ProjectFormDialogProps>(
  ({ project, organization_id, client_id, parent_project_id }) => {
    const modal = useModal();
    const [name, setName] = useState(project?.name || '');
    const [gitRepoPath, setGitRepoPath] = useState(
      project?.git_repo_path || ''
    );
    const [setupScript, setSetupScript] = useState(project?.setup_script ?? '');
    const [devScript, setDevScript] = useState(project?.dev_script ?? '');
    const [cleanupScript, setCleanupScript] = useState(
      project?.cleanup_script ?? ''
    );
    const [copyFiles, setCopyFiles] = useState(project?.copy_files ?? '');
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState('');
    const [repoMode, setRepoMode] = useState<'existing' | 'new'>('existing');
    const [parentPath, setParentPath] = useState('');
    const [folderName, setFolderName] = useState('');
    const [projectType, setProjectType] = useState<ProjectType>('web-app');
    const [projectTemplate, setProjectTemplate] = useState<ProjectTemplate>('none');

    const isEditing = !!project;
    const isContainer = projectType === 'folder';

    // Update form fields when project prop changes
    useEffect(() => {
      if (project) {
        setName(project.name || '');
        setGitRepoPath(project.git_repo_path || '');
        setSetupScript(project.setup_script ?? '');
        setDevScript(project.dev_script ?? '');
        setCleanupScript(project.cleanup_script ?? '');
        setCopyFiles(project.copy_files ?? '');
      } else {
        setName('');
        setGitRepoPath('');
        setSetupScript('');
        setDevScript('');
        setCleanupScript('');
        setCopyFiles('');
      }
    }, [project]);

    // Auto-populate project name from directory name
    const handleGitRepoPathChange = (path: string) => {
      setGitRepoPath(path);

      // Only auto-populate name for new projects
      if (!isEditing && path) {
        const cleanName = generateProjectNameFromPath(path);
        if (cleanName) setName(cleanName);
      }
    };

    // Handle direct project creation from repo selection
    const handleDirectCreate = async (path: string, suggestedName: string) => {
      setError('');
      setLoading(true);

      try {
        const createData: CreateProject = {
          name: suggestedName,
          git_repo_path: path,
          use_existing_repo: true,
          setup_script: null,
          dev_script: null,
          cleanup_script: null,
          copy_files: null,
          organization_id: organization_id || null,
          client_id: client_id || null,
          folder_id: null,
          parent_project_id: parent_project_id || null,
        };

        const created = await projectsApi.create(createData);

        // Scaffold template tasks if selected
        if (projectTemplate !== 'none' && created?.id) {
          try {
            await scaffoldTemplateTasks(created.id, projectTemplate);
          } catch (scaffoldErr) {
            console.warn('Template scaffolding failed (project was still created):', scaffoldErr);
          }
        }

        modal.resolve('saved' as ProjectFormDialogResult);
        modal.hide();
      } catch (error) {
        setError(error instanceof Error ? error.message : 'An error occurred');
      } finally {
        setLoading(false);
      }
    };

    const handleSubmit = async (e: React.FormEvent) => {
      e.preventDefault();
      setError('');
      setLoading(true);

      try {
        if (isEditing) {
          let finalGitRepoPath = gitRepoPath;
          if (repoMode === 'new') {
            const effectiveParentPath = parentPath.trim();
            const cleanFolderName = folderName.trim();
            finalGitRepoPath = effectiveParentPath
              ? `${effectiveParentPath}/${cleanFolderName}`.replace(/\/+/g, '/')
              : cleanFolderName;
          }
          const finalName =
            name.trim() || generateProjectNameFromPath(finalGitRepoPath);

          const updateData: UpdateProject = {
            name: finalName,
            git_repo_path: finalGitRepoPath,
            setup_script: setupScript.trim() || undefined,
            dev_script: devScript.trim() || undefined,
            cleanup_script: cleanupScript.trim() || undefined,
            copy_files: copyFiles.trim() || undefined,
          };

          await projectsApi.update(project!.id, updateData);
        } else if (isContainer) {
          // Creating a container/folder project
          const createData: CreateProject = {
            name: name.trim(),
            git_repo_path: '',
            use_existing_repo: false,
            setup_script: null,
            dev_script: null,
            cleanup_script: null,
            copy_files: null,
            organization_id: organization_id || null,
            client_id: client_id || null,
            folder_id: null,
            parent_project_id: parent_project_id || null,
          };

          await projectsApi.create(createData);
        } else {
          // Creating new project with git repo
          let finalGitRepoPath = gitRepoPath;
          if (repoMode === 'new') {
            const effectiveParentPath = parentPath.trim();
            const cleanFolderName = folderName.trim();
            finalGitRepoPath = effectiveParentPath
              ? `${effectiveParentPath}/${cleanFolderName}`.replace(/\/+/g, '/')
              : cleanFolderName;
          }
          const finalName =
            name.trim() || generateProjectNameFromPath(finalGitRepoPath);

          const createData: CreateProject = {
            name: finalName,
            git_repo_path: finalGitRepoPath,
            use_existing_repo: repoMode === 'existing',
            setup_script: null,
            dev_script: null,
            cleanup_script: null,
            copy_files: null,
            organization_id: organization_id || null,
            client_id: client_id || null,
            folder_id: null,
            parent_project_id: parent_project_id || null,
          };

          const created = await projectsApi.create(createData);

          // Scaffold template tasks if a template was selected
          if (projectTemplate !== 'none' && created?.id) {
            await scaffoldTemplateTasks(created.id, projectTemplate);
          }
        }

        modal.resolve('saved' as ProjectFormDialogResult);
        modal.hide();
      } catch (error) {
        setError(error instanceof Error ? error.message : 'An error occurred');
      } finally {
        setLoading(false);
      }
    };

    const scaffoldTemplateTasks = async (projectId: string, template: ProjectTemplate) => {
      const templates: Record<string, Array<{ title: string; description: string; priority: 'high' | 'medium' | 'low'; completion_criteria: string; output_format: string }>> = {
        software: [
          { title: 'Project Setup & Environment Configuration', description: 'Initialize the development environment, install dependencies, configure build tools, and verify the project compiles and runs locally.', priority: 'high', completion_criteria: 'Project builds successfully, all dependencies installed, development server starts without errors', output_format: 'Working development environment with README updates documenting setup steps' },
          { title: 'Core Feature Implementation', description: 'Implement the primary feature set based on project requirements. Follow established coding patterns and ensure code quality.', priority: 'high', completion_criteria: 'All core features implemented and functional, code follows project conventions, no linting errors', output_format: 'Pull request with implementation code and inline documentation' },
          { title: 'Test Suite Development', description: 'Write comprehensive tests covering core functionality, edge cases, and integration points.', priority: 'medium', completion_criteria: 'Test coverage above 80%, all tests passing, edge cases documented', output_format: 'Test files with clear descriptions, coverage report' },
          { title: 'Code Review & Refactoring', description: 'Review implementation for quality, performance, and maintainability. Refactor as needed.', priority: 'medium', completion_criteria: 'No critical issues, performance benchmarks met, code documented', output_format: 'Review comments addressed, refactored code committed' },
          { title: 'Deployment & Release', description: 'Prepare for production deployment. Update configurations, create release notes, and deploy.', priority: 'high', completion_criteria: 'Successfully deployed to target environment, smoke tests passing, release notes published', output_format: 'Deployment manifest, release notes document, verification checklist' },
        ],
        research: [
          { title: 'Define Research Scope & Objectives', description: 'Clearly define what we are investigating, why it matters, and what deliverables are expected.', priority: 'high', completion_criteria: 'Research questions documented, scope boundaries defined, timeline established', output_format: 'Research brief document with objectives, scope, methodology, and timeline' },
          { title: 'Source Gathering & Literature Review', description: 'Identify and collect relevant sources, data sets, and prior work. Assess source quality and relevance.', priority: 'high', completion_criteria: 'Minimum 10 relevant sources identified, quality assessed, gaps noted', output_format: 'Annotated bibliography with source summaries and relevance ratings' },
          { title: 'Deep Analysis & Pattern Identification', description: 'Analyze gathered information systematically. Identify patterns, trends, contradictions, and insights.', priority: 'high', completion_criteria: 'Key patterns identified, supporting evidence documented, contradictions noted', output_format: 'Analysis document with findings organized by theme, supported by evidence' },
          { title: 'Synthesis & Recommendations', description: 'Synthesize findings into actionable insights and concrete recommendations.', priority: 'medium', completion_criteria: 'Findings synthesized into coherent narrative, recommendations are specific and actionable', output_format: 'Executive summary with key findings, detailed recommendations, and supporting data' },
          { title: 'Presentation & Knowledge Sharing', description: 'Prepare and deliver findings to stakeholders. Ensure knowledge is captured for future reference.', priority: 'medium', completion_criteria: 'Presentation delivered, questions addressed, materials archived in knowledge base', output_format: 'Slide deck or report, Q&A summary, knowledge base entry' },
        ],
        marketing: [
          { title: 'Audience Research & Segmentation', description: 'Research target audience demographics, behaviors, pain points, and preferred channels.', priority: 'high', completion_criteria: 'Audience personas defined, segments identified, channel preferences documented', output_format: 'Audience research document with personas and segment profiles' },
          { title: 'Campaign Strategy & Planning', description: 'Define campaign goals, messaging framework, channel strategy, and success metrics.', priority: 'high', completion_criteria: 'Campaign brief complete, KPIs defined, budget allocated, timeline set', output_format: 'Campaign strategy document with goals, messaging, channels, and metrics' },
          { title: 'Content Creation & Asset Development', description: 'Create campaign content across all planned channels and formats.', priority: 'high', completion_criteria: 'All content pieces created, reviewed, and approved. Assets formatted for each channel', output_format: 'Content package with copy, visuals, and channel-specific formats' },
          { title: 'Campaign Launch & Distribution', description: 'Execute the campaign across all planned channels. Monitor initial performance.', priority: 'medium', completion_criteria: 'Campaign live on all channels, tracking implemented, initial metrics reported', output_format: 'Launch confirmation report with channel status and initial metrics' },
          { title: 'Performance Analysis & Optimization', description: 'Analyze campaign results against KPIs. Identify what worked, what did not, and recommendations.', priority: 'medium', completion_criteria: 'All KPIs measured, ROI calculated, learnings documented', output_format: 'Performance report with metrics, analysis, and recommendations for future campaigns' },
        ],
        client_onboarding: [
          { title: 'Client Discovery & Requirements Gathering', description: 'Meet with the client to understand their needs, goals, existing systems, and success criteria.', priority: 'high', completion_criteria: 'Client needs documented, success criteria agreed upon, stakeholders identified', output_format: 'Discovery document with requirements, constraints, stakeholder map, and success metrics' },
          { title: 'Account Setup & System Configuration', description: 'Provision accounts, configure systems, set up integrations, and prepare the client environment.', priority: 'high', completion_criteria: 'All accounts provisioned, integrations connected, environment tested', output_format: 'Setup checklist with confirmation of each system configured and tested' },
          { title: 'Training & Knowledge Transfer', description: 'Train client team on the platform, processes, and best practices. Provide documentation.', priority: 'medium', completion_criteria: 'Training sessions completed, documentation delivered, client team can perform core tasks independently', output_format: 'Training materials, recorded sessions, quick-start guide' },
          { title: 'Go-Live & Initial Support', description: 'Transition to live operations. Provide hands-on support during the initial period.', priority: 'high', completion_criteria: 'Client operating independently, no critical issues, support escalation path established', output_format: 'Go-live report with status, open items, and support contact information' },
          { title: 'Follow-Up & Satisfaction Review', description: 'Check in with the client after go-live. Address any issues and gather feedback.', priority: 'medium', completion_criteria: 'Client satisfaction survey completed, open issues resolved, improvement areas noted', output_format: 'Follow-up report with satisfaction scores, resolved issues, and improvement recommendations' },
        ],
      };

      const tasks = templates[template];
      if (!tasks) return;

      for (const task of tasks) {
        await tasksApi.create({
          project_id: projectId,
          title: task.title,
          description: task.description,
          priority: task.priority,
          completion_criteria: task.completion_criteria,
          output_format: task.output_format,
          assignee_id: null,
          assigned_agent: null,
          agent_id: null,
          assigned_mcps: null,
          created_by: 'system',
          requires_approval: false,
          parent_task_id: null,
          tags: null,
          due_date: null,
          assignee_type: null,
          screenshot: null,
          custom_properties: null,
          scheduled_start: null,
          scheduled_end: null,
          parent_task_attempt: null,
          image_ids: null,
        });
      }
    };

    const handleCancel = () => {
      // Reset form
      if (project) {
        setName(project.name || '');
        setGitRepoPath(project.git_repo_path || '');
        setSetupScript(project.setup_script ?? '');
        setDevScript(project.dev_script ?? '');
        setCopyFiles(project.copy_files ?? '');
      } else {
        setName('');
        setGitRepoPath('');
        setSetupScript('');
        setDevScript('');
        setCopyFiles('');
      }
      setParentPath('');
      setFolderName('');
      setError('');

      modal.resolve('canceled' as ProjectFormDialogResult);
      modal.hide();
    };

    const handleOpenChange = (open: boolean) => {
      if (!open) {
        handleCancel();
      }
    };

    return (
      <Dialog open={modal.visible} onOpenChange={handleOpenChange}>
        <DialogContent className="sm:max-w-2xl max-h-[90vh] flex flex-col overflow-hidden">
          <DialogHeader>
            <DialogTitle>
              {isEditing ? 'Edit Project' : 'Create Project'}
            </DialogTitle>
            <DialogDescription>
              {isEditing
                ? "Make changes to your project here. Click save when you're done."
                : isContainer
                ? 'Create a folder to organize projects'
                : 'Choose your repository source'}
            </DialogDescription>
          </DialogHeader>

          <div className="mx-auto w-full max-w-2xl overflow-x-hidden overflow-y-auto flex-1 px-1">
            {/* Project Type Selector (create mode only) */}
            {!isEditing && (
              <div className="space-y-2 mb-4">
                <Label htmlFor="project-type">Project Type</Label>
                <Select value={projectType} onValueChange={(v) => setProjectType(v as ProjectType)}>
                  <SelectTrigger id="project-type" className="rounded-md bg-background">
                    <SelectValue placeholder="Select project type">
                      {(() => {
                        const selected = PROJECT_TYPES.find((t) => t.value === projectType);
                        if (!selected) return null;
                        const Icon = selected.icon;
                        return (
                          <span className="flex items-center gap-2">
                            <Icon className="h-4 w-4 text-muted-foreground" />
                            {selected.label}
                          </span>
                        );
                      })()}
                    </SelectValue>
                  </SelectTrigger>
                  <SelectContent>
                    {PROJECT_TYPES.map((type) => {
                      const Icon = type.icon;
                      return (
                        <SelectItem key={type.value} value={type.value}>
                          <span className="flex items-center gap-2">
                            <Icon className="h-4 w-4 text-muted-foreground" />
                            {type.label}
                          </span>
                        </SelectItem>
                      );
                    })}
                  </SelectContent>
                </Select>
              </div>
            )}

            {/* Project Template Selector (create mode, non-folder only) */}
            {!isEditing && !isContainer && (
              <div className="space-y-2 mb-4">
                <Label htmlFor="project-template" className="flex items-center gap-1.5">
                  <Sparkles className="h-3.5 w-3.5 text-amber-500" />
                  Start from Template
                </Label>
                <Select value={projectTemplate} onValueChange={(v) => setProjectTemplate(v as ProjectTemplate)}>
                  <SelectTrigger id="project-template" className="rounded-md bg-background">
                    <SelectValue placeholder="No template" />
                  </SelectTrigger>
                  <SelectContent>
                    {PROJECT_TEMPLATES.map((tmpl) => (
                      <SelectItem key={tmpl.value} value={tmpl.value}>
                        <span className="flex flex-col">
                          <span>{tmpl.label}</span>
                          <span className="text-xs text-muted-foreground">{tmpl.description}</span>
                        </span>
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                {projectTemplate !== 'none' && (
                  <p className="text-xs text-muted-foreground">
                    Creates 5 pre-configured tasks with completion criteria and output formats.
                  </p>
                )}
              </div>
            )}

            {isEditing ? (
              <Tabs defaultValue="general" className="w-full -mt-2">
                <TabsList className="grid w-full grid-cols-2 mb-4">
                  <TabsTrigger value="general">General</TabsTrigger>
                  <TabsTrigger value="templates">Task Templates</TabsTrigger>
                </TabsList>
                <TabsContent value="general" className="space-y-4">
                  <form onSubmit={handleSubmit} className="space-y-4">
                    <ProjectFormFields
                      isEditing={isEditing}
                      repoMode={repoMode}
                      setRepoMode={setRepoMode}
                      gitRepoPath={gitRepoPath}
                      handleGitRepoPathChange={handleGitRepoPathChange}
                      parentPath={parentPath}
                      setParentPath={setParentPath}
                      setFolderName={setFolderName}
                      setName={setName}
                      name={name}
                      setupScript={setupScript}
                      setSetupScript={setSetupScript}
                      devScript={devScript}
                      setDevScript={setDevScript}
                      cleanupScript={cleanupScript}
                      setCleanupScript={setCleanupScript}
                      copyFiles={copyFiles}
                      setCopyFiles={setCopyFiles}
                      error={error}
                      setError={setError}
                      projectId={project ? project.id : undefined}
                    />
                    <DialogFooter>
                      <Button
                        type="submit"
                        disabled={loading || !gitRepoPath.trim()}
                      >
                        {loading ? 'Saving...' : 'Save Changes'}
                      </Button>
                    </DialogFooter>
                  </form>
                </TabsContent>
                <TabsContent value="templates" className="mt-0 pt-0">
                  <TaskTemplateManager
                    projectId={project ? project.id : undefined}
                  />
                </TabsContent>
              </Tabs>
            ) : isContainer ? (
              /* Container / Folder creation form */
              <form onSubmit={handleSubmit} className="space-y-4">
                <div className="space-y-2">
                  <Label htmlFor="container-name">
                    Folder Name <span className="text-red-500">*</span>
                  </Label>
                  <Input
                    id="container-name"
                    type="text"
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                    placeholder="e.g. Frontend Apps, Shared Libraries..."
                    autoFocus
                    required
                  />
                  <p className="text-xs text-muted-foreground">
                    This creates a folder to group related projects together.
                  </p>
                </div>
                {error && (
                  <div className="text-sm text-destructive">{error}</div>
                )}
                <DialogFooter>
                  <Button variant="outline" type="button" onClick={handleCancel}>
                    Cancel
                  </Button>
                  <Button
                    type="submit"
                    disabled={loading || !name.trim()}
                  >
                    {loading ? 'Creating...' : 'Create Folder'}
                  </Button>
                </DialogFooter>
              </form>
            ) : (
              <form onSubmit={handleSubmit} className="space-y-4">
                <ProjectFormFields
                  isEditing={isEditing}
                  repoMode={repoMode}
                  setRepoMode={setRepoMode}
                  gitRepoPath={gitRepoPath}
                  handleGitRepoPathChange={handleGitRepoPathChange}
                  parentPath={parentPath}
                  setParentPath={setParentPath}
                  setFolderName={setFolderName}
                  setName={setName}
                  name={name}
                  setupScript={setupScript}
                  setSetupScript={setSetupScript}
                  devScript={devScript}
                  setDevScript={setDevScript}
                  cleanupScript={cleanupScript}
                  setCleanupScript={setCleanupScript}
                  copyFiles={copyFiles}
                  setCopyFiles={setCopyFiles}
                  error={error}
                  setError={setError}
                  projectId={undefined}
                  onCreateProject={handleDirectCreate}
                />
                {repoMode === 'new' && (
                  <DialogFooter>
                    <Button
                      type="submit"
                      disabled={loading || !folderName.trim()}
                    >
                      {loading ? 'Creating...' : 'Create Project'}
                    </Button>
                  </DialogFooter>
                )}
              </form>
            )}
          </div>
        </DialogContent>
      </Dialog>
    );
  }
);
