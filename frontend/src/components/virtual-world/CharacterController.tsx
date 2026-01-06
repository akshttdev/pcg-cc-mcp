import { KeyboardControls, useKeyboardControls } from '@react-three/drei';
import { useEffect, useMemo, useRef } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';

export type ControllerAction =
  | 'forward'
  | 'backward'
  | 'leftward'
  | 'rightward'
  | 'jump'
  | 'run';

export const keyboardMap = [
  { name: 'forward', keys: ['ArrowUp', 'KeyW'] },
  { name: 'backward', keys: ['ArrowDown', 'KeyS'] },
  { name: 'leftward', keys: ['ArrowLeft', 'KeyA'] },
  { name: 'rightward', keys: ['ArrowRight', 'KeyD'] },
  { name: 'jump', keys: ['Space'] },
  { name: 'run', keys: ['ShiftLeft', 'ShiftRight'] },
] satisfies Parameters<typeof KeyboardControls>[0]['map'];

interface CharacterControllerProps {
  position?: [number, number, number];
}

const WALK_SPEED = 6;
const RUN_MULTIPLIER = 1.6;
const JUMP_FORCE = 8;
const GRAVITY = 24;

const lerpAngle = (current: number, target: number, alpha: number) => {
  let delta = target - current;
  while (delta > Math.PI) delta -= Math.PI * 2;
  while (delta < -Math.PI) delta += Math.PI * 2;
  return current + delta * alpha;
};

// Simple capsule character model
function CapsuleCharacter() {
  return (
    <group>
      <mesh castShadow position={[0, 0.8, 0]}>
        <capsuleGeometry args={[0.35, 0.8, 8, 16]} />
        <meshStandardMaterial
          color="#ff8800"
          emissive="#ff4400"
          emissiveIntensity={0.2}
        />
      </mesh>
      <mesh castShadow position={[0, 1.5, 0]}>
        <sphereGeometry args={[0.25, 16, 16]} />
        <meshStandardMaterial
          color="#ffaa44"
          emissive="#ff6600"
          emissiveIntensity={0.3}
        />
      </mesh>
      <mesh position={[-0.08, 1.55, 0.2]}>
        <sphereGeometry args={[0.05, 8, 8]} />
        <meshBasicMaterial color="#ffffff" />
      </mesh>
      <mesh position={[0.08, 1.55, 0.2]}>
        <sphereGeometry args={[0.05, 8, 8]} />
        <meshBasicMaterial color="#ffffff" />
      </mesh>
    </group>
  );
}

function ControllerCore({ position = [0, 5, 0] }: CharacterControllerProps) {
  const groupRef = useRef<THREE.Group>(null);
  const direction = useMemo(() => new THREE.Vector3(), []);
  const rotationTarget = useRef(0);
  const baseHeight = useRef(position[1]);
  const jumpVelocity = useRef(0);
  const grounded = useRef(true);
  const [subscribeKeys, getKeys] = useKeyboardControls<ControllerAction>();

  useEffect(() => {
    const unsubscribe = subscribeKeys(
      (state) => state.jump,
      (value) => {
        if (value && grounded.current) {
          jumpVelocity.current = JUMP_FORCE;
          grounded.current = false;
        }
      }
    );
    return unsubscribe;
  }, [subscribeKeys]);

  useFrame((_, delta) => {
    if (!groupRef.current) return;
    const { forward, backward, leftward, rightward, run } = getKeys();

    direction.set(
      (rightward ? 1 : 0) - (leftward ? 1 : 0),
      0,
      (backward ? 1 : 0) - (forward ? 1 : 0)
    );

    if (direction.lengthSq() > 0) {
      direction.normalize();
      const moveSpeed = (run ? WALK_SPEED * RUN_MULTIPLIER : WALK_SPEED) * delta;
      groupRef.current.position.addScaledVector(direction, moveSpeed);
      rotationTarget.current = Math.atan2(direction.x, direction.z);
    }

    const lerpedRotation = lerpAngle(
      groupRef.current.rotation.y,
      rotationTarget.current,
      0.2
    );
    groupRef.current.rotation.y = lerpedRotation;

    jumpVelocity.current -= GRAVITY * delta;
    groupRef.current.position.y += jumpVelocity.current * delta;

    if (groupRef.current.position.y <= baseHeight.current) {
      groupRef.current.position.y = baseHeight.current;
      jumpVelocity.current = 0;
      grounded.current = true;
    }
  });

  return (
    <group ref={groupRef} position={position}>
      <CapsuleCharacter />
    </group>
  );
}

export function CharacterController(props: CharacterControllerProps) {
  return <ControllerCore {...props} />;
}

// Wrapper component that includes KeyboardControls
export function CharacterControllerWithKeyboard(props: CharacterControllerProps) {
  return (
    <KeyboardControls map={keyboardMap}>
      <ControllerCore {...props} />
    </KeyboardControls>
  );
}
