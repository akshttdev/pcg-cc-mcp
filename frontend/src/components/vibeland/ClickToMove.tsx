import { useRef, useState } from 'react';
import { useFrame, ThreeEvent } from '@react-three/fiber';
import * as THREE from 'three';

interface ClickToMoveProps {
  onTargetChange: (target: THREE.Vector3 | null) => void;
  groundY?: number;
  enabled?: boolean;
}

export function ClickToMoveIndicator({
  position,
  visible,
}: {
  position: THREE.Vector3;
  visible: boolean;
}) {
  const ringRef = useRef<THREE.Mesh>(null);
  const pulseRef = useRef(0);

  useFrame(() => {
    if (!ringRef.current || !visible) return;

    // Pulsing animation
    pulseRef.current += 0.1;
    const scale = 1 + Math.sin(pulseRef.current) * 0.2;
    ringRef.current.scale.setScalar(scale);

    // Rotation animation
    ringRef.current.rotation.z += 0.02;
  });

  if (!visible) return null;

  return (
    <group position={[position.x, position.y + 0.1, position.z]}>
      {/* Outer ring */}
      <mesh ref={ringRef} rotation={[-Math.PI / 2, 0, 0]}>
        <ringGeometry args={[0.8, 1, 32]} />
        <meshBasicMaterial color="#00ffff" transparent opacity={0.8} side={THREE.DoubleSide} />
      </mesh>

      {/* Inner circle */}
      <mesh rotation={[-Math.PI / 2, 0, 0]}>
        <circleGeometry args={[0.3, 32]} />
        <meshBasicMaterial color="#ffffff" transparent opacity={0.9} />
      </mesh>

      {/* Vertical beam */}
      <mesh position={[0, 1.5, 0]}>
        <cylinderGeometry args={[0.05, 0.05, 3, 8]} />
        <meshBasicMaterial color="#00ffff" transparent opacity={0.4} />
      </mesh>
    </group>
  );
}

export function ClickToMoveGround({
  onTargetChange,
  groundY = 0,
  enabled = true,
}: ClickToMoveProps) {
  const [clickTarget, setClickTarget] = useState<THREE.Vector3 | null>(null);

  const handleClick = (event: ThreeEvent<MouseEvent>) => {
    if (!enabled) return;

    event.stopPropagation();

    // Get the click position on the ground plane
    const point = event.point.clone();
    point.y = groundY;

    setClickTarget(point);
    onTargetChange(point);

    // Clear the indicator after a delay
    setTimeout(() => {
      setClickTarget(null);
    }, 2000);
  };

  return (
    <>
      {/* Invisible ground plane for click detection */}
      <mesh
        rotation={[-Math.PI / 2, 0, 0]}
        position={[0, groundY, 0]}
        onClick={handleClick}
        visible={false}
      >
        <planeGeometry args={[1000, 1000]} />
        <meshBasicMaterial transparent opacity={0} />
      </mesh>

      {/* Click indicator */}
      {clickTarget && (
        <ClickToMoveIndicator position={clickTarget} visible={true} />
      )}
    </>
  );
}
