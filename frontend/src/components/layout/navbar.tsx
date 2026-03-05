import { useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import {
  FolderOpen,
  Settings,
  Plus,
  Command as CommandIcon,
  Menu,
} from 'lucide-react';
import { SearchBar } from '@/components/search-bar';
import { ProfileSection } from '@/components/layout/profile-section';
import { useSearch } from '@/contexts/search-context';
import { openTaskForm } from '@/lib/openTaskForm';
import { useProject } from '@/contexts/project-context';
import { showProjectForm } from '@/lib/modals';
import { useOpenProjectInEditor } from '@/hooks/useOpenProjectInEditor';
import { useCommandStore } from '@/stores/useCommandStore';

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
              src="/orcha-logo.png"
              alt="ORCHA"
              className="h-7 sm:h-8 w-auto"
            />
            <span className="ml-2 text-base sm:text-lg font-bold tracking-wide hidden sm:inline">ORCHA</span>
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

            <ProfileSection />
          </div>
        </div>
      </div>
    </div>
  );
}
