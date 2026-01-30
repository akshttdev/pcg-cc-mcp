import { useDraggable } from '@dnd-kit/core';
import { Card, CardContent } from '@/components/ui/card';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Calendar, DollarSign, Mail, MoreVertical, Building2, Trash2, Edit } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { CrmDealWithContact } from '@/types/crm';
import { formatDistanceToNow } from 'date-fns';

interface CrmDealCardProps {
  deal: CrmDealWithContact;
  onEdit?: (deal: CrmDealWithContact) => void;
  onDelete?: (deal: CrmDealWithContact) => void;
  isDragging?: boolean;
}

export function CrmDealCard({ deal, onEdit, onDelete, isDragging }: CrmDealCardProps) {
  const { attributes, listeners, setNodeRef, transform } = useDraggable({
    id: deal.id,
    data: {
      type: 'deal',
      deal,
    },
  });

  const style = transform
    ? {
        transform: `translate3d(${transform.x}px, ${transform.y}px, 0)`,
      }
    : undefined;

  const initials = deal.contact_name
    ? deal.contact_name
        .split(' ')
        .map((n) => n[0])
        .join('')
        .toUpperCase()
        .slice(0, 2)
    : deal.name.slice(0, 2).toUpperCase();

  const formatAmount = (amount: number | undefined | null) => {
    if (!amount) return null;
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: deal.currency || 'USD',
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    }).format(amount);
  };

  const formattedAmount = formatAmount(deal.amount);
  const lastActivity = deal.last_activity_at
    ? formatDistanceToNow(new Date(deal.last_activity_at), { addSuffix: true })
    : null;

  return (
    <Card
      ref={setNodeRef}
      style={style}
      className={cn(
        'cursor-grab active:cursor-grabbing transition-shadow hover:shadow-md',
        isDragging && 'opacity-50 shadow-lg ring-2 ring-primary'
      )}
      {...listeners}
      {...attributes}
    >
      <CardContent className="p-3 space-y-2">
        {/* Header: Avatar and Name */}
        <div className="flex items-start justify-between gap-2">
          <div className="flex items-center gap-2 min-w-0 flex-1">
            <Avatar className="h-8 w-8 shrink-0">
              {deal.contact_avatar_url && (
                <AvatarImage src={deal.contact_avatar_url} alt={deal.contact_name || deal.name} />
              )}
              <AvatarFallback className="text-xs bg-primary/10 text-primary">
                {initials}
              </AvatarFallback>
            </Avatar>
            <div className="min-w-0 flex-1">
              <p className="font-medium text-sm truncate">{deal.name}</p>
              {deal.contact_name && (
                <p className="text-xs text-muted-foreground truncate">{deal.contact_name}</p>
              )}
            </div>
          </div>

          {/* Actions Menu */}
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button
                variant="ghost"
                size="sm"
                className="h-6 w-6 p-0 opacity-0 group-hover:opacity-100 transition-opacity"
                onClick={(e) => e.stopPropagation()}
              >
                <MoreVertical className="h-3 w-3" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-36">
              <DropdownMenuItem onClick={() => onEdit?.(deal)}>
                <Edit className="h-3 w-3 mr-2" />
                Edit
              </DropdownMenuItem>
              <DropdownMenuItem
                onClick={() => onDelete?.(deal)}
                className="text-destructive focus:text-destructive"
              >
                <Trash2 className="h-3 w-3 mr-2" />
                Delete
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>

        {/* Deal Amount */}
        {formattedAmount && (
          <div className="flex items-center gap-1.5 text-sm">
            <DollarSign className="h-3.5 w-3.5 text-green-600" />
            <span className="font-semibold text-green-600">{formattedAmount}</span>
          </div>
        )}

        {/* Company */}
        {deal.contact_company && (
          <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
            <Building2 className="h-3 w-3" />
            <span className="truncate">{deal.contact_company}</span>
          </div>
        )}

        {/* Email */}
        {deal.contact_email && (
          <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
            <Mail className="h-3 w-3" />
            <span className="truncate">{deal.contact_email}</span>
          </div>
        )}

        {/* Footer: Expected Close Date and Last Activity */}
        <div className="flex items-center justify-between pt-1 border-t">
          {deal.expected_close_date && (
            <div className="flex items-center gap-1 text-xs text-muted-foreground">
              <Calendar className="h-3 w-3" />
              <span>
                {new Date(deal.expected_close_date).toLocaleDateString('en-US', {
                  month: 'short',
                  day: 'numeric',
                })}
              </span>
            </div>
          )}
          {lastActivity && (
            <Badge variant="outline" className="text-[10px] px-1.5 py-0">
              {lastActivity}
            </Badge>
          )}
        </div>

        {/* Probability indicator */}
        {deal.probability > 0 && (
          <div className="h-1 bg-muted rounded-full overflow-hidden">
            <div
              className="h-full bg-primary transition-all"
              style={{ width: `${deal.probability}%` }}
            />
          </div>
        )}
      </CardContent>
    </Card>
  );
}
