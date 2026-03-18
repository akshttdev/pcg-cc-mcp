import { useMemo } from 'react';
import type { AgentWithParsedFields, AgentFlowEvent } from 'shared/types';

interface ResolvedAgent {
  displayName: string;
  tooltip: string;
}

/**
 * Resolves an agent's friendly display name and tooltip from agentsMap.
 * Accepts a raw agent name string or extracts one from workflow events.
 */
export function useResolvedAgent(
  assignedAgent: string | null | undefined,
  agentsMap?: Map<string, AgentWithParsedFields>,
  workflowEvents?: AgentFlowEvent[],
): ResolvedAgent | null {
  return useMemo(() => {
    // Resolve raw name: prefer assigned_agent, fall back to workflow events
    let rawName = assignedAgent || undefined;
    if (!rawName && workflowEvents) {
      for (const event of workflowEvents) {
        try {
          const data = JSON.parse(event.event_data || '{}');
          if (data.agent_name) { rawName = data.agent_name; break; }
        } catch { /* skip */ }
      }
    }
    if (!rawName) return null;

    // Look up friendly name from agentsMap
    const entry = agentsMap
      ? Array.from(agentsMap.values()).find(
          a => a.short_name === rawName || a.designation === rawName
        )
      : undefined;

    const displayName = entry?.short_name || rawName;
    const tooltip = entry?.designation
      ? `Agent: ${displayName} (${entry.designation})`
      : `Agent: ${displayName}`;

    return { displayName, tooltip };
  }, [assignedAgent, agentsMap, workflowEvents]);
}
