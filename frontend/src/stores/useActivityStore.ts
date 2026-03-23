import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { nanoid } from 'nanoid';
import type { ActivityEntry, ActivityType, ActivityFilter } from '@/types/activity';

interface ActivityStore {
  // State
  activities: ActivityEntry[];

  // Actions
  logActivity: (
    taskId: string,
    type: ActivityType,
    description: string,
    metadata?: Record<string, unknown>
  ) => void;
  getActivitiesForTask: (taskId: string) => ActivityEntry[];
  getRecentActivities: (limit?: number) => ActivityEntry[];
  getFilteredActivities: (filter: ActivityFilter) => ActivityEntry[];
  clearActivities: (taskId?: string) => void;
}

export const useActivityStore = create<ActivityStore>()(
  persist(
    (set, get) => ({
      activities: [],

      logActivity: (taskId, type, description, metadata) => {
        const activity: ActivityEntry = {
          id: nanoid(),
          taskId,
          type,
          description,
          metadata,
          timestamp: new Date().toISOString(),
        };

        set((state) => ({
          activities: [activity, ...state.activities].slice(0, 1000),
        }));
      },

      getActivitiesForTask: (taskId) => {
        return get()
          .activities
          .filter((activity) => activity.taskId === taskId)
          .sort(
            (a, b) =>
              new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime()
          );
      },

      getRecentActivities: (limit = 50) => {
        return get()
          .activities
          .sort(
            (a, b) =>
              new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime()
          )
          .slice(0, limit);
      },

      getFilteredActivities: (filter) => {
        let filtered = get().activities;

        if (filter.types && filter.types.length > 0) {
          filtered = filtered.filter((activity) =>
            filter.types!.includes(activity.type)
          );
        }

        if (filter.taskIds && filter.taskIds.length > 0) {
          filtered = filtered.filter((activity) =>
            filter.taskIds!.includes(activity.taskId)
          );
        }

        if (filter.startDate) {
          filtered = filtered.filter(
            (activity) => new Date(activity.timestamp) >= filter.startDate!
          );
        }

        if (filter.endDate) {
          filtered = filtered.filter(
            (activity) => new Date(activity.timestamp) <= filter.endDate!
          );
        }

        return filtered.sort(
          (a, b) =>
            new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime()
        );
      },

      clearActivities: (taskId) => {
        if (taskId) {
          set((state) => ({
            activities: state.activities.filter(
              (activity) => activity.taskId !== taskId
            ),
          }));
        } else {
          set({ activities: [] });
        }
      },
    }),
    {
      name: 'activity-storage',
    }
  )
);
