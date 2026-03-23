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
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Button } from '@/components/ui/button';
import { FormField } from '@/components/ui/form-field';
import { ChevronDown } from 'lucide-react';
import { BaseCodingAgent, ExecutorConfig, ExecutorProfileId } from 'shared/types';

interface TaskExecutionSectionProps {
  t: (key: string) => string;
  draft: { executor_profile?: ExecutorProfileId } | null;
  updateDraft: (patch: Record<string, unknown>) => void;
  profiles: Record<string, ExecutorConfig> | null;
}

export function TaskExecutionSection({
  t,
  draft,
  updateDraft,
  profiles,
}: TaskExecutionSectionProps) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('settings.general.taskExecution.title')}</CardTitle>
        <CardDescription>
          {t('settings.general.taskExecution.description')}
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <FormField
          label={t('settings.general.taskExecution.executor.label')}
          htmlFor="executor"
          description={t('settings.general.taskExecution.executor.helper')}
        >
          <div className="grid grid-cols-2 gap-2">
            <Select
              value={draft?.executor_profile?.executor ?? ''}
              onValueChange={(value: string) => {
                const variants = profiles?.[value];
                const keepCurrentVariant =
                  variants &&
                  draft?.executor_profile?.variant &&
                  variants[draft.executor_profile.variant];

                const newProfile: ExecutorProfileId = {
                  executor: value as BaseCodingAgent,
                  variant: keepCurrentVariant
                    ? draft!.executor_profile!.variant
                    : null,
                };
                updateDraft({
                  executor_profile: newProfile,
                });
              }}
              disabled={!profiles}
            >
              <SelectTrigger id="executor">
                <SelectValue
                  placeholder={t(
                    'settings.general.taskExecution.executor.placeholder'
                  )}
                />
              </SelectTrigger>
              <SelectContent>
                {profiles &&
                  Object.entries(profiles)
                    .sort((a, b) => a[0].localeCompare(b[0]))
                    .map(([profileKey]) => (
                      <SelectItem key={profileKey} value={profileKey}>
                        {profileKey}
                      </SelectItem>
                    ))}
              </SelectContent>
            </Select>

            {/* Show variant selector if selected profile has variants */}
            {(() => {
              const currentProfileVariant = draft?.executor_profile;
              const selectedProfile =
                profiles?.[currentProfileVariant?.executor || ''];
              const hasVariants =
                selectedProfile && Object.keys(selectedProfile).length > 0;

              if (hasVariants) {
                return (
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button
                        variant="outline"
                        className="w-full h-10 px-2 flex items-center justify-between"
                      >
                        <span className="text-sm truncate flex-1 text-left">
                          {currentProfileVariant?.variant || 'DEFAULT'}
                        </span>
                        <ChevronDown className="h-4 w-4 ml-1 flex-shrink-0" />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent>
                      {Object.entries(selectedProfile).map(
                        ([variantLabel]) => (
                          <DropdownMenuItem
                            key={variantLabel}
                            onClick={() => {
                              const newProfile: ExecutorProfileId = {
                                executor: currentProfileVariant!.executor,
                                variant: variantLabel,
                              };
                              updateDraft({
                                executor_profile: newProfile,
                              });
                            }}
                            className={
                              currentProfileVariant?.variant === variantLabel
                                ? 'bg-accent'
                                : ''
                            }
                          >
                            {variantLabel}
                          </DropdownMenuItem>
                        )
                      )}
                    </DropdownMenuContent>
                  </DropdownMenu>
                );
              } else if (selectedProfile) {
                return (
                  <Button
                    variant="outline"
                    className="w-full h-10 px-2 flex items-center justify-between"
                    disabled
                  >
                    <span className="text-sm truncate flex-1 text-left">
                      {t('settings.general.taskExecution.defaultLabel')}
                    </span>
                  </Button>
                );
              }
              return null;
            })()}
          </div>
        </FormField>
      </CardContent>
    </Card>
  );
}
