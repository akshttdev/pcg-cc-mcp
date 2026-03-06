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

  // Check for existing session on mount
  useEffect(() => {
    checkSession();
  }, []);

  // Initialize equipment when user changes
  useEffect(() => {
    if (user) {
      initializeEquipment(user.id, user.is_admin);
    }
  }, [user, initializeEquipment]);

  async function checkSession() {
    try {
      setIsLoading(true);
      const currentUser = await getCurrentUser();
      setUser(currentUser);
    } catch (error: any) {
      console.error('Failed to check session:', error);
      setUser(null);
      // Show session expired message if we had a stored session
      const hadSession = document.cookie.includes('session_id') || localStorage.getItem('session_id');
      if (hadSession) {
        localStorage.removeItem('session_id');
        // Clear the expired cookie
        document.cookie = 'session_id=; Path=/; Max-Age=0';
      }
    } finally {
      setIsLoading(false);
    }
  }

  async function login(username: string, password: string) {
    try {
      const userProfile = await apiLogin({ username, password });
      setUser(userProfile);
      // Hard reload after login: wipes all cached React state so no
      // previous user's data (projects, tasks, etc.) bleeds through.
      window.location.href = '/';
    } catch (error) {
      console.error('Login failed:', error);
      throw error;
    }
  }

  async function logout() {
    try {
      await apiLogout();
      setUser(null);
    } catch (error) {
      console.error('Logout failed:', error);
      setUser(null);
    } finally {
      // Hard reload on logout: guarantees all cached user data is gone
      window.location.href = '/';
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
