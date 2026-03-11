import React, { createContext, useContext, useEffect, useState } from 'react';
import { ThemeMode } from 'shared/types';

export type TextSize = 'small' | 'default' | 'large' | 'extra-large';

const TEXT_SIZE_PX: Record<TextSize, number> = {
  small: 14,
  default: 16,
  large: 18,
  'extra-large': 20,
};

const TEXT_SIZE_KEY = 'orcha-text-size';

function getStoredTextSize(): TextSize {
  const stored = localStorage.getItem(TEXT_SIZE_KEY);
  if (stored && stored in TEXT_SIZE_PX) return stored as TextSize;
  return 'default';
}

type ThemeProviderProps = {
  children: React.ReactNode;
  initialTheme?: ThemeMode;
};

type ThemeProviderState = {
  theme: ThemeMode;
  setTheme: (theme: ThemeMode) => void;
  textSize: TextSize;
  setTextSize: (size: TextSize) => void;
};

const initialState: ThemeProviderState = {
  theme: ThemeMode.SYSTEM,
  setTheme: () => null,
  textSize: 'default',
  setTextSize: () => null,
};

const ThemeProviderContext = createContext<ThemeProviderState>(initialState);

export function ThemeProvider({
  children,
  initialTheme = ThemeMode.SYSTEM,
  ...props
}: ThemeProviderProps) {
  const [theme, setThemeState] = useState<ThemeMode>(initialTheme);
  const [textSize, setTextSizeState] = useState<TextSize>(getStoredTextSize);

  // Update theme when initialTheme changes
  useEffect(() => {
    setThemeState(initialTheme);
  }, [initialTheme]);

  useEffect(() => {
    const root = window.document.documentElement;

    root.classList.remove('light', 'dark');

    if (theme === ThemeMode.SYSTEM) {
      const systemTheme = window.matchMedia('(prefers-color-scheme: dark)')
        .matches
        ? 'dark'
        : 'light';

      root.classList.add(systemTheme);
      return;
    }

    root.classList.add(theme.toLowerCase());
  }, [theme]);

  // Apply text size to root element
  useEffect(() => {
    const root = window.document.documentElement;
    root.style.fontSize = `${TEXT_SIZE_PX[textSize]}px`;
  }, [textSize]);

  const setTheme = (newTheme: ThemeMode) => {
    setThemeState(newTheme);
  };

  const setTextSize = (size: TextSize) => {
    localStorage.setItem(TEXT_SIZE_KEY, size);
    setTextSizeState(size);
  };

  const value = {
    theme,
    setTheme,
    textSize,
    setTextSize,
  };

  return (
    <ThemeProviderContext.Provider {...props} value={value}>
      {children}
    </ThemeProviderContext.Provider>
  );
}

export const useTheme = () => {
  const context = useContext(ThemeProviderContext);

  if (context === undefined)
    throw new Error('useTheme must be used within a ThemeProvider');

  return context;
};
