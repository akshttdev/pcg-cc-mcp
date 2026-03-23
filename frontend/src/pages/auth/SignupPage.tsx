import { useState, useEffect } from 'react';
import { useSearchParams, useNavigate, Link } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Building2 } from 'lucide-react';
import { authApi } from '@/lib/api';

export function SignupPage() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const inviteToken = searchParams.get('invite') ?? '';

  const [orgName, setOrgName] = useState<string | null>(null);
  const [prefillEmail, setPrefillEmail] = useState('');
  const [tokenError, setTokenError] = useState('');

  const [fullName, setFullName] = useState('');
  const [username, setUsername] = useState('');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [error, setError] = useState('');
  const [isLoading, setIsLoading] = useState(false);

  // Fetch invite info on mount
  useEffect(() => {
    if (!inviteToken) {
      setTokenError('No invite token provided. Please use a valid invite link.');
      return;
    }
    authApi.getInviteInfo(inviteToken).then((info) => {
      setOrgName(info.org_name);
      if (info.pending_owner_email) {
        setEmail(info.pending_owner_email);
        setPrefillEmail(info.pending_owner_email);
      }
    }).catch(() => {
      setTokenError('This invite link is invalid or has already been used.');
    });
  }, [inviteToken]);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError('');

    if (password !== confirmPassword) {
      setError('Passwords do not match.');
      return;
    }
    if (password.length < 8) {
      setError('Password must be at least 8 characters.');
      return;
    }

    setIsLoading(true);
    try {
      await authApi.register({
        username,
        password,
        full_name: fullName,
        email: email || undefined,
        invite_token: inviteToken,
      });
      // Redirect to dashboard; AuthContext will pick up the session cookie
      navigate('/');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Registration failed');
    } finally {
      setIsLoading(false);
    }
  }

  if (tokenError) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-gray-50 px-4">
        <Card className="w-full max-w-md">
          <CardHeader>
            <CardTitle>Invalid Invite</CardTitle>
          </CardHeader>
          <CardContent>
            <Alert variant="destructive">
              <AlertDescription>{tokenError}</AlertDescription>
            </Alert>
            <p className="mt-4 text-sm text-muted-foreground">
              Already have an account?{' '}
              <Link to="/login" className="underline">
                Sign in
              </Link>
            </p>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="flex min-h-screen items-center justify-center bg-gray-50 px-4">
      <Card className="w-full max-w-md">
        <CardHeader className="space-y-1">
          {orgName && (
            <div className="flex items-center gap-2 mb-2">
              <div className="p-1.5 bg-blue-100 rounded-lg">
                <Building2 className="h-5 w-5 text-blue-600" />
              </div>
              <span className="text-sm font-medium text-blue-700">{orgName}</span>
            </div>
          )}
          <CardTitle className="text-2xl font-semibold">
            {orgName ? `Join ${orgName}` : 'Create your account'}
          </CardTitle>
          <CardDescription>
            {orgName
              ? `You've been invited to join ${orgName} on the Power Club Global platform.`
              : 'Loading invite information…'}
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit} className="space-y-4">
            {error && (
              <Alert variant="destructive">
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}

            <div className="space-y-2">
              <Label htmlFor="full_name">Full Name</Label>
              <Input
                id="full_name"
                type="text"
                placeholder="Jane Smith"
                value={fullName}
                onChange={(e) => setFullName(e.target.value)}
                required
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="username">Username</Label>
              <Input
                id="username"
                type="text"
                placeholder="janesmith"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                required
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="email">Email</Label>
              <Input
                id="email"
                type="email"
                placeholder="jane@example.com"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                readOnly={!!prefillEmail}
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="password">Password</Label>
              <Input
                id="password"
                type="password"
                placeholder="Min. 8 characters"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                required
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="confirm_password">Confirm Password</Label>
              <Input
                id="confirm_password"
                type="password"
                placeholder="Repeat your password"
                value={confirmPassword}
                onChange={(e) => setConfirmPassword(e.target.value)}
                required
              />
            </div>

            <Button type="submit" className="w-full" disabled={isLoading || !orgName}>
              {isLoading ? 'Creating account…' : 'Create Account'}
            </Button>

            <p className="text-center text-sm text-muted-foreground">
              Already have an account?{' '}
              <Link to="/login" className="underline">
                Sign in
              </Link>
            </p>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
