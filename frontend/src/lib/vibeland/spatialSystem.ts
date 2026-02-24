/**
 * Spatial Navigation System for PCG Command Center
 *
 * This system handles all collision detection and floor height calculations
 * for the multi-level command center structure.
 *
 * Architecture:
 * ============
 *
 * Y=80  ┌─────────────────────┐  COMMAND CENTER (Nora's Platform)
 *       │        NORA         │  R = 0 to 35
 *       │    ┌─────────┐      │
 *       └────┤STAIRWELL├──────┘  Stairwell opening at specific angle
 *            └────┬────┘
 *                 │ SPIRAL STAIRCASE
 *                 │ (270° rotation, R: 32→23)
 * Y=77  ══════════╧═══════════════════════  CEILING (ring with stairwell hole)
 *       │ BAY │ BAY │ ATRIUM │ BAY │ BAY │  WORKSPACE LEVEL
 *       │     │     │        │     │     │  R = 22 to 45
 * Y=65  ══════════════════════════════════  FLOOR (ring with stairwell hole)
 *       ║                                ║
 *       ╚════════════════════════════════╝  OUTER WALL (R=45)
 *                    │
 *                 ATRIUM
 *              (R = 0 to 22)
 * Y=0   ══════════════════════════════════  GROUND
 */

import * as THREE from 'three';

// ============================================================================
// CONSTANTS
// ============================================================================

// Vertical levels
export const GROUND_Y = 0;
export const WORKSPACE_FLOOR_Y = 65;
export const WORKSPACE_CEILING_Y = 77;
export const COMMAND_CENTER_Y = 80;

// Radii
export const COMMAND_CENTER_RADIUS = 35;
export const WORKSPACE_OUTER_RADIUS = 45;
export const WORKSPACE_INNER_RADIUS = 22;

// Hologram and stairwell configuration
// Hologram is in the CENTER, stairs wrap around the OUTSIDE
export const HOLOGRAM_RADIUS = 6;  // Central hologram column - floor hole is here (R = 0 to 6)
export const HOLOGRAM_RAILING_RADIUS = 10;  // Railing around hologram - SAME as stair inner radius for seamless connection
export const STAIRWELL_START_ANGLE = Math.PI / 2;  // Exactly South (6:00 on clock), stairs descend towards West
export const STAIRWELL_END_ANGLE = STAIRWELL_START_ANGLE + (3 * Math.PI / 2);  // +270° rotation
export const STAIRWELL_INNER_RADIUS = 10;  // Inner edge of stairs (has railing) - connects to hologram railing
export const STAIRWELL_OUTER_RADIUS = 14;  // Outer edge of stairs (has railing)
export const STAIRWELL_WIDTH = 4;  // Tangential width of steps

// Avatar dimensions
export const AVATAR_RADIUS = 0.5;
export const AVATAR_HEIGHT = 2.5;

// Wall thickness for collision (reduced for more walkable space)
export const WALL_THICKNESS = 0.3;

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/**
 * Normalize angle to [0, 2π) range
 */
function normalizeAngle(angle: number): number {
  let normalized = angle % (Math.PI * 2);
  if (normalized < 0) normalized += Math.PI * 2;
  return normalized;
}

/**
 * Check if angle is within a range (handling wraparound)
 */
function isAngleInRange(angle: number, start: number, end: number): boolean {
  const normAngle = normalizeAngle(angle);
  const normStart = normalizeAngle(start);
  const normEnd = normalizeAngle(end);

  if (normStart <= normEnd) {
    return normAngle >= normStart && normAngle <= normEnd;
  } else {
    // Range wraps around 0
    return normAngle >= normStart || normAngle <= normEnd;
  }
}

/**
 * Get polar coordinates from cartesian
 */
function toPolar(x: number, z: number): { radius: number; angle: number } {
  return {
    radius: Math.sqrt(x * x + z * z),
    angle: Math.atan2(z, x),
  };
}

/**
 * Get cartesian coordinates from polar
 */
function toCartesian(radius: number, angle: number): { x: number; z: number } {
  return {
    x: Math.cos(angle) * radius,
    z: Math.sin(angle) * radius,
  };
}

// ============================================================================
// ZONE DETECTION
// ============================================================================

