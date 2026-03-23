import { useNavigate } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent } from '@/components/ui/card';
import { ClipboardCheck, Palette } from 'lucide-react';

interface ProjectHeroCardProps {
  projectId: string;
  projectName: string;
  projectUpdatedAt: Date;
  brandInitials: string;
  effectiveBrandProfile: {
    tagline: string;
    industry: string;
    primaryColor: string;
    secondaryColor: string;
  };
  brandHeroGradient: React.CSSProperties;
  totalBoards: number;
  totalTasks: number;
  totalAssets: number;
  activeTasks: number;
  onAdjustBrandProfile: () => void;
}

export function ProjectHeroCard({
  projectId,
  projectName,
  projectUpdatedAt,
  brandInitials,
  effectiveBrandProfile,
  brandHeroGradient,
  totalBoards,
  totalTasks,
  totalAssets,
  activeTasks,
  onAdjustBrandProfile,
}: ProjectHeroCardProps) {
  const navigate = useNavigate();

  return (
    <Card
      className="relative overflow-hidden border p-0 lg:col-span-2"
      style={brandHeroGradient}
    >
      <CardContent className="relative space-y-6 p-6">
        <div className="space-y-4">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div className="flex flex-wrap items-center gap-2 text-xs uppercase text-muted-foreground">
              <span>
                Updated {new Date(projectUpdatedAt).toLocaleDateString()}
              </span>
              <span className="text-muted-foreground/60">&bull;</span>
              <span>
                {totalBoards} board{totalBoards === 1 ? '' : 's'} &middot; {totalTasks}{' '}
                task{totalTasks === 1 ? '' : 's'}
              </span>
            </div>
            <div className="flex gap-2">
              <Button
                variant="default"
                size="sm"
                onClick={() => navigate(`/projects/${projectId}/tasks`)}
              >
                <ClipboardCheck className="mr-1.5 h-3.5 w-3.5" />
                Tasks
                {totalTasks > 0 && (
                  <Badge variant="secondary" className="ml-1.5 h-5 bg-primary-foreground/20 text-primary-foreground">
                    {totalTasks}
                  </Badge>
                )}
              </Button>
              <Button variant="outline" size="sm" onClick={onAdjustBrandProfile}>
                Adjust brand profile
              </Button>
            </div>
          </div>
          <div className="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
            <div className="flex items-center gap-4">
              <div className="flex h-16 w-16 items-center justify-center rounded-2xl border border-background/70 bg-background/80 text-2xl font-semibold text-foreground shadow-sm">
                {brandInitials}
              </div>
              <div>
                <p className="text-xs uppercase tracking-wide text-muted-foreground">
                  Brand Identity
                </p>
                <p className="text-3xl font-bold leading-tight text-foreground">
                  {projectName}
                </p>
                <p className="text-base text-muted-foreground">
                  {effectiveBrandProfile.tagline}
                </p>
                <p className="text-xs uppercase tracking-wide text-muted-foreground/80">
                  Industry
                </p>
                <p className="text-sm font-semibold text-foreground">
                  {effectiveBrandProfile.industry || 'Brand Studio'}
                </p>
              </div>
            </div>
            <div className="grid w-full gap-3 sm:grid-cols-3 lg:w-auto">
              <div className="rounded-2xl bg-background/80 p-4 text-center shadow-sm">
                <p className="text-xs uppercase text-muted-foreground">Boards</p>
                <p className="text-2xl font-semibold text-foreground">
                  {totalBoards}
                </p>
                <p className="text-xs text-muted-foreground">Active lanes</p>
              </div>
              <div className="rounded-2xl bg-background/80 p-4 text-center shadow-sm">
                <p className="text-xs uppercase text-muted-foreground">Active Tasks</p>
                <p className="text-2xl font-semibold text-foreground">
                  {activeTasks}
                </p>
                <p className="text-xs text-muted-foreground">In flight</p>
              </div>
              <div className="rounded-2xl bg-background/80 p-4 text-center shadow-sm">
                <p className="text-xs uppercase text-muted-foreground">Assets</p>
                <p className="text-2xl font-semibold text-foreground">
                  {totalAssets}
                </p>
                <p className="text-xs text-muted-foreground">Catalogued</p>
              </div>
            </div>
          </div>
        </div>
        <div className="space-y-3">
          <div className="flex items-center gap-2 text-xs uppercase tracking-wide text-muted-foreground">
            <Palette className="h-3.5 w-3.5" />
            Brand palette
          </div>
          <div className="flex flex-wrap gap-6">
            <div className="flex items-center gap-3">
              <span
                className="h-10 w-10 rounded-xl shadow-inner"
                style={{ background: effectiveBrandProfile.primaryColor }}
              />
              <div>
                <p className="text-xs uppercase text-muted-foreground">Primary</p>
                <p className="font-mono text-sm">{effectiveBrandProfile.primaryColor}</p>
              </div>
            </div>
            <div className="flex items-center gap-3">
              <span
                className="h-10 w-10 rounded-xl shadow-inner"
                style={{ background: effectiveBrandProfile.secondaryColor }}
              />
              <div>
                <p className="text-xs uppercase text-muted-foreground">Secondary</p>
                <p className="font-mono text-sm">{effectiveBrandProfile.secondaryColor}</p>
              </div>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
