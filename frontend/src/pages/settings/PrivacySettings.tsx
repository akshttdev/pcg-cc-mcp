import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Shield, Eye, Lock, Database, Trash2, Download } from 'lucide-react';

export function PrivacySettings() {
  const [profileVisibility, setProfileVisibility] = useState<'public' | 'team' | 'private'>('team');
  const [showEmail, setShowEmail] = useState(false);
  const [showActivity, setShowActivity] = useState(true);
  const [allowAnalytics, setAllowAnalytics] = useState(true);
  const [marketingEmails, setMarketingEmails] = useState(false);
  const [twoFactorEnabled, setTwoFactorEnabled] = useState(false);
  const [sessionTimeout, setSessionTimeout] = useState('30');
  const [isSaving, setIsSaving] = useState(false);

  const handleSave = async () => {
    setIsSaving(true);
    try {
      await new Promise((resolve) => setTimeout(resolve, 500));
      alert('Privacy settings updated successfully!');
    } catch (error) {
      console.error('Failed to save privacy settings:', error);
      alert('Failed to update privacy settings');
    } finally {
      setIsSaving(false);
    }
  };

  const handleExportData = () => {
    alert('Data export will be sent to your email');
  };

  const handleDeleteAccount = () => {
    if (confirm('Are you sure you want to delete your account? This action cannot be undone.')) {
      alert('Account deletion requires additional verification. Check your email.');
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold">Privacy & Security</h1>
        <p className="text-muted-foreground mt-2">
          Manage your privacy preferences and security settings
        </p>
      </div>

      {/* Profile Visibility */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Eye className="h-5 w-5" />
            Profile Visibility
          </CardTitle>
          <CardDescription>Control who can see your profile information</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div>
            <Label htmlFor="visibility">Who can see your profile</Label>
            <Select value={profileVisibility} onValueChange={(v: string) => setProfileVisibility(v as 'public' | 'team' | 'private')}>
              <SelectTrigger className="mt-1.5">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="public">
                  <div>
                    <div className="font-medium">Public</div>
                    <div className="text-xs text-muted-foreground">Anyone can view your profile</div>
                  </div>
                </SelectItem>
                <SelectItem value="team">
                  <div>
                    <div className="font-medium">Team Only</div>
                    <div className="text-xs text-muted-foreground">Only team members can view</div>
                  </div>
                </SelectItem>
                <SelectItem value="private">
                  <div>
                    <div className="font-medium">Private</div>
                    <div className="text-xs text-muted-foreground">Only you can view</div>
                  </div>
                </SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="flex items-center justify-between py-2">
            <div>
              <Label htmlFor="show-email">Show email address</Label>
              <p className="text-sm text-muted-foreground">Display your email on your profile</p>
            </div>
            <Switch id="show-email" checked={showEmail} onCheckedChange={setShowEmail} />
          </div>

          <div className="flex items-center justify-between py-2">
            <div>
              <Label htmlFor="show-activity">Show activity status</Label>
              <p className="text-sm text-muted-foreground">Let others see when you're active</p>
            </div>
            <Switch id="show-activity" checked={showActivity} onCheckedChange={setShowActivity} />
          </div>
        </CardContent>
      </Card>

      {/* Security Settings */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Lock className="h-5 w-5" />
            Security
          </CardTitle>
          <CardDescription>Manage your account security</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between py-2">
            <div>
              <Label htmlFor="2fa">Two-factor authentication</Label>
              <p className="text-sm text-muted-foreground">Add an extra layer of security</p>
            </div>
            <Switch id="2fa" checked={twoFactorEnabled} onCheckedChange={setTwoFactorEnabled} />
          </div>

          <div>
            <Label htmlFor="session-timeout">Session timeout</Label>
            <Select value={sessionTimeout} onValueChange={setSessionTimeout}>
              <SelectTrigger className="mt-1.5">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="15">15 minutes</SelectItem>
                <SelectItem value="30">30 minutes</SelectItem>
                <SelectItem value="60">1 hour</SelectItem>
                <SelectItem value="240">4 hours</SelectItem>
                <SelectItem value="never">Never</SelectItem>
              </SelectContent>
            </Select>
            <p className="text-xs text-muted-foreground mt-1">
              Automatically log out after inactivity
            </p>
          </div>

          <div className="pt-2">
            <Button variant="outline" className="w-full">
              <Shield className="mr-2 h-4 w-4" />
              Change Password
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Data & Privacy */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Database className="h-5 w-5" />
            Data & Privacy
          </CardTitle>
          <CardDescription>Control how your data is used</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between py-2">
            <div>
              <Label htmlFor="analytics">Analytics & performance</Label>
              <p className="text-sm text-muted-foreground">
                Help us improve by sharing usage data
              </p>
            </div>
            <Switch id="analytics" checked={allowAnalytics} onCheckedChange={setAllowAnalytics} />
          </div>

          <div className="flex items-center justify-between py-2">
            <div>
              <Label htmlFor="marketing">Marketing emails</Label>
              <p className="text-sm text-muted-foreground">Receive updates and offers</p>
            </div>
            <Switch
              id="marketing"
              checked={marketingEmails}
              onCheckedChange={setMarketingEmails}
            />
          </div>

          <div className="pt-2 space-y-2">
            <Button variant="outline" className="w-full" onClick={handleExportData}>
              <Download className="mr-2 h-4 w-4" />
              Export My Data
            </Button>
            <p className="text-xs text-muted-foreground text-center">
              Download a copy of your data
            </p>
          </div>
        </CardContent>
      </Card>

      {/* Danger Zone */}
      <Card className="border-destructive">
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-destructive">
            <Trash2 className="h-5 w-5" />
            Danger Zone
          </CardTitle>
          <CardDescription>Irreversible actions</CardDescription>
        </CardHeader>
        <CardContent>
          <Button variant="destructive" onClick={handleDeleteAccount} className="w-full">
            Delete Account
          </Button>
          <p className="text-xs text-muted-foreground text-center mt-2">
            This action cannot be undone. All your data will be permanently deleted.
          </p>
        </CardContent>
      </Card>

      {/* Save Button */}
      <div className="flex justify-end gap-3">
        <Button variant="outline" onClick={() => window.history.back()}>
          Cancel
        </Button>
        <Button onClick={handleSave} disabled={isSaving}>
          {isSaving ? 'Saving...' : 'Save Changes'}
        </Button>
      </div>
    </div>
  );
}