export type Zone =
  | 'command_center'
  | 'stairwell'
  | 'workspace_ring'
  | 'workspace_atrium'
  | 'ground'
  | 'void';

/**
 * Determine which zone a position is in based on X, Z coordinates
 */
export function getZoneAtPosition(x: number, z: number, y: number): Zone {
  const { radius, angle } = toPolar(x, z);

  // Command Center / Rooftop level - extends to full floor radius
  // This is the roof, must be solid everywhere except stair hole
  // Note: hologram hole (R < 9.5) is blocked by railing collision, not zone detection
  // Use larger margin (Y >= 75) to prevent falling through when Y fluctuates
  if (y >= COMMAND_CENTER_Y - 5 && radius <= WORKSPACE_OUTER_RADIUS + 1) {
    // Check if in stair hole area (arc from 6:00/South to 9:00/West)
    // Hole is from R=10 to R=14, from angle π/2 to π
    const normalAngle = normalizeAngle(angle);
    const holeStartAngle = STAIRWELL_START_ANGLE - 0.1;  // Just before 6:00 (π/2)
    const holeEndAngle = Math.PI + 0.1;                   // Just past 9:00 (π)
    const inStairHoleAngle = normalAngle >= holeStartAngle && normalAngle <= holeEndAngle;
    const inStairHoleRadius = radius >= HOLOGRAM_RAILING_RADIUS && radius <= STAIRWELL_OUTER_RADIUS;

    // If in the stair hole, treat as stairwell (allows falling through)
    if (inStairHoleAngle && inStairHoleRadius) {
      return 'stairwell';
    }

    return 'command_center';
  }

  // Stairwell zone - stairs wrap around the hologram (R=10 to R=14)
  // Stairs go from Y=65 (bottom) to Y=80 (top)
  const inStairwellAngle = isAngleInRange(angle, STAIRWELL_START_ANGLE, STAIRWELL_END_ANGLE);
  const inStairwellRadius = radius >= STAIRWELL_INNER_RADIUS - 1 && radius <= STAIRWELL_OUTER_RADIUS + 1;

  // Check if Y is above floor level (on the stairs themselves)
  if (inStairwellAngle && inStairwellRadius && y >= WORKSPACE_FLOOR_Y && y < COMMAND_CENTER_Y + 2) {
    // Calculate expected stair height at this angle
    const normalizedAngle = normalizeAngle(angle);
    const startNorm = normalizeAngle(STAIRWELL_START_ANGLE);
    const rotationAmount = STAIRWELL_END_ANGLE - STAIRWELL_START_ANGLE;

    let angleProgress: number;
    if (normalizedAngle >= startNorm) {
      angleProgress = (normalizedAngle - startNorm) / rotationAmount;
    } else {
      angleProgress = (normalizedAngle + (2 * Math.PI) - startNorm) / rotationAmount;
    }
    angleProgress = Math.max(0, Math.min(1, angleProgress));

    const expectedStairY = COMMAND_CENTER_Y - angleProgress * (COMMAND_CENTER_Y - WORKSPACE_FLOOR_Y);

    // If player is near the expected stair height, they're on the stairs
    if (y >= expectedStairY - 3 && y <= expectedStairY + 3) {
      return 'stairwell';
    }
  }

  // Workspace level - solid floor from hologram railing (R=10) to outer wall (R=45)
  if (y >= WORKSPACE_FLOOR_Y - 1 && y <= COMMAND_CENTER_Y - 2) {
    if (radius >= HOLOGRAM_RAILING_RADIUS && radius <= WORKSPACE_OUTER_RADIUS + 1) {
      return 'workspace_ring';  // Solid floor area (outside the railing)
    }
  }

  // Ground level
  if (y <= 5) {
    return 'ground';
  }

  return 'void';
}

// ============================================================================
// FLOOR HEIGHT CALCULATION
// ============================================================================

/**
 * Get the floor height at a given X, Z position
 * This determines what Y level the avatar should stand on when grounded.
 *
 * Key insight: The command center floor (Y=80) IS the workspace ceiling.
 * - If you're above Y=77 (workspace ceiling), you're on the roof -> floor = 80
 * - If you're below Y=77, you're in workspace -> floor = 65
 * - Stairs connect the two levels
 */
