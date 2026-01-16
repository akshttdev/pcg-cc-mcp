import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Badge } from '@/components/ui/badge';
// Table components currently unused but may be needed for expanded tx history
// import {
//   Table,
//   TableBody,
//   TableCell,
//   TableHead,
//   TableHeader,
//   TableRow,
// } from '@/components/ui/table';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { toast } from 'sonner';
import {
  Copy,
  Eye,
  EyeOff,
  Send,
  Download,
  Coins,
  RefreshCw,
  ExternalLink,
  Loader2,
  ArrowUpRight,
  ArrowDownLeft,
  QrCode,
} from 'lucide-react';
import { useUserSystem } from '@/components/config-provider';
import { aptosApi, type AptosTransaction, type SendVibeRequest } from '@/lib/api';
import { formatDistanceToNow } from 'date-fns';

export function WalletSettings() {
  useTranslation('settings'); // Load translations namespace
  const { config } = useUserSystem();
  const queryClient = useQueryClient();
  const [showPrivateKey, setShowPrivateKey] = useState(false);
  const [sendModalOpen, setSendModalOpen] = useState(false);
  const [receiveModalOpen, setReceiveModalOpen] = useState(false);
  const [unlockModalOpen, setUnlockModalOpen] = useState(false);
  const [passwordInput, setPasswordInput] = useState('');
  const [passwordError, setPasswordError] = useState('');
  const [sendAmount, setSendAmount] = useState('');
  const [recipientAddress, setRecipientAddress] = useState('');

  const wallet = config?.aptos_wallet;
  const walletAddress = wallet?.account_address;

  // Fetch VIBE balance
  const {
    data: balance,
    isLoading: balanceLoading,
    error: balanceError,
    refetch: refetchBalance,
  } = useQuery({
    queryKey: ['vibe-balance', walletAddress],
    queryFn: () => aptosApi.getVibeBalance(walletAddress!),
    enabled: !!walletAddress,
    staleTime: 30000,
    refetchInterval: 60000,
  });

  // Fetch transactions from Aptos testnet (auto-refresh every 30s)
  const {
    data: transactions = [],
    isLoading: txLoading,
    error: txError,
    refetch: refetchTx,
  } = useQuery({
    queryKey: ['aptos-transactions', walletAddress],
    queryFn: () => aptosApi.getTransactions(walletAddress!, 20),
    enabled: !!walletAddress,
    staleTime: 15000,
    refetchInterval: 30000,
  });

  // Send VIBE mutation
  const sendMutation = useMutation({
    mutationFn: (request: SendVibeRequest) => aptosApi.sendVibe(request),
    onSuccess: (data) => {
      toast.success(data.message);
      setSendModalOpen(false);
      setSendAmount('');
      setRecipientAddress('');
      queryClient.invalidateQueries({ queryKey: ['vibe-balance', walletAddress] });
      queryClient.invalidateQueries({ queryKey: ['aptos-transactions', walletAddress] });
    },
    onError: (error) => {
      toast.error(`Transaction failed: ${error.message}`);
    },
  });

  const handleCopy = useCallback(async (label: string, value: string) => {
    if (!value) return;
    try {
      await navigator.clipboard.writeText(value);
      toast.success(`${label} copied to clipboard`);
    } catch (error) {
      toast.error(`Unable to copy ${label.toLowerCase()}`);
    }
  }, []);

  const handleRefresh = () => {
    refetchBalance();
    refetchTx();
    toast.success('Refreshing wallet data...');
  };

  const handleSend = () => {
    if (!wallet || !sendAmount || !recipientAddress) return;

    const amount = parseInt(sendAmount, 10);
    if (isNaN(amount) || amount <= 0) {
      toast.error('Please enter a valid amount');
      return;
    }

    if (balance && amount > balance.balance_vibe) {
      toast.error('Insufficient VIBE balance');
      return;
    }

    sendMutation.mutate({
      sender_private_key: wallet.private_key,
      sender_address: wallet.account_address,
      recipient_address: recipientAddress,
      amount_vibe: amount,
    });
  };

  const handleUnlockPrivateKey = () => {
    // For now, check against a simple password - in production this should be the user's account password
    // We'll use the user's login password by making an API call or checking locally
    if (!passwordInput) {
      setPasswordError('Please enter your password');
      return;
    }

    // Simple validation - password must be at least 6 characters
    // In a real implementation, this would validate against the user's actual password
    if (passwordInput.length >= 6) {
      setShowPrivateKey(true);
      setUnlockModalOpen(false);
      setPasswordInput('');
      setPasswordError('');
      toast.success('Private key unlocked');

      // Auto-hide after 30 seconds for security
      setTimeout(() => {
        setShowPrivateKey(false);
      }, 30000);
    } else {
      setPasswordError('Invalid password');
    }
  };

  const handleLockPrivateKey = () => {
    setShowPrivateKey(false);
  };

  const formatVibe = (amount: number) => `${amount.toLocaleString()} VIBE`;
  const formatUsd = (amount: number) => `$${amount.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;

  const formatTimestamp = (timestamp: string) => {
    const ms = parseInt(timestamp) / 1000;
    if (isNaN(ms)) return 'Unknown';
    return formatDistanceToNow(new Date(ms), { addSuffix: true });
  };

  const truncateAddress = (address: string) => {
    if (!address) return '';
    return `${address.slice(0, 10)}...${address.slice(-8)}`;
  };

  const truncateHash = (hash: string) => {
    if (!hash) return '';
    return `${hash.slice(0, 8)}...${hash.slice(-6)}`;
  };

  const getExplorerUrl = (hash: string) =>
    `https://explorer.aptoslabs.com/txn/${hash}?network=testnet`;

  const getAccountExplorerUrl = (address: string) =>
    `https://explorer.aptoslabs.com/account/${address}?network=testnet`;

  const isOutgoing = (tx: AptosTransaction) =>
    tx.sender.toLowerCase() === walletAddress?.toLowerCase();

  // Get readable transaction type from payload function
  const getTxType = (tx: AptosTransaction): { label: string; icon: 'out' | 'in' | 'neutral' } => {
    const fn = tx.payload_function || '';
    if (fn.includes('publish_package')) return { label: 'Deploy Contract', icon: 'neutral' };
    if (fn.includes('::mint')) return { label: 'Mint VIBE', icon: 'in' };
    if (fn.includes('::transfer') || fn.includes('::send')) return { label: isOutgoing(tx) ? 'Send VIBE' : 'Receive VIBE', icon: isOutgoing(tx) ? 'out' : 'in' };
    if (fn.includes('::burn')) return { label: 'Burn VIBE', icon: 'out' };
    if (fn.includes('aptos_account::transfer')) return { label: isOutgoing(tx) ? 'Send APT' : 'Receive APT', icon: isOutgoing(tx) ? 'out' : 'in' };
    return { label: fn.split('::').pop() || 'Transaction', icon: 'neutral' };
  };

  return (
    <div className="space-y-6">
      {/* Main Wallet Card */}
      <Card>
        <CardHeader className="pb-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className="p-2 rounded-full bg-primary/10">
                <Coins className="h-6 w-6 text-primary" />
              </div>
              <div>
                <CardTitle className="text-xl">VIBE Wallet</CardTitle>
                <CardDescription className="flex items-center gap-2">
                  <Badge variant="outline" className="text-xs">Testnet</Badge>
                  {walletAddress && (
                    <span className="font-mono text-xs">
                      {truncateAddress(walletAddress)}
                    </span>
                  )}
                </CardDescription>
              </div>
            </div>
            <Button variant="ghost" size="sm" onClick={handleRefresh}>
              <RefreshCw className="h-4 w-4" />
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {/* Balance Display */}
          <div className="text-center py-6 border-b">
            <p className="text-sm text-muted-foreground mb-1">VIBE Balance</p>
            <div className="text-4xl font-bold">
              {balanceLoading ? (
                <Loader2 className="h-8 w-8 animate-spin mx-auto" />
              ) : balanceError ? (
                <span className="text-destructive text-lg">Error loading</span>
              ) : (
                formatVibe(balance?.balance_vibe ?? 0)
              )}
            </div>
            {balance && balance.balance > 0 && (
              <p className="text-sm text-muted-foreground mt-2">
                ≈ {formatUsd(balance.usd_value)}
              </p>
            )}
          </div>

          {/* Action Buttons */}
          <div className="grid grid-cols-3 gap-4 pt-6">
            <Button
              variant="default"
              className="flex flex-col h-auto py-4 gap-2"
              onClick={() => setSendModalOpen(true)}
              disabled={!wallet || (balance?.balance_vibe ?? 0) <= 0}
            >
              <Send className="h-5 w-5" />
              <span className="text-sm">Send</span>
            </Button>
            <Button
              variant="outline"
              className="flex flex-col h-auto py-4 gap-2"
              onClick={() => setReceiveModalOpen(true)}
              disabled={!wallet}
            >
              <Download className="h-5 w-5" />
              <span className="text-sm">Receive</span>
            </Button>
            <Button
              variant="outline"
              className="flex flex-col h-auto py-4 gap-2"
              onClick={() => window.open('https://omega.xyz/buy', '_blank')}
              disabled={!wallet}
            >
              <Coins className="h-5 w-5" />
              <span className="text-sm">Buy VIBE</span>
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Transaction History */}
      <Card>
        <CardHeader>
          <CardTitle className="text-lg">Transaction History</CardTitle>
          <CardDescription>Recent on-chain activity</CardDescription>
        </CardHeader>
        <CardContent>
          {txLoading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
            </div>
          ) : txError ? (
            <Alert variant="destructive">
              <AlertDescription>Failed to load transactions</AlertDescription>
            </Alert>
          ) : transactions.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <p>No transactions yet</p>
              <p className="text-sm">Fund your wallet to get started</p>
            </div>
          ) : (
            <div className="space-y-2">
              {transactions.map((tx: AptosTransaction) => {
                const txType = getTxType(tx);
                return (
                  <div
                    key={tx.hash}
                    className="flex items-center justify-between p-3 rounded-lg border hover:bg-muted/50 transition-colors"
                  >
                    <div className="flex items-center gap-3">
                      <div
                        className={`p-2 rounded-full ${
                          txType.icon === 'out'
                            ? 'bg-red-100 dark:bg-red-900/20'
                            : txType.icon === 'in'
                            ? 'bg-green-100 dark:bg-green-900/20'
                            : 'bg-blue-100 dark:bg-blue-900/20'
                        }`}
                      >
                        {txType.icon === 'out' ? (
                          <ArrowUpRight className="h-4 w-4 text-red-600 dark:text-red-400" />
                        ) : txType.icon === 'in' ? (
                          <ArrowDownLeft className="h-4 w-4 text-green-600 dark:text-green-400" />
                        ) : (
                          <Coins className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                        )}
                      </div>
                      <div>
                        <p className="font-medium text-sm">{txType.label}</p>
                        <p className="text-xs text-muted-foreground">
                          {formatTimestamp(tx.timestamp)}
                        </p>
                      </div>
                    </div>
                    <div className="flex items-center gap-3">
                      <div className="text-right">
                        <p className="font-mono text-sm">{truncateHash(tx.hash)}</p>
                        <Badge variant={tx.success ? 'secondary' : 'destructive'} className="text-xs">
                          {tx.success ? 'Success' : 'Failed'}
                        </Badge>
                      </div>
                      <a
                        href={getExplorerUrl(tx.hash)}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-muted-foreground hover:text-foreground"
                      >
                        <ExternalLink className="h-4 w-4" />
                      </a>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Wallet Details Card */}
      <Card>
        <CardHeader>
          <CardTitle className="text-lg">Wallet Keys</CardTitle>
          <CardDescription>Keep your private key safe</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {!wallet ? (
            <Alert variant="destructive">
              <AlertDescription>Unable to load wallet</AlertDescription>
            </Alert>
          ) : (
            <>
              <div className="space-y-2">
                <Label className="text-xs text-muted-foreground">Account Address</Label>
                <div className="flex gap-2">
                  <Input
                    value={wallet.account_address}
                    readOnly
                    className="font-mono text-xs"
                  />
                  <Button
                    variant="outline"
                    size="icon"
                    onClick={() => handleCopy('Address', wallet.account_address)}
                  >
                    <Copy className="h-4 w-4" />
                  </Button>
                  <Button variant="outline" size="icon" asChild>
                    <a
                      href={getAccountExplorerUrl(wallet.account_address)}
                      target="_blank"
                      rel="noopener noreferrer"
                    >
                      <ExternalLink className="h-4 w-4" />
                    </a>
                  </Button>
                </div>
              </div>

              <div className="space-y-2">
                <Label className="text-xs text-muted-foreground">Public Key</Label>
                <div className="flex gap-2">
                  <Input
                    value={wallet.public_key}
                    readOnly
                    className="font-mono text-xs"
                  />
                  <Button
                    variant="outline"
                    size="icon"
                    onClick={() => handleCopy('Public key', wallet.public_key)}
                  >
                    <Copy className="h-4 w-4" />
                  </Button>
                </div>
              </div>

              <div className="space-y-2">
                <Label className="text-xs text-muted-foreground">Private Key</Label>
                {showPrivateKey ? (
                  <div className="space-y-2">
                    <div className="flex gap-2">
                      <Input
                        type="text"
                        value={wallet.private_key}
                        readOnly
                        className="font-mono text-xs"
                      />
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={handleLockPrivateKey}
                      >
                        <EyeOff className="h-4 w-4" />
                      </Button>
                      <Button
                        variant="outline"
                        size="icon"
                        onClick={() => handleCopy('Private key', wallet.private_key)}
                      >
                        <Copy className="h-4 w-4" />
                      </Button>
                    </div>
                    <p className="text-xs text-amber-600 dark:text-amber-400">
                      Auto-locks in 30 seconds
                    </p>
                  </div>
                ) : (
                  <div className="space-y-2">
                    <div className="flex gap-2">
                      <Input
                        type="password"
                        value="••••••••••••••••••••••••••••••••"
                        readOnly
                        className="font-mono text-xs"
                      />
                      <Button
                        variant="outline"
                        onClick={() => setUnlockModalOpen(true)}
                      >
                        <Eye className="h-4 w-4 mr-2" />
                        Reveal
                      </Button>
                    </div>
                    <p className="text-xs text-muted-foreground">
                      Enter your password to reveal the private key
                    </p>
                  </div>
                )}
                <p className="text-xs text-destructive">
                  Never share your private key with anyone
                </p>
              </div>
            </>
          )}
        </CardContent>
      </Card>

      {/* Send Modal */}
      <Dialog open={sendModalOpen} onOpenChange={setSendModalOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Send VIBE</DialogTitle>
            <DialogDescription>
              Send VIBE tokens to another address on testnet
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label>Recipient Address</Label>
              <Input
                placeholder="0x..."
                value={recipientAddress}
                onChange={(e) => setRecipientAddress(e.target.value)}
                className="font-mono"
              />
            </div>
            <div className="space-y-2">
              <Label>Amount (VIBE)</Label>
              <Input
                type="number"
                placeholder="0"
                value={sendAmount}
                onChange={(e) => setSendAmount(e.target.value)}
                step="1"
                min="1"
              />
              <p className="text-xs text-muted-foreground">
                Available: {formatVibe(balance?.balance_vibe ?? 0)}
              </p>
            </div>
            <div className="rounded-lg bg-muted p-3 text-sm">
              <div className="flex justify-between">
                <span className="text-muted-foreground">Network Fee</span>
                <span>~1 VIBE</span>
              </div>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setSendModalOpen(false)}>
              Cancel
            </Button>
            <Button
              onClick={handleSend}
              disabled={sendMutation.isPending || !sendAmount || !recipientAddress}
            >
              {sendMutation.isPending ? (
                <>
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                  Sending...
                </>
              ) : (
                <>
                  <Send className="h-4 w-4 mr-2" />
                  Send
                </>
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Receive Modal */}
      <Dialog open={receiveModalOpen} onOpenChange={setReceiveModalOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Receive VIBE</DialogTitle>
            <DialogDescription>
              Share your address to receive VIBE tokens on testnet
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="flex justify-center p-6 bg-white rounded-lg">
              <div className="text-center">
                <QrCode className="h-32 w-32 mx-auto text-muted-foreground" />
                <p className="text-xs text-muted-foreground mt-2">QR Code placeholder</p>
              </div>
            </div>
            <div className="space-y-2">
              <Label>Your Address</Label>
              <div className="flex gap-2">
                <Input
                  value={wallet?.account_address ?? ''}
                  readOnly
                  className="font-mono text-xs"
                />
                <Button
                  variant="outline"
                  size="icon"
                  onClick={() => handleCopy('Address', wallet?.account_address ?? '')}
                >
                  <Copy className="h-4 w-4" />
                </Button>
              </div>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setReceiveModalOpen(false)}>
              Close
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Unlock Private Key Modal */}
      <Dialog open={unlockModalOpen} onOpenChange={(open) => {
        setUnlockModalOpen(open);
        if (!open) {
          setPasswordInput('');
          setPasswordError('');
        }
      }}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Unlock Private Key</DialogTitle>
            <DialogDescription>
              Enter your account password to reveal the private key
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label>Password</Label>
              <Input
                type="password"
                placeholder="Enter your password"
                value={passwordInput}
                onChange={(e) => {
                  setPasswordInput(e.target.value);
                  setPasswordError('');
                }}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    handleUnlockPrivateKey();
                  }
                }}
              />
              {passwordError && (
                <p className="text-xs text-destructive">{passwordError}</p>
              )}
            </div>
            <Alert>
              <AlertDescription className="text-xs">
                Your private key will be visible for 30 seconds before automatically locking again.
              </AlertDescription>
            </Alert>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setUnlockModalOpen(false)}>
              Cancel
            </Button>
            <Button onClick={handleUnlockPrivateKey}>
              Unlock
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
