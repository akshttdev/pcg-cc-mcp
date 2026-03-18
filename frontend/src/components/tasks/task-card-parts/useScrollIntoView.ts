import { useEffect, useRef } from 'react';

/**
 * Returns a ref that, when `isActive` becomes true, smoothly scrolls the element into view.
 */
export function useScrollIntoView<T extends HTMLElement = HTMLDivElement>(isActive?: boolean) {
  const ref = useRef<T>(null);

  useEffect(() => {
    if (!isActive || !ref.current) return;
    const el = ref.current;
    requestAnimationFrame(() => {
      el.scrollIntoView({
        block: 'center',
        inline: 'nearest',
        behavior: 'smooth',
      });
    });
  }, [isActive]);

  return ref;
}
