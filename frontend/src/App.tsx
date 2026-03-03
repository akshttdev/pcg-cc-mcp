import { lazy, Suspense, useEffect } from 'react';
import { BrowserRouter, Navigate, Route, Routes, useLocation } from 'react-router-dom';
import { I18nextProvider } from 'react-i18next';
import i18n from '@/i18n';
import { Navbar } from '@/components/layout/navbar';
import { Sidebar } from '@/components/layout/sidebar';
import { TopsiWidget } from '@/components/topsi';
import { useTaskViewManager } from '@/hooks/useTaskViewManager';
import { usePreviousPath } from '@/hooks/usePreviousPath';
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

// ─── Lazy-loaded page routes ────────────────────────────────────────────────
const Projects              = lazy(() => import('@/pages/projects').then(m => ({ default: m.Projects })));
const ProjectTasks          = lazy(() => import('@/pages/project-tasks').then(m => ({ default: m.ProjectTasks })));
const ProjectControllerPage = lazy(() => import('@/pages/project-controller').then(m => ({ default: m.ProjectControllerPage })));
const MyTasksPage           = lazy(() => import('@/pages/my-tasks').then(m => ({ default: m.MyTasksPage })));
const GlobalTasksPage       = lazy(() => import('@/pages/global-tasks').then(m => ({ default: m.GlobalTasksPage })));
const NoraPage              = lazy(() => import('@/pages/nora').then(m => ({ default: m.NoraPage })));
const TopsiPage             = lazy(() => import('@/pages/topsi').then(m => ({ default: m.TopsiPage })));
const KnowledgePage         = lazy(() => import('@/pages/knowledge').then(m => ({ default: m.KnowledgePage })));
const MissionControlPage    = lazy(() => import('@/pages/mission-control'));
const WorkflowsPage         = lazy(() => import('@/pages/workflows').then(m => ({ default: m.WorkflowsPage })));
const SocialPage            = lazy(() => import('@/pages/social').then(m => ({ default: m.SocialPage })));
const CrmPage               = lazy(() => import('@/pages/crm').then(m => ({ default: m.CrmPage })));
const CrmClientsPage        = lazy(() => import('@/pages/crm-clients').then(m => ({ default: m.CrmClientsPage })));
const CrmAcquisitionPage       = lazy(() => import('@/pages/crm-clients').then(m => ({ default: m.CrmAcquisitionPage })));
const CrmLifecyclePage         = lazy(() => import('@/pages/crm-clients').then(m => ({ default: m.CrmLifecyclePage })));
const OrganizationDetailPage   = lazy(() => import('@/pages/organization-detail').then(m => ({ default: m.OrganizationDetailPage })));
const VirtualEnvironmentPage       = lazy(() => import('@/pages/virtual-environment').then(m => ({ default: m.VirtualEnvironmentPage })));
const EmbedVirtualEnvironmentPage  = lazy(() => import('@/pages/embed/virtual-environment').then(m => ({ default: m.EmbedVirtualEnvironmentPage })));
const MeshPage              = lazy(() => import('@/pages/mesh'));
const VibePage              = lazy(() => import('@/pages/vibe'));
const PulsePage             = lazy(() => import('@/pages/pulse'));
const OAuthCallbackPage     = lazy(() => import('@/pages/oauth/OAuthCallbackPage').then(m => ({ default: m.OAuthCallbackPage })));
const CrmSalesPage          = lazy(() => import('@/pages/crm-sales').then(m => ({ default: m.CrmSalesPage })));
const CrmDeliveryPage       = lazy(() => import('@/pages/crm-delivery').then(m => ({ default: m.CrmDeliveryPage })));
const CrmConferencesPage    = lazy(() => import('@/pages/crm-conferences').then(m => ({ default: m.CrmConferencesPage })));
const CrmContactDetailPage  = lazy(() => import('@/pages/crm-contact-detail').then(m => ({ default: m.CrmContactDetailPage })));
const CrmOverviewPage       = lazy(() => import('@/pages/crm-overview').then(m => ({ default: m.CrmOverviewPage })));
const PeoplePage            = lazy(() => import('@/pages/people').then(m => ({ default: m.PeoplePage })));
const PersonDetailPage      = lazy(() => import('@/pages/person-detail').then(m => ({ default: m.PersonDetailPage })));
const ProposalsPage         = lazy(() => import('@/pages/proposals').then(m => ({ default: m.ProposalsPage })));
const CommandCenterPage     = lazy(() => import('@/pages/command-center').then(m => ({ default: m.CommandCenterPage })));
const InvoicesPage          = lazy(() => import('@/pages/invoices').then(m => ({ default: m.InvoicesPage })));
const ProjectDeliverablesPage = lazy(() => import('@/pages/project-deliverables').then(m => ({ default: m.ProjectDeliverablesPage })));

