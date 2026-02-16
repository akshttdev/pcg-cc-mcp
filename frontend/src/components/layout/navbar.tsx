import { useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import {
  FolderOpen,
  Settings,
  Plus,
  Command as CommandIcon,
} from 'lucide-react';
import { SearchBar } from '@/components/search-bar';
import { ProfileSection } from '@/components/layout/profile-section';
import { useSearch } from '@/contexts/search-context';
import { openTaskForm } from '@/lib/openTaskForm';
import { useProject } from '@/contexts/project-context';
import { showProjectForm } from '@/lib/modals';
import { useOpenProjectInEditor } from '@/hooks/useOpenProjectInEditor';
import { useCommandStore } from '@/stores/useCommandStore';


export function Navbar() {
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
      // Settings saved successfully - no additional action needed
    } catch (error) {
      // User cancelled - do nothing
    }
  };

  return (
    <div className="border-b bg-background">
      <div className="w-full px-4">
        <div className="flex items-center h-14 py-2">
          {/* Logo */}
          <div className="flex items-center mr-6 cursor-pointer hover:opacity-80 transition-opacity" onClick={() => navigate('/projects')}>
            <img
              src="/orcha-logo.png"
              alt="ORCHA"
              className="h-8 w-auto"
            />
            <span className="ml-2 text-lg font-bold tracking-wide">ORCHA</span>
          </div>

          <div className="flex-1">
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

          <div className="flex items-center gap-3">
            {/* Command Palette Button */}
            <Button
              variant="outline"
              size="sm"
              onClick={openCommandPalette}
              className="gap-2 text-muted-foreground"
            >
              <CommandIcon className="h-4 w-4" />
              <span className="text-xs">Quick Actions</span>
              <kbd className="pointer-events-none inline-flex h-5 select-none items-center gap-1 rounded border bg-muted px-1.5 font-mono text-[10px] font-medium text-muted-foreground opacity-100">
                <span className="text-xs">⌘</span>K
              </kbd>
            </Button>

            {projectId && (
              <>
                {/* Separator */}
                <div className="h-4 w-px bg-border" />

                <Button
                  variant="ghost"
                  size="icon"
                  onClick={handleOpenInIDE}
                  aria-label="Open project in IDE"
                >
                  <FolderOpen className="h-4 w-4" />
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={handleProjectSettings}
                  aria-label="Project settings"
                >
                  <Settings className="h-4 w-4" />
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={handleCreateTask}
                  aria-label="Create new task"
                >
                  <Plus className="h-4 w-4" />
                </Button>

                {/* Separator */}
                <div className="h-4 w-px bg-border" />
              </>
            )}

            <ProfileSection />
          </div>
        </div>
      </div>
    </div>
  );
}
