import { lazy, Suspense } from 'react';
import { BrowserRouter, Navigate, Route, Routes, useParams } from 'react-router-dom';
import { PageErrorBoundary } from '@/components/PageErrorBoundary';

import {
  UserSystemProvider,
} from '@/components/config-provider';
import { KeyboardShortcutsProvider } from '@/contexts/keyboard-shortcuts-context';
import { HotkeysProvider } from 'react-hotkeys-hook';
import { ProjectProvider } from '@/contexts/project-context';
import { OrganizationProvider } from '@/contexts/organization-context';
import { ViewContextProvider } from '@/contexts/view-context';
import { AuthProvider, useAuth } from '@/contexts/AuthContext';
import { LoginPage } from '@/components/auth/LoginPage';
import { SignupPage } from '@/pages/auth/SignupPage';
import { ProtectedRoute } from '@/components/auth/ProtectedRoute';
import { AdminRoute } from '@/components/auth/AdminRoute';
import { RoleRoute } from '@/components/auth/RoleRoute';
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
const CrmSalesPage          = lazy(() => import('@/pages/crm-sales').then(m => ({ default: m.CrmSalesPage })));
const CrmDeliveryPage       = lazy(() => import('@/pages/crm-delivery').then(m => ({ default: m.CrmDeliveryPage })));
const CrmConferencesPage    = lazy(() => import('@/pages/crm-conferences').then(m => ({ default: m.CrmConferencesPage })));
const CrmContactDetailPage  = lazy(() => import('@/pages/crm-contact-detail').then(m => ({ default: m.CrmContactDetailPage })));
const CrmOverviewPage       = lazy(() => import('@/pages/crm-overview').then(m => ({ default: m.CrmOverviewPage })));
const OrganizationProfilePage  = lazy(() => import('@/pages/organization-profile').then(m => ({ default: m.OrganizationProfilePage })));
const ClientOverview        = lazy(() => import('@/pages/client-overview').then(m => ({ default: m.ClientOverview })));
const VirtualEnvironmentPage       = lazy(() => import('@/pages/virtual-environment').then(m => ({ default: m.VirtualEnvironmentPage })));
const EmbedVirtualEnvironmentPage  = lazy(() => import('@/pages/embed/virtual-environment').then(m => ({ default: m.EmbedVirtualEnvironmentPage })));
// MeshPage merged into Settings > Network & Mesh
const VibePage              = lazy(() => import('@/pages/vibe'));
const CalendarPage          = lazy(() => import('@/pages/calendar'));
const PulsePage             = lazy(() => import('@/pages/pulse'));
const AIUsagePage           = lazy(() => import('@/pages/ai-usage').then(m => ({ default: m.AIUsagePage })));
const AgentExecutionsPage   = lazy(() => import('@/pages/agent-executions').then(m => ({ default: m.AgentExecutionsPage })));
const OAuthCallbackPage     = lazy(() => import('@/pages/oauth/OAuthCallbackPage').then(m => ({ default: m.OAuthCallbackPage })));
const PeoplePage            = lazy(() => import('@/pages/people').then(m => ({ default: m.PeoplePage })));
const PersonDetailPage      = lazy(() => import('@/pages/person-detail').then(m => ({ default: m.PersonDetailPage })));
const ProposalsPage         = lazy(() => import('@/pages/proposals').then(m => ({ default: m.ProposalsPage })));
const CompaniesPage         = lazy(() => import('@/pages/companies').then(m => ({ default: m.CompaniesPage })));
const CompanyProfilePage    = lazy(() => import('@/pages/company-profile').then(m => ({ default: m.CompanyProfilePage })));
const CommandCenterPage     = lazy(() => import('@/pages/command-center').then(m => ({ default: m.CommandCenterPage })));
const InvoicesPage          = lazy(() => import('@/pages/invoices').then(m => ({ default: m.InvoicesPage })));
const ProjectDeliverablesPage = lazy(() => import('@/pages/project-deliverables').then(m => ({ default: m.ProjectDeliverablesPage })));
const OrgDeliverablesPage = lazy(() => import('@/pages/org-deliverables').then(m => ({ default: m.OrgDeliverablesPage })));
const DataSourcesPage      = lazy(() => import('@/pages/data-sources'));
const DataSourceDetailPage = lazy(() => import('@/pages/data-source-detail').then(m => ({ default: m.DataSourceDetailPage })));
const DiscordPage             = lazy(() => import('@/pages/discord').then(m => ({ default: m.DiscordPage })));
const SiteDirectoryPage       = lazy(() => import('@/pages/site-directory').then(m => ({ default: m.SiteDirectoryPage })));
const MediaLibraryPage        = lazy(() => import('@/pages/media-library').then(m => ({ default: m.MediaLibraryPage })));
const ReviewPage              = lazy(() => import('@/pages/review').then(m => ({ default: m.ReviewPage })));
const OssLibraryListenerPage  = lazy(() => import('@/pages/oss-library-listener').then(m => ({ default: m.OssLibraryListenerPage })));

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
const KeysSettings      = lazy(() => import('@/pages/settings/KeysSettings').then(m => ({ default: m.KeysSettings })));
const NetworkSettings   = lazy(() => import('@/pages/settings/NetworkSettings').then(m => ({ default: m.NetworkSettings })));
const DeveloperSettings = lazy(() => import('@/pages/settings/DeveloperSettings').then(m => ({ default: m.DeveloperSettings })));
const TopsiAdminSettings = lazy(() => import('@/pages/settings/TopsiAdminSettings').then(m => ({ default: m.TopsiAdminSettings })));
const TopsiUserSettingsPage = lazy(() => import('@/pages/settings/TopsiUserSettings').then(m => ({ default: m.TopsiUserSettings })));
const TopsiActivityPage = lazy(() => import('@/pages/topsi-activity').then(m => ({ default: m.TopsiActivityPage })));
const BrandIntakePage   = lazy(() => import('@/pages/brand-intake').then(m => ({ default: m.BrandIntakePage })));
const BrandGuidePage    = lazy(() => import('@/pages/brand-guide').then(m => ({ default: m.BrandGuidePage })));
const CallIntakePage      = lazy(() => import('@/pages/call-intake'));
const BusinessReportsPage = lazy(() => import('@/pages/business-reports'));
const ReportDetailPage    = lazy(() => import('@/pages/business-reports').then(m => ({ default: m.ReportDetail })));
const PersonProfilePage   = lazy(() => import('@/pages/person-profile').then(m => ({ default: m.PersonProfilePage })));
const LeadsPage           = lazy(() => import('@/pages/leads').then(m => ({ default: m.LeadsPage })));

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

