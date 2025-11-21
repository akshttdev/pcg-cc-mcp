import { useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
} from '@/components/ui/command';
import { FolderOpen, Plus, Settings, Clock, Star, FileText } from 'lucide-react';
import { useCommandStore } from '@/stores/useCommandStore';
import { useQuery } from '@tanstack/react-query';
import { projectsApi, tasksApi } from '@/lib/api';
import { useProject } from '@/contexts/project-context';
import { openTaskForm } from '@/lib/openTaskForm';
import { showProjectForm } from '@/lib/modals';
import { useAuth } from '@/contexts/AuthContext';

export function CommandPalette() {
  const navigate = useNavigate();
  const { projectId } = useProject();
  const { user } = useAuth();
  const {
    isOpen,
    closeCommandPalette,
    addToHistory,
    getRecentItems,
    favorites
  } = useCommandStore();

  // Only admins can create projects
  const isAdmin = user?.is_admin ?? false;

  // Fetch projects
  const { data: projects = [] } = useQuery({
    queryKey: ['projects'],
    queryFn: projectsApi.getAll,
  });

  // Fetch tasks for current project
  const { data: tasks = [] } = useQuery({
    queryKey: ['tasks', projectId],
    queryFn: () => (projectId ? tasksApi.getAll(projectId) : Promise.resolve([])),
    enabled: !!projectId,
  });

  const recentItems = getRecentItems(5);

  // Handle keyboard shortcut (⌘K)
  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if (e.key === 'k' && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        useCommandStore.getState().toggleCommandPalette();
      }
    };

    document.addEventListener('keydown', down);
    return () => document.removeEventListener('keydown', down);
  }, []);

  const handleSelect = useCallback(
    (callback: () => void) => {
      closeCommandPalette();
      callback();
    },
    [closeCommandPalette]
  );

  return (
    <CommandDialog open={isOpen} onOpenChange={closeCommandPalette}>
      <CommandInput placeholder="Type a command or search..." />
      <CommandList>
        <CommandEmpty>No results found.</CommandEmpty>

        {/* Recent Items */}
        {recentItems.length > 0 && (
          <>
            <CommandGroup heading="Recent">
              {recentItems.map((item) => (
                <CommandItem
                  key={item.id}
                  onSelect={() =>
                    handleSelect(() => {
                      if (item.resourceType === 'project' && item.resourceId) {
                        navigate(`/projects/${item.resourceId}`);
                      } else if (item.resourceType === 'task' && item.resourceId) {
                        navigate(`/projects/${projectId}/tasks/${item.resourceId}`);
                      }
                    })
                  }
                >
                  <Clock className="mr-2 h-4 w-4" />
                  <span>{item.resourceName}</span>
                </CommandItem>
              ))}
            </CommandGroup>
            <CommandSeparator />
          </>
        )}

        {/* Favorites */}
        {favorites.length > 0 && (
          <>
            <CommandGroup heading="Favorites">
              {favorites.map((fav) => (
                <CommandItem
                  key={fav.id}
                  onSelect={() =>
                    handleSelect(() => {
                      addToHistory({
                        commandType: 'project',
                        resourceId: fav.projectId,
                        resourceType: 'project',
                        resourceName: fav.projectName,
                      });
                      navigate(`/projects/${fav.projectId}/tasks`);
                    })
                  }
                >
                  <Star className="mr-2 h-4 w-4" />
                  <span>{fav.projectName}</span>
                </CommandItem>
              ))}
            </CommandGroup>
            <CommandSeparator />
          </>
        )}

        {/* Actions */}
        <CommandGroup heading="Actions">
          <CommandItem
            onSelect={() =>
              handleSelect(() => {
                if (projectId) {
                  openTaskForm({ projectId });
                }
              })
            }
          >
            <Plus className="mr-2 h-4 w-4" />
            <span>Create Task</span>
            <kbd className="ml-auto text-xs">C</kbd>
          </CommandItem>
          {isAdmin && (
            <CommandItem
              onSelect={() =>
                handleSelect(async () => {
                  try {
                    await showProjectForm();
                  } catch (error) {
                    // User cancelled
                  }
                })
              }
            >
              <Plus className="mr-2 h-4 w-4" />
              <span>Create Project</span>
            </CommandItem>
          )}
          <CommandItem
            onSelect={() => handleSelect(() => navigate('/settings'))}
          >
            <Settings className="mr-2 h-4 w-4" />
            <span>Settings</span>
          </CommandItem>
        </CommandGroup>

        {/* Projects */}
        {projects.length > 0 && (
          <>
            <CommandSeparator />
            <CommandGroup heading="Projects">
              {projects.slice(0, 5).map((proj) => (
                <CommandItem
                  key={proj.id}
                  onSelect={() =>
                    handleSelect(() => {
                      addToHistory({
                        commandType: 'project',
                        resourceId: proj.id,
                        resourceType: 'project',
                        resourceName: proj.name,
                      });
                      navigate(`/projects/${proj.id}/tasks`);
                    })
                  }
                >
                  <FolderOpen className="mr-2 h-4 w-4" />
                  <span>{proj.name}</span>
                </CommandItem>
              ))}
            </CommandGroup>
          </>
        )}

        {/* Tasks (if in a project) */}
        {tasks.length > 0 && projectId && (
          <>
            <CommandSeparator />
            <CommandGroup heading="Tasks">
              {tasks.slice(0, 5).map((task) => (
                <CommandItem
                  key={task.id}
                  onSelect={() =>
                    handleSelect(() => {
                      addToHistory({
                        commandType: 'task',
                        resourceId: task.id,
                        resourceType: 'task',
                        resourceName: task.title,
                      });
                      navigate(`/projects/${projectId}/tasks/${task.id}`);
                    })
                  }
                >
                  <FileText className="mr-2 h-4 w-4" />
                  <span>{task.title}</span>
                </CommandItem>
              ))}
            </CommandGroup>
          </>
        )}
      </CommandList>
    </CommandDialog>
  );
}
