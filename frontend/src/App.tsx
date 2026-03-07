import { lazy, Suspense } from 'react';
import { BrowserRouter, Navigate, Route, Routes, useParams } from 'react-router-dom';

// Redirect helpers for consolidated CRM routes
function AcquisitionRedirect() {
  const { orgId } = useParams<{ orgId: string }>();
  return <Navigate to={`/organizations/${orgId}?tab=pipelines&pipeline=sales`} replace />;
}
function LifecycleRedirect() {
  const { orgId } = useParams<{ orgId: string }>();
  return <Navigate to={`/organizations/${orgId}?tab=pipelines&pipeline=lifecycle`} replace />;
}
import {
  UserSystemProvider,
} from '@/components/config-provider';
import { KeyboardShortcutsProvider } from '@/contexts/keyboard-shortcuts-context';
import { HotkeysProvider } from 'react-hotkeys-hook';
import { ProjectProvider } from '@/contexts/project-context';
import { OrganizationProvider } from '@/contexts/organization-context';
import { AuthProvider, useAuth } from '@/contexts/AuthContext';
import { LoginPage } from '@/components/auth/LoginPage';
import { ProtectedRoute } from '@/components/auth/ProtectedRoute';
import { AdminRoute } from '@/components/auth/AdminRoute';
import * as Sentry from '@sentry/react';
import NiceModal from '@ebay/nice-modal-react';
import { AuthLayout } from '@/layouts/AuthLayout';
import { AppShell, PageLoader } from '@/layouts/AppShell';

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
const OrganizationProfilePage  = lazy(() => import('@/pages/organization-profile').then(m => ({ default: m.OrganizationProfilePage })));
const ClientOverview        = lazy(() => import('@/pages/client-overview').then(m => ({ default: m.ClientOverview })));
const VirtualEnvironmentPage       = lazy(() => import('@/pages/virtual-environment').then(m => ({ default: m.VirtualEnvironmentPage })));
const EmbedVirtualEnvironmentPage  = lazy(() => import('@/pages/embed/virtual-environment').then(m => ({ default: m.EmbedVirtualEnvironmentPage })));
// MeshPage merged into Settings > Network & Mesh
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
const CompaniesPage         = lazy(() => import('@/pages/companies').then(m => ({ default: m.CompaniesPage })));
const CompanyProfilePage    = lazy(() => import('@/pages/company-profile').then(m => ({ default: m.CompanyProfilePage })));
const CommandCenterPage     = lazy(() => import('@/pages/command-center').then(m => ({ default: m.CommandCenterPage })));
const InvoicesPage          = lazy(() => import('@/pages/invoices').then(m => ({ default: m.InvoicesPage })));
const ProjectDeliverablesPage = lazy(() => import('@/pages/project-deliverables').then(m => ({ default: m.ProjectDeliverablesPage })));
const DataSourceDetailPage = lazy(() => import('@/pages/data-source-detail').then(m => ({ default: m.DataSourceDetailPage })));
const DiscordPage             = lazy(() => import('@/pages/discord').then(m => ({ default: m.DiscordPage })));

// ─── Lazy-loaded settings pages ─────────────────────────────────────────────
const SettingsLayout    = lazy(() => import('@/pages/settings/SettingsLayout').then(m => ({ default: m.SettingsLayout })));
const GeneralSettings   = lazy(() => import('@/pages/settings/GeneralSettings').then(m => ({ default: m.GeneralSettings })));
const ProfileSettings   = lazy(() => import('@/pages/settings/ProfileSettings').then(m => ({ default: m.ProfileSettings })));
const UsersSettings     = lazy(() => import('@/pages/settings/UsersSettings').then(m => ({ default: m.UsersSettings })));
const ProjectsSettings  = lazy(() => import('@/pages/settings/ProjectsSettings').then(m => ({ default: m.ProjectsSettings })));
const OrganizationsSettings = lazy(() => import('@/pages/settings/OrganizationsSettings').then(m => ({ default: m.OrganizationsSettings })));
const PrivacySettings   = lazy(() => import('@/pages/settings/PrivacySettings').then(m => ({ default: m.PrivacySettings })));
const ActivitySettings  = lazy(() => import('@/pages/settings/ActivitySettings').then(m => ({ default: m.ActivitySettings })));
const AgentSettings     = lazy(() => import('@/pages/settings/AgentSettings').then(m => ({ default: m.AgentSettings })));
const ModelsSettings    = lazy(() => import('@/pages/settings/ModelsSettings').then(m => ({ default: m.ModelsSettings })));
const McpSettings       = lazy(() => import('@/pages/settings/McpSettings').then(m => ({ default: m.McpSettings })));
const WalletSettings    = lazy(() => import('@/pages/settings/WalletSettings').then(m => ({ default: m.WalletSettings })));
const NetworkSettings   = lazy(() => import('@/pages/settings/NetworkSettings').then(m => ({ default: m.NetworkSettings })));

const SentryRoutes = Sentry.withSentryReactRouterV6Routing(Routes);

