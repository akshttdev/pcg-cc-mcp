import { useNavigate } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { Badge } from '@/components/ui/badge';
import {
  User,
  Settings,
  LogOut,
  Shield,
  Activity,
  Users,
  Wallet,
  Building2,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import NiceModal from '@ebay/nice-modal-react';
import { useAuth } from '@/contexts/AuthContext';

interface ProfileSectionProps {
  className?: string;
}

const getInitials = (name: string): string => {
  return name
    .split(' ')
    .map(part => part[0])
    .join('')
    .toUpperCase()
    .slice(0, 2);
};

const getStatusColor = (status: string) => {
  switch (status) {
    case 'online':
      return 'bg-green-500';
    case 'away':
      return 'bg-yellow-500';
    case 'busy':
      return 'bg-red-500';
    case 'offline':
      return 'bg-gray-400';
    default:
      return 'bg-gray-400';
  }
};

const getStatusText = (status: string) => {
  switch (status) {
    case 'online':
      return 'Online';
    case 'away':
      return 'Away';
    case 'busy':
      return 'Busy';
    case 'offline':
      return 'Offline';
    default:
      return 'Unknown';
  }
};

export function ProfileSection({ className }: ProfileSectionProps) {
  const { user, logout } = useAuth();
  const navigate = useNavigate();

  if (!user) {
    return null;
  }

  const userInitials = getInitials(user.full_name);
  const status = 'online'; // Default to online for now

  const handleLogout = async () => {
    await logout();
    navigate('/login');
  };

  const handleProfile = () => {
    // Navigate to profile settings
    navigate('/settings/profile');
  };

  const handleSettings = () => {
    // Navigate to general settings
    navigate('/settings/general');
  };

  const handlePrivacySecurity = () => {
    // Navigate to privacy settings
    navigate('/settings/privacy');
  };

  const handleActivityLog = () => {
    // Navigate to activity log
    navigate('/settings/activity');
  };

  const handleWallet = () => {
    navigate('/settings/wallet');
  };

  const handleTeamManagement = () => {
    NiceModal.show('team-management');
  };

  return (
    <div className={cn("flex items-center gap-4 min-w-0", className)}>
      {/* Status indicator */}
      <div className="hidden lg:flex items-center gap-2 text-sm text-muted-foreground">
        <div className="flex items-center gap-1.5">
          <div className={cn("h-2 w-2 rounded-full", getStatusColor(status))} />
          <span className="text-sm">{getStatusText(status)}</span>
        </div>
      </div>

      {/* Profile Dropdown */}
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button
            variant="ghost"
            className="relative flex items-center gap-3 px-3 py-2 h-auto hover:bg-accent rounded-lg"
          >
            <div className="relative">
              <Avatar className="h-9 w-9">
                <AvatarImage src={undefined} alt={user.full_name} />
                <AvatarFallback className="text-sm font-medium">
                  {userInitials}
                </AvatarFallback>
              </Avatar>
              {/* Status dot overlay */}
              <div
                className={cn(
                  "absolute -bottom-0.5 -right-0.5 h-3 w-3 rounded-full border-2 border-background",
                  getStatusColor(status)
                )}
              />
            </div>
            <div className="hidden lg:block text-left min-w-0">
              <div className="text-sm font-medium truncate">{user.full_name}</div>
              <div className="text-xs text-muted-foreground truncate">{user.username}</div>
            </div>
          </Button>
        </DropdownMenuTrigger>

        <DropdownMenuContent align="end" className="w-64">
          <DropdownMenuLabel className="pb-2">
            <div className="flex items-center gap-3">
              <Avatar className="h-10 w-10">
                <AvatarImage src={undefined} alt={user.full_name} />
                <AvatarFallback>{userInitials}</AvatarFallback>
              </Avatar>
              <div className="flex-1">
                <div className="font-medium">{user.full_name}</div>
                <div className="text-sm text-muted-foreground">{user.email}</div>
                <div className="flex items-center gap-2 mt-1">
                  {user.is_admin && (
                    <Badge variant="secondary" className="text-xs">
                      Admin
                    </Badge>
                  )}
                  <div className="flex items-center gap-1 text-xs text-muted-foreground">
                    <div className={cn("h-1.5 w-1.5 rounded-full", getStatusColor(status))} />
                    {getStatusText(status)}
                  </div>
                </div>
              </div>
            </div>
          </DropdownMenuLabel>

          <DropdownMenuSeparator />

          <DropdownMenuItem onClick={handleProfile} className="cursor-pointer">
            <User className="mr-2 h-4 w-4" />
            <span>Profile</span>
          </DropdownMenuItem>

          <DropdownMenuItem onClick={handleSettings} className="cursor-pointer">
            <Settings className="mr-2 h-4 w-4" />
            <span>Preferences</span>
          </DropdownMenuItem>

          <DropdownMenuItem onClick={handlePrivacySecurity} className="cursor-pointer">
            <Shield className="mr-2 h-4 w-4" />
            <span>Privacy & Security</span>
          </DropdownMenuItem>

          <DropdownMenuItem onClick={handleWallet} className="cursor-pointer">
            <Wallet className="mr-2 h-4 w-4" />
            <span>Wallet</span>
          </DropdownMenuItem>

          <DropdownMenuItem onClick={handleActivityLog} className="cursor-pointer">
            <Activity className="mr-2 h-4 w-4" />
            <span>Activity Log</span>
          </DropdownMenuItem>

          <DropdownMenuSeparator />

          {/* Organizations */}
          {user.organizations && user.organizations.length > 0 && (
            <>
              <DropdownMenuLabel className="text-xs text-muted-foreground">
                Organizations
              </DropdownMenuLabel>
              {user.organizations.map((org) => (
                <DropdownMenuItem key={org.id} className="cursor-pointer">
                  <Building2 className="mr-2 h-4 w-4" />
                  <div className="flex-1">
                    <div className="text-sm">{org.name}</div>
                    <div className="text-xs text-muted-foreground capitalize">{org.role}</div>
                  </div>
                </DropdownMenuItem>
              ))}
              <DropdownMenuSeparator />
            </>
          )}

          <DropdownMenuItem onClick={handleTeamManagement} className="cursor-pointer">
            <Users className="mr-2 h-4 w-4" />
            <span>Team Management</span>
          </DropdownMenuItem>

          {user.is_admin && (
            <>
              <DropdownMenuSeparator />
              <DropdownMenuItem onClick={() => navigate('/admin')} className="cursor-pointer">
                <Shield className="mr-2 h-4 w-4" />
                <span>Admin Panel</span>
              </DropdownMenuItem>
            </>
          )}

          <DropdownMenuSeparator />

          <DropdownMenuItem
            onClick={handleLogout}
            className="cursor-pointer text-red-600 focus:text-red-600"
          >
            <LogOut className="mr-2 h-4 w-4" />
            <span>Sign out</span>
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}
