// External authentication API for federated identity (e.g., Jungleverse SSO)

export interface UserProfile {
  id: string;
  username: string;
  email: string;
  full_name: string;
  avatar_url: string | null;
  is_admin: boolean;
  organizations: Array<{
    id: string;
    name: string;
    slug: string;
    role: string;
  }>;
}

export interface ValidateTokenResponse {
  user: UserProfile;
  session_id: string;
}

/**
 * Validates an external JWT token and creates a PCG session
 * @param token - JWT token from external provider (e.g., Jungleverse)
 * @returns User profile if valid, null if invalid
 */
export async function validateExternalToken(token: string): Promise<UserProfile | null> {
  try {
    const response = await fetch('/api/auth/external/validate', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      credentials: 'include', // Important: include cookies for session
      body: JSON.stringify({ token }),
    });

    if (!response.ok) {
      console.error('External auth validation failed:', response.status);
      return null;
    }

    const result = await response.json();

    if (result.data?.user) {
      return result.data.user as UserProfile;
    }

    return null;
  } catch (error) {
    console.error('External auth validation error:', error);
    return null;
  }
}
