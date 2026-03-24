import { Copy, Edit, ExternalLink, MoreHorizontal, Star,Trash2 } from 'lucide-react';

import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { cn } from '@/lib/utils';

import { IconButton } from './icon-button';

export interface HoverAction {
  id: string;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  onClick: () => void;
  variant?: 'default' | 'destructive';
  separator?: boolean;
}

interface HoverCardActionsProps {
  actions: HoverAction[];
  className?: string;
  triggerClassName?: string;
  showOnHover?: boolean;
}

export function HoverCardActions({
  actions,
  className,
  triggerClassName,
  showOnHover = true,
}: HoverCardActionsProps) {
  // Quick actions (shown as buttons)
  const quickActions = actions.slice(0, 2);
  // Menu actions (shown in dropdown)
  const menuActions = actions.slice(2);

  return (
    <div
      className={cn(
        'flex items-center gap-1',
        showOnHover && 'opacity-0 group-hover:opacity-100 transition-opacity',
        className
      )}
    >
      {/* Quick Action Buttons */}
      {quickActions.map((action) => {
        const Icon = action.icon;
        return (
          <IconButton
            key={action.id}
            variant="ghost" className="h-7 w-7"
            onClick={(e) => {
              e.stopPropagation();
              action.onClick();
            }}
            icon={Icon}
            label={action.label}
            iconClassName="h-3.5 w-3.5"
          />
        );
      })}

      {/* Dropdown Menu for More Actions */}
      {menuActions.length > 0 && (
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <IconButton
              variant="ghost" className={cn('h-7 w-7', triggerClassName)}
              onClick={(e) => e.stopPropagation()}
              icon={MoreHorizontal}
              label="More options"
              iconClassName="h-3.5 w-3.5"
            />
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            {menuActions.map((action) => {
              const Icon = action.icon;
              return (
                <div key={action.id}>
                  {action.separator && <DropdownMenuSeparator />}
                  <DropdownMenuItem
                    onClick={(e) => {
                      e.stopPropagation();
                      action.onClick();
                    }}
                    className={cn(
                      action.variant === 'destructive' &&
                        'text-destructive focus:text-destructive'
                    )}
                  >
                    <Icon className="mr-2 h-4 w-4" />
                    {action.label}
                  </DropdownMenuItem>
                </div>
              );
            })}
          </DropdownMenuContent>
        </DropdownMenu>
      )}
    </div>
  );
}

// Preset action configurations
export const createTaskActions = (handlers: {
  onEdit?: () => void;
  onDelete?: () => void;
  onDuplicate?: () => void;
  onOpen?: () => void;
  onFavorite?: () => void;
}): HoverAction[] => {
  const actions: HoverAction[] = [];

  if (handlers.onEdit) {
    actions.push({
      id: 'edit',
      label: 'Edit',
      icon: Edit,
      onClick: handlers.onEdit,
    });
  }

  if (handlers.onOpen) {
    actions.push({
      id: 'open',
      label: 'Open',
      icon: ExternalLink,
      onClick: handlers.onOpen,
    });
  }

  if (handlers.onDuplicate) {
    actions.push({
      id: 'duplicate',
      label: 'Duplicate',
      icon: Copy,
      onClick: handlers.onDuplicate,
    });
  }

  if (handlers.onFavorite) {
    actions.push({
      id: 'favorite',
      label: 'Add to favorites',
      icon: Star,
      onClick: handlers.onFavorite,
    });
  }

  if (handlers.onDelete) {
    actions.push({
      id: 'delete',
      label: 'Delete',
      icon: Trash2,
      onClick: handlers.onDelete,
      variant: 'destructive',
      separator: true,
    });
  }

  return actions;
};
