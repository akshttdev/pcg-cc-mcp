import { useRef, useState, useEffect } from 'react';
import { useFrame, useThree } from '@react-three/fiber';
import { Line } from '@react-three/drei';
import * as THREE from 'three';
import {
  GROUND_Y,
  getFloorHeightAt,
  getCeilingHeightAt,
  performCollisionCheck,
  AVATAR_HEIGHT,
  AVATAR_RADIUS,
} from '@/lib/vibeland/spatialSystem';
import { HumanoidAvatar } from './HumanoidAvatar';
import type { AnimationDescriptor } from './HumanoidAvatar';
import { useAvatarInput } from './useAvatarInput';

export type { AnimationDescriptor } from './HumanoidAvatar';

export interface BuildingCollider {
  position: [number, number, number];
  entranceDirection: THREE.Vector3;
}

interface UserAvatarProps {
  initialPosition?: [number, number, number];
  color?: string;
  isAdmin?: boolean;
  onPositionChange?: (position: THREE.Vector3) => void;
  onInteract?: () => void;
  isSuspended?: boolean;
  canFly?: boolean;
  buildings?: BuildingCollider[];
  baseFloorHeight?: number;
}

// Movement constants
const WALK_SPEED = 5.0;
const RUN_MULTIPLIER = 1.8;
const ROTATION_LERP = 0.15;
const CAMERA_DISTANCE = 15;
const CAMERA_HEIGHT = 10;
const CAMERA_LERP = 0.08;

// Building collision constants
const BUILDING_HALF_WIDTH = 25;
const BUILDING_HALF_LENGTH = 50;

