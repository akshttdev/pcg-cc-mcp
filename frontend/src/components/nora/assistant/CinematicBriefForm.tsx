import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import type { CinematicFormState } from './types';

interface CinematicBriefFormProps {
  interactionMode: 'chat' | 'cinematic';
  onModeChange: (mode: 'chat' | 'cinematic') => void;
  cinematicForm: CinematicFormState;
  onFormChange: (updates: Partial<CinematicFormState>) => void;
}

export function CinematicBriefForm({
  interactionMode,
  onModeChange,
  cinematicForm,
  onFormChange,
}: CinematicBriefFormProps) {
  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center justify-between gap-3 text-sm">
        <div className="flex items-center gap-2">
          <span className="text-xs uppercase text-muted-foreground">Mode</span>
          <select
            value={interactionMode}
            onChange={(event) => onModeChange(event.target.value as 'chat' | 'cinematic')}
            className="border rounded-md px-2 py-1 text-sm focus:outline-none"
          >
            <option value="chat">Executive Chat</option>
            <option value="cinematic">Cinematic Brief</option>
          </select>
        </div>
        {interactionMode === 'cinematic' && (
          <Badge variant="outline" className="text-purple-600 border-purple-300">
            Master Cinematographer engaged
          </Badge>
        )}
      </div>

      {interactionMode === 'cinematic' && (
        <div className="space-y-3 rounded-lg border border-purple-200/70 bg-purple-50/40 p-3 text-sm">
          <div className="grid gap-3 md:grid-cols-2">
            <div>
              <Label htmlFor="cinematic-project">Project ID</Label>
              <Input
                id="cinematic-project"
                placeholder="UUID for the target project"
                value={cinematicForm.projectId}
                onChange={(e) => onFormChange({ projectId: e.target.value })}
              />
            </div>
            <div>
              <Label htmlFor="cinematic-title">Production Title</Label>
              <Input
                id="cinematic-title"
                placeholder="e.g. Neon Metropolis Flythrough"
                value={cinematicForm.title}
                onChange={(e) => onFormChange({ title: e.target.value })}
              />
            </div>
          </div>

          <div>
            <Label htmlFor="cinematic-summary">Creative Summary</Label>
            <Textarea
              id="cinematic-summary"
              rows={3}
              placeholder="Key beats, tone, and deliverable goals"
              value={cinematicForm.summary}
              onChange={(e) => onFormChange({ summary: e.target.value })}
            />
          </div>

          <div className="grid gap-3 md:grid-cols-2">
            <div>
              <Label htmlFor="cinematic-style">Style Tags</Label>
              <Input
                id="cinematic-style"
                placeholder="cyberpunk, volumetric lighting, slow dolly"
                value={cinematicForm.styleTags}
                onChange={(e) => onFormChange({ styleTags: e.target.value })}
              />
            </div>
            <div>
              <Label htmlFor="cinematic-assets">Asset IDs</Label>
              <Input
                id="cinematic-assets"
                placeholder="Comma separated project asset IDs"
                value={cinematicForm.assetIds}
                onChange={(e) => onFormChange({ assetIds: e.target.value })}
              />
            </div>
          </div>

          <div className="flex items-center justify-between">
            <div>
              <Label htmlFor="cinematic-auto">Auto-render via ComfyUI</Label>
              <p className="text-xs text-muted-foreground">
                Nora will immediately hand the brief to the Master Cinematographer
              </p>
            </div>
            <Switch
              id="cinematic-auto"
              checked={cinematicForm.autoRender}
              onCheckedChange={(checked) => onFormChange({ autoRender: checked })}
            />
          </div>
        </div>
      )}
    </div>
  );
}
