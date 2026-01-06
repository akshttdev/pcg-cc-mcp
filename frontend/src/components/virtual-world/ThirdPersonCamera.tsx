import { useRef, useEffect } from 'react';
import { useThree, useFrame } from '@react-three/fiber';
import * as THREE from 'three';

interface ThirdPersonCameraProps {
  target: THREE.Vector3;
  distance?: number;
  minDistance?: number;
  maxDistance?: number;
  height?: number;
  smoothness?: number;
  enabled?: boolean;
}

export function ThirdPersonCamera({
  target,
  distance = 15,
  minDistance = 5,
  maxDistance = 50,
  height = 8,
  smoothness = 0.08,
  enabled = true,
}: ThirdPersonCameraProps) {
  const { camera, gl } = useThree();
  const orbitAngle = useRef(Math.PI / 4); // Initial angle
  const orbitHeight = useRef(height);
  const currentDistance = useRef(distance);
  const isDragging = useRef(false);
  const lastMousePos = useRef({ x: 0, y: 0 });

  useEffect(() => {
    if (!enabled) return;

    const canvas = gl.domElement;

    const handleMouseDown = (e: MouseEvent) => {
      if (e.button === 2 || e.button === 0) { // Right click or left click for orbit
        isDragging.current = true;
        lastMousePos.current = { x: e.clientX, y: e.clientY };
      }
    };

    const handleMouseUp = () => {
      isDragging.current = false;
    };

    const handleMouseMove = (e: MouseEvent) => {
      if (!isDragging.current) return;

      const deltaX = e.clientX - lastMousePos.current.x;
      const deltaY = e.clientY - lastMousePos.current.y;

      // Horizontal rotation
      orbitAngle.current -= deltaX * 0.005;

      // Vertical rotation (clamped)
      orbitHeight.current = Math.max(
        2,
        Math.min(25, orbitHeight.current + deltaY * 0.05)
      );

      lastMousePos.current = { x: e.clientX, y: e.clientY };
    };

    const handleWheel = (e: WheelEvent) => {
      e.preventDefault();
      currentDistance.current = Math.max(
        minDistance,
        Math.min(maxDistance, currentDistance.current + e.deltaY * 0.02)
      );
    };

    const handleContextMenu = (e: MouseEvent) => {
      e.preventDefault(); // Prevent right-click menu
    };

    canvas.addEventListener('mousedown', handleMouseDown);
    canvas.addEventListener('mouseup', handleMouseUp);
    canvas.addEventListener('mouseleave', handleMouseUp);
    canvas.addEventListener('mousemove', handleMouseMove);
    canvas.addEventListener('wheel', handleWheel, { passive: false });
    canvas.addEventListener('contextmenu', handleContextMenu);

    return () => {
      canvas.removeEventListener('mousedown', handleMouseDown);
      canvas.removeEventListener('mouseup', handleMouseUp);
      canvas.removeEventListener('mouseleave', handleMouseUp);
      canvas.removeEventListener('mousemove', handleMouseMove);
      canvas.removeEventListener('wheel', handleWheel);
      canvas.removeEventListener('contextmenu', handleContextMenu);
    };
  }, [gl, enabled, minDistance, maxDistance]);

  useFrame(() => {
    if (!enabled) return;

    // Calculate desired camera position
    const dist = currentDistance.current;
    const h = orbitHeight.current;
    const angle = orbitAngle.current;

    const desiredPosition = new THREE.Vector3(
      target.x + Math.sin(angle) * dist,
      target.y + h,
      target.z + Math.cos(angle) * dist
    );

    // Smoothly interpolate camera position
    camera.position.lerp(desiredPosition, smoothness);

    // Look at target (slightly above ground for better view)
    const lookTarget = target.clone();
    lookTarget.y += 2; // Look at avatar's head level
    camera.lookAt(lookTarget);
  });

  return null;
}

// Hook to get the current orbit angle (useful for movement direction)
export function useOrbitAngle() {
  const angleRef = useRef(Math.PI / 4);
  return angleRef;
}
