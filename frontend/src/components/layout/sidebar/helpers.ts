import type { SidebarProject as SidebarProjectType } from '@/lib/api';

// Helper: count all projects recursively (including children)
export function countProjects(projects: SidebarProjectType[]): number {
  return projects.reduce((sum, p) => sum + 1 + countProjects(p.children || []), 0);
}

// Helper: check if a project exists in a tree (recursively checking children)
export function isProjectInTree(projects: SidebarProjectType[], projectId: string): boolean {
  return projects.some(p => p.id === projectId || isProjectInTree(p.children || [], projectId));
}