export function getFloorHeightAt(x: number, z: number, currentY: number): number {
  const { radius, angle } = toPolar(x, z);
  const normalizedAngle = normalizeAngle(angle);

  // Threshold between workspace and command center (workspace ceiling is at ~77)
  const LEVEL_THRESHOLD = WORKSPACE_CEILING_Y;  // 77

  // Check if in stair hole area (6:00 to 9:00, R=10-14) - opening in the roof
  const holeStartAngle = STAIRWELL_START_ANGLE - 0.1;
  const holeEndAngle = Math.PI + 0.1;
  const inStairHoleAngle = normalizedAngle >= holeStartAngle && normalizedAngle <= holeEndAngle;
  const inStairHoleRadius = radius >= HOLOGRAM_RAILING_RADIUS && radius <= STAIRWELL_OUTER_RADIUS;
  const inStairHole = inStairHoleAngle && inStairHoleRadius;

  // Check if on stairwell (full spiral from 6:00 around to ~3:00)
  const inStairwellAngle = isAngleInRange(angle, STAIRWELL_START_ANGLE, STAIRWELL_END_ANGLE);
  const inStairwellRadius = radius >= STAIRWELL_INNER_RADIUS - 1 && radius <= STAIRWELL_OUTER_RADIUS + 1;
  const onStairs = inStairwellAngle && inStairwellRadius;

  // ========== STAIRWELL ==========
  if (inStairHole || onStairs) {
    const startNorm = normalizeAngle(STAIRWELL_START_ANGLE);
    const rotationAmount = STAIRWELL_END_ANGLE - STAIRWELL_START_ANGLE;

    // Calculate progress along staircase (0 = top at 6:00, 1 = bottom at ~3:00)
    let angleProgress: number;
    if (normalizedAngle >= startNorm) {
      angleProgress = (normalizedAngle - startNorm) / rotationAmount;
    } else {
      angleProgress = (normalizedAngle + (2 * Math.PI) - startNorm) / rotationAmount;
    }
    angleProgress = Math.max(0, Math.min(1, angleProgress));

    // Snap to discrete step heights (32 steps) for smoother walking
    const STAIR_COUNT = 32;
    const stepIndex = Math.floor(angleProgress * STAIR_COUNT);
    const snappedProgress = stepIndex / STAIR_COUNT;
    const stairY = COMMAND_CENTER_Y - snappedProgress * (COMMAND_CENTER_Y - WORKSPACE_FLOOR_Y);

    // In stair hole: always use stair height
    if (inStairHole) {
      return stairY;
    }

    // CRITICAL FIX: Distinguish between "on stairs" vs "under stairs on workspace floor"
    //
    // Scenario 1: Player is ON the stairs, climbing/descending
    //   - currentY should be close to stairY (within ~3 units)
    //   - Use stairY as the floor
    //
    // Scenario 2: Player is on workspace floor (Y≈65), walking UNDER the higher stairs
    //   - currentY is near WORKSPACE_FLOOR_Y (65)
    //   - stairY at this angle might be 70, 75, etc. (stairs above them)
    //   - Should stay on WORKSPACE_FLOOR_Y, not teleport up to stairY
    //
    // The key distinction: if the player is near workspace floor level AND
    // the stair at this position is significantly higher, they're UNDER the stairs.

    const atWorkspaceFloor = currentY <= WORKSPACE_FLOOR_Y + 3;  // Within 3 units of workspace floor
    const stairsSignificantlyAbove = stairY > currentY + 4;  // Stairs are >4 units above player

    // If at workspace floor and stairs are above, player is UNDER the stairs
    if (atWorkspaceFloor && stairsSignificantlyAbove) {
      return WORKSPACE_FLOOR_Y;
    }

    // If player is near the stair height, they're ON the stairs
    const nearStairHeight = Math.abs(currentY - stairY) < 4;
    if (nearStairHeight && currentY < COMMAND_CENTER_Y - 1) {
      return stairY;
    }

    // If clearly below the stairs (falling through stairwell area), use workspace floor
    if (currentY <= WORKSPACE_FLOOR_Y + 2) {
      return WORKSPACE_FLOOR_Y;
    }

    return COMMAND_CENTER_Y;
  }

  // ========== DETERMINE LEVEL BASED ON HEIGHT ==========
  const withinFloorRadius = radius >= HOLOGRAM_RAILING_RADIUS - 0.5 && radius <= WORKSPACE_OUTER_RADIUS;

  if (withinFloorRadius) {
    // Above workspace ceiling = on the roof
    if (currentY > LEVEL_THRESHOLD) {
      return COMMAND_CENTER_Y;
    }
    // Below workspace ceiling = in workspace
    return WORKSPACE_FLOOR_Y;
  }

  // ========== GROUND ==========
  return GROUND_Y;
}

