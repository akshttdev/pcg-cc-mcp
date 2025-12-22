import { useRef, useState, useEffect } from 'react';
import { useFrame, useThree } from '@react-three/fiber';
import { Line } from '@react-three/drei';
import * as THREE from 'three';

interface UserAvatarProps {
  initialPosition?: [number, number, number];
  color?: string;
  onPositionChange?: (position: THREE.Vector3) => void;
  onInteract?: () => void;
  isSuspended?: boolean;
  canFly?: boolean;
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
}

const FLIGHT_DOUBLE_TAP_WINDOW_MS = 400;
const JUMP_STRENGTH = 0.22;
const WALK_SPEED = 0.5;
const RUN_MULTIPLIER = 1.4;

export function UserAvatar({
  initialPosition = [0, 0, 30],
  color = '#ff8000',
  onPositionChange,
  onInteract,
  isSuspended = false,
  canFly = false,
}: UserAvatarProps) {
  const groupRef = useRef<THREE.Group>(null);
  const { camera } = useThree();

  const [position] = useState(new THREE.Vector3(...initialPosition));
  const [velocity] = useState(new THREE.Vector3());
  const [keys] = useState<MovementKeys>({
    forward: false,
    backward: false,
    left: false,
    right: false,
    up: false,
    down: false,
    sprint: false,
  });
  const lastEmittedPosition = useRef<THREE.Vector3 | null>(null);
  const isGroundedRef = useRef(true);
  const flightModeRef = useRef(false);
  const lastSpaceTapRef = useRef(0);
  const animationStateRef = useRef<AnimationDescriptor>({
    mode: 'idle',
    intensity: 0,
    airborne: false,
  });

  const trailRef = useRef<THREE.Vector3[]>([]);
  const [trailPoints, setTrailPoints] = useState<THREE.Vector3[]>([]);
  const maxTrailLength = 20;

  useEffect(() => {
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
          keys.forward = true;
          break;
        case 's':
          keys.backward = true;
          break;
        case 'a':
          keys.left = true;
          break;
        case 'd':
          keys.right = true;
          break;
        case ' ':
          if (canFly) {
            const now = performance.now();
            if (now - lastSpaceTapRef.current < FLIGHT_DOUBLE_TAP_WINDOW_MS) {
              flightModeRef.current = true;
            }
            lastSpaceTapRef.current = now;
            keys.up = true;
            if (isGroundedRef.current && velocity.y <= 0.01) {
              velocity.y += JUMP_STRENGTH;
            }
          } else if (isGroundedRef.current) {
            velocity.y = JUMP_STRENGTH;
          }
          break;
        case 'control':
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
          keys.forward = false;
          break;
        case 's':
          keys.backward = false;
          break;
        case 'a':
          keys.left = false;
          break;
        case 'd':
          keys.right = false;
          break;
        case ' ':
          keys.up = false;
          flightModeRef.current = false;
          break;
        case 'control':
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
  }, [keys, isSuspended, onInteract, canFly, velocity]);

  useEffect(() => {
    if (isSuspended) {
      (Object.keys(keys) as (keyof MovementKeys)[]).forEach((movementKey) => {
        keys[movementKey] = false;
      });
      velocity.set(0, 0, 0);
      flightModeRef.current = false;
    }
  }, [isSuspended, keys, velocity]);

  useFrame((_state) => {
    if (!groupRef.current) return;

    if (isSuspended) {
      groupRef.current.position.copy(position);
      camera.lookAt(position.clone().add(new THREE.Vector3(0, 4, 0)));
      return;
    }

    const moveSpeed = keys.sprint ? WALK_SPEED * RUN_MULTIPLIER : WALK_SPEED;
    const acceleration = 0.1;
    const deceleration = 0.95;

    const forward = new THREE.Vector3();
    camera.getWorldDirection(forward);
    forward.y = 0;
    forward.normalize();

    const right = new THREE.Vector3();
    right.crossVectors(forward, new THREE.Vector3(0, 1, 0)).normalize();

    const inputVector = new THREE.Vector3();
    if (keys.forward) inputVector.add(forward.multiplyScalar(acceleration));
    if (keys.backward) inputVector.add(forward.multiplyScalar(-acceleration));
    if (keys.left) inputVector.add(right.multiplyScalar(-acceleration));
    if (keys.right) inputVector.add(right.multiplyScalar(acceleration));
    if (canFly && keys.up) inputVector.y += acceleration;
    if (keys.down) inputVector.y -= acceleration * 0.6;

    velocity.add(inputVector);
    velocity.multiplyScalar(deceleration);

    const gravity = canFly ? 0.01 : 0.03;
    if (!isGroundedRef.current || velocity.y > 0) {
      velocity.y -= gravity;
    }

    if (canFly && flightModeRef.current && keys.up) {
      velocity.y = Math.min(velocity.y + 0.01, 0.25);
    }

    const currentSpeed = keys.sprint ? 2.0 : 1.0;
    const movement = velocity.clone().multiplyScalar(moveSpeed * currentSpeed);

    position.add(movement);

    if (position.y < 0) {
      position.y = 0;
      velocity.y = 0;
      isGroundedRef.current = true;
    } else {
      isGroundedRef.current = false;
    }

    groupRef.current.position.copy(position);

    if (onPositionChange) {
      if (!lastEmittedPosition.current) {
        lastEmittedPosition.current = position.clone();
        onPositionChange(position.clone());
      } else if (lastEmittedPosition.current.distanceToSquared(position) > 0.25) {
        lastEmittedPosition.current.copy(position);
        onPositionChange(position.clone());
      }
    }

    if (movement.length() > 0.01) {
      trailRef.current.unshift(position.clone());
      if (trailRef.current.length > maxTrailLength) {
        trailRef.current.pop();
      }
      setTrailPoints(trailRef.current.map((point, index) => {
        const alpha = 1 - index / maxTrailLength;
        return point.clone().setY(point.y + alpha * 0.15);
      }));
    }

    const horizontalSpeed = new THREE.Vector3(velocity.x, 0, velocity.z).length();
    const airborne = !isGroundedRef.current;
    let mode: AnimationDescriptor['mode'] = 'idle';
    if (airborne) {
      mode = canFly && flightModeRef.current ? 'fly' : 'jump';
    } else if (horizontalSpeed > 0.4) {
      mode = keys.sprint ? 'run' : 'walk';
    }
    animationStateRef.current = {
      mode,
      intensity: Math.min(horizontalSpeed * 2.5, 1.0),
      airborne,
    };

    const desiredCameraPosition = position.clone().add(new THREE.Vector3(25, 20, 25));
    camera.position.lerp(desiredCameraPosition, 0.05);
    camera.lookAt(position.clone().add(new THREE.Vector3(0, 4, 0)));
  });

  return (
    <group ref={groupRef} position={position}>
      <HumanoidAvatar
        color={color}
        animationRef={animationStateRef}
        showJetpack={canFly && (flightModeRef.current || !isGroundedRef.current)}
      />
      {trailPoints.length >= 2 && (
        <Line
          points={trailPoints}
          color={color}
          lineWidth={2}
          transparent
          opacity={0.4}
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
  const headRef = useRef<THREE.Group>(null);
  const leftArmRef = useRef<THREE.Group>(null);
  const rightArmRef = useRef<THREE.Group>(null);
  const leftLegRef = useRef<THREE.Group>(null);
  const rightLegRef = useRef<THREE.Group>(null);
  const torsoRef = useRef<THREE.Mesh>(null);

  useFrame((state) => {
    const { mode, intensity } = animationRef.current;
    const swingSpeed = mode === 'run' ? 10 : 6;
    const swingAmount = matchAnimationSwing(mode, intensity);
    const swing = Math.sin(state.clock.elapsedTime * swingSpeed) * swingAmount;
    const oppositeSwing = Math.sin(state.clock.elapsedTime * swingSpeed + Math.PI) * swingAmount;

    if (leftArmRef.current && rightArmRef.current) {
      leftArmRef.current.rotation.x = swing;
      rightArmRef.current.rotation.x = oppositeSwing;
    }

    if (leftLegRef.current && rightLegRef.current) {
      leftLegRef.current.rotation.x = oppositeSwing * 0.8;
      rightLegRef.current.rotation.x = swing * 0.8;
    }

    if (headRef.current) {
      const hover =
        mode === 'fly'
          ? Math.sin(state.clock.elapsedTime * 3) * 0.05
          : Math.sin(state.clock.elapsedTime * 1.5) * 0.02;
      headRef.current.position.y = 2.4 + hover;
    }

    // Breathing animation for torso
    if (torsoRef.current && mode === 'idle') {
      const breathe = Math.sin(state.clock.elapsedTime * 0.8) * 0.02 + 1;
      torsoRef.current.scale.y = breathe;
    }
  });

  return (
    <group>
      {/* Head with facial features */}
      <group ref={headRef} position={[0, 2.4, 0]}>
        {/* Main head sphere */}
        <mesh castShadow receiveShadow>
          <sphereGeometry args={[0.55, 32, 32]} />
          <meshStandardMaterial
            color={color}
            emissive={color}
            emissiveIntensity={0.5}
            metalness={0.2}
            roughness={0.3}
          />
        </mesh>

        {/* Eyes */}
        <mesh position={[-0.2, 0.1, 0.45]}>
          <sphereGeometry args={[0.08, 16, 16]} />
          <meshStandardMaterial
            color="#ffffff"
            emissive="#00ffff"
            emissiveIntensity={0.8}
          />
        </mesh>
        <mesh position={[0.2, 0.1, 0.45]}>
          <sphereGeometry args={[0.08, 16, 16]} />
          <meshStandardMaterial
            color="#ffffff"
            emissive="#00ffff"
            emissiveIntensity={0.8}
          />
        </mesh>

        {/* Visor/Face plate */}
        <mesh position={[0, 0, 0.52]} rotation={[0, 0, 0]}>
          <planeGeometry args={[0.8, 0.4]} />
          <meshPhysicalMaterial
            color="#00ffff"
            transmission={0.9}
            roughness={0.05}
            metalness={0.8}
            transparent
            opacity={0.3}
            emissive="#00ffff"
            emissiveIntensity={0.2}
          />
        </mesh>

        {/* Antenna/Communication device */}
        <group position={[0, 0.55, -0.2]}>
          <mesh>
            <cylinderGeometry args={[0.02, 0.02, 0.3, 8]} />
            <meshStandardMaterial
              color="#00ffff"
              emissive="#00ffff"
              emissiveIntensity={0.6}
              metalness={0.9}
            />
          </mesh>
          <mesh position={[0, 0.18, 0]}>
            <sphereGeometry args={[0.05, 16, 16]} />
            <meshBasicMaterial color="#00ffff" />
            <pointLight intensity={0.5} color="#00ffff" distance={3} />
          </mesh>
        </group>
      </group>

      {/* Torso with tool belt */}
      <group position={[0, 1.1, 0]}>
        <mesh ref={torsoRef} castShadow receiveShadow>
          <capsuleGeometry args={[0.55, 1.7, 16, 32]} />
          <meshStandardMaterial
            color={color}
            emissive={color}
            emissiveIntensity={0.35}
            metalness={0.15}
            roughness={0.35}
          />
        </mesh>

        {/* Tool belt */}
        <mesh position={[0, -0.4, 0]} rotation={[0, 0, 0]}>
          <torusGeometry args={[0.6, 0.08, 16, 32]} />
          <meshStandardMaterial
            color="#1a1a1a"
            metalness={0.9}
            roughness={0.2}
            emissive="#00ffff"
            emissiveIntensity={0.1}
          />
        </mesh>

        {/* Belt pouches */}
        <mesh position={[0.4, -0.4, 0.4]}>
          <boxGeometry args={[0.15, 0.15, 0.1]} />
          <meshStandardMaterial color="#1a1a1a" metalness={0.8} roughness={0.3} />
        </mesh>
        <mesh position={[-0.4, -0.4, 0.4]}>
          <boxGeometry args={[0.15, 0.15, 0.1]} />
          <meshStandardMaterial color="#1a1a1a" metalness={0.8} roughness={0.3} />
        </mesh>

        {/* Chest light indicator */}
        <mesh position={[0, 0.5, 0.5]}>
          <circleGeometry args={[0.08, 16]} />
          <meshBasicMaterial color="#00ffff" transparent opacity={0.8} />
          <pointLight intensity={0.3} color="#00ffff" distance={2} />
        </mesh>
      </group>

      {/* Arms with gloves */}
      <group ref={leftArmRef} position={[0.85, 1.2, 0]}>
        <mesh castShadow>
          <capsuleGeometry args={[0.18, 1.0, 12, 16]} />
          <meshStandardMaterial color={color} emissive={color} emissiveIntensity={0.25} />
        </mesh>
        {/* Glove */}
        <mesh position={[0, -0.6, 0]}>
          <sphereGeometry args={[0.2, 16, 16]} />
          <meshStandardMaterial
            color="#1a1a1a"
            metalness={0.9}
            roughness={0.2}
            emissive="#00ffff"
            emissiveIntensity={0.1}
          />
        </mesh>
      </group>
      <group ref={rightArmRef} position={[-0.85, 1.2, 0]}>
        <mesh castShadow>
          <capsuleGeometry args={[0.18, 1.0, 12, 16]} />
          <meshStandardMaterial color={color} emissive={color} emissiveIntensity={0.25} />
        </mesh>
        {/* Glove */}
        <mesh position={[0, -0.6, 0]}>
          <sphereGeometry args={[0.2, 16, 16]} />
          <meshStandardMaterial
            color="#1a1a1a"
            metalness={0.9}
            roughness={0.2}
            emissive="#00ffff"
            emissiveIntensity={0.1}
          />
        </mesh>
      </group>

      {/* Legs with boots */}
      <group ref={leftLegRef} position={[0.35, 0, 0]}>
        <mesh castShadow>
          <capsuleGeometry args={[0.22, 1.2, 12, 16]} />
          <meshStandardMaterial color={color} emissive={color} emissiveIntensity={0.25} />
        </mesh>
        {/* Boot */}
        <mesh position={[0, -0.7, 0.1]}>
          <boxGeometry args={[0.28, 0.2, 0.4]} />
          <meshStandardMaterial
            color="#1a1a1a"
            metalness={0.9}
            roughness={0.2}
          />
        </mesh>
        {/* Boot accent strip */}
        <mesh position={[0, -0.7, 0.3]}>
          <boxGeometry args={[0.3, 0.05, 0.05]} />
          <meshBasicMaterial color="#00ffff" />
        </mesh>
      </group>
      <group ref={rightLegRef} position={[-0.35, 0, 0]}>
        <mesh castShadow>
          <capsuleGeometry args={[0.22, 1.2, 12, 16]} />
          <meshStandardMaterial color={color} emissive={color} emissiveIntensity={0.25} />
        </mesh>
        {/* Boot */}
        <mesh position={[0, -0.7, 0.1]}>
          <boxGeometry args={[0.28, 0.2, 0.4]} />
          <meshStandardMaterial
            color="#1a1a1a"
            metalness={0.9}
            roughness={0.2}
          />
        </mesh>
        {/* Boot accent strip */}
        <mesh position={[0, -0.7, 0.3]}>
          <boxGeometry args={[0.3, 0.05, 0.05]} />
          <meshBasicMaterial color="#00ffff" />
        </mesh>
      </group>

      {/* Jetpack (visible during flight) */}
      {showJetpack && (
        <group position={[0, 1.2, -0.6]}>
          {/* Main jetpack body */}
          <mesh>
            <boxGeometry args={[0.8, 1.2, 0.4]} />
            <meshStandardMaterial
              color="#1a1a1a"
              metalness={0.9}
              roughness={0.2}
              emissive="#0080ff"
              emissiveIntensity={0.3}
            />
          </mesh>

          {/* Fuel tanks */}
          <mesh position={[-0.25, 0, 0]}>
            <cylinderGeometry args={[0.15, 0.15, 1.0, 16]} />
            <meshStandardMaterial
              color="#0a2f4a"
              metalness={0.8}
              roughness={0.3}
              emissive="#00b4ff"
              emissiveIntensity={0.4}
            />
          </mesh>
          <mesh position={[0.25, 0, 0]}>
            <cylinderGeometry args={[0.15, 0.15, 1.0, 16]} />
            <meshStandardMaterial
              color="#0a2f4a"
              metalness={0.8}
              roughness={0.3}
              emissive="#00b4ff"
              emissiveIntensity={0.4}
            />
          </mesh>

          {/* Thrusters */}
          <mesh position={[-0.25, -0.6, 0]}>
            <coneGeometry args={[0.18, 0.3, 8]} />
            <meshBasicMaterial color="#00ffff" transparent opacity={0.8} />
            <pointLight intensity={1} color="#00ffff" distance={5} />
          </mesh>
          <mesh position={[0.25, -0.6, 0]}>
            <coneGeometry args={[0.18, 0.3, 8]} />
            <meshBasicMaterial color="#00ffff" transparent opacity={0.8} />
            <pointLight intensity={1} color="#00ffff" distance={5} />
          </mesh>

          {/* Exhaust particles */}
          {animationRef.current.mode === 'fly' && (
            <>
              <mesh position={[-0.25, -0.8, 0]}>
                <sphereGeometry args={[0.1, 8, 8]} />
                <meshBasicMaterial
                  color="#00b4ff"
                  transparent
                  opacity={0.6}
                />
              </mesh>
              <mesh position={[0.25, -0.8, 0]}>
                <sphereGeometry args={[0.1, 8, 8]} />
                <meshBasicMaterial
                  color="#00b4ff"
                  transparent
                  opacity={0.6}
                />
              </mesh>
            </>
          )}
        </group>
      )}
    </group>
  );
}

function matchAnimationSwing(mode: AnimationDescriptor['mode'], intensity: number): number {
  switch (mode) {
    case 'run':
      return 0.7 * intensity;
    case 'walk':
      return 0.4 * intensity;
    case 'jump':
    case 'fly':
      return 0.2;
    default:
      return 0.05;
  }
}
