import {
  Backpack,
  Cpu,
  Gamepad2,
  Layers,
  Map as MapIcon,
  Shirt,
} from 'lucide-react';
import type { Project } from 'shared/types';
import type { HudNavItem, HudPanelId, ProjectData, VirtualZone } from './types';

// Topos directory items for the Data Sphere visualization
export const toposDirectoryItems = (typeof __TOPOS_PROJECTS__ !== 'undefined'
  ? __TOPOS_PROJECTS__
  : []) as string[];

export const PROJECT_HALF_WIDTH = 25;
export const PROJECT_HALF_LENGTH = 50;
export const PROJECT_FOOTPRINT_RADIUS = Math.sqrt(PROJECT_HALF_WIDTH ** 2 + PROJECT_HALF_LENGTH ** 2);
export const COMMAND_CENTER_FLOOR_Y = 80; // Elevated floating platform
export const BASE_PROJECT_RADIUS = 180; // Projects arranged around the data sphere
export const TARGET_ARC_SPACING = PROJECT_FOOTPRINT_RADIUS * 2.2;

export const ZONE_RING_RADIUS = 280;

export const HUD_NAV_ITEMS: HudNavItem[] = [
  { id: 'systems', label: 'Systems', description: 'Server diagnostics', icon: Cpu },
  { id: 'intel', label: 'Intel', description: 'Project dossiers', icon: Layers },
  { id: 'map', label: 'Cartography', description: 'Spatial telemetry', icon: MapIcon },
  { id: 'controls', label: 'Controls', description: 'Piloting reference', icon: Gamepad2 },
  { id: 'inventory', label: 'Inventory', description: 'Personal belongings', icon: Backpack },
  { id: 'equipment', label: 'Gear', description: 'Equipped items', icon: Shirt },
];

export const HUD_PANEL_META: Record<HudPanelId, { title: string; description: string }> = {
  systems: { title: 'Systems Console', description: 'Monitor command center throughput and structural integrity.' },
  intel: { title: 'Intel Ledger', description: 'Active engagements ranked by signal strength.' },
  map: { title: 'Aerial Cartography', description: 'Top-down sweep of the monumental grid.' },
  controls: { title: 'Flight Controls', description: 'Reference for movement, chat, and interaction shortcuts.' },
  inventory: { title: 'Inventory', description: 'Items in your possession. Click to equip.' },
  equipment: { title: 'Equipment', description: 'Currently equipped gear. Click slots to unequip.' },
};

// Spawn positions based on role
export const PLAYER_COLOR = '#ff8800';
// Admin: spawn on command center floor, outside hologram railing (R > 10)
export const SPAWN_ADMIN: [number, number, number] = [20, COMMAND_CENTER_FLOOR_Y + 0.6, 0];
// User: spawn south of command center, facing inward
export const SPAWN_USER: [number, number, number] = [0, 1, 60];

// Static zone definitions that are always present in the world
export const STATIC_ZONES: VirtualZone[] = [
  {
    space_name: 'Fine Art Society',
    host_username: 'pcg',
    color: '#ffd700',
    staticAngle: 0,
  },
  {
    space_name: 'Veritwin',
    host_username: 'Andre',
    color: '#00aaff',
    staticAngle: 1,
  },
  {
    space_name: 'Jungleverse',
    host_username: 'pcg',
    color: '#00cc44',
    staticAngle: 2,
  },
  {
    space_name: 'Media Monsters HQ',
    host_username: 'Travers',
    color: '#ff5500',
    staticAngle: 3,
  },
  {
    space_name: 'Sirak Studios',
    host_username: 'Sirak',
    color: '#aa00ff',
    staticAngle: 4,
  },
];

// Static demo project for Fine Art Society (always available)
export const FINE_ART_SOCIETY_PROJECT: Project = {
  id: 'fine-art-society-demo',
  name: 'Fine Art Society',
  git_repo_path: '/demo/fine-art-society',
  setup_script: null,
  dev_script: null,
  cleanup_script: null,
  copy_files: null,
  vibe_budget_limit: null,
  vibe_spent_amount: 0,
  organization_id: null,
  client_id: null,
  folder_id: null,
  parent_project_id: null,
  sort_order: 0,
  aptos_address: null,
  aptos_funded: false,
  deleted_at: null,
  created_at: new Date(),
  updated_at: new Date(),
};

export const PUBLIC_PROJECTS = new Set(['Fine Art Society']);

// ─── Helper Functions ─────────────────────────────────────────────────────────

export function stringEnergy(input: string) {
  let hash = 0;
  for (let i = 0; i < input.length; i += 1) {
    hash = (hash + input.charCodeAt(i) * (i + 11)) % 1000;
  }
  return 0.35 + (hash / 1000) * 0.65;
}

// Evenly distribute N zones around a ring, returning [x, z] for each index
export function zonePosition(index: number, total: number, radius: number): [number, number] {
  const angle = (index / total) * Math.PI * 2 - Math.PI / 2; // start at top
  return [Math.cos(angle) * radius, Math.sin(angle) * radius];
}

export function generateProjectsFromAPI(apiProjects: Project[], accessibleIds: Set<string>): ProjectData[] {
  if (!apiProjects.length) return [];

  const radiusForSpacing = (apiProjects.length * TARGET_ARC_SPACING) / (Math.PI * 2);
  const minVisualRadius = PROJECT_FOOTPRINT_RADIUS * 2.8;
  const radius = Math.max(BASE_PROJECT_RADIUS, minVisualRadius, radiusForSpacing);
  const y = 0; // Buildings rest on ground

  return apiProjects.map((project, index) => {
    const angle = (index / apiProjects.length) * Math.PI * 2;
    const position: [number, number, number] = [
      Math.cos(angle) * radius,
      y,
      Math.sin(angle) * radius,
    ];
    return {
      id: project.id,
      name: project.name,
      position,
      energy: stringEnergy(project.name),
      project,
      // Public projects are always accessible; otherwise check membership
      accessible: PUBLIC_PROJECTS.has(project.name) || accessibleIds.has(project.id),
    };
  });
}