/** Redirects authenticated users to their first organization, or falls back to /projects */
function HomeRedirect() {
  const { user } = useAuth();
  const firstOrg = user?.organizations?.[0];
  if (firstOrg) {
    return <Navigate to={`/organizations/${firstOrg.id}`} replace />;
  }
  return <Projects />;
}

function App() {
  return (
    <BrowserRouter>
      <SentryRoutes>
        {/* Embed routes - no layout, no auth */}
        <Route path="/embed/virtual-environment" element={
          <Suspense fallback={<PageLoader />}>
            <EmbedVirtualEnvironmentPage />
          </Suspense>
        } />

        {/* Auth routes - clean layout, no sidebar/navbar */}
        <Route element={
          <AuthProvider>
            <AuthLayout />
          </AuthProvider>
        }>
          <Route path="/login" element={<LoginPage />} />
        </Route>

        {/* Authenticated app routes - full layout with sidebar/navbar */}
        <Route element={
          <AuthProvider>
            <UserSystemProvider>
              <OrganizationProvider>
                <ProjectProvider>
                  <HotkeysProvider initiallyActiveScopes={['*', 'global', 'kanban']}>
                    <KeyboardShortcutsProvider>
                      <NiceModal.Provider>
                        <AppShell />
                      </NiceModal.Provider>
                    </KeyboardShortcutsProvider>
                  </HotkeysProvider>
                </ProjectProvider>
              </OrganizationProvider>
            </UserSystemProvider>
          </AuthProvider>
        }>
          <Route path="/" element={<ProtectedRoute><HomeRedirect /></ProtectedRoute>} />
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
            element={<ProtectedRoute><OrganizationProfilePage /></ProtectedRoute>}
          />
          <Route
            path="/organizations/:orgId/clients/:clientId"
            element={<ProtectedRoute><ClientOverview /></ProtectedRoute>}
          />
          <Route
            path="/organizations/:orgId/data-sources/:dataSourceId"
            element={<ProtectedRoute><DataSourceDetailPage /></ProtectedRoute>}
          />
          <Route path="/organizations/:orgId/crm/acquisition" element={<AcquisitionRedirect />} />
          <Route path="/organizations/:orgId/crm/lifecycle" element={<LifecycleRedirect />} />
          <Route
            path="/projects/:projectId/knowledge"
            element={<ProtectedRoute><KnowledgePage /></ProtectedRoute>}
          />
          <Route path="/my-tasks" element={<ProtectedRoute><MyTasksPage /></ProtectedRoute>} />
          <Route path="/nora" element={<AdminRoute><NoraPage /></AdminRoute>} />
          <Route path="/topsi" element={<ProtectedRoute><TopsiPage /></ProtectedRoute>} />
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
            path="/companies"
            element={<ProtectedRoute><CompaniesPage /></ProtectedRoute>}
          />
          <Route
            path="/companies/:companyId"
            element={<ProtectedRoute><CompanyProfilePage /></ProtectedRoute>}
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
            path="/discord"
            element={<ProtectedRoute><DiscordPage /></ProtectedRoute>}
          />
          <Route
            path="/virtual-environment"
            element={<ProtectedRoute><VirtualEnvironmentPage /></ProtectedRoute>}
          />
          <Route path="/mesh" element={<Navigate to="/settings/network" replace />} />
          <Route path="/pulse" element={<ProtectedRoute><PulsePage /></ProtectedRoute>} />
          <Route
            path="/projects/:projectId/pulse"
            element={<ProtectedRoute><PulsePage /></ProtectedRoute>}
          />
          <Route path="/vibe" element={<ProtectedRoute><VibePage /></ProtectedRoute>} />
          <Route
            path="/oauth/:provider/callback"
            element={<ProtectedRoute><OAuthCallbackPage /></ProtectedRoute>}
          />
          <Route path="/settings/*" element={<ProtectedRoute><SettingsLayout /></ProtectedRoute>}>
            <Route index element={<Navigate to="general" replace />} />
            <Route path="general" element={<GeneralSettings />} />
            <Route path="wallet" element={<WalletSettings />} />
            <Route path="profile" element={<ProfileSettings />} />
            <Route path="users" element={<AdminRoute><UsersSettings /></AdminRoute>} />
            <Route path="organizations" element={<AdminRoute><OrganizationsSettings /></AdminRoute>} />
            <Route path="projects" element={<AdminRoute><ProjectsSettings /></AdminRoute>} />
            <Route path="privacy" element={<PrivacySettings />} />
            <Route path="activity" element={<ActivitySettings />} />
            <Route path="agents" element={<AgentSettings />} />
            <Route path="models" element={<ModelsSettings />} />
            <Route path="mcp" element={<McpSettings />} />
            <Route path="airtable" element={<Navigate to="/settings/general" replace />} />
            <Route path="network" element={<NetworkSettings />} />
          </Route>
          <Route
            path="/mcp-servers"
            element={<Navigate to="/settings/mcp" replace />}
          />
        </Route>
      </SentryRoutes>
    </BrowserRouter>
  );
}

export default App;
