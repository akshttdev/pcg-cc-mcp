import type React from 'react';
import { Instagram, Linkedin, Twitter, Facebook, Youtube, Share2, MessageSquare } from 'lucide-react';

export function SocialSection() {
  const socialPlatforms: { name: string; icon: React.ElementType; accent: string; desc: string }[] = [
    { name: 'Instagram',  icon: Instagram,    accent: '#E1306C', desc: 'Schedule posts, track mentions, and monitor engagement.' },
    { name: 'LinkedIn',   icon: Linkedin,     accent: '#0A66C2', desc: 'Company page management, posts, and B2B lead tracking.' },
    { name: 'X / Twitter', icon: Twitter,    accent: '#000000', desc: 'Post scheduling, mention monitoring, and DM management.' },
    { name: 'Facebook',   icon: Facebook,     accent: '#1877F2', desc: 'Page management, ads integration, and audience insights.' },
    { name: 'YouTube',    icon: Youtube,      accent: '#FF0000', desc: 'Channel analytics, comment monitoring, and content sync.' },
    { name: 'TikTok',     icon: Share2,       accent: '#010101', desc: 'Video scheduling and performance analytics.' },
    { name: 'Threads',    icon: MessageSquare, accent: '#101010', desc: 'Thread management and audience engagement.' },
    { name: 'Bluesky',    icon: Share2,       accent: '#0085FF', desc: 'Decentralised social — post scheduling and monitoring.' },
    { name: 'Pinterest',  icon: Share2,       accent: '#E60023', desc: 'Pin management, board sync, and product catalogue.' },
  ];

  return (
    <section className="space-y-3">
      <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Social Media</h3>
      <p className="text-xs text-muted-foreground">
        Social accounts are connected per-project to keep brand identities scoped. Visit a project's settings to connect individual platforms.
      </p>
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
        {socialPlatforms.map(({ name, icon: Icon, accent, desc }) => (
          <div
            key={name}
            className="flex items-start gap-3 p-3 rounded-lg border border-border/60 bg-card/60"
          >
            <div className="w-1 self-stretch rounded-full shrink-0" style={{ background: accent }} />
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2 mb-0.5">
                <Icon className="h-4 w-4 shrink-0" style={{ color: accent }} />
                <span className="text-sm font-medium">{name}</span>
              </div>
              <p className="text-xs text-muted-foreground leading-relaxed">{desc}</p>
            </div>
            <span className="text-xs text-muted-foreground italic shrink-0 mt-0.5">Per-project</span>
          </div>
        ))}
      </div>
    </section>
  );
}
