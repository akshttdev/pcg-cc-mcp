import { useEffect, useRef } from 'react';
import { useGLTF } from '@react-three/drei';
import * as THREE from 'three';

interface EnvironmentModelProps {
  url: string;
  position?: [number, number, number];
  rotation?: [number, number, number];
  scale?: number | [number, number, number];
  onLoad?: () => void;
}

export function EnvironmentModel({
  url,
  position = [0, 0, 0],
  rotation = [0, 0, 0],
  scale = 1,
  onLoad,
}: EnvironmentModelProps) {
  const { scene } = useGLTF(url);
  const groupRef = useRef<THREE.Group>(null);

  useEffect(() => {
    if (scene) {
      // Enable shadows for all meshes in the model
      scene.traverse((child) => {
        if (child instanceof THREE.Mesh) {
          child.castShadow = true;
          child.receiveShadow = true;

          // Improve material quality
          if (child.material) {
            const materials = Array.isArray(child.material)
              ? child.material
              : [child.material];

            materials.forEach((mat) => {
              if (mat instanceof THREE.MeshStandardMaterial) {
                mat.envMapIntensity = 1;
                mat.needsUpdate = true;
              }
            });
          }
        }
      });

      onLoad?.();
    }
  }, [scene, onLoad]);

  const scaleArray: [number, number, number] = Array.isArray(scale)
    ? scale
    : [scale, scale, scale];

  return (
    <group ref={groupRef} position={position} rotation={rotation} scale={scaleArray}>
      <primitive object={scene} />
    </group>
  );
}

// Preload function for performance
export function preloadEnvironment(url: string) {
  useGLTF.preload(url);
}
