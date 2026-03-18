import { useState, useRef, useCallback } from 'react';
import { Upload } from 'lucide-react';
import { dataSourcesApi } from '@/lib/api';
import { toast } from 'sonner';

export function UploadZone({ orgId, projectId, onUploaded }: {
  orgId?: string; projectId?: string; onUploaded: () => void;
}) {
  const [dragging, setDragging] = useState(false);
  const [uploading, setUploading] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  const doUpload = useCallback(async (files: FileList | File[]) => {
    const fileArr = Array.from(files);
    if (!fileArr.length) return;
    setUploading(true);
    let ok = 0;
    for (const file of fileArr) {
      const fd = new FormData();
      fd.append('file', file);
      fd.append('title', file.name);
      fd.append('source_type', 'upload');
      fd.append('data_type', 'document');
      if (orgId) fd.append('organization_id', orgId);
      if (projectId) fd.append('project_id', projectId);
      try {
        await dataSourcesApi.upload(fd);
        ok++;
      } catch {
        toast.error(`Failed to upload ${file.name}`);
      }
    }
    if (ok > 0) {
      toast.success(`Uploaded ${ok} file${ok > 1 ? 's' : ''}`);
      onUploaded();
    }
    setUploading(false);
  }, [orgId, projectId, onUploaded]);

  const onDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setDragging(false);
    doUpload(e.dataTransfer.files);
  }, [doUpload]);

  return (
    <div
      onDragOver={(e) => { e.preventDefault(); setDragging(true); }}
      onDragLeave={() => setDragging(false)}
      onDrop={onDrop}
      className={`border-2 border-dashed rounded-xl p-8 text-center transition-colors cursor-pointer
        ${dragging ? 'border-primary bg-primary/5' : 'border-border hover:border-primary/50 hover:bg-muted/30'}`}
      onClick={() => inputRef.current?.click()}
    >
      <input
        ref={inputRef}
        type="file"
        multiple
        className="hidden"
        onChange={(e) => e.target.files && doUpload(e.target.files)}
      />
      <Upload className={`h-8 w-8 mx-auto mb-3 ${dragging ? 'text-primary' : 'text-muted-foreground'}`} />
      {uploading ? (
        <p className="text-sm text-muted-foreground">Uploading...</p>
      ) : (
        <>
          <p className="text-sm font-medium mb-1">Drop files here or click to upload</p>
          <p className="text-xs text-muted-foreground">Documents, images, audio, video — up to 50MB per file</p>
        </>
      )}
    </div>
  );
}