// ============================================================================
// CEILING COLLISION
// ============================================================================

/**
 * Get the ceiling height at a given position
 * Returns null if there's no ceiling (open sky or in a hole)
 */
export function getCeilingHeightAt(x: number, z: number, currentY: number): number | null {
  const { radius, angle } = toPolar(x, z);

  // Inside the hologram railing - no ceiling (hologram shaft goes through)
  if (radius < HOLOGRAM_RAILING_RADIUS) {
    return null;
  }

  // Check if in stair hole area (top of stairs opening)
  const stairHoleStart = STAIRWELL_START_ANGLE - 0.3;
  const stairHoleEnd = STAIRWELL_START_ANGLE + 0.5;
  if (isAngleInRange(angle, stairHoleStart, stairHoleEnd) && radius <= STAIRWELL_OUTER_RADIUS + 2) {
    return null; // No ceiling in stair access area
  }

  // Check if walking on stairs (in stairwell angle range)
  const inStairwellAngle = isAngleInRange(angle, STAIRWELL_START_ANGLE, STAIRWELL_END_ANGLE);
  if (inStairwellAngle && radius >= STAIRWELL_INNER_RADIUS - 1 && radius <= STAIRWELL_OUTER_RADIUS + 1) {
    // On the stairs - no ceiling until command center
    if (currentY < COMMAND_CENTER_Y - 2) {
      return null;
    }
  }

  // Workspace ring ceiling - only applies when INSIDE workspace (below ceiling level)
  // Don't apply ceiling when player is ON the roof (Y >= 78)
  if (radius > STAIRWELL_OUTER_RADIUS + 2 && radius <= WORKSPACE_OUTER_RADIUS) {
    if (currentY >= WORKSPACE_FLOOR_Y - 1 && currentY < WORKSPACE_CEILING_Y) {
      return WORKSPACE_CEILING_Y;
    }
  }

  return null; // No ceiling (on roof or open sky)
}

// ============================================================================
// BOUNDARY COLLISION
// ============================================================================

export interface CollisionResult {
  blocked: boolean;
  correctedPosition: THREE.Vector3;
  hitNormal?: THREE.Vector3;
}

/**
 * Check if a movement from oldPos to newPos is blocked by walls/boundaries
 * Returns corrected position if blocked
 */
