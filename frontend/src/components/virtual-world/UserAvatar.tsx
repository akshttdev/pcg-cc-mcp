import { useRef, useState, useEffect } from 'react';
import { useFrame, useThree } from '@react-three/fiber';
import { Line } from '@react-three/drei';
import * as THREE from 'three';
import {
  BUILDING_HALF_WIDTH,
  BUILDING_HALF_LENGTH,
  COMMAND_CENTER_RADIUS,
  COMMAND_CENTER_FLOOR_Y,
  WORKSPACE_LEVEL_FLOOR_Y,
  WORKSPACE_OUTER_RADIUS,
  WORKSPACE_INNER_RADIUS,
  GROUND_LEVEL,
} from '@/lib/virtual-world/constants';
import { getRampYAtPosition } from '@/components/virtual-world/AgentWorkspaceLevel';

export interface BuildingCollider {
  position: [number, number, number];
  entranceDirection: THREE.Vector3;
}

interface UserAvatarProps {
  initialPosition?: [number, number, number];
  color?: string;
  onPositionChange?: (position: THREE.Vector3) => void;
  onInteract?: () => void;
  isSuspended?: boolean;
  canFly?: boolean;
  buildings?: BuildingCollider[];
  /** Override floor height calculation. When set, disables command center floor detection. */
  baseFloorHeight?: number;
}

type MovementKeys = {
  forward: boolean;
  backward: boolean;
  left: boolean;
  right: boolean;
  up: boolean;
  down: boolean;
  sprint: boolean;
};

interface AnimationDescriptor {
  mode: 'idle' | 'walk' | 'run' | 'jump' | 'fly';
  intensity: number;
  airborne: boolean;
  tiltX: number; // Forward/back lean
  tiltZ: number; // Left/right lean
}

// Tuned movement constants
const FLIGHT_DOUBLE_TAP_WINDOW_MS = 400;
const JUMP_STRENGTH = 0.125;
const WALK_SPEED = 3.0;
const RUN_MULTIPLIER = 1.8;
const ROTATION_LERP = 0.15;
const CAMERA_DISTANCE = 15;
const CAMERA_HEIGHT = 10;
const CAMERA_LERP = 0.08;

