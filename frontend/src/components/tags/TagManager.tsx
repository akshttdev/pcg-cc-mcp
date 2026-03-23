import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import { Plus, Edit2, Trash2 } from 'lucide-react';
import { HexColorPicker } from 'react-colorful';
import { useTagStore } from '@/stores/useTagStore';
import { nanoid } from 'nanoid';
import { toast } from 'sonner';

interface TagManagerProps {
  projectId: string;
}

const PRESET_COLORS = [
  '#ef4444', // red
  '#f97316', // orange
  '#f59e0b', // amber
  '#eab308', // yellow
  '#84cc16', // lime
  '#22c55e', // green
  '#10b981', // emerald
  '#14b8a6', // teal
  '#06b6d4', // cyan
  '#0ea5e9', // sky
  '#3b82f6', // blue
  '#6366f1', // indigo
  '#8b5cf6', // violet
  '#a855f7', // purple
  '#d946ef', // fuchsia
  '#ec4899', // pink
];

export function TagManager({ projectId }: TagManagerProps) {
  const { getTagsForProject, addTag, updateTag, deleteTag } = useTagStore();
  const tags = getTagsForProject(projectId);

  const [isOpen, setIsOpen] = useState(false);
  const [editingTag, setEditingTag] = useState<{ id: string; name: string; color: string } | null>(null);
  const [tagName, setTagName] = useState('');
  const [tagColor, setTagColor] = useState('#3b82f6');

  const handleCreateTag = () => {
    if (!tagName.trim()) {
      toast.error('Tag name is required');
      return;
    }

    const newTag = {
      id: nanoid(),
      projectId,
      name: tagName.trim(),
      color: tagColor,
      createdAt: new Date(),
      updatedAt: new Date(),
    };

    addTag(newTag);
    setTagName('');
    setTagColor('#3b82f6');
    toast.success(`Tag "${newTag.name}" created`);
  };

  const handleUpdateTag = () => {
    if (!editingTag || !tagName.trim()) return;

    updateTag(editingTag.id, {
      name: tagName.trim(),
      color: tagColor,
    });

    toast.success('Tag updated');
    setEditingTag(null);
    setTagName('');
    setTagColor('#3b82f6');
  };

  const handleDeleteTag = (tagId: string, tagName: string) => {
    deleteTag(projectId, tagId);
    toast.success(`Tag "${tagName}" deleted`);
  };

  const startEditingTag = (tag: { id: string; name: string; color: string }) => {
    setEditingTag(tag);
    setTagName(tag.name);
    setTagColor(tag.color);
  };

  const cancelEditing = () => {
    setEditingTag(null);
    setTagName('');
    setTagColor('#3b82f6');
  };

  return (
    <>
      <Button variant="outline" size="sm" className="gap-2" onClick={() => setIsOpen(true)}>
        <Plus className="h-4 w-4" />
        Manage Tags
      </Button>
      <Dialog open={isOpen} onOpenChange={setIsOpen}>
        <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>Manage Tags</DialogTitle>
          <DialogDescription>
            Create and manage tags for organizing tasks in this project
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {/* Create/Edit Tag Form */}
          <div className="border rounded-lg p-4 space-y-3">
            <h4 className="text-sm font-medium">
              {editingTag ? 'Edit Tag' : 'Create New Tag'}
            </h4>

            <div className="flex gap-2">
              <Input
                placeholder="Tag name"
                value={tagName}
                onChange={(e) => setTagName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    editingTag ? handleUpdateTag() : handleCreateTag();
                  }
                }}
                className="flex-1"
              />

              <Popover>
                <PopoverTrigger asChild>
                  <Button
                    variant="outline"
                    size="sm"
                    className="w-24"
                    style={{ backgroundColor: tagColor }}
                  >
                    <span className="text-white mix-blend-difference">Color</span>
                  </Button>
                </PopoverTrigger>
                <PopoverContent className="w-auto p-3">
                  <div className="space-y-3">
                    <HexColorPicker color={tagColor} onChange={setTagColor} />
                    <div className="grid grid-cols-8 gap-2">
                      {PRESET_COLORS.map((color) => (
                        <button
                          key={color}
                          className="w-6 h-6 rounded border-2 border-white shadow-sm hover:scale-110 transition-transform"
                          style={{ backgroundColor: color }}
                          onClick={() => setTagColor(color)}
                        />
                      ))}
                    </div>
                  </div>
                </PopoverContent>
              </Popover>

              {editingTag ? (
                <>
                  <Button size="sm" onClick={handleUpdateTag}>
                    Save
                  </Button>
                  <Button size="sm" variant="outline" onClick={cancelEditing}>
                    Cancel
                  </Button>
                </>
              ) : (
                <Button size="sm" onClick={handleCreateTag}>
                  <Plus className="h-4 w-4 mr-1" />
                  Add
                </Button>
              )}
            </div>
          </div>

          {/* Existing Tags */}
          <div className="space-y-2">
            <h4 className="text-sm font-medium">Existing Tags ({tags.length})</h4>
            {tags.length === 0 ? (
              <p className="text-sm text-muted-foreground py-4 text-center">
                No tags yet. Create one above.
              </p>
            ) : (
              <div className="space-y-2 max-h-64 overflow-y-auto">
                {tags.map((tag) => (
                  <div
                    key={tag.id}
                    className="flex items-center justify-between p-2 border rounded-lg hover:bg-accent/50 transition-colors"
                  >
                    <Badge
                      style={{
                        backgroundColor: `${tag.color}20`,
                        color: tag.color,
                        borderColor: `${tag.color}40`,
                      }}
                      className="border"
                    >
                      {tag.name}
                    </Badge>
                    <div className="flex items-center gap-1">
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-7 w-7"
                        onClick={() => startEditingTag(tag)}
                        title="Edit tag"
                      >
                        <Edit2 className="h-3 w-3" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-7 w-7 text-destructive"
                        onClick={() => handleDeleteTag(tag.id, tag.name)}
                        title="Delete tag"
                      >
                        <Trash2 className="h-3 w-3" />
                      </Button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => setIsOpen(false)}>
            Close
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
    </>
  );
}
