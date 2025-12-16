import { useRef, useState, useEffect } from 'react';
import { useFrame, useThree } from '@react-three/fiber';
import { Line } from '@react-three/drei';
import * as THREE from 'three';

interface UserAvatarProps {
  initialPosition?: [number, number, number];
  color?: string;
}

export function UserAvatar({ initialPosition = [0, 0, 30], color = '#ff8000' }: UserAvatarProps) {
  const groupRef = useRef<THREE.Group>(null);
  const { camera } = useThree();

  const [position] = useState(new THREE.Vector3(...initialPosition));
  const [velocity] = useState(new THREE.Vector3());
  const [keys] = useState({
    forward: false,
    backward: false,
    left: false,
    right: false,
    up: false,
    down: false,
    sprint: false,
  });

  // Trail positions for light trail effect
  const trailRef = useRef<THREE.Vector3[]>([]);
  const maxTrailLength = 20;

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      switch (e.key.toLowerCase()) {
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
          keys.up = true;
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
  }, [keys]);

  useFrame(() => {
    if (!groupRef.current) return;

    // Movement parameters
    const moveSpeed = keys.sprint ? 1.0 : 0.5;
    const acceleration = 0.1;
    const deceleration = 0.95;

    // Get camera direction (forward vector)
    const forward = new THREE.Vector3();
    camera.getWorldDirection(forward);
    forward.y = 0;
    forward.normalize();

    // Get right vector
    const right = new THREE.Vector3();
    right.crossVectors(forward, new THREE.Vector3(0, 1, 0)).normalize();

    // Apply input
    const inputVector = new THREE.Vector3();
    if (keys.forward) inputVector.add(forward.multiplyScalar(acceleration));
    if (keys.backward) inputVector.add(forward.multiplyScalar(-acceleration));
    if (keys.left) inputVector.add(right.multiplyScalar(-acceleration));
    if (keys.right) inputVector.add(right.multiplyScalar(acceleration));
    if (keys.up) inputVector.y += acceleration;
    if (keys.down) inputVector.y -= acceleration;

    // Update velocity
    velocity.add(inputVector);
    velocity.multiplyScalar(deceleration);

    // Apply sprint multiplier
    const currentSpeed = keys.sprint ? 2.0 : 1.0;
    const movement = velocity.clone().multiplyScalar(moveSpeed * currentSpeed);

    // Update position
    position.add(movement);

    // Ground collision (keep above grid)
    if (position.y < 0) {
      position.y = 0;
      velocity.y = 0;
    }

    // Update avatar position
    groupRef.current.position.copy(position);

    // Update trail
    if (movement.length() > 0.01) {
      trailRef.current.unshift(position.clone());
      if (trailRef.current.length > maxTrailLength) {
        trailRef.current.pop();
      }
    }

    // Camera follow (third-person)
    const cameraOffset = new THREE.Vector3(0, 8, 12);
    const targetCameraPos = position.clone().add(cameraOffset);
    camera.position.lerp(targetCameraPos, 0.1);
    camera.lookAt(position.clone().add(new THREE.Vector3(0, 4, 0)));
  });

  const colorObj = new THREE.Color(color);

  return (
    <group ref={groupRef}>
      {/* Body - Tron style suit */}
      <mesh position={[0, 3, 0]} castShadow>
        <capsuleGeometry args={[0.5, 2, 8, 16]} />
        <meshStandardMaterial
          color="#0a0a0a"
          emissive={colorObj}
          emissiveIntensity={0.3}
          roughness={0.8}
          metalness={0.2}
        />
      </mesh>

      {/* Head */}
      <mesh position={[0, 4.5, 0]} castShadow>
        <sphereGeometry args={[0.6, 16, 16]} />
        <meshStandardMaterial color="#0a0a0a" roughness={0.3} metalness={0.8} />
      </mesh>

      {/* Visor glow */}
      <mesh position={[0, 4.5, 0.5]}>
        <planeGeometry args={[0.8, 0.2]} />
        <meshBasicMaterial color={colorObj} transparent opacity={0.9} />
      </mesh>

      {/* Circuit lines on body */}
      <Line
        points={[
          [0, 2, 0.5],
          [0, 4, 0.5],
          [0.3, 4.5, 0.5],
        ]}
        color={color}
        lineWidth={2}
      />
      <Line
        points={[
          [-0.5, 3, 0.5],
          [-0.8, 2.5, 0.5],
        ]}
        color={color}
        lineWidth={2}
      />
      <Line
        points={[
          [0.5, 3, 0.5],
          [0.8, 2.5, 0.5],
        ]}
        color={color}
        lineWidth={2}
      />

      {/* Arms */}
      <mesh position={[-0.8, 3, 0]} rotation={[0, 0, Math.PI / 6]}>
        <cylinderGeometry args={[0.15, 0.15, 1.5, 8]} />
        <meshStandardMaterial
          color="#0a0a0a"
          emissive={colorObj}
          emissiveIntensity={0.2}
          roughness={0.8}
        />
      </mesh>
      <mesh position={[0.8, 3, 0]} rotation={[0, 0, -Math.PI / 6]}>
        <cylinderGeometry args={[0.15, 0.15, 1.5, 8]} />
        <meshStandardMaterial
          color="#0a0a0a"
          emissive={colorObj}
          emissiveIntensity={0.2}
          roughness={0.8}
        />
      </mesh>

      {/* Legs */}
      <mesh position={[-0.3, 1.5, 0]}>
        <cylinderGeometry args={[0.18, 0.18, 2, 8]} />
        <meshStandardMaterial
          color="#0a0a0a"
          emissive={colorObj}
          emissiveIntensity={0.2}
          roughness={0.8}
        />
      </mesh>
      <mesh position={[0.3, 1.5, 0]}>
        <cylinderGeometry args={[0.18, 0.18, 2, 8]} />
        <meshStandardMaterial
          color="#0a0a0a"
          emissive={colorObj}
          emissiveIntensity={0.2}
          roughness={0.8}
        />
      </mesh>

      {/* Light trail */}
      {trailRef.current.length > 1 && (
        <Line points={trailRef.current} color={color} lineWidth={1} transparent opacity={0.5} />
      )}

      {/* Avatar lighting */}
      <pointLight position={[0, 4, 0]} intensity={0.5} color={color} distance={5} />
    </group>
  );
}
