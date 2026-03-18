import { useCallback } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { X, Download, Trash2, Play } from 'lucide-react';
import { dataSourcesApi, type DataSourceRecord } from '@/lib/api';
import { formatDate } from '@/lib/formatters';
import { fileIcon } from './FileIcon';
import { parseMetadata, getFolderContext, formatSize, hasLocalFile } from './types';

export function PreviewPanel({ source, onClose, onDelete, onRunWorkflow }: {
  source: DataSourceRecord;
  onClose: () => void;
  onDelete: (id: string) => void;
  onRunWorkflow?: (source: DataSourceRecord) => void;
}) {
  const meta = parseMetadata(source.metadata);
  const ext = (source.file_type || '').toLowerCase();
  const isImage = ['jpg', 'jpeg', 'png', 'gif', 'webp', 'svg'].includes(ext);
  const isText = source.source_type === 'text' || ['txt', 'md', 'csv'].includes(ext);
  const downloadable = hasLocalFile(source);

  const handleDownload = useCallback(() => {
    const url = dataSourcesApi.downloadUrl(source.id);
    const a = document.createElement('a');
    a.href = url;
    a.download = source.title;
    a.click();
  }, [source]);

  return (
    <div className="w-80 shrink-0 border-l bg-card flex flex-col h-full">
      {/* Header */}
      <div className="flex items-start gap-2 p-4 border-b">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1">
            {fileIcon(source)}
            <span className="text-sm font-medium truncate">{source.title}</span>
          </div>
          <div className="flex items-center gap-2 flex-wrap">
            <Badge variant="outline" className="text-[10px] capitalize">{source.data_type}</Badge>
            {source.file_type && <Badge variant="outline" className="text-[10px] uppercase">{source.file_type}</Badge>}
            {!downloadable && <Badge variant="secondary" className="text-[10px]">No local file</Badge>}
          </div>
        </div>
        <button onClick={onClose} className="p-1 rounded hover:bg-muted shrink-0">
          <X className="h-4 w-4 text-muted-foreground" />
        </button>
      </div>

      {/* Preview area */}
      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        {/* Visual preview */}
        {isImage && downloadable ? (
          <div className="rounded-lg overflow-hidden border bg-muted/30">
            <img
              src={dataSourcesApi.downloadUrl(source.id)}
              alt={source.title}
              className="w-full object-contain max-h-48"
              onError={(e) => { (e.target as HTMLImageElement).style.display = 'none'; }}
            />
          </div>
        ) : (
          <div className="rounded-lg border bg-muted/20 flex items-center justify-center h-32">
            {fileIcon(source, 'lg')}
          </div>
        )}

        {/* Summary / text preview */}
        {(source as any).summary && (
          <div>
            <p className="text-[11px] font-semibold text-muted-foreground uppercase tracking-wide mb-1">Summary</p>
            <p className="text-sm text-muted-foreground leading-relaxed">{(source as any).summary}</p>
          </div>
        )}

        {/* Text content preview */}
        {isText && source.content && (
          <div>
            <p className="text-[11px] font-semibold text-muted-foreground uppercase tracking-wide mb-1">Content Preview</p>
            <pre className="text-xs bg-muted/40 rounded-md p-3 overflow-auto max-h-40 whitespace-pre-wrap font-mono">
              {source.content.slice(0, 800)}{source.content.length > 800 ? '...' : ''}
            </pre>
          </div>
        )}

        {/* Metadata */}
        <div>
          <p className="text-[11px] font-semibold text-muted-foreground uppercase tracking-wide mb-2">Details</p>
          <div className="space-y-1.5 text-sm">
            {source.file_size_bytes && (
              <div className="flex justify-between">
                <span className="text-muted-foreground">Size</span>
                <span className="font-mono text-xs">{formatSize(source.file_size_bytes)}</span>
              </div>
            )}
            <div className="flex justify-between">
              <span className="text-muted-foreground">Type</span>
              <span className="capitalize">{source.source_type}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-muted-foreground">Added</span>
              <span>{formatDate(source.created_at)}</span>
            </div>
            {meta.dropbox_path && (
              <div className="flex flex-col gap-0.5">
                <span className="text-muted-foreground">Dropbox path</span>
                <span className="text-xs text-muted-foreground/70 truncate font-mono">{meta.dropbox_path}</span>
              </div>
            )}
            {getFolderContext(source) && (
              <div className="flex flex-col gap-0.5">
                <span className="text-muted-foreground">Location</span>
                <span className="text-xs text-muted-foreground/70">{getFolderContext(source)}</span>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Actions */}
      <div className="p-4 border-t flex flex-col gap-2">
        {source.status === 'ready' && onRunWorkflow && (
          <Button size="sm" variant="outline" className="w-full" onClick={() => onRunWorkflow(source)}>
            <Play className="h-3.5 w-3.5 mr-2" />
            Run Workflow
          </Button>
        )}
        {downloadable ? (
          <Button size="sm" className="w-full" onClick={handleDownload}>
            <Download className="h-3.5 w-3.5 mr-2" />
            Download
          </Button>
        ) : (
          <Button size="sm" variant="outline" className="w-full" disabled>
            <Download className="h-3.5 w-3.5 mr-2" />
            Not Available Locally
          </Button>
        )}
        <Button
          size="sm"
          variant="outline"
          className="w-full text-destructive hover:text-destructive"
          onClick={() => {
            if (confirm(`Delete "${source.title}"?`)) onDelete(source.id);
          }}
        >
          <Trash2 className="h-3.5 w-3.5 mr-2" />
          Delete
        </Button>
      </div>
    </div>
  );
}
