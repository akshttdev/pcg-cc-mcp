import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export type ViewType = 'board' | 'table' | 'gallery' | 'timeline' | 'calendar';

export interface ViewConfig {
  id: string;
  projectId: string;
  name: string;
  viewType: ViewType;
  filters: Record<string, any>;
  sorts: Array<{ field: string; direction: 'asc' | 'desc' }>;
  visibleProperties: string[];
}

interface ViewStore {
  // Current view state
  currentViewType: ViewType;
  currentViewId: string | null;

  // Enhanced cards preference
  useEnhancedCards: boolean;

  // Saved views
  savedViews: Record<string, ViewConfig[]>; // projectId -> views[]

  // Actions
  setViewType: (viewType: ViewType) => void;
  setCurrentView: (viewId: string | null) => void;
  setUseEnhancedCards: (enabled: boolean) => void;
  saveView: (view: ViewConfig) => void;
  deleteView: (projectId: string, viewId: string) => void;
  getSavedViews: (projectId: string) => ViewConfig[];
  getCurrentView: (projectId: string) => ViewConfig | null;
}

export const useViewStore = create<ViewStore>()(
  persist(
    (set, get) => ({
      currentViewType: 'board',
      currentViewId: null,
      useEnhancedCards: true, // Default to enhanced cards
      savedViews: {},

      setViewType: (viewType) => set({ currentViewType: viewType }),

      setCurrentView: (viewId) => set({ currentViewId: viewId }),

      setUseEnhancedCards: (enabled) => set({ useEnhancedCards: enabled }),

      saveView: (view) =>
        set((state) => {
          const projectViews = state.savedViews[view.projectId] || [];
          const existingIndex = projectViews.findIndex((v) => v.id === view.id);

          const updatedViews =
            existingIndex >= 0
              ? projectViews.map((v, i) => (i === existingIndex ? view : v))
              : [...projectViews, view];

          return {
            savedViews: {
              ...state.savedViews,
              [view.projectId]: updatedViews,
            },
          };
        }),

      deleteView: (projectId, viewId) =>
        set((state) => {
          const projectViews = state.savedViews[projectId] || [];
          return {
            savedViews: {
              ...state.savedViews,
              [projectId]: projectViews.filter((v) => v.id !== viewId),
            },
          };
        }),

      getSavedViews: (projectId) => {
        return get().savedViews[projectId] || [];
      },

      getCurrentView: (projectId) => {
        const { currentViewId, savedViews } = get();
        if (!currentViewId) return null;
        const projectViews = savedViews[projectId] || [];
        return projectViews.find((v) => v.id === currentViewId) || null;
      },
    }),
    {
      name: 'pcg-view-storage',
    }
  )
);
