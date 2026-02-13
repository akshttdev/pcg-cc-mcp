// Auth API functions

// Types for auth (matching Rust backend)
export interface LoginRequest {
  username: string;
  password: string;
}

export interface UserOrganization {
  id: string;
  name: string;
  slug: string;
  role: 'admin' | 'member' | 'viewer';
}

export interface UserProfile {
  id: string;
  username: string;
  email: string;
  full_name: string;
  avatar_url: string | null;
  is_admin: boolean;
  organizations: UserOrganization[];
}

export interface LoginResponse {
  user: UserProfile;
  session_id: string;
}

export interface LogoutResponse {
  message: string;
}

export interface ApiResponse<T> {
  data: T;
  error?: string;
}

/**
 * Login with username and password
 * Session cookie is automatically set by the server
 */
export async function login(credentials: LoginRequest): Promise<UserProfile> {
  const response = await fetch('/api/auth/login', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    credentials: 'include', // Important: allows cookies to be sent/received
    body: JSON.stringify(credentials),
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: 'Login failed' }));
    throw new Error(error.error || 'Login failed');
  }

  const result: ApiResponse<LoginResponse> = await response.json();

  // Store session_id in localStorage for Bearer token authentication
  // (fallback for when cookies don't work due to browser privacy settings)
  if (result.data.session_id) {
    localStorage.setItem('session_id', result.data.session_id);
  }

  return result.data.user;
}

/**
 * Get current user from session
 * Session cookie is automatically sent
 */
export async function getCurrentUser(): Promise<UserProfile | null> {
  try {
    const response = await fetch('/api/auth/me', {
      method: 'GET',
      credentials: 'include', // Important: sends session cookie
    });

    if (!response.ok) {
      return null;
    }

    const result: ApiResponse<UserProfile> = await response.json();
    return result.data;
  } catch (error) {
    console.error('Failed to get current user:', error);
    return null;
  }
}

/**
 * Logout - clears session cookie and deletes session from DB
 */
export async function logout(): Promise<void> {
  try {
    await fetch('/api/auth/logout', {
      method: 'POST',
      credentials: 'include', // Important: sends session cookie to be deleted
    });
  } catch (error) {
    console.error('Logout failed:', error);
  } finally {
    // Clear session_id from localStorage
    localStorage.removeItem('session_id');
    sessionStorage.removeItem('session_id');
  }
}
