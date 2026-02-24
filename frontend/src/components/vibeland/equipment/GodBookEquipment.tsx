import { useRef } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';

// The Book of Law - Armadyl's sacred text
// OSRS-style simple low-poly book with white cover and gold page edges
// Easter egg: This book will eventually contain the source code to redeploy the dashboard

export function GodBookEquipment() {
  const bookRef = useRef<THREE.Group>(null);

  // Colors matching OSRS Book of Law
  const coverColor = '#d8d8d8'; // Light gray/white cover
  const pageEdgeColor = '#c9b44a'; // Yellow/gold page edges (like the reference)
  const titleColor = '#a8a090'; // Slightly darker rectangle for title area

  useFrame(({ clock }) => {
    if (!bookRef.current) return;
    const time = clock.elapsedTime;
    // Subtle idle animation
    bookRef.current.rotation.z = Math.sin(time * 0.8) * 0.02;
    bookRef.current.position.y = Math.sin(time * 1.2) * 0.008;
  });

  return (
    <group
      ref={bookRef}
      position={[0, -0.72, 0.12]}
      rotation={[Math.PI * 0.15, Math.PI * 0.1, Math.PI * -0.05]}
    >
      {/* Main book body - closed book shape */}
      <group>
        {/* Front cover - white/gray with beveled top corners */}
        <mesh position={[0, 0, 0.045]} castShadow>
          <boxGeometry args={[0.14, 0.18, 0.012]} />
          <meshStandardMaterial
            color={coverColor}
            metalness={0}
            roughness={0.8}
          />
        </mesh>

        {/* Back cover */}
        <mesh position={[0, 0, -0.045]} castShadow>
          <boxGeometry args={[0.14, 0.18, 0.012]} />
          <meshStandardMaterial
            color={coverColor}
            metalness={0}
            roughness={0.8}
          />
        </mesh>

        {/* Spine - gold/yellow colored */}
        <mesh position={[-0.076, 0, 0]} castShadow>
          <boxGeometry args={[0.014, 0.18, 0.1]} />
          <meshStandardMaterial
            color={pageEdgeColor}
            metalness={0.1}
            roughness={0.6}
          />
        </mesh>

        {/* Page edges - visible gold/yellow on right side */}
        <mesh position={[0.065, 0, 0]}>
          <boxGeometry args={[0.02, 0.165, 0.075]} />
          <meshStandardMaterial
            color={pageEdgeColor}
            metalness={0.1}
            roughness={0.5}
          />
        </mesh>

        {/* Page edges - top */}
        <mesh position={[0, 0.082, 0]}>
          <boxGeometry args={[0.12, 0.012, 0.075]} />
          <meshStandardMaterial
            color={pageEdgeColor}
            metalness={0.1}
            roughness={0.5}
          />
        </mesh>

        {/* Page edges - bottom */}
        <mesh position={[0, -0.082, 0]}>
          <boxGeometry args={[0.12, 0.012, 0.075]} />
          <meshStandardMaterial
            color={pageEdgeColor}
            metalness={0.1}
            roughness={0.5}
          />
        </mesh>

        {/* Title rectangle on front cover - simple inset */}
        <mesh position={[0, 0.045, 0.052]}>
          <boxGeometry args={[0.08, 0.025, 0.003]} />
          <meshStandardMaterial
            color={titleColor}
            metalness={0}
            roughness={0.9}
          />
        </mesh>

        {/* Beveled corner - top right (like OSRS reference) */}
        <mesh position={[0.055, 0.075, 0.045]} rotation={[0, 0, Math.PI / 4]}>
          <boxGeometry args={[0.03, 0.03, 0.013]} />
          <meshStandardMaterial
            color={coverColor}
            metalness={0}
            roughness={0.8}
          />
        </mesh>

        {/* Cover edge detail - subtle gold trim on front */}
        <mesh position={[0, 0, 0.052]}>
          <boxGeometry args={[0.135, 0.175, 0.002]} />
          <meshStandardMaterial
            color={pageEdgeColor}
            metalness={0.2}
            roughness={0.6}
            transparent
            opacity={0.3}
          />
        </mesh>
      </group>
    </group>
  );
}
