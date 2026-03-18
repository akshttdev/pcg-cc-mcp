import {
  FileText, Table2, Music, Image,
  Film, Layers, MessageSquare, File, Palette,
} from 'lucide-react';
import type { DataSourceRecord } from '@/lib/api';

export function fileIcon(source: DataSourceRecord, size = 'sm') {
  const cls = size === 'lg' ? 'h-10 w-10' : 'h-4 w-4';
  const ext = (source.file_type || '').toLowerCase();
  if (source.data_type === 'conversation') return <MessageSquare className={`${cls} text-amber-500`} />;
  if (['pdf'].includes(ext)) return <FileText className={`${cls} text-red-500`} />;
  if (['doc', 'docx', 'rtf', 'pages', 'txt', 'md'].includes(ext)) return <FileText className={`${cls} text-blue-500`} />;
  if (['xls', 'xlsx', 'csv', 'numbers'].includes(ext)) return <Table2 className={`${cls} text-green-500`} />;
  if (['ppt', 'pptx', 'key'].includes(ext)) return <FileText className={`${cls} text-orange-500`} />;
  if (['mp3', 'wav', 'aiff', 'm4a', 'flac', 'ogg'].includes(ext)) return <Music className={`${cls} text-purple-500`} />;
  if (['jpg', 'jpeg', 'png', 'gif', 'heic', 'tiff', 'tif', 'webp', 'svg'].includes(ext)) return <Image className={`${cls} text-amber-500`} />;
  if (['cr3', 'cr2', 'arw', 'dng', 'raw', 'nef'].includes(ext)) return <Image className={`${cls} text-amber-600`} />;
  if (['mp4', 'mov', 'avi', 'mkv'].includes(ext)) return <Film className={`${cls} text-blue-600`} />;
  if (['prproj', 'aep', 'ppro'].includes(ext)) return <Film className={`${cls} text-indigo-600`} />;
  if (['psd', 'psb', 'ai', 'sketch'].includes(ext)) return <Layers className={`${cls} text-indigo-500`} />;
  if (['cube', 'lrtemplate', 'xmp'].includes(ext)) return <Palette className={`${cls} text-pink-500`} />;
  return <File className={`${cls} text-muted-foreground`} />;
}
