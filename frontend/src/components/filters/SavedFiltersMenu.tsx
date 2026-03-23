import { BookmarkPlus, Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { IconButton } from '@/components/ui/icon-button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { useFilterStore } from '@/stores/useFilterStore';
import { toast } from 'sonner';

interface SavedFiltersMenuProps {
  projectId: string;
}

export function SavedFiltersMenu({ projectId }: SavedFiltersMenuProps) {
  const { getPresets, loadPreset, deletePreset } = useFilterStore();
  const presets = getPresets(projectId);

  const handleLoadPreset = (presetId: string) => {
    const preset = presets.find((p) => p.id === presetId);
    if (preset) {
      loadPreset(projectId, presetId);
      toast.success(`Loaded filter preset "${preset.name}"`);
    }
  };

  const handleDeletePreset = (presetId: string, presetName: string) => {
    deletePreset(projectId, presetId);
    toast.success(`Deleted filter preset "${presetName}"`);
  };

  if (presets.length === 0) {
    return null;
  }

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" size="sm" className="gap-2">
          <BookmarkPlus className="h-4 w-4" />
          Saved Filters
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-56">
        <DropdownMenuLabel>Saved Filter Presets</DropdownMenuLabel>
        <DropdownMenuSeparator />
        {presets.map((preset) => (
          <DropdownMenuItem
            key={preset.id}
            className="flex items-center justify-between"
            onClick={() => handleLoadPreset(preset.id)}
          >
            <span className="truncate flex-1">{preset.name}</span>
            <IconButton
              variant="ghost" className="h-6 w-6 shrink-0"
              onClick={(e) => {
                e.stopPropagation();
                handleDeletePreset(preset.id, preset.name);
              }}
              icon={Trash2}
              label="Delete filter"
              iconClassName="h-3 w-3 text-destructive"
            />
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}