import { useState, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { LogOut, ChevronUp, RotateCcw, Eye } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useAuth } from '@/contexts/AuthContext';
import { useViewContext } from '@/contexts/view-context';
import { ROLE_LABELS, type EffectiveRole } from '@/lib/roles';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { Badge } from '@/components/ui/badge';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { useKeyToggleViewAs, Scope } from '@/keyboard';

function getInitials(name: string): string {
  return name
    .split(' ')
    .map((part) => part[0])
    .join('')
    .toUpperCase()
    .slice(0, 2);
}

interface SidebarUserCardProps {
  isCollapsed: boolean;
}

export function SidebarUserCard({ isCollapsed }: SidebarUserCardProps) {
  const { user, logout } = useAuth();
  const navigate = useNavigate();
  const {
    viewAsRole,
    setViewAsRole,
    clearViewAs,
    isOverridden,
    availableRoleGroups,
    naturalRole,
  } = useViewContext();

  const [popoverOpen, setPopoverOpen] = useState(false);

  // Keyboard shortcut: Cmd+Shift+V toggles the view-as popover
  const handleToggleViewAs = useCallback(() => {
    setPopoverOpen((prev) => !prev);
  }, []);
  useKeyToggleViewAs(handleToggleViewAs, { scope: Scope.GLOBAL, preventDefault: true });

  if (!user) return null;

  const initials = getInitials(user.full_name);
  const displayRole = isOverridden && viewAsRole ? viewAsRole : naturalRole;

  const handleLogout = async () => {
    await logout();
    navigate('/login');
  };

  const handleRoleSelect = (role: EffectiveRole) => {
    if (role === naturalRole) {
      clearViewAs();
    } else {
      setViewAsRole(role);
    }
  };

  const popoverContent = (
    <PopoverContent
      side={isCollapsed ? 'right' : 'top'}
      align="start"
      className="w-64 p-0"
    >
      {/* User info */}
      <div className="p-3 border-b border-border/40">
        <div className="flex items-center gap-3">
          <Avatar className="h-9 w-9 shrink-0">
            <AvatarImage src={user.avatar_url ?? undefined} alt={user.full_name} />
            <AvatarFallback className="text-sm font-medium">{initials}</AvatarFallback>
          </Avatar>
          <div className="min-w-0 flex-1">
            <div className="text-sm font-medium truncate">{user.full_name}</div>
            <div className="text-xs text-muted-foreground truncate">{user.email}</div>
          </div>
        </div>
        <div className="mt-2 flex items-center gap-1.5">
          <Badge variant="outline" className="text-xs px-1.5 py-0">
            {ROLE_LABELS[naturalRole]}
          </Badge>
          {isOverridden && viewAsRole && (
            <Badge variant="secondary" className="text-xs px-1.5 py-0 bg-amber-500/15 text-amber-600 border-amber-500/20">
              <Eye className="h-2.5 w-2.5 mr-0.5" />
              {ROLE_LABELS[viewAsRole]}
            </Badge>
          )}
        </div>
      </div>

      {/* View as selector */}
      {availableRoleGroups.length > 0 && (
        <div className="p-2 border-b border-border/40">
          <div className="text-xs font-medium text-muted-foreground uppercase tracking-wider px-2 mb-1">
            View as...
          </div>
          {availableRoleGroups.map(({ group, label, roles }) => (
            <div key={group} className="mb-1 last:mb-0">
              <div className="text-xs text-muted-foreground/70 px-2 py-0.5 uppercase tracking-wider">
                {label}
              </div>
              {roles.map((role) => (
                <button
                  key={role}
                  onClick={() => handleRoleSelect(role)}
                  className={cn(
                    'w-full text-left px-2 py-1 text-sm rounded-md transition-colors',
                    'hover:bg-accent/60',
                    viewAsRole === role
                      ? 'bg-accent text-accent-foreground font-medium'
                      : 'text-foreground/80'
                  )}
                >
                  {ROLE_LABELS[role]}
                </button>
              ))}
            </div>
          ))}

          {isOverridden && (
            <button
              onClick={clearViewAs}
              className="w-full flex items-center gap-2 px-2 py-1.5 mt-1 text-sm text-muted-foreground hover:text-foreground hover:bg-accent/60 rounded-md transition-colors"
            >
              <RotateCcw className="h-3 w-3" />
              Reset to my role
            </button>
          )}
        </div>
      )}

      {/* Sign out */}
      <div className="p-2">
        <button
          onClick={handleLogout}
          className="w-full flex items-center gap-2 px-2 py-1.5 text-sm text-red-600 hover:bg-red-500/10 rounded-md transition-colors"
        >
          <LogOut className="h-3.5 w-3.5" />
          Sign out
        </button>
      </div>
    </PopoverContent>
  );

  // Collapsed mode — avatar only
  if (isCollapsed) {
    return (
      <Popover open={popoverOpen} onOpenChange={setPopoverOpen}>
        <Tooltip>
          <TooltipTrigger asChild>
            <PopoverTrigger asChild>
              <button className="relative w-full flex justify-center p-2">
                <Avatar className="h-8 w-8">
                  <AvatarImage src={user.avatar_url ?? undefined} alt={user.full_name} />
                  <AvatarFallback className="text-xs font-medium">{initials}</AvatarFallback>
                </Avatar>
                {isOverridden && (
                  <div className="absolute top-1.5 right-1.5 h-2.5 w-2.5 rounded-full bg-amber-500 border-2 border-background" />
                )}
              </button>
            </PopoverTrigger>
          </TooltipTrigger>
          <TooltipContent side="right">
            {user.full_name}
            {isOverridden && viewAsRole && ` · Viewing as ${ROLE_LABELS[viewAsRole]}`}
          </TooltipContent>
        </Tooltip>
        {popoverContent}
      </Popover>
    );
  }

  // Expanded mode
  return (
    <Popover open={popoverOpen} onOpenChange={setPopoverOpen}>
      <PopoverTrigger asChild>
        <button className="w-full flex items-center gap-3 px-3 py-2 rounded-lg hover:bg-accent/60 transition-colors text-left">
          <div className="relative shrink-0">
            <Avatar className="h-8 w-8">
              <AvatarImage src={user.avatar_url ?? undefined} alt={user.full_name} />
              <AvatarFallback className="text-xs font-medium">{initials}</AvatarFallback>
            </Avatar>
            {isOverridden && (
              <div className="absolute -top-0.5 -right-0.5 h-2.5 w-2.5 rounded-full bg-amber-500 border-2 border-background" />
            )}
          </div>
          <div className="flex-1 min-w-0">
            <div className="text-sm font-medium truncate">{user.full_name}</div>
            <div className="text-xs text-muted-foreground truncate">
              {ROLE_LABELS[displayRole]}
            </div>
          </div>
          <ChevronUp className="h-4 w-4 text-muted-foreground shrink-0" />
        </button>
      </PopoverTrigger>
      {popoverContent}
    </Popover>
  );
}
