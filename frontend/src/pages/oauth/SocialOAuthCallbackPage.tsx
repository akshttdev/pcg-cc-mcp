import { useEffect, useMemo, useRef, useState } from 'react';
import { useNavigate, useParams, useSearchParams } from 'react-router-dom';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Loader } from '@/components/ui/loader';
import { AlertCircle, CheckCircle2, Share2, RefreshCw } from 'lucide-react';

const PLATFORM_INFO: Record<string, {
  name: string;
  headline: string;
  accent: string;
  badgeClass: string;
  icon?: string;
}> = {
  linkedin: {
    name: 'LinkedIn',
    headline: 'Connect your LinkedIn account to automate content distribution.',
    accent: 'text-[#0A66C2]',
    badgeClass: 'bg-[#0A66C2] text-white',
  },
  instagram: {
    name: 'Instagram',
    headline: 'Connect your Instagram Business account for social publishing.',
    accent: 'text-[#E4405F]',
    badgeClass: 'bg-gradient-to-r from-[#F58529] via-[#DD2A7B] to-[#8134AF] text-white',
  },
  twitter: {
    name: 'X (Twitter)',
    headline: 'Connect your X account for real-time social updates.',
    accent: 'text-black',
    badgeClass: 'bg-black text-white',
  },
  tiktok: {
    name: 'TikTok',
    headline: 'Connect your TikTok account for short-form video distribution.',
    accent: 'text-black',
    badgeClass: 'bg-black text-white',
  },
};

type Status = 'processing' | 'success' | 'error';

interface ApiResponse<T> {
  success: boolean;
  data?: T;
  message?: string;
}

interface SocialAccount {
  id: string;
  platform: string;
  username?: string;
  display_name?: string;
}

