import { useMemo } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { AlertCircle, CheckCircle2, AlertTriangle } from 'lucide-react';
import { cn } from '@/lib/utils';
import type {
  ContentBlock,
  SocialPlatform,
  PlatformAdaptation
} from '@/types/social';
import { PLATFORM_LIMITS, PLATFORM_ICONS } from '@/types/social';

interface PlatformPreviewProps {
  blocks: ContentBlock[];
  platforms: SocialPlatform[];
  className?: string;
}

function assembleContent(blocks: ContentBlock[]): string {
  const parts: string[] = [];

  // Order: hook -> body -> cta -> emoji -> hashtags
  const orderedTypes: ContentBlock['type'][] = ['hook', 'body', 'cta', 'emoji', 'hashtags'];

  for (const type of orderedTypes) {
    const block = blocks.find(b => b.type === type);
    if (block?.content) {
      if (type === 'hashtags') {
        parts.push('\n\n' + block.content);
      } else if (type === 'emoji') {
        // Emojis get appended inline
        parts.push(' ' + block.content);
      } else {
        parts.push(block.content);
      }
    }
  }

  return parts.join('\n\n').trim();
}

function countHashtags(content: string): number {
  const matches = content.match(/#\w+/g);
  return matches ? matches.length : 0;
}

function adaptForPlatform(
  content: string,
  platform: SocialPlatform
): PlatformAdaptation {
  const limits = PLATFORM_LIMITS[platform];
  const characterCount = content.length;
  const hashtagCount = countHashtags(content);

  const warnings: string[] = [];
  let isValid = true;

  // Check character limit
  if (characterCount > limits.characterLimit) {
    warnings.push(`Exceeds ${platform} character limit (${characterCount}/${limits.characterLimit})`);
    isValid = false;
  } else if (characterCount > limits.characterLimit * 0.9) {
    warnings.push(`Approaching character limit (${characterCount}/${limits.characterLimit})`);
  }

  // Check hashtag limit
  if (hashtagCount > limits.hashtagLimit) {
    warnings.push(`Too many hashtags (${hashtagCount}/${limits.hashtagLimit})`);
    isValid = false;
  }

  // Platform-specific warnings
  if (platform === 'twitter' && characterCount > 250) {
    warnings.push('Long tweets may get less engagement');
  }

  if (platform === 'linkedin' && hashtagCount > 5) {
    warnings.push('LinkedIn recommends 3-5 hashtags');
  }

  if (platform === 'instagram' && hashtagCount < 5) {
    warnings.push('Consider adding more hashtags for reach');
  }

  return {
    platform,
    content,
    characterCount,
    characterLimit: limits.characterLimit,
    hashtagCount,
    hashtagLimit: limits.hashtagLimit,
    isValid,
    warnings,
  };
}

function PlatformPreviewCard({
  adaptation,
  platform
}: {
  adaptation: PlatformAdaptation;
  platform: SocialPlatform;
}) {
  const charPercentage = (adaptation.characterCount / adaptation.characterLimit) * 100;

  return (
    <div className="space-y-4">
      {/* Status indicators */}
      <div className="flex items-center gap-2">
        {adaptation.isValid ? (
          <Badge variant="outline" className="bg-green-50 text-green-700 border-green-200">
            <CheckCircle2 className="h-3 w-3 mr-1" />
            Ready to post
          </Badge>
        ) : (
          <Badge variant="outline" className="bg-red-50 text-red-700 border-red-200">
            <AlertCircle className="h-3 w-3 mr-1" />
            Issues found
          </Badge>
        )}
        <Badge variant="secondary" className="font-mono">
          {adaptation.characterCount}/{adaptation.characterLimit}
        </Badge>
        <Badge variant="secondary" className="font-mono">
          #{adaptation.hashtagCount}/{adaptation.hashtagLimit}
        </Badge>
      </div>

      {/* Character limit bar */}
      <div className="space-y-1">
        <div className="h-2 bg-muted rounded-full overflow-hidden">
          <div
            className={cn(
              'h-full transition-all',
              charPercentage > 100 ? 'bg-red-500' :
              charPercentage > 90 ? 'bg-yellow-500' :
              'bg-green-500'
            )}
            style={{ width: `${Math.min(charPercentage, 100)}%` }}
          />
        </div>
      </div>

      {/* Warnings */}
      {adaptation.warnings.length > 0 && (
        <div className="space-y-1">
          {adaptation.warnings.map((warning, i) => (
            <div
              key={i}
              className={cn(
                'flex items-center gap-2 text-sm',
                adaptation.isValid ? 'text-yellow-600' : 'text-red-600'
              )}
            >
              <AlertTriangle className="h-3 w-3 flex-shrink-0" />
              {warning}
            </div>
          ))}
        </div>
      )}

      {/* Preview mockup */}
      <div className={cn(
        'rounded-lg border p-4',
        platform === 'linkedin' && 'bg-[#f3f2ef]',
        platform === 'instagram' && 'bg-gradient-to-br from-purple-50 to-pink-50',
        platform === 'twitter' && 'bg-black text-white',
        platform === 'facebook' && 'bg-[#f0f2f5]',
        platform === 'tiktok' && 'bg-black text-white',
        !['linkedin', 'instagram', 'twitter', 'facebook', 'tiktok'].includes(platform) && 'bg-muted/50'
      )}>
        <div className="flex items-start gap-3">
          <div className={cn(
            'w-10 h-10 rounded-full flex items-center justify-center text-xs font-bold',
            platform === 'twitter' || platform === 'tiktok' ? 'bg-gray-700' : 'bg-gray-300'
          )}>
            {PLATFORM_ICONS[platform]}
          </div>
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-1">
              <span className="font-semibold text-sm">Your Account</span>
              <span className="text-xs text-muted-foreground">Just now</span>
            </div>
            <div className={cn(
              'text-sm whitespace-pre-wrap break-words',
              platform === 'twitter' || platform === 'tiktok' ? 'text-gray-100' : 'text-gray-800'
            )}>
              {adaptation.content || <span className="text-muted-foreground italic">No content yet...</span>}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

export function PlatformPreview({
  blocks,
  platforms,
  className,
}: PlatformPreviewProps) {
  const content = useMemo(() => assembleContent(blocks), [blocks]);

  const adaptations = useMemo(() => {
    return platforms.reduce((acc, platform) => {
      acc[platform] = adaptForPlatform(content, platform);
      return acc;
    }, {} as Record<SocialPlatform, PlatformAdaptation>);
  }, [content, platforms]);

  const validCount = Object.values(adaptations).filter(a => a.isValid).length;

  if (platforms.length === 0) {
    return (
      <Card className={className}>
        <CardContent className="p-6 text-center text-muted-foreground">
          Select platforms to preview your content
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className={className}>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <CardTitle className="text-lg">Platform Previews</CardTitle>
          <Badge
            variant={validCount === platforms.length ? 'default' : 'secondary'}
            className={cn(
              validCount === platforms.length && 'bg-green-500'
            )}
          >
            {validCount}/{platforms.length} ready
          </Badge>
        </div>
      </CardHeader>
      <CardContent>
        <Tabs defaultValue={platforms[0]} className="w-full">
          <TabsList className="w-full justify-start overflow-x-auto">
            {platforms.map((platform) => {
              const adaptation = adaptations[platform];
              return (
                <TabsTrigger
                  key={platform}
                  value={platform}
                  className="gap-1.5"
                >
                  <span className="font-mono text-xs">{PLATFORM_ICONS[platform]}</span>
                  <span className="capitalize">{platform}</span>
                  {!adaptation.isValid && (
                    <AlertCircle className="h-3 w-3 text-red-500" />
                  )}
                </TabsTrigger>
              );
            })}
          </TabsList>
          {platforms.map((platform) => (
            <TabsContent key={platform} value={platform} className="mt-4">
              <PlatformPreviewCard
                adaptation={adaptations[platform]}
                platform={platform}
              />
            </TabsContent>
          ))}
        </Tabs>
      </CardContent>
    </Card>
  );
}
