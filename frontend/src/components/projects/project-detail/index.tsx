import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent } from '@/components/ui/card';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Label } from '@/components/ui/label';
import type { ProjectBoard } from 'shared/types';
import {
  AlertCircle,
  ArrowLeft,
  Edit,
  Loader2,
  Trash2,
} from 'lucide-react';

import { AirtableBaseConnect } from '../AirtableBaseConnect';
import type { ProjectDetailProps } from './types';
import { useProjectData } from './hooks/useProjectData';
import {
  formatStatusLabel,
  BOARD_TYPE_LABELS,
  BOARD_TYPE_OPTIONS,
  ASSET_CATEGORY_OPTIONS,
  ASSET_SCOPE_OPTIONS,
} from './helpers';

import { ProjectHeroCard } from './sections/ProjectHeroCard';
import { ProjectStatsPanel } from './sections/ProjectStatsPanel';
import { IntegrationsSection } from './sections/IntegrationsSection';
import { BoardsSection } from './sections/BoardsSection';
import { AssetVaultSection } from './sections/AssetVaultSection';

export function ProjectDetail({ projectId, onBack }: ProjectDetailProps) {
  const data = useProjectData(projectId, onBack);

  if (data.loading) {
    return (
      <div className="flex items-center justify-center py-12">
        <Loader2 className="mr-2 h-4 w-4 animate-spin" />
        Loading project...
      </div>
    );
  }

  if (data.error || !data.project) {
    return (
      <div className="space-y-4 py-12 px-4">
        <Button variant="outline" onClick={onBack}>
          <ArrowLeft className="mr-2 h-4 w-4" />
          Back to Projects
        </Button>
        <Card>
          <CardContent className="py-12 text-center">
            <div className="mx-auto flex h-12 w-12 items-center justify-center rounded-lg bg-muted">
              <AlertCircle className="h-6 w-6 text-muted-foreground" />
            </div>
            <h3 className="mt-4 text-lg font-semibold">Project not found</h3>
            <p className="mt-2 text-sm text-muted-foreground">
              {data.error ||
                "The project you're looking for doesn't exist or has been deleted."}
            </p>
            <Button className="mt-4" onClick={onBack}>
              Back to Projects
            </Button>
          </CardContent>
        </Card>
      </div>
    );
  }

  const { project } = data;

  return (
    <>
      <div className="space-y-6 py-12 px-4">
        {/* Header bar */}
        <div className="flex flex-col gap-4 border-b border-border/60 pb-6 sm:flex-row sm:items-center sm:justify-between">
          <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:gap-4">
            <Button variant="outline" onClick={onBack} className="w-fit">
              <ArrowLeft className="mr-2 h-4 w-4" />
              Back to Projects
            </Button>
            <div>
              <div className="flex flex-wrap items-center gap-3">
                <h1 className="text-2xl font-bold">{project.name}</h1>
                <Badge variant="secondary">Active</Badge>
              </div>
              <p className="text-sm text-muted-foreground">
                Brand identity, integrations, and delivery health
              </p>
            </div>
          </div>
          <div className="flex gap-2">
            <Button variant="outline" onClick={data.handleEditClick}>
              <Edit className="mr-2 h-4 w-4" />
              Edit
            </Button>
            <Button
              variant="outline"
              onClick={data.handleDelete}
              className="text-destructive hover:text-destructive-foreground hover:bg-destructive/10"
            >
              <Trash2 className="mr-2 h-4 w-4" />
              Delete
            </Button>
          </div>
        </div>

        {/* Hero + Stats row */}
        <div className="grid gap-6 lg:grid-cols-3">
          <ProjectHeroCard
            projectId={projectId}
            projectName={project.name}
            projectUpdatedAt={project.updated_at}
            brandInitials={data.brandInitials}
            effectiveBrandProfile={data.effectiveBrandProfile}
            brandHeroGradient={data.brandHeroGradient}
            totalBoards={data.totalBoards}
            totalTasks={data.totalTasks}
            totalAssets={data.totalAssets}
            activeTasks={data.activeTasks}
            onAdjustBrandProfile={data.handleBrandProfileDialogOpen}
          />
          <ProjectStatsPanel project={project} repoLabel={data.repoLabel} />
        </div>

        {/* Integrations */}
        <IntegrationsSection
          integrationCategories={data.integrationCategories}
          emailIntegrationError={data.emailIntegrationError}
          socialIntegrationError={data.socialIntegrationError}
          integrationsRefreshing={data.integrationsRefreshing}
          onRefresh={data.refreshIntegrationStatuses}
        />

        {/* Airtable */}
        <div ref={data.airtableSectionRef}>
          <AirtableBaseConnect
            projectId={project.id}
            projectName={project.name}
            onConnectionsChange={data.setAirtableConnections}
          />
        </div>

        {/* Boards + Assets row */}
        <div className="grid gap-6 lg:grid-cols-2">
          <BoardsSection
            projectId={projectId}
            boards={data.boards}
            boardsLoading={data.boardsLoading}
            boardsError={data.boardsError}
            tasksLoading={data.tasksLoading}
            tasksError={data.tasksError}
            tasksByBoard={data.tasksByBoard}
            unassignedTasks={data.unassignedTasks}
            isDefaultBoard={data.isDefaultBoard}
            onCreateBoard={() => {
              data.resetBoardForm();
              data.setIsCreateBoardOpen(true);
            }}
            onDeleteBoard={data.handleDeleteBoard}
          />
          <AssetVaultSection
            assets={data.assets}
            assetsLoading={data.assetsLoading}
            assetsError={data.assetsError}
            boardById={data.boardById}
            onAddAsset={() => {
              data.resetAssetForm();
              data.setIsCreateAssetOpen(true);
            }}
            onDeleteAsset={data.handleDeleteAsset}
          />
        </div>
      </div>

      {/* Brand Profile Dialog */}
      <Dialog
        open={data.isBrandProfileDialogOpen}
        onOpenChange={(open) => {
          data.setIsBrandProfileDialogOpen(open);
          if (!open) {
            data.setBrandProfileDraft(null);
          }
        }}
      >
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>Adjust Brand Profile</DialogTitle>
            <p className="text-sm text-muted-foreground">
              Tune the tagline, industry, and palette for this project overview.
            </p>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-1.5">
              <Label htmlFor="brand-tagline">Tagline</Label>
              <Input
                id="brand-tagline"
                value={data.brandProfileDraft?.tagline ?? ''}
                onChange={(event) =>
                  data.setBrandProfileDraft((prev) => ({
                    ...(prev ?? data.defaultBrandProfileValues),
                    tagline: event.target.value,
                  }))
                }
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="brand-industry">Industry</Label>
              <Input
                id="brand-industry"
                value={data.brandProfileDraft?.industry ?? ''}
                onChange={(event) =>
                  data.setBrandProfileDraft((prev) => ({
                    ...(prev ?? data.defaultBrandProfileValues),
                    industry: event.target.value,
                  }))
                }
                placeholder="e.g. SaaS, Agency, E-commerce"
              />
            </div>
            <div className="grid gap-4 sm:grid-cols-2">
              <div className="space-y-1.5">
                <Label htmlFor="brand-primary">Primary Color</Label>
                <div className="flex items-center gap-3">
                  <Input
                    id="brand-primary"
                    type="color"
                    value={data.brandProfileDraft?.primaryColor ?? data.defaultBrandProfileValues.primaryColor}
                    onChange={(event) =>
                      data.setBrandProfileDraft((prev) => ({
                        ...(prev ?? data.defaultBrandProfileValues),
                        primaryColor: event.target.value,
                      }))
                    }
                    className="h-10 w-16 cursor-pointer"
                  />
                  <Input
                    value={data.brandProfileDraft?.primaryColor ?? data.defaultBrandProfileValues.primaryColor}
                    onChange={(event) =>
                      data.setBrandProfileDraft((prev) => ({
                        ...(prev ?? data.defaultBrandProfileValues),
                        primaryColor: event.target.value,
                      }))
                    }
                  />
                </div>
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="brand-secondary">Secondary Color</Label>
                <div className="flex items-center gap-3">
                  <Input
                    id="brand-secondary"
                    type="color"
                    value={data.brandProfileDraft?.secondaryColor ?? data.defaultBrandProfileValues.secondaryColor}
                    onChange={(event) =>
                      data.setBrandProfileDraft((prev) => ({
                        ...(prev ?? data.defaultBrandProfileValues),
                        secondaryColor: event.target.value,
                      }))
                    }
                    className="h-10 w-16 cursor-pointer"
                  />
                  <Input
                    value={data.brandProfileDraft?.secondaryColor ?? data.defaultBrandProfileValues.secondaryColor}
                    onChange={(event) =>
                      data.setBrandProfileDraft((prev) => ({
                        ...(prev ?? data.defaultBrandProfileValues),
                        secondaryColor: event.target.value,
                      }))
                    }
                  />
                </div>
              </div>
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="ghost"
              onClick={() => {
                data.setIsBrandProfileDialogOpen(false);
                data.setBrandProfileDraft(null);
              }}
            >
              Cancel
            </Button>
            <Button onClick={data.handleBrandProfileSave} disabled={!data.brandProfileDraft}>
              Save profile
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Create Board Dialog */}
      <Dialog
        open={data.isCreateBoardOpen}
        onOpenChange={(open) => {
          data.setIsCreateBoardOpen(open);
          if (!open) {
            data.resetBoardForm();
          }
        }}
      >
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>Create Project Board</DialogTitle>
            <p className="text-sm text-muted-foreground">
              Add a dedicated board to hold tasks and assets for a new focus area.
            </p>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-1.5">
              <Label htmlFor="board-name">Name</Label>
              <Input
                id="board-name"
                value={data.boardForm.name}
                onChange={(e) =>
                  data.setBoardForm((prev) => ({ ...prev, name: e.target.value }))
                }
                disabled={data.boardFormSubmitting}
                placeholder="e.g. Lifecycle Experiments"
              />
            </div>
            <div className="grid gap-4 sm:grid-cols-2">
              <div className="space-y-1.5">
                <Label>Board type</Label>
                <Select
                  value={data.boardForm.boardType}
                  onValueChange={(value) =>
                    data.setBoardForm((prev) => ({
                      ...prev,
                      boardType: value as ProjectBoard['board_type'],
                    }))
                  }
                  disabled={data.boardFormSubmitting}
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {BOARD_TYPE_OPTIONS.map((option) => (
                      <SelectItem key={option} value={option}>
                        {BOARD_TYPE_LABELS[option]}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="board-description">Description (optional)</Label>
              <Textarea
                id="board-description"
                value={data.boardForm.description}
                onChange={(e) =>
                  data.setBoardForm((prev) => ({
                    ...prev,
                    description: e.target.value,
                  }))
                }
                disabled={data.boardFormSubmitting}
                rows={3}
                placeholder="Clarify what lives inside this board."
              />
            </div>
            {data.boardFormError && (
              <p className="text-sm text-destructive">{data.boardFormError}</p>
            )}
          </div>
          <DialogFooter>
            <Button
              variant="ghost"
              onClick={() => {
                data.setIsCreateBoardOpen(false);
                data.resetBoardForm();
              }}
              disabled={data.boardFormSubmitting}
            >
              Cancel
            </Button>
            <Button onClick={data.handleCreateBoard} disabled={data.boardFormSubmitting}>
              {data.boardFormSubmitting ? 'Creating\u2026' : 'Create Board'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Create Asset Dialog */}
      <Dialog
        open={data.isCreateAssetOpen}
        onOpenChange={(open) => {
          data.setIsCreateAssetOpen(open);
          if (!open) {
            data.resetAssetForm();
          }
        }}
      >
        <DialogContent className="sm:max-w-xl">
          <DialogHeader>
            <DialogTitle>Add Brand Asset</DialogTitle>
            <p className="text-sm text-muted-foreground">
              Catalogue files, transcripts, or collateral to keep the team aligned.
            </p>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-1.5">
              <Label htmlFor="asset-name">Name</Label>
              <Input
                id="asset-name"
                value={data.assetForm.name}
                onChange={(e) =>
                  data.setAssetForm((prev) => ({ ...prev, name: e.target.value }))
                }
                disabled={data.assetFormSubmitting}
                placeholder="e.g. Primary logo"
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="asset-storage">Storage path / URL</Label>
              <Input
                id="asset-storage"
                value={data.assetForm.storagePath}
                onChange={(e) =>
                  data.setAssetForm((prev) => ({ ...prev, storagePath: e.target.value }))
                }
                disabled={data.assetFormSubmitting}
                placeholder="s3://bucket/logo.svg or https://..."
              />
            </div>
            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
              <div className="space-y-1.5">
                <Label>Category</Label>
                <Select
                  value={data.assetForm.category}
                  onValueChange={(value) =>
                    data.setAssetForm((prev) => ({ ...prev, category: value }))
                  }
                  disabled={data.assetFormSubmitting}
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {ASSET_CATEGORY_OPTIONS.map((category) => (
                      <SelectItem key={category} value={category}>
                        {formatStatusLabel(category)}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-1.5">
                <Label>Scope</Label>
                <Select
                  value={data.assetForm.scope}
                  onValueChange={(value) =>
                    data.setAssetForm((prev) => ({ ...prev, scope: value }))
                  }
                  disabled={data.assetFormSubmitting}
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {ASSET_SCOPE_OPTIONS.map((scope) => (
                      <SelectItem key={scope} value={scope}>
                        {formatStatusLabel(scope)}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-1.5">
                <Label>Board</Label>
                <Select
                  value={data.assetForm.boardId}
                  onValueChange={(value) =>
                    data.setAssetForm((prev) => ({ ...prev, boardId: value }))
                  }
                  disabled={data.assetFormSubmitting || data.boardsLoading}
                >
                  <SelectTrigger>
                    <SelectValue
                      placeholder={
                        data.boardsLoading
                          ? 'Loading boards\u2026'
                          : data.boards.length
                          ? 'Select board'
                          : 'No boards available'
                      }
                    />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="none">No board</SelectItem>
                    {data.boards.map((board) => (
                      <SelectItem key={board.id} value={board.id}>
                        {board.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </div>
            <div className="grid gap-4 sm:grid-cols-2">
              <div className="space-y-1.5">
                <Label htmlFor="asset-byte-size">Byte size (optional)</Label>
                <Input
                  id="asset-byte-size"
                  type="number"
                  value={data.assetForm.byteSize}
                  onChange={(e) =>
                    data.setAssetForm((prev) => ({ ...prev, byteSize: e.target.value }))
                  }
                  disabled={data.assetFormSubmitting}
                />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="asset-mime">MIME type (optional)</Label>
                <Input
                  id="asset-mime"
                  value={data.assetForm.mimeType}
                  onChange={(e) =>
                    data.setAssetForm((prev) => ({ ...prev, mimeType: e.target.value }))
                  }
                  disabled={data.assetFormSubmitting}
                />
              </div>
            </div>
            <div className="grid gap-4 sm:grid-cols-2">
              <div className="space-y-1.5">
                <Label htmlFor="asset-checksum">Checksum (optional)</Label>
                <Input
                  id="asset-checksum"
                  value={data.assetForm.checksum}
                  onChange={(e) =>
                    data.setAssetForm((prev) => ({ ...prev, checksum: e.target.value }))
                  }
                  disabled={data.assetFormSubmitting}
                />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="asset-uploaded-by">Uploaded by (optional)</Label>
                <Input
                  id="asset-uploaded-by"
                  value={data.assetForm.uploadedBy}
                  onChange={(e) =>
                    data.setAssetForm((prev) => ({ ...prev, uploadedBy: e.target.value }))
                  }
                  disabled={data.assetFormSubmitting}
                />
              </div>
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="asset-metadata">Metadata (optional)</Label>
              <Textarea
                id="asset-metadata"
                value={data.assetForm.metadata}
                onChange={(e) =>
                  data.setAssetForm((prev) => ({ ...prev, metadata: e.target.value }))
                }
                disabled={data.assetFormSubmitting}
                rows={3}
                placeholder="JSON or notes for this asset"
              />
            </div>
            {data.assetFormError && (
              <p className="text-sm text-destructive">{data.assetFormError}</p>
            )}
          </div>
          <DialogFooter>
            <Button
              variant="ghost"
              onClick={() => {
                data.setIsCreateAssetOpen(false);
                data.resetAssetForm();
              }}
              disabled={data.assetFormSubmitting}
            >
              Cancel
            </Button>
            <Button onClick={data.handleCreateAsset} disabled={data.assetFormSubmitting}>
              {data.assetFormSubmitting ? 'Adding\u2026' : 'Add Asset'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