export function UserAvatar({
  initialPosition = [0, 0, 30],
  color = '#ff8800',
  onPositionChange,
  onInteract,
  isSuspended = false,
  canFly = false,
  buildings = [],
  baseFloorHeight,
}: UserAvatarProps) {
  const groupRef = useRef<THREE.Group>(null);
  const avatarRef = useRef<THREE.Group>(null);
  const { camera, gl } = useThree();

  const positionRef = useRef(new THREE.Vector3(...initialPosition));
  const velocityRef = useRef(new THREE.Vector3());
  const keysRef = useRef<MovementKeys>({
    forward: false,
    backward: false,
    left: false,
    right: false,
    up: false,
    down: false,
    sprint: false,
  });

  // Camera orbit angle (controlled by mouse)
  const cameraAngleRef = useRef(Math.PI / 4);
  const cameraAngleTargetRef = useRef(Math.PI / 4);
  const isDraggingRef = useRef(false);
  const lastMouseXRef = useRef(0);

  // Avatar rotation (faces movement direction)
  const avatarRotationRef = useRef(0);
  const targetRotationRef = useRef(0);
  const hasMovedRef = useRef(false);

  // Tilt for leaning into movement
  const tiltRef = useRef({ x: 0, z: 0 });

  const lastEmittedPosition = useRef<THREE.Vector3 | null>(null);
  const isGroundedRef = useRef(true);
  const flightModeRef = useRef(false);
  const lastSpaceTapRef = useRef(0);
  const animationStateRef = useRef<AnimationDescriptor>({
    mode: 'idle',
    intensity: 0,
    airborne: false,
    tiltX: 0,
    tiltZ: 0,
  });

  const trailRef = useRef<THREE.Vector3[]>([]);
  const [trailPoints, setTrailPoints] = useState<THREE.Vector3[]>([]);
  const maxTrailLength = 25;

  // Mouse controls for camera orbit
  useEffect(() => {
    const canvas = gl.domElement;

    const handleMouseDown = (e: MouseEvent) => {
      if (e.button === 0 || e.button === 2) {
        isDraggingRef.current = true;
        lastMouseXRef.current = e.clientX;
        canvas.style.cursor = 'grabbing';
      }
    };

    const handleMouseUp = () => {
      isDraggingRef.current = false;
      canvas.style.cursor = 'grab';
    };

    const handleMouseMove = (e: MouseEvent) => {
      if (isDraggingRef.current && !isSuspended) {
        const deltaX = e.clientX - lastMouseXRef.current;
        cameraAngleTargetRef.current -= deltaX * 0.004;
        lastMouseXRef.current = e.clientX;
      }
    };

    const handleContextMenu = (e: MouseEvent) => {
      e.preventDefault();
    };

    const handleWheel = (_event: WheelEvent) => {
      // Could add zoom here
    };

    canvas.style.cursor = 'grab';
    canvas.addEventListener('mousedown', handleMouseDown);
    canvas.addEventListener('mouseup', handleMouseUp);
    canvas.addEventListener('mousemove', handleMouseMove);
    canvas.addEventListener('mouseleave', handleMouseUp);
    canvas.addEventListener('contextmenu', handleContextMenu);
    canvas.addEventListener('wheel', handleWheel);

    return () => {
      canvas.removeEventListener('mousedown', handleMouseDown);
      canvas.removeEventListener('mouseup', handleMouseUp);
      canvas.removeEventListener('mousemove', handleMouseMove);
      canvas.removeEventListener('mouseleave', handleMouseUp);
      canvas.removeEventListener('contextmenu', handleContextMenu);
      canvas.removeEventListener('wheel', handleWheel);
    };
  }, [gl, isSuspended]);

  useEffect(() => {
    const keys = keysRef.current;
    const velocity = velocityRef.current;

    const handleKeyDown = (e: KeyboardEvent) => {
      const key = e.key.toLowerCase();
      if (key === 'e') {
        if (!isSuspended && onInteract) {
          onInteract();
        }
        return;
      }

      if (isSuspended) return;

      switch (key) {
        case 'w':
        case 'arrowup':
          keys.forward = true;
          break;
        case 's':
        case 'arrowdown':
          keys.backward = true;
          break;
        case 'a':
        case 'arrowleft':
          keys.left = true;
          break;
        case 'd':
        case 'arrowright':
          keys.right = true;
          break;
        case ' ':
          e.preventDefault();
          if (canFly) {
            const now = performance.now();
            if (now - lastSpaceTapRef.current < FLIGHT_DOUBLE_TAP_WINDOW_MS) {
              flightModeRef.current = true;
            }
            lastSpaceTapRef.current = now;
            keys.up = true;
            if (isGroundedRef.current && velocity.y <= 0.01) {
              velocity.y = JUMP_STRENGTH;
              isGroundedRef.current = false;
            }
          } else if (isGroundedRef.current) {
            velocity.y = JUMP_STRENGTH;
            isGroundedRef.current = false;
          }
          break;
        case 'control':
        case 'q':
          keys.down = true;
          break;
        case 'shift':
          keys.sprint = true;
          break;
      }
    };

    const handleKeyUp = (e: KeyboardEvent) => {
      if (isSuspended) return;
      switch (e.key.toLowerCase()) {
        case 'w':
        case 'arrowup':
          keys.forward = false;
          break;
        case 's':
        case 'arrowdown':
          keys.backward = false;
          break;
        case 'a':
        case 'arrowleft':
          keys.left = false;
          break;
        case 'd':
        case 'arrowright':
          keys.right = false;
          break;
        case ' ':
          keys.up = false;
          flightModeRef.current = false;
          break;
        case 'control':
        case 'q':
          keys.down = false;
          break;
        case 'shift':
          keys.sprint = false;
          break;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keyup', handleKeyUp);

    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('keyup', handleKeyUp);
    };
  }, [isSuspended, onInteract, canFly]);

  useEffect(() => {
    if (isSuspended) {
      const keys = keysRef.current;
      keys.forward = false;
      keys.backward = false;
      keys.left = false;
      keys.right = false;
      keys.up = false;
      keys.down = false;
      keys.sprint = false;
      velocityRef.current.set(0, 0, 0);
      flightModeRef.current = false;
    }
  }, [isSuspended]);

  const AVATAR_FOOT_OFFSET = 0.7;

  const getFloorHeight = (x: number, z: number, currentY: number): number => {
    // If baseFloorHeight is provided (e.g., for interiors), use it directly
    if (baseFloorHeight !== undefined) {
      return baseFloorHeight + AVATAR_FOOT_OFFSET;
    }

    const distFromCenter = Math.sqrt(x * x + z * z);

    // Check if on spiral ramp
    const rampY = getRampYAtPosition(x, z);
    if (rampY !== null) {
      // Only use ramp floor if player is close to ramp height
      if (Math.abs(currentY - rampY) < 5) {
        return rampY + AVATAR_FOOT_OFFSET;
      }
    }

    // Check if on Command Center platform (upper level)
    if (distFromCenter <= COMMAND_CENTER_RADIUS && currentY > WORKSPACE_LEVEL_FLOOR_Y + 5) {
      return COMMAND_CENTER_FLOOR_Y + AVATAR_FOOT_OFFSET;
    }

    // Check if on Workspace Level ring (lower level)
    // The ring goes from WORKSPACE_INNER_RADIUS to WORKSPACE_OUTER_RADIUS
    if (distFromCenter >= WORKSPACE_INNER_RADIUS - 2 && distFromCenter <= WORKSPACE_OUTER_RADIUS + 5) {
      // If player is at or below command center but above ground, use workspace floor
      if (currentY < COMMAND_CENTER_FLOOR_Y - 5 && currentY > GROUND_LEVEL + 10) {
        return WORKSPACE_LEVEL_FLOOR_Y + AVATAR_FOOT_OFFSET;
      }
    }

    // Check if in workspace atrium (inner circle of lower level)
    if (distFromCenter < WORKSPACE_INNER_RADIUS && currentY < COMMAND_CENTER_FLOOR_Y - 5 && currentY > GROUND_LEVEL + 10) {
      return WORKSPACE_LEVEL_FLOOR_Y + AVATAR_FOOT_OFFSET;
    }

    // Default: ground level
    return AVATAR_FOOT_OFFSET;
  };

  useFrame((_, delta) => {
    if (!groupRef.current || !avatarRef.current) return;

    const position = positionRef.current;
    const velocity = velocityRef.current;
    const keys = keysRef.current;

    // Clamp delta to prevent huge jumps
    const dt = Math.min(delta, 0.1);
    const dtScale = dt * 60; // Normalize to 60fps

    // Smooth camera angle
    cameraAngleRef.current += (cameraAngleTargetRef.current - cameraAngleRef.current) * 0.08 * dtScale;

    if (isSuspended) {
      groupRef.current.position.copy(position);
      camera.lookAt(position.clone().add(new THREE.Vector3(0, 4, 0)));
      return;
    }

    const currentFloorHeight = getFloorHeight(position.x, position.z, position.y);

    // Floor snap
    if (position.y < currentFloorHeight) {
      position.y = currentFloorHeight;
      velocity.y = 0;
      isGroundedRef.current = true;
    }

    const onGround = position.y <= currentFloorHeight + 0.15;
    if (onGround && velocity.y <= 0) {
      isGroundedRef.current = true;
      velocity.y = 0;
      position.y = currentFloorHeight;
    }

    // Calculate movement direction based on camera angle
    const cameraAngle = cameraAngleRef.current;
    const forward = new THREE.Vector3(-Math.sin(cameraAngle), 0, -Math.cos(cameraAngle));
    const right = new THREE.Vector3(-Math.cos(cameraAngle), 0, Math.sin(cameraAngle));

    // Acceleration based on grounded state
    const baseAccel = isGroundedRef.current ? 0.08 : 0.03;
    const accel = keys.sprint ? baseAccel * 1.4 : baseAccel;
    const friction = isGroundedRef.current ? 0.88 : 0.96;

    // Calculate input direction
    const inputDir = new THREE.Vector3();

    if (keys.forward) { inputDir.add(forward); }
    if (keys.backward) { inputDir.sub(forward); }
    if (keys.left) { inputDir.add(right); }
    if (keys.right) { inputDir.sub(right); }

    if (inputDir.lengthSq() > 0) {
      inputDir.normalize();
      velocity.x += inputDir.x * accel * dtScale;
      velocity.z += inputDir.z * accel * dtScale;
    }

    // Vertical movement
    if (canFly && keys.up) velocity.y += accel * dtScale;
    if (keys.down) velocity.y -= accel * 0.7 * dtScale;

    // Apply friction
    velocity.x *= Math.pow(friction, dtScale);
    velocity.z *= Math.pow(friction, dtScale);

    // Gravity
    const gravity = canFly ? 0.018 : 0.045;
    if (!isGroundedRef.current) {
      velocity.y -= gravity * dtScale;
    }

    // Flight boost
    if (canFly && flightModeRef.current && keys.up) {
      velocity.y = Math.min(velocity.y + 0.025 * dtScale, 0.4);
    }

    // Apply velocity with speed limit
    const maxSpeed = keys.sprint ? WALK_SPEED * RUN_MULTIPLIER : WALK_SPEED;
    const horizontalSpeed = Math.sqrt(velocity.x * velocity.x + velocity.z * velocity.z);

    if (horizontalSpeed > maxSpeed * 0.035) {
      const scale = Math.min(1, (maxSpeed * 0.035) / horizontalSpeed);
      velocity.x *= scale;
      velocity.z *= scale;
    }

    const movement = velocity.clone().multiplyScalar(maxSpeed * dtScale);
    const oldPosition = position.clone();
    position.add(movement);

    const newFloorHeight = getFloorHeight(position.x, position.z, position.y);

    // Building collisions
    for (const building of buildings) {
      const [bx, , bz] = building.position;
      const dx = position.x - bx;
      const dz = position.z - bz;
      const angle = Math.atan2(building.entranceDirection.x, building.entranceDirection.z);
      const cos = Math.cos(-angle);
      const sin = Math.sin(-angle);
      const localX = dx * cos - dz * sin;
      const localZ = dx * sin + dz * cos;

      if (Math.abs(localX) < BUILDING_HALF_WIDTH && Math.abs(localZ) < BUILDING_HALF_LENGTH) {
        const atDoor = localZ > BUILDING_HALF_LENGTH - 8 && Math.abs(localX) < 6;
        if (!atDoor) {
          position.x = oldPosition.x;
          position.z = oldPosition.z;
          velocity.x = 0;
          velocity.z = 0;
        }
      }
    }

    // Floor collision
    if (position.y < newFloorHeight) {
      position.y = newFloorHeight;
      velocity.y = 0;
      isGroundedRef.current = true;
    } else if (position.y > newFloorHeight + 0.5) {
      isGroundedRef.current = false;
    }

    // Safety floor
    if (position.y < AVATAR_FOOT_OFFSET) {
      position.y = AVATAR_FOOT_OFFSET;
      velocity.y = 0;
      isGroundedRef.current = true;
    }

    groupRef.current.position.copy(position);

    // No tilt - keep avatar upright
    tiltRef.current.x = 0;
    tiltRef.current.z = 0;

    // Rotate avatar to face movement direction
    const speed = Math.sqrt(velocity.x * velocity.x + velocity.z * velocity.z);

    if (speed > 0.002) {
      targetRotationRef.current = Math.atan2(velocity.x, velocity.z);
      hasMovedRef.current = true;
    }

    if (hasMovedRef.current) {
      // Smooth rotation with shortest path
      let rotationDiff = targetRotationRef.current - avatarRotationRef.current;
      while (rotationDiff > Math.PI) rotationDiff -= Math.PI * 2;
      while (rotationDiff < -Math.PI) rotationDiff += Math.PI * 2;
      avatarRotationRef.current += rotationDiff * ROTATION_LERP * dtScale;
    }

    avatarRef.current.rotation.y = avatarRotationRef.current;
    avatarRef.current.rotation.x = 0;
    avatarRef.current.rotation.z = 0;

    // Position change callback
    if (onPositionChange) {
      if (!lastEmittedPosition.current) {
        lastEmittedPosition.current = position.clone();
        onPositionChange(position.clone());
      } else if (lastEmittedPosition.current.distanceToSquared(position) > 0.25) {
        lastEmittedPosition.current.copy(position);
        onPositionChange(position.clone());
      }
    }

    // Motion trail
    if (speed > 0.005) {
      trailRef.current.unshift(position.clone());
      if (trailRef.current.length > maxTrailLength) {
        trailRef.current.pop();
      }
      setTrailPoints(trailRef.current.map((point, index) => {
        const alpha = 1 - index / maxTrailLength;
        return point.clone().setY(point.y + alpha * 0.1);
      }));
    }

    // Animation state
    const airborne = !isGroundedRef.current;
    let mode: AnimationDescriptor['mode'] = 'idle';
    if (airborne) {
      mode = canFly && flightModeRef.current ? 'fly' : 'jump';
    } else if (speed > 0.005) {
      mode = keys.sprint ? 'run' : 'walk';
    }

    animationStateRef.current = {
      mode,
      intensity: Math.min(speed * 50, 1.0),
      airborne,
      tiltX: tiltRef.current.x,
      tiltZ: tiltRef.current.z,
    };

    // Camera follow
    const cameraOffset = new THREE.Vector3(
      Math.sin(cameraAngleRef.current) * CAMERA_DISTANCE,
      CAMERA_HEIGHT,
      Math.cos(cameraAngleRef.current) * CAMERA_DISTANCE
    );
    const desiredCameraPosition = position.clone().add(cameraOffset);
    camera.position.lerp(desiredCameraPosition, CAMERA_LERP * dtScale);

    const lookTarget = position.clone().add(new THREE.Vector3(0, 2.5, 0));
    camera.lookAt(lookTarget);
  });

  return (
    <group ref={groupRef}>
      <group ref={avatarRef}>
        <HumanoidAvatar
          color={color}
          animationRef={animationStateRef}
          showJetpack={canFly && flightModeRef.current}
        />
      </group>
      {trailPoints.length >= 2 && (
        <Line
          points={trailPoints}
          color="#00ffff"
          lineWidth={2}
          transparent
          opacity={0.3}
        />
      )}
    </group>
  );
}

interface HumanoidAvatarProps {
  color: string;
  animationRef: React.MutableRefObject<AnimationDescriptor>;
  showJetpack?: boolean;
}

function HumanoidAvatar({ color, animationRef, showJetpack = false }: HumanoidAvatarProps) {
  const bodyRef = useRef<THREE.Group>(null);
  const headRef = useRef<THREE.Group>(null);
  const leftArmRef = useRef<THREE.Group>(null);
  const rightArmRef = useRef<THREE.Group>(null);
  const leftLegRef = useRef<THREE.Group>(null);
  const rightLegRef = useRef<THREE.Group>(null);

  useFrame((state) => {
    const { mode, intensity } = animationRef.current;
    const time = state.clock.elapsedTime;

    // Animation speeds based on mode
    const cycleSpeed = mode === 'run' ? 14 : mode === 'walk' ? 9 : 2;
    const cycle = Math.sin(time * cycleSpeed);
    const oppositeCycle = Math.sin(time * cycleSpeed + Math.PI);

    // Arm swing
    if (leftArmRef.current && rightArmRef.current) {
      const armSwing = mode === 'idle' ? 0.03 : (mode === 'run' ? 0.9 : 0.5) * intensity;
      leftArmRef.current.rotation.x = cycle * armSwing;
      rightArmRef.current.rotation.x = oppositeCycle * armSwing;
      // Slight outward rotation when running
      if (mode === 'run') {
        leftArmRef.current.rotation.z = -0.1;
        rightArmRef.current.rotation.z = 0.1;
      } else {
        leftArmRef.current.rotation.z = 0;
        rightArmRef.current.rotation.z = 0;
      }
    }

    // Leg swing
    if (leftLegRef.current && rightLegRef.current) {
      const legSwing = mode === 'idle' ? 0 : (mode === 'run' ? 0.7 : 0.4) * intensity;
      leftLegRef.current.rotation.x = oppositeCycle * legSwing;
      rightLegRef.current.rotation.x = cycle * legSwing;
    }

    // Head bob
    if (headRef.current) {
      const bobAmount = mode === 'run' ? 0.04 : mode === 'walk' ? 0.02 : 0.01;
      const bobSpeed = mode === 'run' ? 28 : mode === 'walk' ? 18 : 1.5;
      headRef.current.position.y = 2.4 + Math.abs(Math.sin(time * bobSpeed)) * bobAmount * intensity;

      // Subtle head turn
      if (mode === 'idle') {
        headRef.current.rotation.y = Math.sin(time * 0.3) * 0.1;
      } else {
        headRef.current.rotation.y = 0;
      }
    }

    // Body bounce when moving
    if (bodyRef.current) {
      if (mode === 'run' || mode === 'walk') {
        const bounce = Math.abs(Math.sin(time * cycleSpeed * 2)) * 0.03 * intensity;
        bodyRef.current.position.y = bounce;
      } else {
        bodyRef.current.position.y = 0;
      }
    }
  });

  const mainColor = '#f5f5f5'; // White
  const accentColor = '#ffd700'; // Gold
  const darkColor = '#1a1a1a'; // Dark accents

  return (
    <group ref={bodyRef}>
      {/* Head */}
      <group ref={headRef} position={[0, 2.4, 0]}>
        {/* Helmet/Head */}
        <mesh castShadow>
          <sphereGeometry args={[0.5, 32, 32]} />
          <meshStandardMaterial
            color={mainColor}
            emissive={accentColor}
            emissiveIntensity={0.05}
            metalness={0.1}
            roughness={0.8}
          />
        </mesh>

        {/* Visor */}
        <mesh position={[0, 0.05, 0.35]} rotation={[0.1, 0, 0]}>
          <boxGeometry args={[0.6, 0.25, 0.15]} />
          <meshPhysicalMaterial
            color={accentColor}
            emissive={accentColor}
            emissiveIntensity={0.5}
            metalness={0.9}
            roughness={0.1}
            transparent
            opacity={0.8}
          />
        </mesh>

        {/* Antenna */}
        <group position={[0.2, 0.45, -0.1]}>
          <mesh>
            <cylinderGeometry args={[0.02, 0.015, 0.25, 8]} />
            <meshStandardMaterial color={darkColor} metalness={0.8} />
          </mesh>
          <mesh position={[0, 0.15, 0]}>
            <sphereGeometry args={[0.04, 12, 12]} />
            <meshBasicMaterial color={accentColor} />
            <pointLight color={accentColor} intensity={0.3} distance={2} />
          </mesh>
        </group>
      </group>

      {/* Torso */}
      <group position={[0, 1.3, 0]}>
        {/* Main body */}
        <mesh castShadow>
          <capsuleGeometry args={[0.4, 0.9, 12, 24]} />
          <meshStandardMaterial
            color={mainColor}
            emissive={accentColor}
            emissiveIntensity={0.03}
            metalness={0.1}
            roughness={0.85}
          />
        </mesh>

        {/* Chest plate */}
        <mesh position={[0, 0.15, 0.3]}>
          <boxGeometry args={[0.5, 0.4, 0.15]} />
          <meshStandardMaterial
            color={darkColor}
            metalness={0.7}
            roughness={0.2}
          />
        </mesh>

        {/* Core light */}
        <mesh position={[0, 0.15, 0.38]}>
          <circleGeometry args={[0.08, 16]} />
          <meshBasicMaterial color={accentColor} />
          <pointLight color={accentColor} intensity={0.4} distance={3} />
        </mesh>

        {/* Belt */}
        <mesh position={[0, -0.35, 0]}>
          <torusGeometry args={[0.42, 0.06, 12, 24]} />
          <meshStandardMaterial
            color={darkColor}
            metalness={0.8}
            roughness={0.2}
          />
        </mesh>
      </group>

      {/* Left Arm */}
      <group ref={leftArmRef} position={[0.65, 1.5, 0]}>
        {/* Upper arm */}
        <mesh castShadow position={[0, -0.25, 0]}>
          <capsuleGeometry args={[0.12, 0.4, 8, 12]} />
          <meshStandardMaterial color={mainColor} emissive={accentColor} emissiveIntensity={0.02} metalness={0.1} roughness={0.85} />
        </mesh>
        {/* Forearm */}
        <mesh castShadow position={[0, -0.6, 0]}>
          <capsuleGeometry args={[0.1, 0.35, 8, 12]} />
          <meshStandardMaterial color={darkColor} metalness={0.6} roughness={0.3} />
        </mesh>
        {/* Hand */}
        <mesh position={[0, -0.85, 0]}>
          <sphereGeometry args={[0.1, 12, 12]} />
          <meshStandardMaterial color={darkColor} metalness={0.7} />
        </mesh>
      </group>

      {/* Right Arm */}
      <group ref={rightArmRef} position={[-0.65, 1.5, 0]}>
        <mesh castShadow position={[0, -0.25, 0]}>
          <capsuleGeometry args={[0.12, 0.4, 8, 12]} />
          <meshStandardMaterial color={mainColor} emissive={accentColor} emissiveIntensity={0.02} metalness={0.1} roughness={0.85} />
        </mesh>
        <mesh castShadow position={[0, -0.6, 0]}>
          <capsuleGeometry args={[0.1, 0.35, 8, 12]} />
          <meshStandardMaterial color={darkColor} metalness={0.6} roughness={0.3} />
        </mesh>
        <mesh position={[0, -0.85, 0]}>
          <sphereGeometry args={[0.1, 12, 12]} />
          <meshStandardMaterial color={darkColor} metalness={0.7} />
        </mesh>
      </group>

      {/* Left Leg */}
      <group ref={leftLegRef} position={[0.22, 0.4, 0]}>
        {/* Thigh */}
        <mesh castShadow position={[0, -0.25, 0]}>
          <capsuleGeometry args={[0.14, 0.4, 8, 12]} />
          <meshStandardMaterial color={mainColor} emissive={accentColor} emissiveIntensity={0.02} metalness={0.1} roughness={0.85} />
        </mesh>
        {/* Shin */}
        <mesh castShadow position={[0, -0.65, 0]}>
          <capsuleGeometry args={[0.11, 0.4, 8, 12]} />
          <meshStandardMaterial color={darkColor} metalness={0.5} roughness={0.3} />
        </mesh>
        {/* Boot */}
        <mesh position={[0, -0.95, 0.05]}>
          <boxGeometry args={[0.18, 0.12, 0.28]} />
          <meshStandardMaterial color={darkColor} metalness={0.7} roughness={0.2} />
        </mesh>
        {/* Boot accent */}
        <mesh position={[0, -0.92, 0.15]}>
          <boxGeometry args={[0.19, 0.04, 0.04]} />
          <meshBasicMaterial color={accentColor} />
        </mesh>
      </group>

      {/* Right Leg */}
      <group ref={rightLegRef} position={[-0.22, 0.4, 0]}>
        <mesh castShadow position={[0, -0.25, 0]}>
          <capsuleGeometry args={[0.14, 0.4, 8, 12]} />
          <meshStandardMaterial color={mainColor} emissive={mainColor} emissiveIntensity={0.15} />
        </mesh>
        <mesh castShadow position={[0, -0.65, 0]}>
          <capsuleGeometry args={[0.11, 0.4, 8, 12]} />
          <meshStandardMaterial color={darkColor} metalness={0.5} roughness={0.3} />
        </mesh>
        <mesh position={[0, -0.95, 0.05]}>
          <boxGeometry args={[0.18, 0.12, 0.28]} />
          <meshStandardMaterial color={darkColor} metalness={0.7} roughness={0.2} />
        </mesh>
        <mesh position={[0, -0.92, 0.15]}>
          <boxGeometry args={[0.19, 0.04, 0.04]} />
          <meshBasicMaterial color={accentColor} />
        </mesh>
      </group>

      {/* Jetpack */}
      {showJetpack && (
        <group position={[0, 1.3, -0.45]}>
          {/* Main pack */}
          <mesh>
            <boxGeometry args={[0.5, 0.7, 0.25]} />
            <meshStandardMaterial color={darkColor} metalness={0.8} roughness={0.2} />
          </mesh>

          {/* Thrusters */}
          <mesh position={[-0.15, -0.35, 0]}>
            <cylinderGeometry args={[0.1, 0.12, 0.2, 12]} />
            <meshStandardMaterial color={darkColor} metalness={0.9} />
          </mesh>
          <mesh position={[0.15, -0.35, 0]}>
            <cylinderGeometry args={[0.1, 0.12, 0.2, 12]} />
            <meshStandardMaterial color={darkColor} metalness={0.9} />
          </mesh>

          {/* Thruster flames */}
          <mesh position={[-0.15, -0.5, 0]}>
            <coneGeometry args={[0.08, 0.25, 8]} />
            <meshBasicMaterial color={accentColor} transparent opacity={0.8} />
            <pointLight color={accentColor} intensity={1.5} distance={4} />
          </mesh>
          <mesh position={[0.15, -0.5, 0]}>
            <coneGeometry args={[0.08, 0.25, 8]} />
            <meshBasicMaterial color={accentColor} transparent opacity={0.8} />
            <pointLight color={accentColor} intensity={1.5} distance={4} />
          </mesh>
        </group>
      )}
    </group>
  );
}
