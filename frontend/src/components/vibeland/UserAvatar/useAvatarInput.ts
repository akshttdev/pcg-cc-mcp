import { useEffect, useRef } from 'react';
import type * as THREE from 'three';

export type MovementKeys = {
  forward: boolean;
  backward: boolean;
  left: boolean;
  right: boolean;
  up: boolean;
  down: boolean;
  sprint: boolean;
};

const FLIGHT_DOUBLE_TAP_WINDOW_MS = 400;
const JUMP_STRENGTH = 0.125;

interface UseAvatarInputOptions {
  gl: THREE.WebGLRenderer;
  canFly: boolean;
  isSuspended: boolean;
  onInteract?: () => void;
}

export function useAvatarInput({
  gl,
  canFly,
  isSuspended,
  onInteract,
}: UseAvatarInputOptions) {
  const keysRef = useRef<MovementKeys>({
    forward: false,
    backward: false,
    left: false,
    right: false,
    up: false,
    down: false,
    sprint: false,
  });

  const cameraAngleTargetRef = useRef(Math.PI / 4);
  const isDraggingRef = useRef(false);
  const lastMouseXRef = useRef(0);
  const isGroundedRef = useRef(true);
  const flightModeRef = useRef(false);
  const lastSpaceTapRef = useRef(0);
  const velocityRef = useRef<THREE.Vector3 | null>(null);

  // Always-current refs
  const isSuspendedRef = useRef(isSuspended);
  useEffect(() => {
    isSuspendedRef.current = isSuspended;
  }, [isSuspended]);

  const onInteractRef = useRef(onInteract);
  useEffect(() => {
    onInteractRef.current = onInteract;
  }, [onInteract]);

  // Mouse controls
  useEffect(() => {
    const canvas = gl.domElement;

    const handleMouseDown = (e: MouseEvent) => {
      if (e.button === 0 || e.button === 2) {
        isDraggingRef.current = true;
        lastMouseXRef.current = e.clientX;
        canvas.style.cursor = 'grabbing';
      }
    };

    const handleMouseUp = () => {
      isDraggingRef.current = false;
      canvas.style.cursor = 'grab';
    };

    const handleMouseMove = (e: MouseEvent) => {
      if (isDraggingRef.current && !isSuspendedRef.current) {
        const deltaX = e.clientX - lastMouseXRef.current;
        cameraAngleTargetRef.current -= deltaX * 0.004;
        lastMouseXRef.current = e.clientX;
      }
    };

    const handleContextMenu = (e: MouseEvent) => e.preventDefault();

    canvas.style.cursor = 'grab';
    canvas.addEventListener('mousedown', handleMouseDown);
    canvas.addEventListener('mouseup', handleMouseUp);
    canvas.addEventListener('mousemove', handleMouseMove);
    canvas.addEventListener('mouseleave', handleMouseUp);
    canvas.addEventListener('contextmenu', handleContextMenu);

    return () => {
      canvas.removeEventListener('mousedown', handleMouseDown);
      canvas.removeEventListener('mouseup', handleMouseUp);
      canvas.removeEventListener('mousemove', handleMouseMove);
      canvas.removeEventListener('mouseleave', handleMouseUp);
      canvas.removeEventListener('contextmenu', handleContextMenu);
    };
  }, [gl]);

  // Keyboard controls
  useEffect(() => {
    const keys = keysRef.current;

    const handleKeyDown = (e: KeyboardEvent) => {
      const key = e.key.toLowerCase();
      if (['w','a','s','d','arrowup','arrowdown','arrowleft','arrowright',' '].includes(key)) {
        // intentionally empty — key handled by state updates below
      }
      if (key === 'e') {
        if (!isSuspendedRef.current && onInteractRef.current) onInteractRef.current();
        return;
      }

      if (isSuspendedRef.current) return;

      switch (key) {
        case 'w':
        case 'arrowup':
          keys.forward = true;
          break;
        case 's':
        case 'arrowdown':
          keys.backward = true;
          break;
        case 'a':
        case 'arrowleft':
          keys.left = true;
          break;
        case 'd':
        case 'arrowright':
          keys.right = true;
          break;
        case ' ':
          e.preventDefault();
          if (canFly) {
            const now = performance.now();
            if (now - lastSpaceTapRef.current < FLIGHT_DOUBLE_TAP_WINDOW_MS) {
              flightModeRef.current = true;
            }
            lastSpaceTapRef.current = now;
            keys.up = true;
            if (velocityRef.current && isGroundedRef.current && velocityRef.current.y <= 0.01) {
              velocityRef.current.y = JUMP_STRENGTH;
              isGroundedRef.current = false;
            }
          } else if (velocityRef.current && isGroundedRef.current) {
            velocityRef.current.y = JUMP_STRENGTH;
            isGroundedRef.current = false;
          }
          break;
        case 'control':
        case 'q':
          keys.down = true;
          break;
        case 'shift':
          keys.sprint = true;
          break;
      }
    };

    const handleKeyUp = (e: KeyboardEvent) => {
      if (isSuspendedRef.current) return;
      switch (e.key.toLowerCase()) {
        case 'w':
        case 'arrowup':
          keys.forward = false;
          break;
        case 's':
        case 'arrowdown':
          keys.backward = false;
          break;
        case 'a':
        case 'arrowleft':
          keys.left = false;
          break;
        case 'd':
        case 'arrowright':
          keys.right = false;
          break;
        case ' ':
          keys.up = false;
          flightModeRef.current = false;
          break;
        case 'control':
        case 'q':
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
  }, [canFly]);

  // Reset on suspend change
  useEffect(() => {
    const keys = keysRef.current;
    keys.forward = keys.backward = keys.left = keys.right = keys.up = keys.down = keys.sprint = false;
    if (isSuspended && velocityRef.current) {
      velocityRef.current.set(0, 0, 0);
      flightModeRef.current = false;
    }
  }, [isSuspended]);

  return {
    keysRef,
    cameraAngleTargetRef,
    isGroundedRef,
    flightModeRef,
    velocityRef,
    isSuspendedRef,
  };
}