export function checkBoundaryCollision(
  _oldPos: THREE.Vector3,
  newPos: THREE.Vector3
): CollisionResult {
  const { radius: newRadius, angle: newAngle } = toPolar(newPos.x, newPos.z);
  const zone = getZoneAtPosition(newPos.x, newPos.z, newPos.y);

  let correctedPos = newPos.clone();
  let blocked = false;
  let hitNormal: THREE.Vector3 | undefined;

  // ========== COMMAND CENTER BOUNDARY ==========
  // Allow jumping/falling off the edge of the command center (no invisible wall)
  // The stairwell entrance still has railings (handled below)
  // Players can freely walk/jump off the roof and fall to workspace level or ground

  // ========== WORKSPACE OUTER WALL ==========
  if (zone === 'workspace_ring' && newPos.y >= WORKSPACE_FLOOR_Y - 1 && newPos.y <= WORKSPACE_CEILING_Y + 2) {
    if (newRadius > WORKSPACE_OUTER_RADIUS - 1) {
      // Hit outer wall - push back (1 unit buffer from edge)
      const safeRadius = WORKSPACE_OUTER_RADIUS - 1.2;
      const { x, z } = toCartesian(safeRadius, newAngle);
      correctedPos.x = x;
      correctedPos.z = z;
      blocked = true;
      hitNormal = new THREE.Vector3(-Math.cos(newAngle), 0, -Math.sin(newAngle));
    }
  }

  // ========== WORKSPACE INNER RAILING ==========
  // The workspace ring has a railing around the inner edge
  // But there's no need to block - the atrium is open and accessible via stairs
  // The stair exit at the bottom connects atrium floor to workspace ring

  // ========== STAIRWELL RAILINGS (both sides) ==========
  if (zone === 'stairwell') {
    // Inner railing
    if (newRadius < STAIRWELL_INNER_RADIUS) {
      const { x, z } = toCartesian(STAIRWELL_INNER_RADIUS + AVATAR_RADIUS, newAngle);
      correctedPos.x = x;
      correctedPos.z = z;
      blocked = true;
      hitNormal = new THREE.Vector3(Math.cos(newAngle), 0, Math.sin(newAngle));
    }
    // Outer railing
    if (newRadius > STAIRWELL_OUTER_RADIUS) {
      const { x, z } = toCartesian(STAIRWELL_OUTER_RADIUS - AVATAR_RADIUS, newAngle);
      correctedPos.x = x;
      correctedPos.z = z;
      blocked = true;
      hitNormal = new THREE.Vector3(-Math.cos(newAngle), 0, -Math.sin(newAngle));
    }
  }

  // ========== HOLOGRAM RAILING (continuous 360° around hologram hole) ==========
  // Prevents walking/jumping into the hologram area at BOTH workspace floor AND command center level
  // The railing is at R=10, full circle around the hologram hole
  const nearWorkspaceFloor = newPos.y >= WORKSPACE_FLOOR_Y - 1 && newPos.y <= WORKSPACE_FLOOR_Y + 6;
  const nearCommandCenter = newPos.y >= COMMAND_CENTER_Y - 2 && newPos.y <= COMMAND_CENTER_Y + 3;

  if ((nearWorkspaceFloor || nearCommandCenter) && newRadius < HOLOGRAM_RAILING_RADIUS) {
    const { x, z } = toCartesian(HOLOGRAM_RAILING_RADIUS + AVATAR_RADIUS, newAngle);
    correctedPos.x = x;
    correctedPos.z = z;
    blocked = true;
    hitNormal = new THREE.Vector3(Math.cos(newAngle), 0, Math.sin(newAngle));
  }

  // ========== HOLOGRAM COLLISION (central column - all heights) ==========
  if (newRadius < HOLOGRAM_RADIUS + AVATAR_RADIUS) {
    const { x, z } = toCartesian(HOLOGRAM_RADIUS + AVATAR_RADIUS + 0.2, newAngle);
    correctedPos.x = x;
    correctedPos.z = z;
    blocked = true;
    hitNormal = new THREE.Vector3(Math.cos(newAngle), 0, Math.sin(newAngle));
  }

  // ========== CEILING COLLISION ==========
  const ceiling = getCeilingHeightAt(correctedPos.x, correctedPos.z, newPos.y);
  if (ceiling !== null && correctedPos.y + AVATAR_HEIGHT > ceiling) {
    correctedPos.y = ceiling - AVATAR_HEIGHT;
    blocked = true;
  }

  // ========== FLOOR COLLISION ==========
  const floor = getFloorHeightAt(correctedPos.x, correctedPos.z, newPos.y);
  if (correctedPos.y < floor + AVATAR_RADIUS) {
    correctedPos.y = floor + AVATAR_RADIUS;
  }

  return {
    blocked,
    correctedPosition: correctedPos,
    hitNormal,
  };
}

// ============================================================================
// BAY DIVIDER COLLISION
// ============================================================================

const BAY_COUNT = 8;
const BAY_ANGLE = (Math.PI * 2) / BAY_COUNT;
const DIVIDER_THICKNESS = 0.25;  // Thin dividers for easy passage

/**
 * Check collision with bay divider walls
 */
