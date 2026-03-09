// Authenticated app shell with sidebar, navbar, and scoped providers
import { Suspense, useEffect, useCallback } from 'react';
import { Outlet, useLocation } from 'react-router-dom';
import { I18nextProvider } from 'react-i18next';
import i18n from '@/i18n';
import { Navbar } from '@/components/layout/navbar';
import { Sidebar } from '@/components/layout/sidebar';
import { useViewStore } from '@/stores/useViewStore';
import { TopsiWidget } from '@/components/topsi';
import { useTaskViewManager } from '@/hooks/useTaskViewManager';
import { usePreviousPath } from '@/hooks/usePreviousPath';
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
import { DevBanner } from '@/components/DevBanner';
import NiceModal from '@ebay/nice-modal-react';
import { OnboardingResult } from '@/components/dialogs/global/OnboardingDialog';
import { Toaster } from '@/components/ui/toaster';
import { BreadcrumbNav } from '@/components/breadcrumb/BreadcrumbNav';
import { CommandPalette } from '@/components/command/CommandPalette';
import { KeyboardShortcutsOverlay } from '@/components/keyboard-shortcuts/KeyboardShortcutsOverlay';
import { ErrorDisplay } from '@/components/ErrorDisplay';

// Shared suspense fallback
export const PageLoader = () => (
  <div className="flex-1 flex items-center justify-center min-h-[200px]">
    <Loader message="Loading..." size={28} />
  </div>
);

export function AppShell() {
  const { config, updateAndSaveConfig, loading } = useUserSystem();
  const { isFullscreen } = useTaskViewManager();
  const location = useLocation();
  const isVirtualEnv = location.pathname.startsWith('/virtual-environment');
  const { sidebarCollapsed, toggleSidebar, setSidebarCollapsed } = useViewStore();

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

  const showNavbar = !isFullscreen;

  useEffect(() => {
    let cancelled = false;

    const handleOnboardingComplete = async (
      onboardingConfig: OnboardingResult
    ) => {
      if (cancelled) return;
      const updatedConfig = {
        ...config,
        onboarding_acknowledged: true,
        executor_profile: onboardingConfig.profile,
        editor: onboardingConfig.editor,
      };

      updateAndSaveConfig(updatedConfig);
    };

    const handleDisclaimerAccept = async () => {
      if (cancelled) return;
      await updateAndSaveConfig({ disclaimer_acknowledged: true });
    };

    const handleGitHubLoginComplete = async () => {
      if (cancelled) return;
      await updateAndSaveConfig({ github_login_acknowledged: true });
    };

    const handleTelemetryOptIn = async (analyticsEnabled: boolean) => {
      if (cancelled) return;
      await updateAndSaveConfig({
        telemetry_acknowledged: true,
        analytics_enabled: analyticsEnabled,
      });
    };

    const handleReleaseNotesClose = async () => {
      if (cancelled) return;
      await updateAndSaveConfig({ show_release_notes: false });
    };

    const checkOnboardingSteps = async () => {
      if (!config || cancelled) return;

      if (!config.disclaimer_acknowledged) {
        await NiceModal.show('disclaimer');
        await handleDisclaimerAccept();
        await NiceModal.hide('disclaimer');
      }

      if (!config.onboarding_acknowledged) {
        const onboardingResult: OnboardingResult =
          await NiceModal.show('onboarding');
        await handleOnboardingComplete(onboardingResult);
        await NiceModal.hide('onboarding');
      }

      if (!config.github_login_acknowledged) {
        await NiceModal.show('github-login');
        await handleGitHubLoginComplete();
        await NiceModal.hide('github-login');
      }

      if (!config.telemetry_acknowledged) {
        const analyticsEnabled: boolean =
          await NiceModal.show('privacy-opt-in');
        await handleTelemetryOptIn(analyticsEnabled);
        await NiceModal.hide('privacy-opt-in');
      }

      if (config.show_release_notes) {
        await NiceModal.show('release-notes');
        await handleReleaseNotesClose();
        await NiceModal.hide('release-notes');
      }
    };

    const runOnboarding = async () => {
      if (!config || cancelled) return;
      await checkOnboardingSteps();
    };

    runOnboarding();

    return () => {
      cancelled = true;
    };
  }, [config]);

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
              {showNavbar && <DevBanner />}
              {showNavbar && <Navbar onToggleSidebar={handleToggleSidebar} />}
              {showNavbar && <BreadcrumbNav />}

              <div className="flex-1 flex min-h-0 relative">
                {/* Mobile/tablet backdrop overlay when sidebar is open */}
                {!sidebarCollapsed && (
                  <div
                    className="absolute inset-0 bg-black/50 z-40 lg:hidden"
                    onClick={() => setSidebarCollapsed(true)}
                  />
                )}

                {/* Sidebar: hidden on small screens when collapsed, overlay when open; always visible on lg+ */}
                <div className={`
                  lg:relative lg:flex lg:shrink-0
                  ${sidebarCollapsed
                    ? 'hidden lg:flex'
                    : 'absolute top-0 left-0 bottom-0 z-50 lg:relative lg:z-auto flex'
                  }
                `}>
                  <Sidebar className="shrink-0 bg-background h-full" />
                </div>

                <div className="flex-1 overflow-y-auto">
                  <Suspense fallback={<PageLoader />}>
                    <Outlet />
                  </Suspense>
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
