import { useState, useRef, useEffect } from 'react';
import { Input } from '@/components/ui/input';
import { cn } from '@/lib/utils';
import { Check, X } from 'lucide-react';
import { Button } from '@/components/ui/button';

interface InlineEditProps {
  value: string;
  onSave: (value: string) => void;
  onCancel?: () => void;
  className?: string;
  inputClassName?: string;
  placeholder?: string;
  multiline?: boolean;
}

export function InlineEdit({
  value: initialValue,
  onSave,
  onCancel,
  className,
  inputClassName,
  placeholder = 'Enter text...',
  multiline = false,
}: InlineEditProps) {
  const [isEditing, setIsEditing] = useState(false);
  const [value, setValue] = useState(initialValue);
  const inputRef = useRef<HTMLInputElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    setValue(initialValue);
  }, [initialValue]);

  useEffect(() => {
    if (isEditing) {
      const ref = multiline ? textareaRef : inputRef;
      ref.current?.focus();
      ref.current?.select();
    }
  }, [isEditing, multiline]);

  const handleSave = () => {
    if (value.trim() !== initialValue) {
      onSave(value.trim());
    }
    setIsEditing(false);
  };

  const handleCancel = () => {
    setValue(initialValue);
    setIsEditing(false);
    onCancel?.();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey && !multiline) {
      e.preventDefault();
      handleSave();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      handleCancel();
    }
  };

  if (!isEditing) {
    return (
      <div
        className={cn(
          'cursor-pointer hover:bg-accent/50 px-2 py-1 rounded transition-colors',
          className
        )}
        onClick={() => setIsEditing(true)}
        title="Click to edit"
      >
        {value || placeholder}
      </div>
    );
  }

  return (
    <div className={cn('flex items-center gap-2', className)}>
      {multiline ? (
        <textarea
          ref={textareaRef}
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={handleKeyDown}
          onBlur={handleSave}
          className={cn(
            'flex min-h-[60px] w-full rounded-md border border-input bg-transparent px-3 py-2 text-sm shadow-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50',
            inputClassName
          )}
          placeholder={placeholder}
        />
      ) : (
        <Input
          ref={inputRef}
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={handleKeyDown}
          onBlur={handleSave}
          className={inputClassName}
          placeholder={placeholder}
        />
      )}
      <div className="flex items-center gap-1 shrink-0">
        <Button
          size="icon"
          variant="ghost"
          className="h-7 w-7"
          onClick={(e) => {
            e.stopPropagation();
            handleSave();
          }}
          title="Save"
        >
          <Check className="h-4 w-4 text-green-600" />
        </Button>
        <Button
          size="icon"
          variant="ghost"
          className="h-7 w-7"
          onClick={(e) => {
            e.stopPropagation();
            handleCancel();
          }}
          title="Cancel"
        >
          <X className="h-4 w-4 text-red-600" />
        </Button>
      </div>
    </div>
  );
}