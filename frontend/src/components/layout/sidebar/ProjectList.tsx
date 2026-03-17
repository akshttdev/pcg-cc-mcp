import { projectsApi } from '@/lib/api';
import type { SidebarProject as SidebarProjectType } from '@/lib/api';
import { useProjectOrderStore } from '@/stores/useProjectOrderStore';
import type { QueryClient } from '@tanstack/react-query';
import {
  DndContext,
  closestCenter,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from '@dnd-kit/core';
import {
  SortableContext,
  arrayMove,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable';
import { SortableSidebarProjectFolder } from './ProjectFolder';

// ============================================================================
// SortableProjectList — DnD wrapper for a list of projects within a scope
// ============================================================================

export function SortableProjectList({
  scopeKey,
  projects,
  projectId,
  expandedProjects,
  onToggleProject,
  queryClient,
  orgId,
  clientId,
}: {
  scopeKey: string;
  projects: SidebarProjectType[];
  projectId?: string;
  expandedProjects: Set<string>;
  onToggleProject: (id: string) => void;
  queryClient?: QueryClient;
  orgId?: string;
  clientId?: string;
}) {
  const { getOrderedProjects, setOrder } = useProjectOrderStore();

  // Deduplicate by project ID (keep first occurrence)
  const seen = new Set<string>();
  const uniqueProjects = projects.filter(p => {
    if (seen.has(p.id)) return false;
    seen.add(p.id);
    return true;
  });

  const orderedProjects = getOrderedProjects(scopeKey, uniqueProjects);

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } })
  );

  const handleDragEnd = async (event: DragEndEvent) => {
    const { active, over } = event;
    if (!over || active.id === over.id) return;

    const overId = String(over.id);

    // Check if dropped onto a container project
    if (overId.startsWith('container:')) {
      const parentId = overId.replace('container:', '');
      const projectDragId = String(active.id);
      try {
        await projectsApi.setParent(projectDragId, parentId);
        queryClient?.invalidateQueries({ queryKey: ['sidebarTree'] });
      } catch (err) {
        console.error('Failed to reparent project:', err);
      }
      return;
    }

    // Otherwise, it's a reorder within the list
    const oldIndex = orderedProjects.findIndex((p) => p.id === active.id);
    const newIndex = orderedProjects.findIndex((p) => p.id === over.id);
    if (oldIndex === -1 || newIndex === -1) return;

    const newOrder = arrayMove(orderedProjects, oldIndex, newIndex);
    setOrder(scopeKey, newOrder.map((p) => p.id));
  };

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCenter}
      onDragEnd={handleDragEnd}
    >
      <SortableContext
        items={orderedProjects.map((p) => p.id)}
        strategy={verticalListSortingStrategy}
      >
        {orderedProjects.map((project) => (
          <SortableSidebarProjectFolder
            key={project.id}
            project={project}
            projectId={projectId}
            isExpanded={expandedProjects.has(project.id)}
            onToggle={() => onToggleProject(project.id)}
            expandedProjects={expandedProjects}
            onToggleProject={onToggleProject}
            queryClient={queryClient}
            orgId={orgId}
            clientId={clientId}
          />
        ))}
      </SortableContext>
    </DndContext>
  );
}
