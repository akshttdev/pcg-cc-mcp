import type { DataSourceRecord } from '@/lib/api';

export interface FolderNode {
  name: string;
  path: string;
  children: Record<string, FolderNode>;
  count: number;
}

export type SortField = 'title' | 'data_type' | 'file_size' | 'created_at';
export type SortDir = 'asc' | 'desc';

// ─── Helpers ──────────────────────────────────────────────────────────────────

export function parseMetadata(raw: string): Record<string, any> {
  try { return JSON.parse(raw); } catch { return {}; }
}

export function getFolderContext(source: DataSourceRecord): string {
  // Use the folder column (slash-delimited) as the canonical source
  if (source.folder) {
    return source.folder.split('/').map((p: string) => p.trim()).filter(Boolean).join(' > ');
  }
  const meta = parseMetadata(source.metadata);
  return meta.folder_context || '';
}

export function buildFolderTree(sources: DataSourceRecord[]): FolderNode {
  const root: FolderNode = { name: 'root', path: '', children: {}, count: 0 };
  for (const s of sources) {
    const ctx = getFolderContext(s);
    if (!ctx) continue;
    const parts = ctx.split(' > ').map((p: string) => p.trim()).filter(Boolean);
    let node = root;
    let pathSoFar = '';
    for (const part of parts) {
      pathSoFar = pathSoFar ? `${pathSoFar} > ${part}` : part;
      if (!node.children[part]) {
        node.children[part] = { name: part, path: pathSoFar, children: {}, count: 0 };
      }
      node.children[part].count++;
      node = node.children[part];
    }
  }
  return root;
}

export function formatSize(bytes?: number | null): string {
  if (!bytes) return '—';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

export function hasLocalFile(source: DataSourceRecord): boolean {
  const meta = parseMetadata(source.metadata);
  return !!(meta.file_path || source.file_path || source.source_type === 'text');
}