/** Redirects /workflows/staging/:runId to /workflows?tab=staging&run=:runId */
function StagingRedirect() {
  const { runId } = useParams();
  return <Navigate to={`/workflows?tab=staging&run=${runId}`} replace />;
}

function App() {
  return (
    <BrowserRouter>
      <SentryRoutes>
        {/* Embed routes - no layout, no auth */}
        <Route path="/embed/virtual-environment" element={
          <PageErrorBoundary label="Virtual Environment">
            <Suspense fallback={<PageLoader />}>
              <EmbedVirtualEnvironmentPage />
            </Suspense>
          </PageErrorBoundary>
        } />

        {/* Auth routes - clean layout, no sidebar/navbar */}
        <Route element={
          <AuthProvider>
            <AuthLayout />
          </AuthProvider>
        }>
          <Route path="/login" element={<LoginPage />} />
          <Route path="/signup" element={<SignupPage />} />
        </Route>

        {/* Authenticated app routes - full layout with sidebar/navbar */}
        <Route element={
          <AuthProvider>
            <UserSystemProvider>
              <OrganizationProvider>
                <ViewContextProvider>
                  <ProjectProvider>
                    <HotkeysProvider initiallyActiveScopes={['*', 'global', 'kanban']}>
                      <KeyboardShortcutsProvider>
                        <NiceModal.Provider>
                          <AppShell />
                        </NiceModal.Provider>
                      </KeyboardShortcutsProvider>
                    </HotkeysProvider>
                  </ProjectProvider>
                </ViewContextProvider>
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
          {/* Project CRM - scoped to project (from main) */}
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
          {/* Organization - base route */}
          <Route
            path="/organizations/:orgId"
            element={<ProtectedRoute><OrganizationProfilePage defaultTab="overview" /></ProtectedRoute>}
          />
          {/* Organization - CRM sub-routes */}
          <Route
            path="/organizations/:orgId/crm"
            element={<ProtectedRoute><OrganizationProfilePage defaultTab="overview" /></ProtectedRoute>}
          />
          <Route
            path="/organizations/:orgId/crm/contacts"
            element={<ProtectedRoute><OrganizationProfilePage defaultTab="contacts" /></ProtectedRoute>}
          />
          <Route
            path="/organizations/:orgId/crm/companies"
            element={<ProtectedRoute><CompaniesPage /></ProtectedRoute>}
          />
          <Route
            path="/organizations/:orgId/crm/pipeline"
            element={<ProtectedRoute><OrganizationProfilePage defaultTab="pipelines" /></ProtectedRoute>}
          />
          <Route
            path="/organizations/:orgId/crm/acquisition"
            element={<ProtectedRoute><OrganizationProfilePage defaultTab="pipelines" defaultPipeline="acquisition" /></ProtectedRoute>}
          />
          <Route
            path="/organizations/:orgId/crm/lifecycle"
            element={<ProtectedRoute><OrganizationProfilePage defaultTab="pipelines" defaultPipeline="lifecycle" /></ProtectedRoute>}
          />
          <Route
            path="/organizations/:orgId/crm/deliverables"
            element={<ProtectedRoute><OrgDeliverablesPage /></ProtectedRoute>}
          />
          {/* Organization - Social */}
          <Route
            path="/organizations/:orgId/social"
            element={<ProtectedRoute><OrganizationProfilePage defaultTab="social" /></ProtectedRoute>}
          />
          {/* Organization - Intelligence sub-routes */}
          <Route
            path="/organizations/:orgId/intelligence"
            element={<ProtectedRoute><OrganizationProfilePage defaultTab="intelligence" /></ProtectedRoute>}
          />
          <Route
            path="/organizations/:orgId/intelligence/data-sources"
            element={<ProtectedRoute><OrganizationProfilePage defaultTab="intelligence" /></ProtectedRoute>}
          />
          <Route
            path="/organizations/:orgId/intelligence/artifacts"
            element={<ProtectedRoute><OrganizationProfilePage defaultTab="intelligence" /></ProtectedRoute>}
          />
          <Route
            path="/organizations/:orgId/intelligence/workflows"
            element={<ProtectedRoute><OrganizationProfilePage defaultTab="intelligence" /></ProtectedRoute>}
          />
          <Route
            path="/organizations/:orgId/intelligence/pulse"
            element={<ProtectedRoute><OrganizationProfilePage defaultTab="intelligence" /></ProtectedRoute>}
          />
          <Route
            path="/organizations/:orgId/intelligence/topology"
            element={<ProtectedRoute><OrganizationProfilePage defaultTab="intelligence" /></ProtectedRoute>}
          />
          {/* Organization - Members, Projects, Integrations */}
          <Route
            path="/organizations/:orgId/members"
            element={<ProtectedRoute><OrganizationProfilePage defaultTab="members" /></ProtectedRoute>}
          />
          <Route
            path="/organizations/:orgId/projects"
            element={<ProtectedRoute><OrganizationProfilePage defaultTab="projects" /></ProtectedRoute>}
          />
          <Route
            path="/organizations/:orgId/integrations"
            element={<ProtectedRoute><OrganizationProfilePage defaultTab="integrations" /></ProtectedRoute>}
          />
          {/* Organization - Clients and Data Sources */}
          <Route
            path="/organizations/:orgId/clients/:clientId"
            element={<ProtectedRoute><ClientOverview /></ProtectedRoute>}
          />
          <Route
            path="/organizations/:orgId/data-sources"
            element={<ProtectedRoute><DataSourcesPage /></ProtectedRoute>}
          />
          <Route
            path="/organizations/:orgId/data-sources/:dataSourceId"
            element={<ProtectedRoute><DataSourceDetailPage /></ProtectedRoute>}
          />
          <Route path="/intake/:token" element={<BrandIntakePage />} />
          <Route
            path="/organizations/:orgId/brand-guide"
            element={<ProtectedRoute><BrandGuidePage /></ProtectedRoute>}
          />
          <Route
            path="/projects/:projectId/knowledge"
            element={<ProtectedRoute><KnowledgePage /></ProtectedRoute>}
          />
          <Route path="/my-tasks" element={<ProtectedRoute><MyTasksPage /></ProtectedRoute>} />
          <Route path="/site-directory" element={<AdminRoute><SiteDirectoryPage /></AdminRoute>} />
          <Route path="/nora" element={<AdminRoute><NoraPage /></AdminRoute>} />
          <Route path="/topsi" element={<ProtectedRoute><TopsiPage /></ProtectedRoute>} />
          <Route path="/topsi-activity" element={<AdminRoute><TopsiActivityPage /></AdminRoute>} />
          <Route path="/global-tasks" element={<AdminRoute><GlobalTasksPage /></AdminRoute>} />
          <Route path="/mission-control" element={<RoleRoute minRole="platform_member"><MissionControlPage /></RoleRoute>} />
          <Route path="/workflows" element={<ProtectedRoute><WorkflowsPage /></ProtectedRoute>} />
          <Route path="/workflows/staging/:runId" element={<ProtectedRoute><StagingRedirect /></ProtectedRoute>} />
          <Route
            path="/social-command"
            element={<RoleRoute minRole="org_editor"><SocialPage /></RoleRoute>}
          />
          <Route
            path="/projects/:projectId/social"
            element={<Navigate to="/social-command" replace />}
          />
          {/* Management routes — require operator+ role */}
          <Route
            path="/crm"
            element={<RoleRoute minRole="platform_member"><CrmPage /></RoleRoute>}
          />
          <Route
            path="/people"
            element={<RoleRoute minRole="platform_member"><PeoplePage /></RoleRoute>}
          />
          <Route
            path="/people/:personId"
            element={<RoleRoute minRole="platform_member"><PersonDetailPage /></RoleRoute>}
          />
          <Route
            path="/proposals"
            element={<RoleRoute minRole="platform_member"><ProposalsPage /></RoleRoute>}
          />
          <Route
            path="/companies"
            element={<RoleRoute minRole="platform_member"><CompaniesPage /></RoleRoute>}
          />
          <Route
            path="/companies/:companyId"
            element={<RoleRoute minRole="platform_member"><CompanyProfilePage /></RoleRoute>}
          />
          <Route
            path="/command-center"
            element={<RoleRoute minRole="platform_member"><CommandCenterPage /></RoleRoute>}
          />
          <Route
            path="/invoices"
            element={<RoleRoute minRole="platform_member"><InvoicesPage /></RoleRoute>}
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
            path="/projects/:projectId/media"
            element={<ProtectedRoute><MediaLibraryPage /></ProtectedRoute>}
          />
          <Route path="/review/:token" element={<ReviewPage />} />
          <Route
            path="/oss-library-listener"
            element={<ProtectedRoute><OssLibraryListenerPage /></ProtectedRoute>}
          />
          <Route
            path="/virtual-environment"
            element={<RoleRoute minRole="org_editor"><VirtualEnvironmentPage /></RoleRoute>}
          />
          <Route path="/mesh" element={<Navigate to="/settings/network" replace />} />
          <Route path="/pulse" element={<RoleRoute minRole="platform_member"><PulsePage /></RoleRoute>} />
          <Route
            path="/projects/:projectId/pulse"
            element={<ProtectedRoute><PulsePage /></ProtectedRoute>}
          />
          <Route path="/ai-usage" element={<AdminRoute><AIUsagePage /></AdminRoute>} />
          <Route path="/agent-executions" element={<AdminRoute><AgentExecutionsPage /></AdminRoute>} />
          <Route path="/vibe" element={<RoleRoute minRole="org_editor"><VibePage /></RoleRoute>} />
          <Route path="/calendar" element={<ProtectedRoute><CalendarPage /></ProtectedRoute>} />
          <Route path="/call-intake" element={<AdminRoute><CallIntakePage /></AdminRoute>} />
          <Route path="/business-reports" element={<AdminRoute><BusinessReportsPage /></AdminRoute>} />
          <Route path="/business-reports/:id" element={<AdminRoute><ReportDetailPage /></AdminRoute>} />
          <Route path="/persons/:personId" element={<ProtectedRoute><PersonProfilePage /></ProtectedRoute>} />
          <Route path="/leads" element={<AdminRoute><LeadsPage /></AdminRoute>} />
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
            <Route path="keys" element={<KeysSettings />} />
            <Route path="models" element={<ModelsSettings />} />
            <Route path="mcp" element={<McpSettings />} />
            <Route path="airtable" element={<Navigate to="/settings/general" replace />} />
            <Route path="network" element={<NetworkSettings />} />
            <Route path="topsi" element={<AdminRoute><TopsiAdminSettings /></AdminRoute>} />
            <Route path="topsi-preferences" element={<TopsiUserSettingsPage />} />
            <Route path="developer" element={<AdminRoute><DeveloperSettings /></AdminRoute>} />
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
