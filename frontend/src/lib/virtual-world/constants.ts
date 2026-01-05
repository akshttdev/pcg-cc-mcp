// Re-export from spatial system for backward compatibility
export {
  GROUND_Y as GROUND_LEVEL,
  COMMAND_CENTER_Y as COMMAND_CENTER_FLOOR_Y,
  WORKSPACE_FLOOR_Y as WORKSPACE_LEVEL_FLOOR_Y,
  WORKSPACE_CEILING_Y,
  WORKSPACE_OUTER_RADIUS,
  WORKSPACE_INNER_RADIUS,
  COMMAND_CENTER_RADIUS,
} from './spatialSystem';

// Entry trigger distance for entering buildings
export const ENTRY_TRIGGER_DISTANCE = 26;
export const ENTRY_PROMPT_HEIGHT = 6;

// Interior camera settings
export const INTERIOR_CAMERA = {
  position: [0, 14, 30] as [number, number, number],
  fov: 55,
};

// Interior room dimensions
export const INTERIOR_ROOM = {
  width: 36,
  height: 14,
  depth: 36,
};

// Interior agent positions
export const INTERIOR_AGENT_POSITIONS: [number, number, number][] = [
  [-10, 2, -6],
  [10, 2, -4],
  [0, 2.5, 8],
];

// Building dimensions for collision (external project buildings)
export const BUILDING_HALF_WIDTH = 25;
export const BUILDING_HALF_LENGTH = 50;
export const BUILDING_HEIGHT = 20;

// Door dimensions
export const DOOR_WIDTH = 8;
export const DOOR_HEIGHT = 12;
