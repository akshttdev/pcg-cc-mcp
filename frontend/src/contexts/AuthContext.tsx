// Auth Context for managing user authentication state
import { createContext, useContext, useState, useEffect, ReactNode } from 'react';
import { UserProfile, login as apiLogin, logout as apiLogout, getCurrentUser } from '../lib/auth-api';
import { useEquipmentStore } from '../stores/useEquipmentStore';

interface AuthContextType {
  user: UserProfile | null;
  isLoading: boolean;
  isAuthenticated: boolean;
  login: (username: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
  refreshUser: () => Promise<void>;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<UserProfile | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const initializeEquipment = useEquipmentStore((state) => state.initializeForUser);
  const resetEquipment = useEquipmentStore((state) => state.resetEquipment);

  // Check for existing session on mount
  useEffect(() => {
    checkSession();
  }, []);

  // Initialize equipment when user changes
  useEffect(() => {
    if (user) {
      console.log('[AuthContext] User changed, initializing equipment:', {
        userId: user.id,
        username: user.username,
        is_admin: user.is_admin,
        typeof_is_admin: typeof user.is_admin
      });
      initializeEquipment(user.id, user.is_admin);
    }
  }, [user, initializeEquipment]);

  async function checkSession() {
    try {
      setIsLoading(true);
      const currentUser = await getCurrentUser();
      setUser(currentUser);
    } catch (error) {
      console.error('Failed to check session:', error);
      setUser(null);
    } finally {
      setIsLoading(false);
    }
  }

  async function login(username: string, password: string) {
    try {
      const userProfile = await apiLogin({ username, password });
      setUser(userProfile);
      // Equipment will be initialized by the useEffect
    } catch (error) {
      console.error('Login failed:', error);
      throw error;
    }
  }

  async function logout() {
    try {
      await apiLogout();
      resetEquipment(); // Clear equipment on logout
      setUser(null);
    } catch (error) {
      console.error('Logout failed:', error);
      // Even if logout API fails, clear local state
      resetEquipment();
      setUser(null);
    }
  }

  async function refreshUser() {
    await checkSession();
  }

  const value: AuthContextType = {
    user,
    isLoading,
    isAuthenticated: user !== null,
    login,
    logout,
    refreshUser,
  };

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (context === undefined) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
}