// ─── Lazy-loaded settings pages ─────────────────────────────────────────────
const SettingsLayout    = lazy(() => import('@/pages/settings/SettingsLayout').then(m => ({ default: m.SettingsLayout })));
const GeneralSettings   = lazy(() => import('@/pages/settings/GeneralSettings').then(m => ({ default: m.GeneralSettings })));
const ProfileSettings   = lazy(() => import('@/pages/settings/ProfileSettings').then(m => ({ default: m.ProfileSettings })));
const UsersSettings     = lazy(() => import('@/pages/settings/UsersSettings').then(m => ({ default: m.UsersSettings })));
const ProjectsSettings  = lazy(() => import('@/pages/settings/ProjectsSettings').then(m => ({ default: m.ProjectsSettings })));
const PrivacySettings   = lazy(() => import('@/pages/settings/PrivacySettings').then(m => ({ default: m.PrivacySettings })));
const ActivitySettings  = lazy(() => import('@/pages/settings/ActivitySettings').then(m => ({ default: m.ActivitySettings })));
const AgentSettings     = lazy(() => import('@/pages/settings/AgentSettings').then(m => ({ default: m.AgentSettings })));
const ModelsSettings    = lazy(() => import('@/pages/settings/ModelsSettings').then(m => ({ default: m.ModelsSettings })));
const McpSettings       = lazy(() => import('@/pages/settings/McpSettings').then(m => ({ default: m.McpSettings })));
const WalletSettings    = lazy(() => import('@/pages/settings/WalletSettings').then(m => ({ default: m.WalletSettings })));
const AirtableSettings  = lazy(() => import('@/pages/settings/AirtableSettings').then(m => ({ default: m.AirtableSettings })));
const NetworkSettings   = lazy(() => import('@/pages/settings/NetworkSettings').then(m => ({ default: m.NetworkSettings })));

// ─── Shared suspense fallback ────────────────────────────────────────────────
const PageLoader = () => (
  <div className="flex-1 flex items-center justify-center min-h-[200px]">
    <Loader message="Loading..." size={28} />
  </div>
);

const SentryRoutes = Sentry.withSentryReactRouterV6Routing(Routes);

