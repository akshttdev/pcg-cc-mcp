import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { FormField } from '@/components/ui/form-field';
import { Checkbox } from '@/components/ui/checkbox';
import { Volume2 } from 'lucide-react';
import { SoundFile } from 'shared/types';
import { toPrettyCase } from '@/utils/string';

interface NotificationsSectionProps {
  t: (key: string) => string;
  draft: {
    notifications: {
      sound_enabled: boolean;
      sound_file: SoundFile;
      push_enabled: boolean;
    };
  } | null;
  updateDraft: (patch: Record<string, unknown>) => void;
}

export function NotificationsSection({
  t,
  draft,
  updateDraft,
}: NotificationsSectionProps) {
  const playSound = async (soundFile: SoundFile) => {
    const audio = new Audio(`/api/sounds/${soundFile}`);
    try {
      await audio.play();
    } catch (err) {
      console.error('Failed to play sound:', err);
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('settings.general.notifications.title')}</CardTitle>
        <CardDescription>
          {t('settings.general.notifications.description')}
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex items-center space-x-2">
          <Checkbox
            id="sound-enabled"
            checked={draft?.notifications.sound_enabled}
            onCheckedChange={(checked: boolean) =>
              updateDraft({
                notifications: {
                  ...draft!.notifications,
                  sound_enabled: checked,
                },
              })
            }
          />
          <div className="space-y-0.5">
            <Label htmlFor="sound-enabled" className="cursor-pointer">
              {t('settings.general.notifications.sound.label')}
            </Label>
            <p className="text-sm text-muted-foreground">
              {t('settings.general.notifications.sound.helper')}
            </p>
          </div>
        </div>
        {draft?.notifications.sound_enabled && (
          <FormField
            label={t('settings.general.notifications.sound.fileLabel')}
            htmlFor="sound-file"
            description={t('settings.general.notifications.sound.fileHelper')}
            className="ml-6"
          >
            <div className="flex gap-2">
              <Select
                value={draft.notifications.sound_file}
                onValueChange={(value: SoundFile) =>
                  updateDraft({
                    notifications: {
                      ...draft.notifications,
                      sound_file: value,
                    },
                  })
                }
              >
                <SelectTrigger id="sound-file" className="flex-1">
                  <SelectValue
                    placeholder={t(
                      'settings.general.notifications.sound.filePlaceholder'
                    )}
                  />
                </SelectTrigger>
                <SelectContent>
                  {Object.values(SoundFile).map((soundFile) => (
                    <SelectItem key={soundFile} value={soundFile}>
                      {toPrettyCase(soundFile)}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <Button
                variant="outline"
                size="sm"
                onClick={() => playSound(draft.notifications.sound_file)}
                className="px-3"
              >
                <Volume2 className="h-4 w-4" />
              </Button>
            </div>
          </FormField>
        )}
        <div className="flex items-center space-x-2">
          <Checkbox
            id="push-notifications"
            checked={draft?.notifications.push_enabled}
            onCheckedChange={(checked: boolean) =>
              updateDraft({
                notifications: {
                  ...draft!.notifications,
                  push_enabled: checked,
                },
              })
            }
          />
          <div className="space-y-0.5">
            <Label htmlFor="push-notifications" className="cursor-pointer">
              {t('settings.general.notifications.push.label')}
            </Label>
            <p className="text-sm text-muted-foreground">
              {t('settings.general.notifications.push.helper')}
            </p>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
