import { useState, useCallback } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Badge } from '@/components/ui/badge';
import {
  GripVertical,
  Plus,
  Trash2,
  Sparkles,
  Hash,
  Image as ImageIcon,
  Type,
  Megaphone,
  Smile,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { ContentBlock, ContentBlockType } from '@/types/social';

interface ContentBlockEditorProps {
  blocks: ContentBlock[];
  onChange: (blocks: ContentBlock[]) => void;
  onGenerateAI?: (blockType: ContentBlockType) => Promise<string>;
  readOnly?: boolean;
  className?: string;
}

const blockTypeConfig: Record<ContentBlockType, {
  label: string;
  icon: React.ReactNode;
  placeholder: string;
  color: string;
}> = {
  hook: {
    label: 'Hook',
    icon: <Sparkles className="h-4 w-4" />,
    placeholder: 'Write an attention-grabbing opening...',
    color: 'bg-yellow-100 text-yellow-800 border-yellow-300',
  },
  body: {
    label: 'Body',
    icon: <Type className="h-4 w-4" />,
    placeholder: 'Main content goes here...',
    color: 'bg-blue-100 text-blue-800 border-blue-300',
  },
  cta: {
    label: 'Call to Action',
    icon: <Megaphone className="h-4 w-4" />,
    placeholder: 'What action should readers take?',
    color: 'bg-green-100 text-green-800 border-green-300',
  },
  hashtags: {
    label: 'Hashtags',
    icon: <Hash className="h-4 w-4" />,
    placeholder: '#hashtag1 #hashtag2 #hashtag3',
    color: 'bg-purple-100 text-purple-800 border-purple-300',
  },
  media: {
    label: 'Media',
    icon: <ImageIcon className="h-4 w-4" />,
    placeholder: 'Add image or video URLs...',
    color: 'bg-pink-100 text-pink-800 border-pink-300',
  },
  emoji: {
    label: 'Emoji',
    icon: <Smile className="h-4 w-4" />,
    placeholder: 'Add emojis to enhance your post...',
    color: 'bg-orange-100 text-orange-800 border-orange-300',
  },
};

function generateBlockId(): string {
  return `block-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}

export function ContentBlockEditor({
  blocks,
  onChange,
  onGenerateAI,
  readOnly = false,
  className,
}: ContentBlockEditorProps) {
  const [draggedIndex, setDraggedIndex] = useState<number | null>(null);
  const [generatingBlock, setGeneratingBlock] = useState<string | null>(null);

  const handleAddBlock = useCallback((type: ContentBlockType) => {
    const newBlock: ContentBlock = {
      id: generateBlockId(),
      type,
      content: '',
    };
    onChange([...blocks, newBlock]);
  }, [blocks, onChange]);

  const handleUpdateBlock = useCallback((id: string, content: string) => {
    onChange(blocks.map(block =>
      block.id === id ? { ...block, content } : block
    ));
  }, [blocks, onChange]);

  const handleDeleteBlock = useCallback((id: string) => {
    onChange(blocks.filter(block => block.id !== id));
  }, [blocks, onChange]);

  const handleDragStart = useCallback((index: number) => {
    setDraggedIndex(index);
  }, []);

  const handleDragOver = useCallback((e: React.DragEvent, index: number) => {
    e.preventDefault();
    if (draggedIndex === null || draggedIndex === index) return;

    const newBlocks = [...blocks];
    const draggedBlock = newBlocks[draggedIndex];
    newBlocks.splice(draggedIndex, 1);
    newBlocks.splice(index, 0, draggedBlock);
    onChange(newBlocks);
    setDraggedIndex(index);
  }, [blocks, draggedIndex, onChange]);

  const handleDragEnd = useCallback(() => {
    setDraggedIndex(null);
  }, []);

  const handleGenerateAI = useCallback(async (block: ContentBlock) => {
    if (!onGenerateAI) return;

    setGeneratingBlock(block.id);
    try {
      const generated = await onGenerateAI(block.type);
      handleUpdateBlock(block.id, generated);
    } finally {
      setGeneratingBlock(null);
    }
  }, [onGenerateAI, handleUpdateBlock]);

  const totalCharacters = blocks.reduce((sum, block) => sum + block.content.length, 0);

  return (
    <Card className={cn('w-full', className)}>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <CardTitle className="text-lg">Content Blocks</CardTitle>
          <Badge variant="outline" className="font-mono">
            {totalCharacters} chars
          </Badge>
        </div>
      </CardHeader>
      <CardContent className="space-y-3">
        {blocks.map((block, index) => {
          const config = blockTypeConfig[block.type];
          const isGenerating = generatingBlock === block.id;

          return (
            <div
              key={block.id}
              draggable={!readOnly}
              onDragStart={() => handleDragStart(index)}
              onDragOver={(e) => handleDragOver(e, index)}
              onDragEnd={handleDragEnd}
              className={cn(
                'relative rounded-lg border transition-all',
                config.color,
                draggedIndex === index && 'opacity-50 scale-95',
                !readOnly && 'cursor-grab active:cursor-grabbing'
              )}
            >
              <div className="flex items-center gap-2 px-3 py-2 border-b border-inherit">
                {!readOnly && (
                  <GripVertical className="h-4 w-4 text-muted-foreground" />
                )}
                <div className="flex items-center gap-1.5">
                  {config.icon}
                  <span className="text-sm font-medium">{config.label}</span>
                </div>
                <div className="ml-auto flex items-center gap-1">
                  <Badge variant="secondary" className="text-xs font-mono">
                    {block.content.length}
                  </Badge>
                  {onGenerateAI && !readOnly && (
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => handleGenerateAI(block)}
                      disabled={isGenerating}
                      className="h-7 px-2"
                    >
                      <Sparkles className={cn(
                        'h-3 w-3',
                        isGenerating && 'animate-spin'
                      )} />
                    </Button>
                  )}
                  {!readOnly && (
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => handleDeleteBlock(block.id)}
                      className="h-7 px-2 text-destructive hover:text-destructive"
                    >
                      <Trash2 className="h-3 w-3" />
                    </Button>
                  )}
                </div>
              </div>
              <div className="p-3">
                <Textarea
                  value={block.content}
                  onChange={(e) => handleUpdateBlock(block.id, e.target.value)}
                  placeholder={config.placeholder}
                  readOnly={readOnly}
                  className={cn(
                    'min-h-[80px] resize-none border-0 bg-transparent p-0 focus-visible:ring-0',
                    readOnly && 'cursor-default'
                  )}
                />
              </div>
            </div>
          );
        })}

        {!readOnly && (
          <div className="flex flex-wrap gap-2 pt-2">
            {(Object.keys(blockTypeConfig) as ContentBlockType[]).map((type) => {
              const config = blockTypeConfig[type];
              const hasType = blocks.some(b => b.type === type);

              return (
                <Button
                  key={type}
                  size="sm"
                  variant="outline"
                  onClick={() => handleAddBlock(type)}
                  className={cn(
                    'gap-1.5',
                    hasType && 'opacity-60'
                  )}
                >
                  <Plus className="h-3 w-3" />
                  {config.icon}
                  {config.label}
                </Button>
              );
            })}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