export function UserAvatar({
  initialPosition = [0, 0, 30],
  color = '#ff8800',
  isAdmin = false,
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

  // Input handling (keyboard, mouse, flight)
  const {
    keysRef,
    cameraAngleTargetRef,
    isGroundedRef,
    flightModeRef,
    velocityRef,
    isSuspendedRef,
  } = useAvatarInput({ gl, canFly, isSuspended, onInteract });

  // Initialize velocity ref inside the hook by connecting it
  const localVelocityRef = useRef(new THREE.Vector3());
  useEffect(() => {
    velocityRef.current = localVelocityRef.current;
  }, [velocityRef]);

  // Camera orbit
  const cameraAngleRef = useRef(Math.PI / 4);

  // Avatar rotation
  const avatarRotationRef = useRef(0);
  const targetRotationRef = useRef(0);
  const hasMovedRef = useRef(false);

  // Tilt
  const tiltRef = useRef({ x: 0, z: 0 });

  const lastEmittedPosition = useRef<THREE.Vector3 | null>(null);
  const wasMovingRef = useRef(false);
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
  const hasCameraSnappedRef = useRef(false);

  // Main update loop
  useFrame((_, delta) => {
    if (!groupRef.current || !avatarRef.current) return;

    const position = positionRef.current;
    const velocity = localVelocityRef.current;
    const keys = keysRef.current;

    const dt = Math.min(delta, 0.1);
    const dtScale = dt * 60;

    // Smooth camera angle
    cameraAngleRef.current += (cameraAngleTargetRef.current - cameraAngleRef.current) * 0.08 * dtScale;

    if (isSuspendedRef.current) {
      groupRef.current.position.copy(position);
      camera.lookAt(position.clone().add(new THREE.Vector3(0, 4, 0)));
      return;
    }

    // Get floor height (use override if provided)
    const getFloor = (x: number, z: number, y: number) => {
      if (baseFloorHeight !== undefined) {
        return baseFloorHeight + AVATAR_RADIUS;
      }
      return getFloorHeightAt(x, z, y) + AVATAR_RADIUS;
    };

    const currentFloorHeight = getFloor(position.x, position.z, position.y);

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

    // Movement direction based on camera
    const cameraAngle = cameraAngleRef.current;
    const forward = new THREE.Vector3(-Math.sin(cameraAngle), 0, -Math.cos(cameraAngle));
    const right = new THREE.Vector3(-Math.cos(cameraAngle), 0, Math.sin(cameraAngle));

    // Acceleration
    const baseAccel = isGroundedRef.current ? 0.08 : 0.03;
    const accel = keys.sprint ? baseAccel * 1.4 : baseAccel;
    const friction = isGroundedRef.current ? 0.88 : 0.96;

    // Input direction
    const inputDir = new THREE.Vector3();
    if (keys.forward) inputDir.add(forward);
    if (keys.backward) inputDir.sub(forward);
    if (keys.left) inputDir.add(right);
    if (keys.right) inputDir.sub(right);

    if (inputDir.lengthSq() > 0) {
      inputDir.normalize();
      velocity.x += inputDir.x * accel * dtScale;
      velocity.z += inputDir.z * accel * dtScale;
    }

    // Vertical movement
    if (canFly && keys.up) velocity.y += accel * dtScale;
    if (keys.down) velocity.y -= accel * 0.7 * dtScale;

    // Friction
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

    // Speed limit
    const maxSpeed = keys.sprint ? WALK_SPEED * RUN_MULTIPLIER : WALK_SPEED;
    const horizontalSpeed = Math.sqrt(velocity.x * velocity.x + velocity.z * velocity.z);

    if (horizontalSpeed > maxSpeed * 0.035) {
      const scale = Math.min(1, (maxSpeed * 0.035) / horizontalSpeed);
      velocity.x *= scale;
      velocity.z *= scale;
    }

    // Apply movement
    const movement = velocity.clone().multiplyScalar(maxSpeed * dtScale);
    const oldPosition = position.clone();
    const newPosition = position.clone().add(movement);

    // ============ COLLISION DETECTION ============
    const isFlying = canFly && flightModeRef.current;

    // 1. Building collisions
    if (!isFlying) {
      for (const building of buildings) {
        const [bx, , bz] = building.position;
        const dx = newPosition.x - bx;
        const dz = newPosition.z - bz;
        const angle = Math.atan2(building.entranceDirection.x, building.entranceDirection.z);
        const cos = Math.cos(-angle);
        const sin = Math.sin(-angle);
        const localX = dx * cos - dz * sin;
        const localZ = dx * sin + dz * cos;

        if (Math.abs(localX) < BUILDING_HALF_WIDTH && Math.abs(localZ) < BUILDING_HALF_LENGTH) {
          const atDoor = localZ > BUILDING_HALF_LENGTH - 8 && Math.abs(localX) < 6;
          if (!atDoor) {
            newPosition.x = oldPosition.x;
            newPosition.z = oldPosition.z;
            velocity.x = 0;
            velocity.z = 0;
          }
        }
      }
    }

    // 2. Workspace/Command Center collisions
    if (!isFlying) {
      const collisionResult = performCollisionCheck(oldPosition, newPosition);
      if (collisionResult.blocked) {
        newPosition.copy(collisionResult.correctedPosition);
        if (collisionResult.hitNormal) {
          const dot = velocity.dot(collisionResult.hitNormal);
          if (dot < 0) {
            velocity.sub(collisionResult.hitNormal.clone().multiplyScalar(dot));
          }
        } else {
          velocity.x *= 0.5;
          velocity.z *= 0.5;
        }
      }
    }

    // 3. Ceiling collision
    if (!isFlying) {
      const ceiling = getCeilingHeightAt(newPosition.x, newPosition.z, newPosition.y);
      if (ceiling !== null && newPosition.y + AVATAR_HEIGHT > ceiling) {
        newPosition.y = ceiling - AVATAR_HEIGHT;
        velocity.y = Math.min(velocity.y, 0);
      }
    }

    // 4. Floor collision
    const newFloorHeight = getFloor(newPosition.x, newPosition.z, newPosition.y);
    if (newPosition.y < newFloorHeight) {
      newPosition.y = newFloorHeight;
      velocity.y = 0;
      isGroundedRef.current = true;
    } else if (newPosition.y > newFloorHeight + 0.5) {
      isGroundedRef.current = false;
    }

    // 5. Safety floor
    if (newPosition.y < GROUND_Y + AVATAR_RADIUS) {
      newPosition.y = GROUND_Y + AVATAR_RADIUS;
      velocity.y = 0;
      isGroundedRef.current = true;
    }

    // Apply final position
    position.copy(newPosition);
    groupRef.current.position.copy(position);

    // Avatar rotation
    tiltRef.current.x = 0;
    tiltRef.current.z = 0;

    const speed = Math.sqrt(velocity.x * velocity.x + velocity.z * velocity.z);

    if (speed > 0.002) {
      targetRotationRef.current = Math.atan2(velocity.x, velocity.z);
      hasMovedRef.current = true;
    }

    if (hasMovedRef.current) {
      let rotationDiff = targetRotationRef.current - avatarRotationRef.current;
      while (rotationDiff > Math.PI) rotationDiff -= Math.PI * 2;
      while (rotationDiff < -Math.PI) rotationDiff += Math.PI * 2;
      avatarRotationRef.current += rotationDiff * ROTATION_LERP * dtScale;
    }

    avatarRef.current.rotation.y = avatarRotationRef.current;
    avatarRef.current.rotation.x = 0;
    avatarRef.current.rotation.z = 0;

    // Position callback
    if (onPositionChange) {
      const isCurrentlyMoving = speed > 0.01;

      if (!lastEmittedPosition.current) {
        lastEmittedPosition.current = position.clone();
        onPositionChange(position.clone());
      } else if (lastEmittedPosition.current.distanceToSquared(position) > 0.25) {
        lastEmittedPosition.current.copy(position);
        onPositionChange(position.clone());
      } else if (wasMovingRef.current && !isCurrentlyMoving) {
        lastEmittedPosition.current.copy(position);
        onPositionChange(position.clone());
      }

      wasMovingRef.current = isCurrentlyMoving;
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
    if (!hasCameraSnappedRef.current) {
      camera.position.copy(desiredCameraPosition);
      hasCameraSnappedRef.current = true;
    } else {
      camera.position.lerp(desiredCameraPosition, CAMERA_LERP * dtScale);
    }

    const lookTarget = position.clone().add(new THREE.Vector3(0, 2.5, 0));
    camera.lookAt(lookTarget);
  });

  return (
    <group ref={groupRef}>
      <group ref={avatarRef}>
        <HumanoidAvatar
          color={color}
          isAdmin={isAdmin}
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
