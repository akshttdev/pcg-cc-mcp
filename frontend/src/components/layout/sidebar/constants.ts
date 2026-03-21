import {
  FolderOpen,
  Settings,
  BookOpen,
  MessageCircleQuestion,
  AlertTriangle,
  ListTodo,
  Network,
  Users,
  Building2,
  Receipt,
  LayoutDashboard,
  Box,
  Crown,
  Bot,
  Calendar,
  Activity,
  Globe,
  Headphones,
  Workflow,
  Map,
  Coins,
  Rocket,
  Cpu,
  Megaphone,
  PhoneIncoming,
  ClipboardList,
  FileText,
  Brain,
} from 'lucide-react';

// Navigation items with role-based visibility
export interface NavItem {
  label: string;
  icon: typeof FolderOpen;
  to: string;
  id: string;
  adminOnly?: boolean;
  memberOnly?: boolean;
  tooltip?: string;
}

// Admin tools — separated visually at top
export const ADMIN_NAV_ITEMS: NavItem[] = [
  { label: 'Site Directory', icon: Map, to: '/site-directory', id: 'site-directory', adminOnly: true },
  { label: 'Nora Command', icon: Crown, to: '/nora', id: 'nora', adminOnly: true },
  { label: 'Topsi Platform', icon: Network, to: '/topsi', id: 'topsi', adminOnly: true },
  { label: 'Mission Control', icon: Rocket, to: '/mission-control', id: 'mission-control', adminOnly: true },
  { label: 'Pulse Engine', icon: Activity, to: '/pulse', id: 'pulse', adminOnly: true },
  { label: 'Mesh Network', icon: Globe, to: '/mesh', id: 'mesh', adminOnly: true },
  { label: 'Agent Executions', icon: Bot, to: '/agent-executions', id: 'agent-executions', adminOnly: true },
  { label: 'Topsi Activity', icon: Activity, to: '/topsi-activity', id: 'topsi-activity', adminOnly: true },
];

// Primary navigation - workspace destinations (user-level pages)
export const PRIMARY_NAV_ITEMS: NavItem[] = [
  { label: 'Intelligence', icon: Brain, to: '/intelligence', id: 'intelligence' },
  { label: 'My Projects', icon: FolderOpen, to: '/projects', id: 'projects' },
  { label: 'My Tasks', icon: ListTodo, to: '/my-tasks', id: 'my-tasks', memberOnly: true },
  { label: 'My Workflows', icon: Workflow, to: '/workflows', id: 'workflows' },
  { label: 'Calendar', icon: Calendar, to: '/calendar', id: 'calendar' },
  { label: 'VIBELAND', icon: Box, to: '/virtual-environment', id: 'virtual-environment', tooltip: '3D virtual environment' },
  { label: 'VIBE', icon: Coins, to: '/vibe', id: 'vibe', tooltip: 'Token treasury & transactions' },
];

// Management nav — admin-only, collapsible
export const MANAGEMENT_NAV_ITEMS: NavItem[] = [
  { label: 'All People', icon: Users, to: '/people', id: 'people', adminOnly: true },
  { label: 'All Companies', icon: Building2, to: '/companies', id: 'companies', adminOnly: true },
  { label: 'Proposals', icon: FileText, to: '/proposals', id: 'proposals', adminOnly: true },
  { label: 'Invoices', icon: Receipt, to: '/invoices', id: 'invoices', adminOnly: true },
  { label: 'Call Intake', icon: PhoneIncoming, to: '/call-intake', id: 'call-intake', adminOnly: true },
  { label: 'Reports', icon: ClipboardList, to: '/business-reports', id: 'business-reports', adminOnly: true },
  { label: 'Command Center', icon: LayoutDashboard, to: '/command-center', id: 'command-center', adminOnly: true },
  { label: 'Discord Voice', icon: Headphones, to: '/discord', id: 'discord', adminOnly: true },
  { label: 'AI Usage', icon: Cpu, to: '/ai-usage', id: 'ai-usage', adminOnly: true },
];

// Global views - admin only, collapsible
export const GLOBAL_VIEW_ITEMS: NavItem[] = [
  { label: 'All Tasks', icon: ListTodo, to: '/global-tasks', id: 'global-tasks', adminOnly: true },
  { label: 'CRM Admin', icon: Users, to: '/crm', id: 'crm', adminOnly: true },
  { label: 'All Social', icon: Megaphone, to: '/social-command', id: 'social-command', adminOnly: true },
];

// Utility nav — pinned to bottom above external links
export const UTILITY_NAV_ITEMS: NavItem[] = [
  { label: 'Settings', icon: Settings, to: '/settings', id: 'settings' },
];

export const EXTERNAL_LINKS = [
  {
    label: 'Docs',
    icon: BookOpen,
    href: 'https://duckkanban.com/docs',
    external: true,
  },
  {
    label: 'Feedback & Support',
    icon: MessageCircleQuestion,
    action: 'feedback',
    external: false,
  },
  {
    label: 'Report Friction',
    icon: AlertTriangle,
    action: 'friction',
    external: false,
  },
] as const;
