import { useState, useCallback } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { Upload, X, FileUp, Loader2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { toast } from 'sonner';
import { orgCloudApi } from '@/lib/api/org-cloud';

interface CloudUploadProps {
  orgId: string;
}

export function CloudUpload({ orgId }: CloudUploadProps) {
  const queryClient = useQueryClient();
  const [files, setFiles] = useState<File[]>([]);
  const [visibility, setVisibility] = useState('org');
  const [dragOver, setDragOver] = useState(false);

  const uploadMutation = useMutation({
    mutationFn: async (file: File) => {
      const formData = new FormData();
      formData.append('file', file);
      formData.append('visibility', visibility);
      return orgCloudApi.contribute(orgId, formData);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['org-cloud', orgId] });
      queryClient.invalidateQueries({ queryKey: ['org-cloud-stats', orgId] });
    },
  });

  const handleUploadAll = async () => {
    let success = 0;
    let failed = 0;
    for (const file of files) {
      try {
        await uploadMutation.mutateAsync(file);
        success++;
      } catch {
        failed++;
      }
    }
    if (success > 0) toast.success(`Uploaded ${success} file${success > 1 ? 's' : ''}`);
    if (failed > 0) toast.error(`Failed to upload ${failed} file${failed > 1 ? 's' : ''}`);
    setFiles([]);
  };

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setDragOver(false);
    const dropped = Array.from(e.dataTransfer.files);
    setFiles(prev => [...prev, ...dropped]);
  }, []);

  const removeFile = (index: number) => {
    setFiles(prev => prev.filter((_, i) => i !== index));
  };

  return (
    <div className="space-y-4 mt-4">
      {/* Drop zone */}
      <div
        className={`border-2 border-dashed rounded-lg p-8 text-center transition-colors ${
          dragOver
            ? 'border-primary bg-primary/5'
            : 'border-border hover:border-muted-foreground/30'
        }`}
        onDragOver={e => { e.preventDefault(); setDragOver(true); }}
        onDragLeave={() => setDragOver(false)}
        onDrop={handleDrop}
      >
        <FileUp className="h-8 w-8 mx-auto text-muted-foreground mb-2" />
        <p className="text-sm text-muted-foreground mb-2">
          Drag & drop files here, or click to browse
        </p>
        <input
          type="file"
          multiple
          className="hidden"
          id="cloud-upload-input"
          onChange={e => {
            if (e.target.files) setFiles(prev => [...prev, ...Array.from(e.target.files!)]);
          }}
        />
        <Button
          variant="outline"
          size="sm"
          onClick={() => document.getElementById('cloud-upload-input')?.click()}
        >
          <Upload className="h-3.5 w-3.5 mr-1.5" />
          Choose Files
        </Button>
      </div>

      {/* Options */}
      <div className="flex items-center gap-3">
        <div className="flex items-center gap-1.5">
          <Label className="text-xs">Visibility:</Label>
          <Select value={visibility} onValueChange={setVisibility}>
            <SelectTrigger className="w-[120px] h-8 text-xs">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="org" className="text-xs">Organization</SelectItem>
              <SelectItem value="project" className="text-xs">Project</SelectItem>
              <SelectItem value="private" className="text-xs">Private</SelectItem>
            </SelectContent>
          </Select>
        </div>
      </div>

      {/* File list */}
      {files.length > 0 && (
        <div className="space-y-2">
          {files.map((file, i) => (
            <div key={`${file.name}-${i}`} className="flex items-center gap-2 p-2 bg-muted/50 rounded text-xs">
              <FileUp className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
              <span className="truncate flex-1">{file.name}</span>
              <span className="text-muted-foreground shrink-0">
                {(file.size / 1024).toFixed(0)} KB
              </span>
              <Button
                variant="ghost" size="sm"
                onClick={() => removeFile(i)}
                className="h-5 w-5 p-0"
              >
                <X className="h-3 w-3" />
              </Button>
            </div>
          ))}

          <Button
            onClick={handleUploadAll}
            disabled={uploadMutation.isPending}
            className="w-full"
            size="sm"
          >
            {uploadMutation.isPending ? (
              <><Loader2 className="h-3.5 w-3.5 mr-1.5 animate-spin" />Uploading...</>
            ) : (
              <><Upload className="h-3.5 w-3.5 mr-1.5" />Upload {files.length} file{files.length > 1 ? 's' : ''}</>
            )}
          </Button>
        </div>
      )}
    </div>
  );
}
