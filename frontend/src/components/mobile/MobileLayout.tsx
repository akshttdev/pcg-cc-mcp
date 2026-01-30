import { ReactNode } from 'react';
import { MobileNav } from './MobileNav';
import { MobileHeader } from './MobileHeader';
import { useMobile } from '@/hooks/useMobile';

interface MobileLayoutProps {
  children: ReactNode;
  title?: string;
  showNav?: boolean;
  showHeader?: boolean;
  meshConnected?: boolean;
}

export function MobileLayout({
  children,
  title,
  showNav = true,
  showHeader = true,
  meshConnected = false,
}: MobileLayoutProps) {
  const { isMobile, isMobileApp } = useMobile();

  // Only use mobile layout on mobile devices
  if (!isMobile && !isMobileApp) {
    return <>{children}</>;
  }

  return (
    <div className="min-h-screen bg-background">
      {showHeader && (
        <MobileHeader
          title={title}
          meshConnected={meshConnected}
        />
      )}

      {/* Main Content Area with safe area padding */}
      <main
        className={`
          ${showHeader ? 'pt-14' : ''}
          ${showNav ? 'pb-20' : ''}
          px-4 py-4
          safe-area-left safe-area-right
        `}
      >
        {children}
      </main>

      {showNav && <MobileNav />}
    </div>
  );
}

export default MobileLayout;
