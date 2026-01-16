import { useEffect, useMemo, useRef, useState } from 'react';
import { useNavigate, useParams, useSearchParams } from 'react-router-dom';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Loader } from '@/components/ui/loader';
import { AlertCircle, CheckCircle2, Mail, RefreshCw } from 'lucide-react';

const PROVIDER_INFO = {
  gmail: {
    name: 'Gmail',
    headline: 'Using Gmail as master credentials unlocks unified sign-ins for social platforms.',
    accent: 'text-[#EA4335]',
    badgeClass: 'bg-[#EA4335] text-white',
  },
  zoho: {
    name: 'Zoho Mail + CRM',
    headline: 'Zoho keeps internal operations and CRM sync in lock-step.',
    accent: 'text-[#C8202B]',
    badgeClass: 'bg-[#C8202B] text-white',
  },
};

type SupportedProvider = keyof typeof PROVIDER_INFO;

type Status = 'processing' | 'success' | 'error';

interface ApiResponse<T> {
  success: boolean;
  data?: T;
  message?: string;
}

interface EmailAccountPayload {
  email_address: string;
  provider: string;
}

export function OAuthCallbackPage() {
  const { provider: providerParam } = useParams<{ provider: string }>();
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const providerKey = (providerParam || '').toLowerCase() as SupportedProvider;
  const providerInfo = PROVIDER_INFO[providerKey];
  const [status, setStatus] = useState<Status>('processing');
  const [statusMessage, setStatusMessage] = useState('Completing account connection…');
  const [statusDetail, setStatusDetail] = useState('We are finalizing the OAuth handshake.');
  const [connectedEmail, setConnectedEmail] = useState<string | null>(null);
  const redirectTimer = useRef<number>();

  const projectIdFromState = useMemo(() => {
    const stateParam = searchParams.get('state');
    if (!stateParam) return null;
    const [projectIdCandidate] = stateParam.split(':');
    return projectIdCandidate || null;
  }, [searchParams]);

  useEffect(() => {
    if (!providerInfo) {
      setStatus('error');
      setStatusMessage('Unsupported provider');
      setStatusDetail('This OAuth callback route only supports Gmail and Zoho at the moment.');
      return;
    }

    const providerError = searchParams.get('error') || searchParams.get('error_description');
    if (providerError) {
      setStatus('error');
      setStatusMessage('Provider declined authorization');
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
        const response = await fetch(`/api/email/oauth/${providerKey}/callback?${query.toString()}`, {
          method: 'GET',
          credentials: 'include',
        });

        if (!response.ok) {
          const errorBody = await response.json().catch(() => ({}));
          throw new Error(errorBody.message || `Server responded with ${response.status}.`);
        }

        const result: ApiResponse<EmailAccountPayload> = await response.json();
        if (!result.success) {
          throw new Error(result.message || 'Unable to complete account connection.');
        }

        if (cancelled) {
          return;
        }

        setConnectedEmail(result.data?.email_address ?? null);
        setStatus('success');
        setStatusMessage('Account connected');
        setStatusDetail('Redirecting you back to CRM…');

        if (projectIdFromState) {
          redirectTimer.current = window.setTimeout(() => {
            const params = new URLSearchParams();
            params.set('projectId', projectIdFromState);
            params.set('tab', 'email');
            navigate(`/crm?${params.toString()}`, { replace: true });
          }, 1800);
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
  }, [providerInfo, providerKey, projectIdFromState, searchParams, navigate]);

  const handleBackToCrm = () => {
    if (projectIdFromState) {
      const params = new URLSearchParams();
      params.set('projectId', projectIdFromState);
      params.set('tab', 'email');
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
              <Mail className="h-6 w-6" />
            </div>
            <div>
              <CardTitle className="text-2xl">Finalize Email Connection</CardTitle>
              <CardDescription className="text-slate-300">
                {providerInfo ? providerInfo.headline : 'Complete the OAuth flow.'}
              </CardDescription>
            </div>
          </div>
        </CardHeader>
        <CardContent className="space-y-6">
          {providerInfo && (
            <Badge className={providerInfo.badgeClass}>{providerInfo.name}</Badge>
          )}

          <div className="rounded-2xl border border-white/5 bg-white/5 p-6">
            {status === 'processing' && (
              <div className="flex flex-col items-center text-center gap-3">
                <Loader message="Finishing OAuth…" size={32} />
                <p className="text-base font-medium">{statusMessage}</p>
                <p className="text-sm text-slate-300">{statusDetail}</p>
              </div>
            )}

            {status === 'success' && (
              <div className="flex flex-col items-center text-center gap-2">
                <CheckCircle2 className="h-12 w-12 text-emerald-400" />
                <p className="text-xl font-semibold">{statusMessage}</p>
                <p className="text-sm text-slate-300">{statusDetail}</p>
                {connectedEmail && (
                  <p className="text-sm text-white/90">Connected account: {connectedEmail}</p>
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
                <span className="text-white font-mono">{projectIdFromState}</span>
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

export default OAuthCallbackPage;
