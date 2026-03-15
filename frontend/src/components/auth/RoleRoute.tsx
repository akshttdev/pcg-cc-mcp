import { Navigate } from 'react-router-dom';
import { useAuth } from '../../contexts/AuthContext';
import { useEffectiveRole, type EffectiveRole } from '@/hooks/useEffectiveRole';
import { hasMinRole } from '@/lib/roles';
import { Loader } from '../ui/loader';

interface RoleRouteProps {
  children: React.ReactNode;
  /** Minimum role required (inclusive). User must have this role or higher. */
  minRole?: EffectiveRole;
  /** Explicit list of allowed roles. If provided, minRole is ignored. */
  allowedRoles?: EffectiveRole[];
  /** Where to redirect if access is denied. Defaults to "/" */
  fallback?: string;
}

/**
 * Route guard that checks effective role before rendering children.
 * Uses the same role computation as the sidebar, ensuring consistency
 * between what users see in navigation and what they can access via URL.
 */
export function RoleRoute({ children, minRole, allowedRoles, fallback = '/' }: RoleRouteProps) {
  const { isLoading, isAuthenticated } = useAuth();
  const { role } = useEffectiveRole();

  if (isLoading) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <Loader message="Loading..." size={32} />
      </div>
    );
  }

  if (!isAuthenticated) {
    return <Navigate to="/login" replace />;
  }

  // Check access
  const hasAccess = allowedRoles
    ? allowedRoles.includes(role)
    : minRole
      ? hasMinRole(role, minRole)
      : true;

  if (!hasAccess) {
    return <Navigate to={fallback} replace />;
  }

  return <>{children}</>;
}
