import { create } from 'zustand';
import { persist } from 'zustand/middleware';

interface ProjectOrderStore {
  orders: Record<string, string[]>; // scopeKey → ordered project IDs
  setOrder: (scopeKey: string, projectIds: string[]) => void;
  getOrderedProjects: <T extends { id: string }>(scopeKey: string, projects: T[]) => T[];
}

export const useProjectOrderStore = create<ProjectOrderStore>()(
  persist(
    (set, get) => ({
      orders: {},

      setOrder: (scopeKey, projectIds) => {
        set((state) => ({
          orders: { ...state.orders, [scopeKey]: projectIds },
        }));
      },

      getOrderedProjects: <T extends { id: string }>(scopeKey: string, projects: T[]): T[] => {
        const savedOrder = get().orders[scopeKey];
        if (!savedOrder || savedOrder.length === 0) return projects;

        const projectMap = new Map(projects.map((p) => [p.id, p]));
        const ordered: T[] = [];

        // Add projects in saved order
        for (const id of savedOrder) {
          const project = projectMap.get(id);
          if (project) {
            ordered.push(project);
            projectMap.delete(id);
          }
        }

        // Append any new projects not in saved order at the end
        for (const project of projects) {
          if (projectMap.has(project.id)) {
            ordered.push(project);
          }
        }

        return ordered;
      },
    }),
    {
      name: 'pcg-project-order-storage',
    }
  )
);
