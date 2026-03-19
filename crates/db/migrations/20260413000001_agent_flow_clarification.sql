-- Add clarification support to agent_flows
-- Stores clarification request JSON when status = 'needs_clarification'
ALTER TABLE agent_flows ADD COLUMN clarification_request TEXT;
