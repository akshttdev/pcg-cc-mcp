import { useEffect } from 'react';
import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom';
import { I18nextProvider } from 'react-i18next';
import i18n from '@/i18n';
import { Navbar } from '@/components/layout/navbar';
import { Sidebar } from '@/components/layout/sidebar';
import { Projects } from '@/pages/projects';
import { ProjectTasks } from '@/pages/project-tasks';
import { NoraPage } from '@/pages/nora';
import { VirtualEnvironmentPage } from '@/pages/virtual-environment';
import { useTaskViewManager } from '@/hooks/useTaskViewManager';
import { usePreviousPath } from '@/hooks/usePreviousPath';

import {
  AgentSettings,
  GeneralSettings,
  McpSettings,
  SettingsLayout,
  ProfileSettings,
  PrivacySettings,
  ActivitySettings,
  WalletSettings,
  UsersSettings,
  ProjectsSettings,
} from '@/pages/settings/';
import {
  UserSystemProvider,
  useUserSystem,
} from '@/components/config-provider';
import { ThemeProvider } from '@/components/theme-provider';
import { SearchProvider } from '@/contexts/search-context';
import { KeyboardShortcutsProvider } from '@/contexts/keyboard-shortcuts-context';
import { ShortcutsHelp } from '@/components/shortcuts-help';
import { HotkeysProvider } from 'react-hotkeys-hook';

import { ProjectProvider } from '@/contexts/project-context';
import { AuthProvider } from '@/contexts/AuthContext';
import { LoginPage } from '@/components/auth/LoginPage';
import { ProtectedRoute } from '@/components/auth/ProtectedRoute';
import { AdminRoute } from '@/components/auth/AdminRoute';
import { ThemeMode } from 'shared/types';
import * as Sentry from '@sentry/react';
import { Loader } from '@/components/ui/loader';

import { AppWithStyleOverride } from '@/utils/style-override';
import { WebviewContextMenu } from '@/vscode/ContextMenu';
import { DevBanner } from '@/components/DevBanner';
import NiceModal from '@ebay/nice-modal-react';
import { OnboardingResult } from './components/dialogs/global/OnboardingDialog';
import { Toaster } from '@/components/ui/toaster';
import { BreadcrumbNav } from '@/components/breadcrumb/BreadcrumbNav';
import { CommandPalette } from '@/components/command/CommandPalette';
import { KeyboardShortcutsOverlay } from '@/components/keyboard-shortcuts/KeyboardShortcutsOverlay';
import { ErrorDisplay } from '@/components/ErrorDisplay';

const SentryRoutes = Sentry.withSentryReactRouterV6Routing(Routes);

function AppContent() {
  const { config, updateAndSaveConfig, loading } = useUserSystem();
  const { isFullscreen } = useTaskViewManager();

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
              {/* Custom context menu and VS Code-friendly interactions when embedded in iframe */}
              <WebviewContextMenu />

              {/* Top Navigation Bar - extends across full width */}
              {showNavbar && <DevBanner />}
              {showNavbar && <Navbar />}
              {showNavbar && <BreadcrumbNav />}

              {/* Main Content Area with Sidebar */}
              <div className="flex-1 flex min-h-0">
                {/* Left Sidebar - positioned below navbar */}
                <Sidebar className="w-64 shrink-0" />
                
                {/* Content Area */}
                <div className="flex-1 overflow-y-auto">
                  <SentryRoutes>
                    <Route path="/login" element={<LoginPage />} />
                    <Route path="/" element={<ProtectedRoute><Projects /></ProtectedRoute>} />
                    <Route path="/projects" element={<ProtectedRoute><Projects /></ProtectedRoute>} />
                    <Route path="/projects/:projectId" element={<ProtectedRoute><Projects /></ProtectedRoute>} />
                    <Route
                      path="/projects/:projectId/tasks"
                      element={<ProtectedRoute><ProjectTasks /></ProtectedRoute>}
                    />
                    <Route
                      path="/projects/:projectId/tasks/:taskId/attempts/:attemptId"
                      element={<ProtectedRoute><ProjectTasks /></ProtectedRoute>}
                    />
                    <Route
                      path="/projects/:projectId/tasks/:taskId/attempts/:attemptId/full"
                      element={<ProtectedRoute><ProjectTasks /></ProtectedRoute>}
                    />
                    <Route
                      path="/projects/:projectId/tasks/:taskId/full"
                      element={<ProtectedRoute><ProjectTasks /></ProtectedRoute>}
                    />
                    <Route
                      path="/projects/:projectId/tasks/:taskId"
                      element={<ProtectedRoute><ProjectTasks /></ProtectedRoute>}
                    />
                    <Route path="/nora" element={<ProtectedRoute><NoraPage /></ProtectedRoute>} />
                    <Route
                      path="/virtual-environment"
                      element={<ProtectedRoute><VirtualEnvironmentPage /></ProtectedRoute>}
                    />
                    <Route path="/settings/*" element={<ProtectedRoute><SettingsLayout /></ProtectedRoute>}>
                      <Route index element={<Navigate to="general" replace />} />
                      <Route path="general" element={<GeneralSettings />} />
                      <Route path="wallet" element={<WalletSettings />} />
                      <Route path="profile" element={<ProfileSettings />} />
                      <Route path="users" element={<AdminRoute><UsersSettings /></AdminRoute>} />
                      <Route path="projects" element={<AdminRoute><ProjectsSettings /></AdminRoute>} />
                      <Route path="privacy" element={<PrivacySettings />} />
                      <Route path="activity" element={<ActivitySettings />} />
                      <Route path="agents" element={<AgentSettings />} />
                      <Route path="mcp" element={<McpSettings />} />
                    </Route>
                    {/* Redirect old MCP route */}
                    <Route
                      path="/mcp-servers"
                      element={<Navigate to="/settings/mcp" replace />}
                    />
                  </SentryRoutes>
                </div>
              </div>
            </div>
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

function App() {
  return (
    <BrowserRouter>
      <AuthProvider>
        <UserSystemProvider>
          <ProjectProvider>
            <HotkeysProvider initiallyActiveScopes={['*', 'global', 'kanban']}>
              <KeyboardShortcutsProvider>
                <NiceModal.Provider>
                  <AppContent />
                </NiceModal.Provider>
              </KeyboardShortcutsProvider>
            </HotkeysProvider>
          </ProjectProvider>
        </UserSystemProvider>
      </AuthProvider>
    </BrowserRouter>
  );
}

export default App;
