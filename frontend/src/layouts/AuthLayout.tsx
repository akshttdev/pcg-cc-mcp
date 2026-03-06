// Clean layout for unauthenticated pages (login, etc.)
// No sidebar, navbar, or API-dependent providers
import { Outlet } from 'react-router-dom';
import { ThemeProvider } from '@/components/theme-provider';
import { AppWithStyleOverride } from '@/utils/style-override';
import { Toaster } from '@/components/ui/toaster';
import { ThemeMode } from 'shared/types';

export function AuthLayout() {
  return (
    <ThemeProvider initialTheme={ThemeMode.SYSTEM}>
      <AppWithStyleOverride>
        <Outlet />
      </AppWithStyleOverride>
      <Toaster />
    </ThemeProvider>
  );
}
