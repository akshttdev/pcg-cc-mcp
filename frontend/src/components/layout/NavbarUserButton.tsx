import { useNavigate } from 'react-router-dom';
import { LogOut, RotateCcw, Eye } from 'lucide-react';
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

function getInitials(name: string): string {
  return name
    .split(' ')
    .map((part) => part[0])
    .join('')
    .toUpperCase()
    .slice(0, 2);
}

/**
 * Compact user avatar for the navbar — visible on mobile/tablet only.
 * Provides quick access to view-as switcher without opening the sidebar.
 */
export function NavbarUserButton() {
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

  if (!user) return null;

  const initials = getInitials(user.full_name);

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

  return (
    <Popover>
      <PopoverTrigger asChild>
        <button className="relative lg:hidden shrink-0" aria-label="User menu">
          <Avatar className="h-7 w-7">
            <AvatarImage src={user.avatar_url ?? undefined} alt={user.full_name} />
            <AvatarFallback className="text-xs font-medium">{initials}</AvatarFallback>
          </Avatar>
          {isOverridden && (
            <div className="absolute -top-0.5 -right-0.5 h-2 w-2 rounded-full bg-amber-500 border border-background" />
          )}
        </button>
      </PopoverTrigger>
      <PopoverContent side="bottom" align="end" className="w-64 p-0">
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
    </Popover>
  );
}
