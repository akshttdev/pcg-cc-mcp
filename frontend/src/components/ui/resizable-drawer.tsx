import { useCallback, useEffect, useRef, useState } from 'react';
import { cn } from '@/lib/utils';
import { useViewStore } from '@/stores/useViewStore';

const SIDEBAR_COLLAPSED_WIDTH = 56;
const SIDEBAR_EXPANDED_WIDTH = 288;
const DEFAULT_MIN_WIDTH = 400;
const EDGE_GAP = 10;

export interface DrawerRenderProps {
  isExpanded: boolean;
  toggleExpand: () => void;
}

interface ResizableDrawerProps {
  open: boolean;
  onClose: () => void;
  children: React.ReactNode | ((props: DrawerRenderProps) => React.ReactNode);
  defaultWidth?: number;
  minWidth?: number;
  storageKey?: string;
  className?: string;
}

export function ResizableDrawer({
  open,
  onClose,
  children,
  defaultWidth = 600,
  minWidth = DEFAULT_MIN_WIDTH,
  storageKey = 'orcha:drawer-width',
  className,
}: ResizableDrawerProps) {
  const { sidebarCollapsed } = useViewStore();
  const isDraggingRef = useRef(false);
  const [isExpanded, setIsExpanded] = useState(false);
  const preExpandWidthRef = useRef<number | null>(null);

  const getMaxWidth = useCallback(() => {
    const sidebarWidth = sidebarCollapsed ? SIDEBAR_COLLAPSED_WIDTH : SIDEBAR_EXPANDED_WIDTH;
    return window.innerWidth - sidebarWidth - EDGE_GAP;
  }, [sidebarCollapsed]);

  const [width, setWidth] = useState(() => {
    try {
      const saved = localStorage.getItem(storageKey);
      return saved ? Math.max(minWidth, parseInt(saved, 10)) : defaultWidth;
    } catch {
      return defaultWidth;
    }
  });

  // Persist width
  useEffect(() => {
    try {
      localStorage.setItem(storageKey, String(width));
    } catch {}
  }, [width, storageKey]);

  // Clamp when sidebar state changes
  useEffect(() => {
    const maxWidth = getMaxWidth();
    if (isExpanded) {
      setWidth(maxWidth);
    } else if (width > maxWidth) {
      setWidth(Math.max(minWidth, maxWidth));
    }
  }, [sidebarCollapsed, width, minWidth, isExpanded, getMaxWidth]);

  const toggleExpand = useCallback(() => {
    setIsExpanded((prev) => {
      if (!prev) {
        // Expanding — save current width, go to max
        preExpandWidthRef.current = width;
        setWidth(getMaxWidth());
        return true;
      } else {
        // Collapsing — restore saved width
        setWidth(preExpandWidthRef.current ?? defaultWidth);
        preExpandWidthRef.current = null;
        return false;
      }
    });
  }, [width, defaultWidth, getMaxWidth]);

  const handleDragStart = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      isDraggingRef.current = true;

      const sidebarWidth = sidebarCollapsed ? SIDEBAR_COLLAPSED_WIDTH : SIDEBAR_EXPANDED_WIDTH;
      const maxWidth = window.innerWidth - sidebarWidth - EDGE_GAP;

      const handleMouseMove = (moveEvent: MouseEvent) => {
        if (!isDraggingRef.current) return;
        setIsExpanded(false);
        preExpandWidthRef.current = null;
        const newWidth = Math.max(
          minWidth,
          Math.min(maxWidth, window.innerWidth - moveEvent.clientX)
        );
        setWidth(newWidth);
      };

      const handleMouseUp = () => {
        isDraggingRef.current = false;
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mouseup', handleMouseUp);
        document.body.style.cursor = '';
        document.body.style.userSelect = '';
      };

      document.body.style.cursor = 'col-resize';
      document.body.style.userSelect = 'none';
      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
    },
    [sidebarCollapsed, minWidth]
  );

  // Close drawer when clicking outside it
  const drawerRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const handleMouseDown = (e: MouseEvent) => {
      if (isDraggingRef.current) return;
      if (drawerRef.current && !drawerRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    document.addEventListener('mousedown', handleMouseDown);
    return () => document.removeEventListener('mousedown', handleMouseDown);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <>
      {/* Drawer */}
      <div
        ref={drawerRef}
        className={cn(
          'absolute inset-y-0 right-0 z-50 animate-in slide-in-from-right duration-300 shadow-xl',
          className
        )}
        style={{ width }}
      >
        {/* Resize handle — wider hit area, grippy dots indicator */}
        <div
          onMouseDown={handleDragStart}
          className="absolute inset-y-0 -left-2 w-5 cursor-col-resize z-10 group flex items-center justify-center"
        >
          {/* Hover highlight stripe */}
          <div className="absolute inset-y-0 left-2 w-[3px] rounded-full transition-colors bg-transparent group-hover:bg-primary/30 group-active:bg-primary/50" />
          {/* Grip indicator — 3 rows of 2 dots */}
          <div className="relative flex flex-col items-center gap-[3px] rounded-md px-[5px] py-2 bg-muted/80 border border-border/60 shadow-sm opacity-0 group-hover:opacity-100 group-active:opacity-100 transition-opacity">
            {[0, 1, 2].map((i) => (
              <div key={i} className="flex gap-[3px]">
                <div className="w-[3px] h-[3px] rounded-full bg-muted-foreground/50" />
                <div className="w-[3px] h-[3px] rounded-full bg-muted-foreground/50" />
              </div>
            ))}
          </div>
        </div>
        {typeof children === 'function'
          ? children({ isExpanded, toggleExpand })
          : children}
      </div>
    </>
  );
}
