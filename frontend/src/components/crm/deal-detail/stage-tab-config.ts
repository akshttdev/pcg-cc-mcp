/**
 * Stage-aware tab visibility configuration.
 *
 * Maps normalized stage names to the tabs that should be visible.
 * Tabs accumulate as deals progress — later stages include earlier tabs.
 * Use the "Show all tabs" toggle to reveal everything.
 *
 * Backlog:
 * - stage_config.visible_tabs: drive tab visibility from backend config
 * - Progressive unlock: tabs unlock as deal advances, never hide once seen
 * - Per-pipeline tab config in pipeline settings UI
 */

/** All available tab values in display order */
export const ALL_TAB_VALUES = [
  'overview',
  'intel',
  'review',
  'transcripts',
  'proposal',
  'deck',
  'projects',
  'activity',
  'agents',
] as const;

/** Tabs visible per stage (lowercase, normalized). Falls back to all tabs for unknown stages. */
export const STAGE_TAB_MAP: Record<string, readonly string[]> = {
  'lead':                ['overview', 'activity'],
  'intel':               ['overview', 'intel', 'transcripts', 'review', 'activity', 'agents'],
  'business_analysis':   ['overview', 'intel', 'transcripts', 'review', 'activity', 'agents'],
  'discovery':           ['overview', 'intel', 'transcripts', 'review', 'activity', 'agents'],
  'proposal':            ['overview', 'intel', 'transcripts', 'proposal', 'review', 'activity', 'agents'],
  'build_proposal':      ['overview', 'intel', 'transcripts', 'proposal', 'review', 'activity', 'agents'],
  'polish':              ['overview', 'intel', 'transcripts', 'proposal', 'deck', 'review', 'activity', 'agents'],
  'present':             ['overview', 'intel', 'transcripts', 'proposal', 'deck', 'review', 'activity', 'agents'],
  'present_&_invoice':   ['overview', 'intel', 'transcripts', 'proposal', 'deck', 'review', 'activity', 'agents'],
  'invoice':             ['overview', 'intel', 'transcripts', 'proposal', 'deck', 'review', 'activity', 'agents'],
  'proposal_meeting':    ['overview', 'intel', 'transcripts', 'proposal', 'deck', 'review', 'activity', 'agents'],
  'negotiation':         ['overview', 'intel', 'transcripts', 'proposal', 'deck', 'review', 'activity', 'agents'],
  'follow_up':           ['overview', 'intel', 'transcripts', 'proposal', 'deck', 'review', 'activity', 'agents'],
  'won':                 ['overview', 'intel', 'transcripts', 'proposal', 'deck', 'projects', 'activity', 'agents'],
  'closed_won':          ['overview', 'intel', 'transcripts', 'proposal', 'deck', 'projects', 'activity', 'agents'],
  'lost':                ['overview', 'intel', 'transcripts', 'proposal', 'deck', 'activity', 'agents'],
  'closed_lost':         ['overview', 'intel', 'transcripts', 'proposal', 'deck', 'activity', 'agents'],
};

/** Get visible tabs for a stage name. Normalizes to lowercase + underscores. */
export function getVisibleTabs(stageName: string): readonly string[] {
  const normalized = stageName.toLowerCase().replace(/\s+/g, '_');
  return STAGE_TAB_MAP[normalized] ?? ALL_TAB_VALUES;
}
