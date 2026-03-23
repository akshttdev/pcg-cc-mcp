import { useNavigate } from 'react-router-dom';
import {
  Building2,
  Edit,
  Globe,
  Linkedin,
  Mail,
  MoreVertical,
  Phone,
  Star,
  Trash2,
  Twitter,
} from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { IconButton } from '@/components/ui/icon-button';
import type { CrmContactRecord } from '@/lib/api';
import type { LifecycleStage } from '@/types/crm';
import { LIFECYCLE_STAGE_INFO } from '@/types/crm';

interface ContactCardProps {
  contact: CrmContactRecord;
  projectId: string;
  onEdit: () => void;
  onDelete: () => void;
}

export function ContactCard({
  contact,
  projectId,
  onEdit,
  onDelete,
}: ContactCardProps) {
  const navigate = useNavigate();
  const stageInfo = LIFECYCLE_STAGE_INFO[contact.lifecycle_stage as LifecycleStage] ?? {
    label: contact.lifecycle_stage,
    color: '#6B7280',
  };

  const formatRelativeTime = (dateString: string | null) => {
    if (!dateString) return 'Never';
    const date = new Date(dateString);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffDays === 0) return 'Today';
    if (diffDays === 1) return 'Yesterday';
    if (diffDays < 7) return `${diffDays} days ago`;
    if (diffDays < 30) return `${Math.floor(diffDays / 7)} weeks ago`;
    return `${Math.floor(diffDays / 30)} months ago`;
  };

  return (
    <div className="group flex items-center gap-3 sm:gap-4 p-3 sm:p-4 border border-border/40 rounded-lg hover:bg-muted/30 hover:border-border/70 transition-all duration-200">
      {/* Avatar */}
      <div className="w-10 h-10 sm:w-12 sm:h-12 rounded-full bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center text-white font-semibold shrink-0 text-sm sm:text-base">
        {contact.avatar_url ? (
          <img
            src={contact.avatar_url}
            alt={contact.full_name || 'Contact'}
            className="w-full h-full rounded-full object-cover"
          />
        ) : (
          (contact.first_name?.[0] || contact.email?.[0] || 'C').toUpperCase()
        )}
      </div>

      {/* Info */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-0.5">
          <h4
            className="font-medium truncate cursor-pointer hover:text-info transition-colors"
            onClick={() => navigate(`/projects/${projectId}/crm/contacts/${contact.id}`)}
          >
            {contact.full_name || contact.email || 'Unnamed Contact'}
          </h4>
          <Badge
            variant="outline"
            style={{
              borderColor: stageInfo.color,
              color: stageInfo.color,
            }}
            className="text-xs shrink-0"
          >
            {stageInfo.label}
          </Badge>
          {contact.lead_score > 50 && (
            <Badge className="text-xs bg-yellow-100 text-yellow-700 dark:bg-yellow-950/40 dark:text-yellow-300 shrink-0">
              <Star className="h-3 w-3 mr-1" />
              {contact.lead_score}
            </Badge>
          )}
        </div>
        <div className="flex items-center gap-3 sm:gap-4 text-sm text-muted-foreground flex-wrap">
          {contact.email && (
            <span className="flex items-center gap-1 truncate">
              <Mail className="h-3 w-3 shrink-0" />
              <span className="truncate">{contact.email}</span>
            </span>
          )}
          {contact.company_name && (
            <span className="hidden sm:flex items-center gap-1">
              <Building2 className="h-3 w-3 shrink-0" />
              {contact.company_name}
            </span>
          )}
          {contact.phone && (
            <span className="hidden md:flex items-center gap-1">
              <Phone className="h-3 w-3 shrink-0" />
              {contact.phone}
            </span>
          )}
        </div>
        <div className="flex items-center gap-4 mt-1 text-xs text-muted-foreground">
          <span>Last activity: {formatRelativeTime(contact.last_activity_at)}</span>
          <span className="hidden sm:inline">{contact.email_count} emails</span>
        </div>
      </div>

      {/* Social Links — hidden on mobile */}
      <div className="hidden md:flex items-center gap-1">
        {contact.linkedin_url && (
          <Button variant="ghost" size="icon" className="h-8 w-8" asChild title="LinkedIn profile">
            <a href={contact.linkedin_url} target="_blank" rel="noopener noreferrer">
              <Linkedin className="h-3.5 w-3.5" />
            </a>
          </Button>
        )}
        {contact.twitter_handle && (
          <Button variant="ghost" size="icon" className="h-8 w-8" asChild title="Twitter profile">
            <a
              href={`https://twitter.com/${contact.twitter_handle}`}
              target="_blank"
              rel="noopener noreferrer"
            >
              <Twitter className="h-3.5 w-3.5" />
            </a>
          </Button>
        )}
        {contact.website && (
          <Button variant="ghost" size="icon" className="h-8 w-8" asChild title="Website">
            <a href={contact.website} target="_blank" rel="noopener noreferrer">
              <Globe className="h-3.5 w-3.5" />
            </a>
          </Button>
        )}
      </div>

      {/* Actions */}
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <IconButton
            variant="ghost" className="h-8 w-8 opacity-0 group-hover:opacity-100 transition-opacity"
            icon={MoreVertical}
            label="More options"
          />
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem onClick={onEdit}>
            <Edit className="h-4 w-4 mr-2" />
            Edit
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem onClick={onDelete} className="text-red-600">
            <Trash2 className="h-4 w-4 mr-2" />
            Delete
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}
