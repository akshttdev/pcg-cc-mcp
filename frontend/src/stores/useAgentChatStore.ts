import { create } from 'zustand';

export type WidgetState = 'collapsed' | 'chat' | 'call' | 'meeting';

export interface AgentContext {
  entityType: 'project' | 'task' | 'crm_contact' | 'crm_deal';
  entityId: string;
  entityName: string;
}

interface AgentChatStore {
  // State
  widgetState: WidgetState;
  activeAgent: string;
  pendingMessage: string | null;
  pendingContext: AgentContext | null;

  // Actions
  openChat: (message?: string, context?: AgentContext, agent?: string) => void;
  collapse: () => void;
  setWidgetState: (state: WidgetState) => void;
  clearPending: () => void;
}

export const useAgentChatStore = create<AgentChatStore>()((set) => ({
  widgetState: 'collapsed',
  activeAgent: 'topsi',
  pendingMessage: null,
  pendingContext: null,

  openChat: (message, context, agent) => {
    set({
      widgetState: 'chat',
      pendingMessage: message ?? null,
      pendingContext: context ?? null,
      ...(agent ? { activeAgent: agent } : {}),
    });
  },

  collapse: () => {
    set({ widgetState: 'collapsed' });
  },

  setWidgetState: (state) => {
    set({ widgetState: state });
  },

  clearPending: () => {
    set({ pendingMessage: null, pendingContext: null });
  },
}));
