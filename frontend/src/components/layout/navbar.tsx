import { useCallback } from 'react';
import { useNavigate, useLocation, Link } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import {
  FolderOpen,
  Settings,
  Plus,
  Command as CommandIcon,
  Menu,
} from 'lucide-react';
import { SearchBar } from '@/components/search-bar';
import { useSearch } from '@/contexts/search-context';
import { openTaskForm } from '@/lib/openTaskForm';
import { useProject } from '@/contexts/project-context';
import { useOrganization } from '@/contexts/organization-context';
import { showProjectForm } from '@/lib/modals';
import { useOpenProjectInEditor } from '@/hooks/useOpenProjectInEditor';
import { useCommandStore } from '@/stores/useCommandStore';
import { NotificationCenter } from '@/components/notifications/NotificationCenter';
import { NavbarUserButton } from '@/components/layout/NavbarUserButton';

const ADMIN_ROUTES = ['/site-directory', '/nora', '/mission-control', '/admin'];

function ScopeIndicator() {
  const location = useLocation();
  const { organization, orgId } = useOrganization();
  const { project, projectId } = useProject();

  // Project scope takes priority (more specific)
  if (projectId && project) {
    return (
      <Link to={`/projects/${projectId}`} className="inline-flex items-center px-2 py-0.5 rounded-full text-[11px] font-medium bg-emerald-500/15 text-emerald-600 dark:text-emerald-400 border border-emerald-500/20 truncate max-w-[180px] hover:bg-emerald-500/25 transition-colors">
        Project: {project.name}
      </Link>
    );
  }

  if (orgId && organization) {
    return (
      <Link to={`/organizations/${orgId}`} className="inline-flex items-center px-2 py-0.5 rounded-full text-[11px] font-medium bg-blue-500/15 text-blue-600 dark:text-blue-400 border border-blue-500/20 truncate max-w-[180px] hover:bg-blue-500/25 transition-colors">
        Org: {organization.name}
      </Link>
    );
  }

  if (ADMIN_ROUTES.some((r) => location.pathname.startsWith(r))) {
    return (
      <Link to="/site-directory" className="inline-flex items-center px-2 py-0.5 rounded-full text-[11px] font-medium bg-orange-500/15 text-orange-600 dark:text-orange-400 border border-orange-500/20 hover:bg-orange-500/25 transition-colors">
        Admin
      </Link>
    );
  }

  return null;
}

interface NavbarProps {
  onToggleSidebar?: () => void;
}

export function Navbar({ onToggleSidebar }: NavbarProps) {
  const navigate = useNavigate();
  const { projectId, project } = useProject();
  const { query, setQuery, active, clear, registerInputRef } = useSearch();
  const handleOpenInEditor = useOpenProjectInEditor(project || null);
  const { openCommandPalette } = useCommandStore();

  const setSearchBarRef = useCallback(
    (node: HTMLInputElement | null) => {
      registerInputRef(node);
    },
    [registerInputRef]
  );

  const handleCreateTask = () => {
    if (projectId) {
      openTaskForm({ projectId });
    }
  };

  const handleOpenInIDE = () => {
    handleOpenInEditor();
  };

  const handleProjectSettings = async () => {
    try {
      await showProjectForm({ project });
    } catch (error) {
      // User cancelled
    }
  };

  return (
    <div className="border-b border-border/40 bg-card/80 backdrop-blur-xl sticky top-0 z-30">
      <div className="w-full max-w-[1600px] mx-auto px-3 sm:px-4 lg:px-6">
        <div className="flex items-center h-14 py-2 gap-2 sm:gap-3">
          {/* Mobile menu button */}
          <Button
            variant="ghost"
            size="icon"
            onClick={onToggleSidebar}
            className="lg:hidden shrink-0 h-8 w-8"
            aria-label="Toggle navigation"
          >
            <Menu className="h-5 w-5" />
          </Button>

          {/* Logo */}
          <div className="flex items-center mr-3 sm:mr-5 cursor-pointer hover:opacity-80 transition-opacity shrink-0" onClick={() => navigate('/projects')}>
            <img
              src="/pcg-globe-logo.png"
              alt="Powerclub Global"
              className="h-8 sm:h-9 w-auto"
            />
            <span className="ml-2 text-sm sm:text-base font-semibold hidden sm:inline tracking-widest uppercase" style={{ fontFamily: "'Cinzel', serif", color: 'var(--brand-gold)' }}>Powerclub Global</span>
          </div>

          <div className="hidden sm:flex items-center shrink-0">
            <ScopeIndicator />
          </div>

          <div className="flex-1 min-w-0">
            <SearchBar
              ref={setSearchBarRef}
              className="max-w-md"
              value={query}
              onChange={setQuery}
              disabled={!active}
              onClear={clear}
              project={project || null}
            />
          </div>

          <div className="flex items-center gap-1 sm:gap-2 shrink-0">
            {/* Command Palette — desktop */}
            <Button
              variant="outline"
              size="sm"
              onClick={openCommandPalette}
              className="gap-2 text-muted-foreground hidden sm:inline-flex"
            >
              <CommandIcon className="h-3.5 w-3.5" />
              <span className="text-xs hidden md:inline">Quick Actions</span>
              <kbd className="kbd hidden md:inline-flex">
                <span className="text-xs">&#8984;</span>K
              </kbd>
            </Button>
            {/* Command Palette — mobile icon */}
            <Button
              variant="ghost"
              size="icon"
              onClick={openCommandPalette}
              className="sm:hidden h-8 w-8"
              aria-label="Quick actions"
            >
              <CommandIcon className="h-4 w-4" />
            </Button>

            {projectId && (
              <>
                <div className="h-4 w-px bg-border/50 hidden sm:block mx-1" />

                <Button
                  variant="ghost"
                  size="icon"
                  onClick={handleOpenInIDE}
                  aria-label="Open project in IDE"
                  className="hidden md:inline-flex h-8 w-8"
                >
                  <FolderOpen className="h-4 w-4" />
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={handleProjectSettings}
                  aria-label="Project settings"
                  className="hidden md:inline-flex h-8 w-8"
                >
                  <Settings className="h-4 w-4" />
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={handleCreateTask}
                  aria-label="Create new task"
                  className="h-8 w-8"
                >
                  <Plus className="h-4 w-4" />
                </Button>

                <div className="h-4 w-px bg-border/50 hidden sm:block mx-1" />
              </>
            )}

            <NotificationCenter />

            {/* Mobile user avatar — quick access without opening sidebar */}
            <NavbarUserButton />
          </div>
        </div>
      </div>
    </div>
  );
}