export function checkBayDividerCollision(
  _oldPos: THREE.Vector3,
  newPos: THREE.Vector3
): CollisionResult {
  const { radius: newRadius, angle: newAngle } = toPolar(newPos.x, newPos.z);

  // Only check in workspace ring
  if (newRadius < WORKSPACE_INNER_RADIUS || newRadius > WORKSPACE_OUTER_RADIUS) {
    return { blocked: false, correctedPosition: newPos.clone() };
  }

  if (newPos.y < WORKSPACE_FLOOR_Y - 1 || newPos.y > WORKSPACE_CEILING_Y + 2) {
    return { blocked: false, correctedPosition: newPos.clone() };
  }

  const normalizedAngle = normalizeAngle(newAngle);

  // Check each divider
  for (let i = 0; i < BAY_COUNT; i++) {
    const dividerAngle = normalizeAngle(i * BAY_ANGLE);

    // Calculate angular distance to divider
    let angleDiff = Math.abs(normalizedAngle - dividerAngle);
    if (angleDiff > Math.PI) angleDiff = Math.PI * 2 - angleDiff;

    // Convert angular distance to linear distance at this radius
    const linearDist = angleDiff * newRadius;

    if (linearDist < DIVIDER_THICKNESS / 2 + AVATAR_RADIUS) {
      // Hit a divider - slide along it
      // Determine which side to push to
      const pushAngle = normalizedAngle > dividerAngle ?
        dividerAngle + (DIVIDER_THICKNESS / 2 + AVATAR_RADIUS + 0.1) / newRadius :
        dividerAngle - (DIVIDER_THICKNESS / 2 + AVATAR_RADIUS + 0.1) / newRadius;

      const { x, z } = toCartesian(newRadius, pushAngle);
      return {
        blocked: true,
        correctedPosition: new THREE.Vector3(x, newPos.y, z),
        hitNormal: new THREE.Vector3(
          Math.cos(dividerAngle + Math.PI / 2),
          0,
          Math.sin(dividerAngle + Math.PI / 2)
        ),
      };
    }
  }

  return { blocked: false, correctedPosition: newPos.clone() };
}

// ============================================================================
// MAIN COLLISION CHECK
// ============================================================================

/**
 * Main collision check that combines all collision types
 */
export function performCollisionCheck(
  oldPos: THREE.Vector3,
  newPos: THREE.Vector3
): CollisionResult {
  // First check boundary collisions
  let result = checkBoundaryCollision(oldPos, newPos);

  // Then check bay dividers
  if (!result.blocked) {
    const dividerResult = checkBayDividerCollision(oldPos, result.correctedPosition);
    if (dividerResult.blocked) {
      result = dividerResult;
    }
  }

  return result;
}

// ============================================================================
// STAIRWELL GEOMETRY HELPERS
// ============================================================================

/**
 * Get the stairwell opening shape for cutting holes in floor/ceiling
 * Returns an array of points defining the arc
 */
export function getStairwellOpeningPoints(
  radius: number,
  segments: number = 32
): THREE.Vector2[] {
  const points: THREE.Vector2[] = [];
  const arcLength = STAIRWELL_END_ANGLE - STAIRWELL_START_ANGLE;

  // Inner arc
  for (let i = 0; i <= segments; i++) {
    const t = i / segments;
    const angle = STAIRWELL_START_ANGLE + t * arcLength;
    const r = radius - STAIRWELL_WIDTH / 2;
    points.push(new THREE.Vector2(
      Math.cos(angle) * r,
      Math.sin(angle) * r
    ));
  }

  // Outer arc (reverse direction)
  for (let i = segments; i >= 0; i--) {
    const t = i / segments;
    const angle = STAIRWELL_START_ANGLE + t * arcLength;
    const r = radius + STAIRWELL_WIDTH / 2;
    points.push(new THREE.Vector2(
      Math.cos(angle) * r,
      Math.sin(angle) * r
    ));
  }

  return points;
}

/**
 * Check if a point is inside the stairwell opening
 */
export function isInStairwellOpening(x: number, z: number, openingRadius: number): boolean {
  const { radius, angle } = toPolar(x, z);

  if (!isAngleInRange(angle, STAIRWELL_START_ANGLE, STAIRWELL_END_ANGLE)) {
    return false;
  }

  const innerR = openingRadius - STAIRWELL_WIDTH / 2;
  const outerR = openingRadius + STAIRWELL_WIDTH / 2;

  return radius >= innerR && radius <= outerR;
}
