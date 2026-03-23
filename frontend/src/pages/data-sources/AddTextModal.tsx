import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { DialogFooter } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { X } from 'lucide-react';
import { dataSourcesApi } from '@/lib/api';
import { toast } from 'sonner';

export function AddTextModal({ orgId, projectId, onClose, onAdded }: {
  orgId?: string; projectId?: string; onClose: () => void; onAdded: () => void;
}) {
  const [title, setTitle] = useState('');
  const [content, setContent] = useState('');
  const [loading, setLoading] = useState(false);

  const handleSave = async () => {
    if (!title.trim()) return;
    setLoading(true);
    try {
      await dataSourcesApi.create({
        title: title.trim(),
        source_type: 'text',
        data_type: 'document',
        content: content || undefined,
        organization_id: orgId,
        project_id: projectId,
      });
      toast.success('Text source added');
      onAdded();
      onClose();
    } catch {
      toast.error('Failed to add text source');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/50 z-50 flex items-center justify-center p-4" onClick={onClose}>
      <div className="bg-card rounded-xl border shadow-xl w-full max-w-lg" onClick={e => e.stopPropagation()}>
        <div className="flex items-center justify-between p-4 border-b">
          <h2 className="font-semibold">Add Text Source</h2>
          <button onClick={onClose}><X className="h-4 w-4 text-muted-foreground" /></button>
        </div>
        <div className="p-4 space-y-3">
          <div>
            <label className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Title</label>
            <Input value={title} onChange={e => setTitle(e.target.value)} placeholder="Document title..." className="mt-1" />
          </div>
          <div>
            <label className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Content</label>
            <textarea
              value={content}
              onChange={e => setContent(e.target.value)}
              placeholder="Paste or type content..."
              className="mt-1 w-full h-40 text-sm rounded-md border bg-background px-3 py-2 resize-none focus:outline-none focus:ring-2 focus:ring-ring"
            />
          </div>
        </div>
        <DialogFooter className="p-4 border-t">
          <Button variant="outline" size="sm" onClick={onClose}>Cancel</Button>
          <Button size="sm" onClick={handleSave} disabled={loading || !title.trim()}>
            {loading ? 'Saving...' : 'Add Source'}
          </Button>
        </DialogFooter>
      </div>
    </div>
  );
}
