import { useState, useEffect } from 'react';

/**
 * Hook to detect mobile devices and screen sizes
 */
export function useMobile() {
  const [isMobile, setIsMobile] = useState(false);
  const [isTablet, setIsTablet] = useState(false);
  const [isTauri, setIsTauri] = useState(false);
  const [platform, setPlatform] = useState<'ios' | 'android' | 'desktop' | 'web'>('web');

  useEffect(() => {
    // Check if running in Tauri
    const tauriCheck = typeof window !== 'undefined' && '__TAURI__' in window;
    setIsTauri(tauriCheck);

    // Detect platform
    const userAgent = navigator.userAgent.toLowerCase();
    if (/iphone|ipad|ipod/.test(userAgent)) {
      setPlatform('ios');
    } else if (/android/.test(userAgent)) {
      setPlatform('android');
    } else if (tauriCheck) {
      setPlatform('desktop');
    } else {
      setPlatform('web');
    }

    // Check screen size
    const checkSize = () => {
      const width = window.innerWidth;
      setIsMobile(width < 768);
      setIsTablet(width >= 768 && width < 1024);
    };

    checkSize();
    window.addEventListener('resize', checkSize);
    return () => window.removeEventListener('resize', checkSize);
  }, []);

  return {
    isMobile,
    isTablet,
    isDesktop: !isMobile && !isTablet,
    isTauri,
    platform,
    isMobileApp: isTauri && (platform === 'ios' || platform === 'android'),
  };
}

export default useMobile;
