import { Toaster as Sonner } from 'sonner';
import { useTheme } from '@/components/theme-provider';
import { ThemeMode } from 'shared/types';

export function Toaster() {
  const { theme } = useTheme();

  const resolvedTheme = (() => {
    if (theme === ThemeMode.DARK) return 'dark';
    if (theme === ThemeMode.LIGHT) return 'light';
    if (typeof window !== 'undefined') {
      return window.matchMedia('(prefers-color-scheme: dark)').matches
        ? 'dark'
        : 'light';
    }
    return 'light';
  })();

  return (
    <Sonner
      theme={resolvedTheme}
      position="top-right"
      visibleToasts={4}
      duration={5000}
      gap={8}
      expand
      toastOptions={{
        classNames: {
          toast:
            'group toast group-[.toaster]:bg-background group-[.toaster]:text-foreground group-[.toaster]:border-border group-[.toaster]:shadow-lg',
          description: 'group-[.toast]:text-muted-foreground',
          actionButton:
            'group-[.toast]:bg-primary group-[.toast]:text-primary-foreground',
          cancelButton:
            'group-[.toast]:bg-muted group-[.toast]:text-muted-foreground',
        },
      }}
    />
  );
}
