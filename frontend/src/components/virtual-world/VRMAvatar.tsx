import { useEffect, useRef, useState } from 'react';
import { useFrame } from '@react-three/fiber';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { VRM, VRMLoaderPlugin, VRMUtils } from '@pixiv/three-vrm';
import * as THREE from 'three';

interface VRMAvatarProps {
  url: string;
  position?: [number, number, number];
  rotation?: [number, number, number];
  scale?: number;
  animation?: 'idle' | 'walking' | 'running';
}

export function VRMAvatar({
  url,
  position = [0, 0, 0],
  rotation = [0, 0, 0],
  scale = 1,
  animation = 'idle',
}: VRMAvatarProps) {
  const [vrm, setVrm] = useState<VRM | null>(null);
  const groupRef = useRef<THREE.Group>(null);
  const mixerRef = useRef<THREE.AnimationMixer | null>(null);

  // Load VRM model
  useEffect(() => {
    const loader = new GLTFLoader();
    loader.register((parser) => new VRMLoaderPlugin(parser));

    loader.load(
      url,
      (gltf) => {
        const loadedVrm = gltf.userData.vrm as VRM;

        // Optimize the VRM
        VRMUtils.removeUnnecessaryVertices(gltf.scene);
        VRMUtils.removeUnnecessaryJoints(gltf.scene);

        // Rotate to face forward (VRM models face +Z by default)
        VRMUtils.rotateVRM0(loadedVrm);

        setVrm(loadedVrm);

        // Setup animation mixer
        mixerRef.current = new THREE.AnimationMixer(loadedVrm.scene);
      },
      (progress) => {
        console.log('Loading VRM:', (progress.loaded / progress.total) * 100, '%');
      },
      (error) => {
        console.error('Error loading VRM:', error);
      }
    );

    return () => {
      if (vrm) {
        VRMUtils.deepDispose(vrm.scene);
      }
    };
  }, [url]);

  // Animation loop
  useFrame((state, delta) => {
    if (!vrm) return;

    const time = state.clock.elapsedTime;

    // Update VRM
    vrm.update(delta);

    // Simple idle animation using expression and bone manipulation
    if (vrm.expressionManager) {
      // Blink occasionally
      const blinkCycle = Math.sin(time * 3) > 0.95;
      vrm.expressionManager.setValue('blink', blinkCycle ? 1 : 0);
    }

    // Subtle body sway for idle
    if (animation === 'idle' && vrm.humanoid) {
      const spine = vrm.humanoid.getNormalizedBoneNode('spine');
      if (spine) {
        spine.rotation.z = Math.sin(time * 0.5) * 0.02;
        spine.rotation.x = Math.sin(time * 0.3) * 0.01;
      }

      const head = vrm.humanoid.getNormalizedBoneNode('head');
      if (head) {
        head.rotation.y = Math.sin(time * 0.4) * 0.05;
        head.rotation.x = Math.sin(time * 0.6) * 0.02;
      }
    }

    // Update animation mixer
    if (mixerRef.current) {
      mixerRef.current.update(delta);
    }
  });

  if (!vrm) {
    // Loading placeholder
    return (
      <group ref={groupRef} position={position}>
        <mesh>
          <capsuleGeometry args={[0.3, 1, 8, 16]} />
          <meshStandardMaterial color="#00ffff" transparent opacity={0.5} />
        </mesh>
      </group>
    );
  }

  return (
    <group ref={groupRef} position={position} rotation={rotation} scale={[scale, scale, scale]}>
      <primitive object={vrm.scene} />
    </group>
  );
}

// Simpler avatar for the player character that works with ecctrl
export function VRMPlayerModel({ url }: { url: string }) {
  const [vrm, setVrm] = useState<VRM | null>(null);

  useEffect(() => {
    const loader = new GLTFLoader();
    loader.register((parser) => new VRMLoaderPlugin(parser));

    loader.load(url, (gltf) => {
      const loadedVrm = gltf.userData.vrm as VRM;
      VRMUtils.removeUnnecessaryVertices(gltf.scene);
      VRMUtils.removeUnnecessaryJoints(gltf.scene);
      VRMUtils.rotateVRM0(loadedVrm);
      setVrm(loadedVrm);
    });

    return () => {
      if (vrm) {
        VRMUtils.deepDispose(vrm.scene);
      }
    };
  }, [url]);

  useFrame((_, delta) => {
    vrm?.update(delta);
  });

  if (!vrm) {
    return (
      <mesh>
        <capsuleGeometry args={[0.3, 1, 8, 16]} />
        <meshStandardMaterial color="#ff8800" transparent opacity={0.8} />
      </mesh>
    );
  }

  return <primitive object={vrm.scene} />;
}
