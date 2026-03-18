// Authenticated app shell with sidebar, navbar, and scoped providers
import { Suspense, useEffect, useCallback } from 'react';
import { Outlet, useLocation } from 'react-router-dom';
import { I18nextProvider } from 'react-i18next';
import i18n from '@/i18n';
import { PageErrorBoundary } from '@/components/PageErrorBoundary';
import { Navbar } from '@/components/layout/navbar';
import { Sidebar } from '@/components/layout/sidebar';
import { useViewStore } from '@/stores/useViewStore';
import { TopsiWidget } from '@/components/topsi';
import { useTaskViewManager } from '@/hooks/useTaskViewManager';
import { usePreviousPath } from '@/hooks/usePreviousPath';
import { usePageTitle } from '@/hooks/usePageTitle';
import {
  useUserSystem,
} from '@/components/config-provider';
import { ThemeProvider } from '@/components/theme-provider';
import { SearchProvider } from '@/contexts/search-context';
import { ShortcutsHelp } from '@/components/shortcuts-help';
import { ThemeMode } from 'shared/types';
import { Loader } from '@/components/ui/loader';
import { AppWithStyleOverride } from '@/utils/style-override';
import { WebviewContextMenu } from '@/vscode/ContextMenu';
import NiceModal from '@ebay/nice-modal-react';
import type { WelcomeWizardResult } from '@/components/onboarding';
import { Toaster } from '@/components/ui/toaster';
import { BreadcrumbNav } from '@/components/breadcrumb/BreadcrumbNav';
import { CommandPalette } from '@/components/command/CommandPalette';
import { KeyboardShortcutsOverlay } from '@/components/keyboard-shortcuts/KeyboardShortcutsOverlay';
import { ErrorDisplay } from '@/components/ErrorDisplay';
import { ViewAsBanner } from '@/components/layout/ViewAsBanner';

// Shared suspense fallback
export const PageLoader = () => (
  <div className="flex-1 flex items-center justify-center min-h-[200px]">
    <Loader message="Loading..." size={28} />
  </div>
);

export function AppShell() {
  const { config, updateAndSaveConfig, loading } = useUserSystem();
  const { isFullscreen, toggleFullscreen } = useTaskViewManager();
  const location = useLocation();
  const isVirtualEnv = location.pathname.startsWith('/virtual-environment');
  const { sidebarCollapsed, toggleSidebar, setSidebarCollapsed, contentFullscreen } = useViewStore();

  // On mobile, toggle sidebar means show/hide the overlay sidebar
  // We reuse sidebarCollapsed: collapsed=true means hidden on mobile
  const handleToggleSidebar = useCallback(() => {
    toggleSidebar();
  }, [toggleSidebar]);

  // Collapse sidebar on mobile/tablet viewports initially and on route change
  useEffect(() => {
    if (window.innerWidth < 1024) {
      setSidebarCollapsed(true);
    }
  }, [location.pathname, setSidebarCollapsed]);

  // Track previous path for back navigation
  usePreviousPath();

  // Dynamic page title based on current route
  usePageTitle();

  const showNavbar = !isFullscreen && !contentFullscreen;

  // Unified onboarding wizard — replaces 4 sequential modals with single stepped dialog
  useEffect(() => {
    let cancelled = false;

    const runOnboarding = async () => {
      if (!config || cancelled) return;

      // Skip onboarding entirely if env var is set (for E2E testing)
      const skipOnboarding = import.meta.env.VITE_SKIP_ONBOARDING === '1';
      if (skipOnboarding) {
        // Auto-acknowledge all flags silently
        const needsUpdate =
          !config.disclaimer_acknowledged ||
          !config.onboarding_acknowledged ||
          !config.github_login_acknowledged ||
          !config.telemetry_acknowledged;
        if (needsUpdate) {
          await updateAndSaveConfig({
            disclaimer_acknowledged: true,
            onboarding_acknowledged: true,
            github_login_acknowledged: true,
            telemetry_acknowledged: true,
          });
        }
        return;
      }

      // Check if any onboarding steps remain incomplete
      const needsOnboarding =
        !config.disclaimer_acknowledged ||
        !config.onboarding_acknowledged ||
        !config.github_login_acknowledged ||
        !config.telemetry_acknowledged;

      if (needsOnboarding) {
        try {
          const result: WelcomeWizardResult =
            await NiceModal.show('welcome-wizard');
          if (cancelled) return;

          // Persist all onboarding results atomically
          await updateAndSaveConfig({
            disclaimer_acknowledged: true,
            onboarding_acknowledged: true,
            executor_profile: result.profile,
            editor: result.editor,
            github_login_acknowledged: true,
            telemetry_acknowledged: true,
            analytics_enabled: result.analyticsEnabled,
          });
          await NiceModal.hide('welcome-wizard');
        } catch {
          // Wizard was dismissed — that's fine, partial state persists
          await NiceModal.hide('welcome-wizard');
        }
      }

      // Release notes — separate from onboarding wizard
      if (!cancelled && config.show_release_notes) {
        try {
          await NiceModal.show('release-notes');
          if (!cancelled) {
            await updateAndSaveConfig({ show_release_notes: false });
          }
          await NiceModal.hide('release-notes');
        } catch {
          await NiceModal.hide('release-notes');
        }
      }
    };

    runOnboarding();

    return () => {
      cancelled = true;
    };
  }, [config, updateAndSaveConfig]);

  if (loading) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <Loader message="Loading..." size={32} />
      </div>
    );
  }

  return (
    <I18nextProvider i18n={i18n}>
      <ThemeProvider initialTheme={config?.theme || ThemeMode.SYSTEM}>
        <AppWithStyleOverride>
          <SearchProvider>
            <div className="h-screen flex flex-col bg-background">
              <WebviewContextMenu />
              {showNavbar && <Navbar onToggleSidebar={handleToggleSidebar} />}
              <BreadcrumbNav onToggleFullscreen={toggleFullscreen} isFullscreen={isFullscreen} />

              <div className="flex-1 flex min-h-0 relative">
                {/* Mobile/tablet backdrop overlay when sidebar is open */}
                {!sidebarCollapsed && (
                  <div
                    className="absolute inset-0 bg-black/50 z-40 lg:hidden"
                    onClick={() => setSidebarCollapsed(true)}
                  />
                )}

                {/* Sidebar: hidden on small screens when collapsed, overlay when open; always visible on lg+ */}
                {!showNavbar ? null : (
                  <div className={`
                    lg:relative lg:flex lg:shrink-0
                    ${sidebarCollapsed
                      ? 'hidden lg:flex'
                      : 'absolute top-0 left-0 bottom-0 z-50 lg:relative lg:z-auto flex'
                    }
                  `}>
                    <Sidebar className="shrink-0 bg-background h-full" />
                  </div>
                )}

                <div className="flex-1 overflow-y-auto">
                  <ViewAsBanner />
                  <PageErrorBoundary label="Page">
                    <Suspense fallback={<PageLoader />}>
                      <Outlet />
                    </Suspense>
                  </PageErrorBoundary>
                </div>
              </div>
            </div>
            {!isVirtualEnv && <TopsiWidget />}
            <ShortcutsHelp />
            <CommandPalette />
            <KeyboardShortcutsOverlay />
            <ErrorDisplay />
          </SearchProvider>
        </AppWithStyleOverride>
        <Toaster />
      </ThemeProvider>
    </I18nextProvider>
  );
}
