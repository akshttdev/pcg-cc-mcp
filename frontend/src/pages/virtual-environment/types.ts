import type { LucideIcon } from 'lucide-react';
import type { Project } from 'shared/types';

export interface ProjectData {
  id: string;
  name: string;
  position: [number, number, number];
  energy: number;
  project: Project; // Full project data from API
  /** true = user is a member or building is universally public */
  accessible: boolean;
}

export type HudPanelId = 'systems' | 'intel' | 'map' | 'controls' | 'inventory' | 'equipment';

export interface HudNavItem {
  id: HudPanelId;
  label: string;
  description: string;
  icon: LucideIcon;
}

export interface VirtualZone {
  space_name: string;
  host_username: string;
  color: string;
  staticAngle: number; // index determining angular position in ring
  world_x?: number;
  spawn_x?: number;
  spawn_y?: number;
  spawn_z?: number;
}

export interface SystemsPanelProps {
  projects: ProjectData[];
  selectedProject: ProjectData | null;
  userPosition: [number, number, number];
  zones?: VirtualZone[];
}

export interface IntelPanelProps {
  projects: ProjectData[];
}

export type MapPanelProps = SystemsPanelProps;

export interface MiniMapProps {
  projects: ProjectData[];
  selectedProject: ProjectData | null;
  userPosition: [number, number, number];
  size?: number;
  zones?: VirtualZone[];
}
