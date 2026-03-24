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
import { FormField } from '@/components/ui/form-field';
import { ThemeMode, UiLanguage } from 'shared/types';
import { toPrettyCase } from '@/utils/string';
import type { TextSize } from '@/components/theme-provider';

interface AppearanceSectionProps {
  t: (key: string, opts?: Record<string, unknown>) => string;
  draft: { theme?: ThemeMode; language?: UiLanguage } | null;
  updateDraft: (patch: Record<string, unknown>) => void;
  languageOptions: Array<{ value: string; label: string }>;
  textSize: TextSize;
  setTextSize: (size: TextSize) => void;
}

export function AppearanceSection({
  t,
  draft,
  updateDraft,
  languageOptions,
  textSize,
  setTextSize,
}: AppearanceSectionProps) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('settings.general.appearance.title')}</CardTitle>
        <CardDescription>
          {t('settings.general.appearance.description')}
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <FormField
          label={t('settings.general.appearance.theme.label')}
          htmlFor="theme"
          description={t('settings.general.appearance.theme.helper')}
        >
          <Select
            value={draft?.theme}
            onValueChange={(value: ThemeMode) =>
              updateDraft({ theme: value })
            }
          >
            <SelectTrigger id="theme">
              <SelectValue
                placeholder={t(
                  'settings.general.appearance.theme.placeholder'
                )}
              />
            </SelectTrigger>
            <SelectContent>
              {Object.values(ThemeMode).map((theme) => (
                <SelectItem key={theme} value={theme}>
                  {toPrettyCase(theme)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </FormField>

        <FormField
          label={t('settings.general.appearance.language.label')}
          htmlFor="language"
          description={t('settings.general.appearance.language.helper')}
        >
          <Select
            value={draft?.language}
            onValueChange={(value: UiLanguage) =>
              updateDraft({ language: value })
            }
          >
            <SelectTrigger id="language">
              <SelectValue
                placeholder={t(
                  'settings.general.appearance.language.placeholder'
                )}
              />
            </SelectTrigger>
            <SelectContent>
              {languageOptions.map((option) => (
                <SelectItem key={option.value} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </FormField>

        <FormField
          label="Text Size"
          htmlFor="text-size"
          description="Adjust the base text size across the application. All UI elements scale proportionally."
        >
          <Select
            value={textSize}
            onValueChange={(value: string) => setTextSize(value as TextSize)}
          >
            <SelectTrigger id="text-size">
              <SelectValue placeholder="Select text size" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="small">Small (14px)</SelectItem>
              <SelectItem value="default">Default (16px)</SelectItem>
              <SelectItem value="large">Large (18px)</SelectItem>
              <SelectItem value="extra-large">Extra Large (20px)</SelectItem>
            </SelectContent>
          </Select>
        </FormField>
      </CardContent>
    </Card>
  );
}
