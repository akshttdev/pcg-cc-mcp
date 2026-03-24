import { useState, useCallback, useEffect } from 'react';
import MDEditor, { commands } from '@uiw/react-md-editor';
import { Card } from '@/components/ui/card';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Eye, Code, Split } from 'lucide-react';
import { cn } from '@/lib/utils';
import './rich-text-editor.css';

interface RichTextEditorProps {
  value?: string;
  onChange?: (value: string) => void;
  placeholder?: string;
  height?: number;
  readOnly?: boolean;
  enableToolbar?: boolean;
  className?: string;
}

export function RichTextEditor({
  value = '',
  onChange,
  placeholder = 'Enter description...',
  height = 400,
  readOnly = false,
  enableToolbar = true,
  className,
}: RichTextEditorProps) {
  const [localValue, setLocalValue] = useState(value);
  const [viewMode, setViewMode] = useState<'edit' | 'preview' | 'split'>('split');

  useEffect(() => {
    setLocalValue(value);
  }, [value]);

  const handleChange = useCallback(
    (newValue?: string) => {
      const val = newValue || '';
      setLocalValue(val);
      onChange?.(val);
    },
    [onChange]
  );

  // Custom toolbar commands
  const customCommands = [
    commands.bold,
    commands.italic,
    commands.strikethrough,
    commands.hr,
    commands.title,
    commands.divider,
    commands.link,
    commands.quote,
    commands.code,
    commands.codeBlock,
    commands.divider,
    commands.unorderedListCommand,
    commands.orderedListCommand,
    commands.checkedListCommand,
    commands.divider,
    commands.table,
    commands.divider,
    commands.help,
  ];

  if (readOnly) {
    return (
      <Card className={cn('p-4 overflow-auto', className)}>
        <MDEditor.Markdown
          source={localValue || placeholder}
          style={{ whiteSpace: 'pre-wrap' }}
        />
      </Card>
    );
  }

  return (
    <div className={cn('w-full', className)}>
      {enableToolbar && (
        <div className="flex items-center justify-between mb-2">
          <div className="flex items-center gap-2">
            <span className="text-sm font-medium text-muted-foreground">
              Markdown Editor
            </span>
          </div>
          <Tabs value={viewMode} onValueChange={(v) => setViewMode(v as 'edit' | 'preview' | 'split')}>
            <TabsList>
              <TabsTrigger value="edit" className="gap-1">
                <Code className="h-3 w-3" />
                <span className="hidden sm:inline">Edit</span>
              </TabsTrigger>
              <TabsTrigger value="split" className="gap-1">
                <Split className="h-3 w-3" />
                <span className="hidden sm:inline">Split</span>
              </TabsTrigger>
              <TabsTrigger value="preview" className="gap-1">
                <Eye className="h-3 w-3" />
                <span className="hidden sm:inline">Preview</span>
              </TabsTrigger>
            </TabsList>
          </Tabs>
        </div>
      )}

      <div className="border rounded-md overflow-hidden">
        <MDEditor
          value={localValue}
          onChange={handleChange}
          preview={viewMode === 'edit' ? 'edit' : viewMode === 'preview' ? 'preview' : 'live'}
          height={height}
          commands={customCommands}
          extraCommands={[]}
          visibleDragbar={false}
          textareaProps={{
            placeholder,
          }}
        />
      </div>

      {enableToolbar && (
        <div className="mt-2 text-xs text-muted-foreground">
          <p>
            Supports <strong>Markdown</strong> formatting: **bold**, *italic*, `code`,
            [links](url), lists, tables, and more.
          </p>
        </div>
      )}
    </div>
  );
}

// Compact version for inline editing
export function CompactRichTextEditor({
  value = '',
  onChange,
  placeholder = 'Add description...',
  className,
}: Omit<RichTextEditorProps, 'height' | 'enableToolbar' | 'readOnly'>) {
  return (
    <RichTextEditor
      value={value}
      onChange={onChange}
      placeholder={placeholder}
      height={200}
      enableToolbar={false}
      className={className}
    />
  );
}

// Read-only preview component
export function MarkdownPreview({
  value = '',
  className,
}: {
  value?: string;
  className?: string;
}) {
  return (
    <div className={cn('prose dark:prose-invert max-w-none', className)}>
      <MDEditor.Markdown source={value || '_No description provided_'} />
    </div>
  );
}
