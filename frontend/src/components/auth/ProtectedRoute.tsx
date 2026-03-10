// Protected route component
import { ReactNode } from 'react';
import { Navigate } from 'react-router-dom';
import { useAuth } from '../../contexts/AuthContext';
import { Loader } from '../ui/loader';

interface ProtectedRouteProps {
  children: ReactNode;
  adminOnly?: boolean;
}

export function ProtectedRoute({ children, adminOnly }: ProtectedRouteProps) {
  const { isAuthenticated, isLoading, user } = useAuth();

  if (isLoading) {
    return (
      <div className="flex min-h-screen items-center justify-center">
        <Loader />
      </div>
    );
  }

  if (!isAuthenticated) {
    // Check if there was a prior session (cookie or localStorage) to show expired message
    const hadSession = document.cookie.includes('session_id') || localStorage.getItem('session_id');
    const target = hadSession ? '/login?expired=1' : '/login';
    return <Navigate to={target} replace />;
  }

  if (adminOnly && !user?.is_admin) {
    return <Navigate to="/" replace />;
  }

  return <>{children}</>;
}
