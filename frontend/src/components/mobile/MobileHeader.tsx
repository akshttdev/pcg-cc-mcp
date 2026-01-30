import { useState } from 'react';
import { Menu, Bell, User, Wifi, WifiOff } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from '@/components/ui/sheet';
import { Badge } from '@/components/ui/badge';

interface MobileHeaderProps {
  title?: string;
  showMeshStatus?: boolean;
  meshConnected?: boolean;
  notificationCount?: number;
}

export function MobileHeader({
  title = 'Vibertas',
  showMeshStatus = true,
  meshConnected = false,
  notificationCount = 0,
}: MobileHeaderProps) {
  const [menuOpen, setMenuOpen] = useState(false);

  return (
    <header className="fixed top-0 left-0 right-0 z-50 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 border-b border-border safe-area-top">
      <div className="flex items-center justify-between h-14 px-4">
        {/* Menu Button */}
        <Sheet open={menuOpen} onOpenChange={setMenuOpen}>
          <SheetTrigger asChild>
            <Button variant="ghost" size="icon" className="touch-manipulation">
              <Menu className="h-5 w-5" />
            </Button>
          </SheetTrigger>
          <SheetContent side="left" className="w-[280px]">
            <SheetHeader>
              <SheetTitle>Vibertas</SheetTitle>
            </SheetHeader>
            <nav className="flex flex-col gap-2 mt-4">
              <MobileMenuLink href="/" label="Dashboard" onClick={() => setMenuOpen(false)} />
              <MobileMenuLink href="/projects" label="Projects" onClick={() => setMenuOpen(false)} />
              <MobileMenuLink href="/tasks" label="Tasks" onClick={() => setMenuOpen(false)} />
              <MobileMenuLink href="/mesh" label="Mesh Network" onClick={() => setMenuOpen(false)} />
              <MobileMenuLink href="/agents" label="AI Agents" onClick={() => setMenuOpen(false)} />
              <MobileMenuLink href="/vibe" label="Vibe Treasury" onClick={() => setMenuOpen(false)} />
              <div className="border-t border-border my-2" />
              <MobileMenuLink href="/settings" label="Settings" onClick={() => setMenuOpen(false)} />
            </nav>
          </SheetContent>
        </Sheet>

        {/* Title with Mesh Status */}
        <div className="flex items-center gap-2">
          <h1 className="text-lg font-semibold">{title}</h1>
          {showMeshStatus && (
            <Badge
              variant={meshConnected ? 'default' : 'secondary'}
              className="h-5 px-1.5"
            >
              {meshConnected ? (
                <Wifi className="h-3 w-3" />
              ) : (
                <WifiOff className="h-3 w-3" />
              )}
            </Badge>
          )}
        </div>

        {/* Right Actions */}
        <div className="flex items-center gap-1">
          <Button variant="ghost" size="icon" className="relative touch-manipulation">
            <Bell className="h-5 w-5" />
            {notificationCount > 0 && (
              <span className="absolute -top-1 -right-1 h-4 w-4 bg-destructive text-destructive-foreground text-xs rounded-full flex items-center justify-center">
                {notificationCount > 9 ? '9+' : notificationCount}
              </span>
            )}
          </Button>
          <Button variant="ghost" size="icon" className="touch-manipulation">
            <User className="h-5 w-5" />
          </Button>
        </div>
      </div>
    </header>
  );
}

function MobileMenuLink({
  href,
  label,
  onClick,
}: {
  href: string;
  label: string;
  onClick: () => void;
}) {
  return (
    <a
      href={href}
      onClick={(e) => {
        e.preventDefault();
        window.location.href = href;
        onClick();
      }}
      className="flex items-center h-10 px-3 rounded-md text-sm font-medium hover:bg-accent transition-colors touch-manipulation"
    >
      {label}
    </a>
  );
}

export default MobileHeader;
