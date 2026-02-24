import * as THREE from 'three';

export function CrownEquipment() {
  return (
    <group position={[0, 0.52, 0]}>
      {/* Crown base band - ring/cylinder */}
      <mesh>
        <cylinderGeometry args={[0.38, 0.35, 0.15, 32, 1, true]} />
        <meshStandardMaterial
          color="#ffd700"
          emissive="#ffd700"
          emissiveIntensity={0.3}
          metalness={0.95}
          roughness={0.1}
          side={THREE.DoubleSide}
        />
      </mesh>
      {/* Inner band */}
      <mesh>
        <cylinderGeometry args={[0.3, 0.28, 0.15, 32, 1, true]} />
        <meshStandardMaterial
          color="#ffd700"
          emissive="#ffd700"
          emissiveIntensity={0.2}
          metalness={0.95}
          roughness={0.1}
          side={THREE.DoubleSide}
        />
      </mesh>
      {/* Top rim - hollow ring */}
      <mesh position={[0, 0.075, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <ringGeometry args={[0.3, 0.38, 32]} />
        <meshStandardMaterial
          color="#ffd700"
          emissive="#ffd700"
          emissiveIntensity={0.3}
          metalness={0.95}
          roughness={0.1}
          side={THREE.DoubleSide}
        />
      </mesh>
      {/* Bottom rim */}
      <mesh position={[0, -0.075, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <ringGeometry args={[0.28, 0.35, 32]} />
        <meshStandardMaterial
          color="#ffd700"
          emissive="#ffd700"
          emissiveIntensity={0.3}
          metalness={0.95}
          roughness={0.1}
          side={THREE.DoubleSide}
        />
      </mesh>
      {/* Decorative band around middle */}
      <mesh position={[0, 0, 0]} rotation={[Math.PI / 2, 0, 0]}>
        <torusGeometry args={[0.365, 0.025, 8, 32]} />
        <meshStandardMaterial
          color="#ffd700"
          emissive="#ffd700"
          emissiveIntensity={0.5}
          metalness={0.95}
          roughness={0.05}
        />
      </mesh>
      {/* Crown points/spikes */}
      {[0, 1, 2, 3, 4, 5, 6, 7].map((i) => {
        const angle = (i / 8) * Math.PI * 2;
        const x = Math.sin(angle) * 0.34;
        const z = Math.cos(angle) * 0.34;
        const isMainSpike = i % 2 === 0;
        const spikeHeight = isMainSpike ? 0.22 : 0.14;
        return (
          <group key={i} position={[x, 0.075 + spikeHeight / 2, z]}>
            {/* Spike */}
            <mesh>
              <coneGeometry args={[0.05, spikeHeight, 4]} />
              <meshStandardMaterial
                color="#ffd700"
                emissive="#ffd700"
                emissiveIntensity={0.3}
                metalness={0.95}
                roughness={0.1}
              />
            </mesh>
            {/* Diamond on main spikes */}
            {isMainSpike && (
              <mesh position={[0, spikeHeight / 2 + 0.03, 0]}>
                <octahedronGeometry args={[0.035]} />
                <meshPhysicalMaterial
                  color="#ffffff"
                  emissive="#88ffff"
                  emissiveIntensity={0.4}
                  metalness={0.1}
                  roughness={0}
                  transmission={0.9}
                  thickness={0.5}
                  ior={2.4}
                />
              </mesh>
            )}
          </group>
        );
      })}
      {/* Front center large diamond on band */}
      <mesh position={[0, 0, 0.37]}>
        <octahedronGeometry args={[0.06]} />
        <meshPhysicalMaterial
          color="#ffffff"
          emissive="#aaffff"
          emissiveIntensity={0.6}
          metalness={0.1}
          roughness={0}
          transmission={0.9}
          thickness={0.5}
          ior={2.4}
        />
      </mesh>
      {/* Back diamond */}
      <mesh position={[0, 0, -0.37]}>
        <octahedronGeometry args={[0.05]} />
        <meshPhysicalMaterial
          color="#ffffff"
          emissive="#aaffff"
          emissiveIntensity={0.5}
          metalness={0.1}
          roughness={0}
          transmission={0.9}
          thickness={0.5}
          ior={2.4}
        />
      </mesh>
      {/* Side diamonds */}
      <mesh position={[0.37, 0, 0]}>
        <octahedronGeometry args={[0.05]} />
        <meshPhysicalMaterial
          color="#ffffff"
          emissive="#aaffff"
          emissiveIntensity={0.5}
          metalness={0.1}
          roughness={0}
          transmission={0.9}
          thickness={0.5}
          ior={2.4}
        />
      </mesh>
      <mesh position={[-0.37, 0, 0]}>
        <octahedronGeometry args={[0.05]} />
        <meshPhysicalMaterial
          color="#ffffff"
          emissive="#aaffff"
          emissiveIntensity={0.5}
          metalness={0.1}
          roughness={0}
          transmission={0.9}
          thickness={0.5}
          ior={2.4}
        />
      </mesh>
    </group>
  );
}