function AppContent() {
  const { config, updateAndSaveConfig, loading } = useUserSystem();
  const { isFullscreen } = useTaskViewManager();
  const location = useLocation();
  const isVirtualEnv = location.pathname.startsWith('/virtual-environment');

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
              {showNavbar && <Navbar />}
              {showNavbar && <BreadcrumbNav />}

              <div className="flex-1 flex min-h-0">
                <Sidebar className="w-64 shrink-0" />

                <div className="flex-1 overflow-y-auto">
                  <Suspense fallback={<PageLoader />}>
                    <SentryRoutes>
                      <Route path="/login" element={<LoginPage />} />
                      <Route
                        path="/oauth/:provider/callback"
                        element={<ProtectedRoute><OAuthCallbackPage /></ProtectedRoute>}
                      />
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
                      <Route
                        path="/projects/:projectId/control"
                        element={<ProtectedRoute><ProjectControllerPage /></ProtectedRoute>}
                      />
                      {/* Project CRM - scoped to project */}
                      <Route
                        path="/projects/:projectId/crm"
                        element={<ProtectedRoute><CrmPage /></ProtectedRoute>}
                      />
                      <Route
                        path="/projects/:projectId/crm/sales"
                        element={<ProtectedRoute><CrmSalesPage /></ProtectedRoute>}
                      />
                      <Route
                        path="/projects/:projectId/crm/delivery"
                        element={<ProtectedRoute><CrmDeliveryPage /></ProtectedRoute>}
                      />
                      <Route
                        path="/projects/:projectId/crm/clients"
                        element={<ProtectedRoute><CrmClientsPage /></ProtectedRoute>}
                      />
                      <Route
                        path="/projects/:projectId/crm/conferences"
                        element={<ProtectedRoute><CrmConferencesPage /></ProtectedRoute>}
                      />
                      <Route
                        path="/projects/:projectId/crm/contacts/:contactId"
                        element={<ProtectedRoute><CrmContactDetailPage /></ProtectedRoute>}
                      />
                      <Route
                        path="/projects/:projectId/crm/overview"
                        element={<ProtectedRoute><CrmOverviewPage /></ProtectedRoute>}
                      />
                      <Route
                        path="/organizations/:orgId"
                        element={<ProtectedRoute><OrganizationDetailPage /></ProtectedRoute>}
                      />
                      <Route
                        path="/organizations/:orgId/crm/acquisition"
                        element={<ProtectedRoute><CrmAcquisitionPage /></ProtectedRoute>}
                      />
                      <Route
                        path="/organizations/:orgId/crm/lifecycle"
                        element={<ProtectedRoute><CrmLifecyclePage /></ProtectedRoute>}
                      />
                      <Route
                        path="/projects/:projectId/knowledge"
                        element={<ProtectedRoute><KnowledgePage /></ProtectedRoute>}
                      />
                      <Route path="/my-tasks" element={<ProtectedRoute><MyTasksPage /></ProtectedRoute>} />
                      <Route path="/nora" element={<AdminRoute><NoraPage /></AdminRoute>} />
                      <Route path="/topsi" element={<AdminRoute><TopsiPage /></AdminRoute>} />
                      <Route path="/global-tasks" element={<AdminRoute><GlobalTasksPage /></AdminRoute>} />
                      <Route path="/mission-control" element={<ProtectedRoute><MissionControlPage /></ProtectedRoute>} />
                      <Route path="/workflows" element={<ProtectedRoute><WorkflowsPage /></ProtectedRoute>} />
                      <Route
                        path="/social-command"
                        element={<ProtectedRoute><SocialPage /></ProtectedRoute>}
                      />
                      <Route
                        path="/projects/:projectId/social"
                        element={<Navigate to="/social-command" replace />}
                      />
                      <Route
                        path="/crm"
                        element={<ProtectedRoute><CrmPage /></ProtectedRoute>}
                      />
                      <Route
                        path="/people"
                        element={<ProtectedRoute><PeoplePage /></ProtectedRoute>}
                      />
                      <Route
                        path="/people/:personId"
                        element={<ProtectedRoute><PersonDetailPage /></ProtectedRoute>}
                      />
                      <Route
                        path="/proposals"
                        element={<ProtectedRoute><ProposalsPage /></ProtectedRoute>}
                      />
                      <Route
                        path="/command-center"
                        element={<ProtectedRoute><CommandCenterPage /></ProtectedRoute>}
                      />
                      <Route
                        path="/invoices"
                        element={<ProtectedRoute><InvoicesPage /></ProtectedRoute>}
                      />
                      <Route
                        path="/projects/:projectId/deliverables"
                        element={<ProtectedRoute><ProjectDeliverablesPage /></ProtectedRoute>}
                      />
                      <Route
                        path="/virtual-environment"
                        element={<ProtectedRoute><VirtualEnvironmentPage /></ProtectedRoute>}
                      />
                      <Route path="/mesh" element={<ProtectedRoute><MeshPage /></ProtectedRoute>} />
                      <Route path="/pulse" element={<ProtectedRoute><PulsePage /></ProtectedRoute>} />
                      <Route
                        path="/projects/:projectId/pulse"
                        element={<ProtectedRoute><PulsePage /></ProtectedRoute>}
                      />
                      <Route path="/vibe" element={<ProtectedRoute><VibePage /></ProtectedRoute>} />
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
                        <Route path="models" element={<ModelsSettings />} />
                        <Route path="mcp" element={<McpSettings />} />
                        <Route path="airtable" element={<AirtableSettings />} />
                        <Route path="network" element={<NetworkSettings />} />
                      </Route>
                      <Route
                        path="/mcp-servers"
                        element={<Navigate to="/settings/mcp" replace />}
                      />
                    </SentryRoutes>
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

function App() {
  return (
    <BrowserRouter>
      <Routes>
        {/* Embed routes - no layout, no auth wrapper */}
        <Route path="/embed/virtual-environment" element={
          <Suspense fallback={<PageLoader />}>
            <EmbedVirtualEnvironmentPage />
          </Suspense>
        } />

        {/* Main app with full layout */}
        <Route path="*" element={
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
        } />
      </Routes>
    </BrowserRouter>
  );
}

export default App;
