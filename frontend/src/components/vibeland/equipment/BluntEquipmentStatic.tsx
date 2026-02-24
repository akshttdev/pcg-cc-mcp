/**
 * Static version of BluntEquipment for remote avatars.
 * No arm animation - just the visual blunt held in hand.
 */
export function BluntEquipmentStatic() {
  return (
    <group position={[0, -0.85, 0.12]} rotation={[Math.PI * 0.4, 0, 0]}>
      {/* Blunt body */}
      <mesh>
        <cylinderGeometry args={[0.035, 0.03, 0.35, 12]} />
        <meshStandardMaterial color="#5c4033" roughness={0.9} metalness={0} />
      </mesh>
      {/* Wrap lines */}
      <mesh position={[0, -0.06, 0]}>
        <cylinderGeometry args={[0.037, 0.037, 0.02, 12]} />
        <meshStandardMaterial color="#3d2817" roughness={1} metalness={0} />
      </mesh>
      <mesh position={[0, -0.12, 0]}>
        <cylinderGeometry args={[0.032, 0.032, 0.018, 12]} />
        <meshStandardMaterial color="#3d2817" roughness={1} metalness={0} />
      </mesh>
      {/* Cherry/lit end */}
      <mesh position={[0, 0.18, 0]}>
        <sphereGeometry args={[0.038, 12, 12]} />
        <meshStandardMaterial
          color="#ff4500"
          emissive="#ff2200"
          emissiveIntensity={0.8}
          roughness={0.5}
        />
      </mesh>
      {/* Ash tip */}
      <mesh position={[0, 0.21, 0]}>
        <cylinderGeometry args={[0.015, 0.032, 0.04, 8]} />
        <meshStandardMaterial color="#606060" roughness={1} metalness={0} />
      </mesh>
      {/* Ambient smoke wisp from cherry */}
      <mesh position={[0, 0.26, 0]}>
        <sphereGeometry args={[0.02, 8, 8]} />
        <meshStandardMaterial color="#aaaaaa" transparent opacity={0.25} />
      </mesh>
    </group>
  );
}
