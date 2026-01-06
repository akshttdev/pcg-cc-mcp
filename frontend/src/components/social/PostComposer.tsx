import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import type { SocialPlatform } from '@/types/social';

interface PostComposerProps {
  isOpen: boolean;
  onClose: () => void;
  defaultPlatforms?: SocialPlatform[];
}

export function PostComposer({ isOpen, onClose, defaultPlatforms }: PostComposerProps) {
  if (!isOpen) return null;

  return (
    <Card className="border-dashed">
      <CardHeader>
        <CardTitle>Social Post Composer</CardTitle>
      </CardHeader>
      <CardContent className="space-y-3 text-sm text-muted-foreground">
        <p>
          The rich composer is still being wired up. In the meantime Nora can
          draft copy directly from the command center.
        </p>
        {defaultPlatforms && defaultPlatforms.length > 0 && (
          <p>
            Target platforms:{' '}
            <span className="font-medium text-foreground">
              {defaultPlatforms.join(', ')}
            </span>
          </p>
        )}
        <Button size="sm" onClick={onClose}>
          Close
        </Button>
      </CardContent>
    </Card>
  );
}
