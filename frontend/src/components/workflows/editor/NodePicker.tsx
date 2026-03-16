import { X } from 'lucide-react';
import { cn } from '@/lib/utils';
import { NODE_TYPES } from './node-types';

interface NodePickerProps {
  onSelect: (type: string) => void;
  onClose: () => void;
}

export function NodePicker({ onSelect, onClose }: NodePickerProps) {
  const sourceNodes = NODE_TYPES.filter(nt => nt.type === 'data_source');
  const llmNodes = NODE_TYPES.filter(nt => nt.type.startsWith('llm_'));
  const transformNodes = NODE_TYPES.filter(nt => ['transform', 'filter', 'merge', 'conditional'].includes(nt.type));
  const actionNodes = NODE_TYPES.filter(nt => ['send_notification', 'assign_to_agent', 'http_request', 'update_crm_contact', 'update_crm_deal', 'update_crm_company'].includes(nt.type));
  const outputNodes = NODE_TYPES.filter(nt => nt.type.startsWith('output_'));
  const processingNodes = [...sourceNodes, ...llmNodes, ...transformNodes];

  return (
    <div className="rounded-lg border bg-card shadow-lg p-2 space-y-1">
      <div className="flex items-center justify-between px-2 pb-1">
        <span className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
          Add Node
        </span>
        <button
          onClick={onClose}
          className="p-0.5 rounded hover:bg-muted transition-colors"
        >
          <X className="h-3.5 w-3.5" />
        </button>
      </div>
      {processingNodes.map((nt) => {
        const Icon = nt.icon;
        return (
          <button
            key={nt.type}
            onClick={() => onSelect(nt.type)}
            className={cn(
              'w-full flex items-center gap-3 rounded-md px-2 py-2',
              'hover:bg-accent transition-colors text-left'
            )}
          >
            <div
              className={cn(
                'w-8 h-8 rounded-md flex items-center justify-center text-white shrink-0',
                nt.color
              )}
            >
              <Icon className="h-4 w-4" />
            </div>
            <div className="min-w-0">
              <div className="text-sm font-medium">{nt.label}</div>
              <div className="text-xs text-muted-foreground">
                {nt.description}
              </div>
            </div>
          </button>
        );
      })}
      {actionNodes.length > 0 && (
        <>
          <div className="border-t my-1" />
          <div className="px-2 py-0.5">
            <span className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">Actions</span>
          </div>
          {actionNodes.map((nt) => {
            const Icon = nt.icon;
            return (
              <button
                key={nt.type}
                onClick={() => onSelect(nt.type)}
                className={cn(
                  'w-full flex items-center gap-3 rounded-md px-2 py-2',
                  'hover:bg-accent transition-colors text-left'
                )}
              >
                <div className={cn('w-8 h-8 rounded-md flex items-center justify-center text-white shrink-0', nt.color)}>
                  <Icon className="h-4 w-4" />
                </div>
                <div className="min-w-0">
                  <div className="text-sm font-medium">{nt.label}</div>
                  <div className="text-xs text-muted-foreground">{nt.description}</div>
                </div>
              </button>
            );
          })}
        </>
      )}
      {outputNodes.length > 0 && (
        <>
          <div className="border-t my-1" />
          <div className="px-2 py-0.5">
            <span className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">Outputs</span>
          </div>
          {outputNodes.map((nt) => {
            const Icon = nt.icon;
            return (
              <button
                key={nt.type}
                onClick={() => onSelect(nt.type)}
                className={cn(
                  'w-full flex items-center gap-3 rounded-md px-2 py-2',
                  'hover:bg-accent transition-colors text-left'
                )}
              >
                <div
                  className={cn(
                    'w-8 h-8 rounded-md flex items-center justify-center text-white shrink-0',
                    nt.color
                  )}
                >
                  <Icon className="h-4 w-4" />
                </div>
                <div className="min-w-0">
                  <div className="text-sm font-medium">{nt.label}</div>
                  <div className="text-xs text-muted-foreground">
                    {nt.description}
                  </div>
                </div>
              </button>
            );
          })}
        </>
      )}
    </div>
  );
}
