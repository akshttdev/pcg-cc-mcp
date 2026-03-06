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
import { projectsApi } from '@/lib/api';
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

        await projectsApi.create(createData);
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

          await projectsApi.create(createData);
        }

        modal.resolve('saved' as ProjectFormDialogResult);
        modal.hide();
      } catch (error) {
        setError(error instanceof Error ? error.message : 'An error occurred');
      } finally {
        setLoading(false);
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
        <DialogContent className="overflow-x-hidden">
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

          <div className="mx-auto w-full max-w-2xl overflow-x-hidden px-1">
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