export function SocialOAuthCallbackPage() {
  const { platform: platformParam } = useParams<{ platform: string }>();
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const platformKey = (platformParam || '').toLowerCase();
  const platformInfo = PLATFORM_INFO[platformKey];
  const [status, setStatus] = useState<Status>('processing');
  const [statusMessage, setStatusMessage] = useState('Completing account connection...');
  const [statusDetail, setStatusDetail] = useState('Finalizing the OAuth handshake with the platform.');
  const [connectedAccount, setConnectedAccount] = useState<SocialAccount | null>(null);
  const redirectTimer = useRef<number>();

  const projectIdFromState = useMemo(() => {
    const stateParam = searchParams.get('state');
    if (!stateParam) return null;
    // State format: "project_id:platform:nonce"
    const [projectIdCandidate] = stateParam.split(':');
    return projectIdCandidate || null;
  }, [searchParams]);

  useEffect(() => {
    if (!platformInfo) {
      setStatus('error');
      setStatusMessage('Unsupported platform');
      setStatusDetail(`The platform "${platformKey}" is not currently supported for OAuth connections.`);
      return;
    }

    const providerError = searchParams.get('error') || searchParams.get('error_description');
    if (providerError) {
      setStatus('error');
      setStatusMessage('Platform declined authorization');
      setStatusDetail(providerError);
      return;
    }

    const code = searchParams.get('code');
    const state = searchParams.get('state');
    if (!code || !state) {
      setStatus('error');
      setStatusMessage('Missing OAuth parameters');
      setStatusDetail('We did not receive both the authorization code and state. Please restart the connection flow.');
      return;
    }

    let cancelled = false;

    const finalizeOAuth = async () => {
      try {
        const query = new URLSearchParams({ code, state });
        const response = await fetch(`/api/social/oauth/${platformKey}/callback?${query.toString()}`, {
          method: 'GET',
          credentials: 'include',
        });

        if (!response.ok) {
          const errorBody = await response.json().catch(() => ({}));
          throw new Error(errorBody.message || `Server responded with ${response.status}.`);
        }

        const result: ApiResponse<SocialAccount> = await response.json();
        if (!result.success) {
          throw new Error(result.message || 'Unable to complete account connection.');
        }

        if (cancelled) {
          return;
        }

        setConnectedAccount(result.data ?? null);
        setStatus('success');
        setStatusMessage('Account connected!');
        setStatusDetail('Your social account has been linked successfully.');

        // Redirect after a short delay
        if (projectIdFromState) {
          redirectTimer.current = window.setTimeout(() => {
            const params = new URLSearchParams();
            params.set('projectId', projectIdFromState);
            params.set('tab', 'social');
            navigate(`/crm?${params.toString()}`, { replace: true });
          }, 2000);
        }
      } catch (error) {
        if (cancelled) {
          return;
        }
        setStatus('error');
        setStatusMessage('Unable to complete authorization');
        setStatusDetail(
          error instanceof Error
            ? error.message
            : 'An unknown error occurred while contacting the PCG API.'
        );
      }
    };

    finalizeOAuth();

    return () => {
      cancelled = true;
      if (redirectTimer.current) {
        window.clearTimeout(redirectTimer.current);
      }
    };
  }, [platformInfo, platformKey, projectIdFromState, searchParams, navigate]);

  const handleBackToCrm = () => {
    if (projectIdFromState) {
      const params = new URLSearchParams();
      params.set('projectId', projectIdFromState);
      params.set('tab', 'social');
      navigate(`/crm?${params.toString()}`);
    } else {
      navigate('/crm');
    }
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-950 via-slate-900 to-slate-950 text-white flex items-center justify-center p-4">
      <Card className="w-full max-w-2xl bg-slate-900 border-slate-800 text-white shadow-2xl">
        <CardHeader>
          <div className="flex items-center gap-3">
            <div className="h-12 w-12 rounded-full bg-white/10 flex items-center justify-center">
              <Share2 className="h-6 w-6" />
            </div>
            <div>
              <CardTitle className="text-2xl">Connect Social Account</CardTitle>
              <CardDescription className="text-slate-300">
                {platformInfo ? platformInfo.headline : 'Complete the OAuth flow.'}
              </CardDescription>
            </div>
          </div>
        </CardHeader>
        <CardContent className="space-y-6">
          {platformInfo && (
            <Badge className={platformInfo.badgeClass}>{platformInfo.name}</Badge>
          )}

          <div className="rounded-2xl border border-white/5 bg-white/5 p-6">
            {status === 'processing' && (
              <div className="flex flex-col items-center text-center gap-3">
                <Loader message="Finishing OAuth..." size={32} />
                <p className="text-base font-medium">{statusMessage}</p>
                <p className="text-sm text-slate-300">{statusDetail}</p>
              </div>
            )}

            {status === 'success' && (
              <div className="flex flex-col items-center text-center gap-2">
                <CheckCircle2 className="h-12 w-12 text-emerald-400" />
                <p className="text-xl font-semibold">{statusMessage}</p>
                <p className="text-sm text-slate-300">{statusDetail}</p>
                {connectedAccount && (
                  <div className="mt-2 text-sm text-white/90">
                    <p>Platform: <span className="font-medium">{connectedAccount.platform}</span></p>
                    {connectedAccount.display_name && (
                      <p>Account: <span className="font-medium">{connectedAccount.display_name}</span></p>
                    )}
                    {connectedAccount.username && (
                      <p>Username: <span className="font-mono">@{connectedAccount.username}</span></p>
                    )}
                  </div>
                )}
              </div>
            )}

            {status === 'error' && (
              <div className="flex flex-col items-center text-center gap-2">
                <AlertCircle className="h-12 w-12 text-red-400" />
                <p className="text-xl font-semibold">{statusMessage}</p>
                <p className="text-sm text-red-200">{statusDetail}</p>
              </div>
            )}
          </div>

          <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
            <div className="text-xs uppercase tracking-wide text-slate-400">
              Project:{' '}
              {projectIdFromState ? (
                <span className="text-white font-mono">{projectIdFromState.slice(0, 8)}...</span>
              ) : (
                <span className="text-slate-500">Not detected</span>
              )}
            </div>
            <div className="flex gap-2">
              <Button variant="secondary" onClick={() => navigate('/')}
                className="bg-white/10 text-white hover:bg-white/20">
                <RefreshCw className="mr-2 h-4 w-4" />
                Dashboard
              </Button>
              <Button onClick={handleBackToCrm}>
                Return to CRM
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

export default SocialOAuthCallbackPage;
