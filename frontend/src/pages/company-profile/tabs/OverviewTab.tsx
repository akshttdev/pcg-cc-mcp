import {
  Briefcase,
  Calendar,
  CheckCircle2,
  ChevronRight,
  ExternalLink,
  Globe,
  Mail,
  MapPin,
  MessageSquare,
  Phone,
  Tag,
  Users,
  X,
} from 'lucide-react';
import { Link } from 'react-router-dom';

import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
  type CompanyContactMethod,
  type CompanyRecord,
  type CrmContactRecord,
  type ProposalRecord,
} from '@/lib/api';

import {
  BusinessHoursCard,
  ContactRail,
  ProposalStatusBadge,
  StarRating,
} from '../components/helpers';
import type { Tab } from '../index';

export function OverviewTab({
  company,
  proposals,
  contacts,
  contactMethods,
  onNavigate,
  onRemoveMethod,
}: {
  company: CompanyRecord;
  proposals: ProposalRecord[];
  contacts: CrmContactRecord[];
  contactMethods: CompanyContactMethod[];
  onNavigate: (tab: Tab) => void;
  onRemoveMethod: (id: string) => Promise<void>;
}) {
  const tags: string[] = (() => {
    try {
      return JSON.parse(company.tags ?? '[]');
    } catch {
      return [];
    }
  })();

  return (
    <div className="grid grid-cols-1 lg:grid-cols-3 gap-5">
      {/* -- Left column (2/3) -- */}
      <div className="lg:col-span-2 space-y-4">
        {/* Contact action rail */}
        <ContactRail company={company} />

        {/* Star rating + GMB */}
        {(company.gmb_rating != null || company.gmb_review_count != null) && (
          <div className="flex items-center gap-3 flex-wrap">
            <StarRating
              rating={company.gmb_rating}
              count={company.gmb_review_count}
            />
            {company.gmb_place_id && (
              <a
                href={`https://maps.google.com/?cid=${company.gmb_place_id}`}
                target="_blank"
                rel="noopener noreferrer"
                className="text-xs text-blue-500 hover:underline flex items-center gap-0.5"
              >
                View on Google Maps
                <ExternalLink className="h-3 w-3 ml-0.5" />
              </a>
            )}
          </div>
        )}

        {/* About */}
        {company.description && (
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium">About</CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-sm text-muted-foreground leading-relaxed whitespace-pre-line">
                {company.description}
              </p>
            </CardContent>
          </Card>
        )}

        {/* Internal notes */}
        {company.notes && (
          <Card className="border-amber-200 bg-amber-50/50">
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium text-amber-800 flex items-center gap-1.5">
                <MessageSquare className="h-4 w-4" />
                Internal Notes
              </CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-sm text-amber-900 whitespace-pre-line">
                {company.notes}
              </p>
            </CardContent>
          </Card>
        )}

        {/* Stats row */}
        <div className="grid grid-cols-3 gap-3">
          <button
            onClick={() => onNavigate('proposals')}
            className="flex flex-col items-center gap-1 p-4 rounded-xl bg-muted hover:bg-muted/70 transition-colors text-center"
          >
            <span className="text-2xl font-semibold">{proposals.length}</span>
            <span className="text-xs text-muted-foreground">
              Proposal{proposals.length !== 1 ? 's' : ''}
            </span>
          </button>
          <button
            onClick={() => onNavigate('contacts')}
            className="flex flex-col items-center gap-1 p-4 rounded-xl bg-muted hover:bg-muted/70 transition-colors text-center"
          >
            <span className="text-2xl font-semibold">{contacts.length}</span>
            <span className="text-xs text-muted-foreground">
              Contact{contacts.length !== 1 ? 's' : ''}
            </span>
          </button>
          <div className="flex flex-col items-center gap-1 p-4 rounded-xl bg-muted text-center">
            <span className="text-2xl font-semibold">
              {proposals.filter((p) => p.status === 'contract_signed').length}
            </span>
            <span className="text-xs text-muted-foreground">Won Deals</span>
          </div>
        </div>

        {/* Recent proposals */}
        {proposals.length > 0 && (
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium flex items-center justify-between">
                <span>Recent Proposals</span>
                <button
                  onClick={() => onNavigate('proposals')}
                  className="text-xs text-muted-foreground hover:text-foreground flex items-center gap-0.5"
                >
                  View all <ChevronRight className="h-3 w-3" />
                </button>
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-2">
              {proposals.slice(0, 3).map((p) => (
                <div
                  key={p.id}
                  className="flex items-center justify-between text-sm py-1 border-b last:border-0"
                >
                  <span className="truncate flex-1">{p.title}</span>
                  <ProposalStatusBadge status={p.status} />
                </div>
              ))}
            </CardContent>
          </Card>
        )}

        {/* Recent contacts */}
        {contacts.length > 0 && (
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium flex items-center justify-between">
                <span>Contacts</span>
                <button
                  onClick={() => onNavigate('contacts')}
                  className="text-xs text-muted-foreground hover:text-foreground flex items-center gap-0.5"
                >
                  View all <ChevronRight className="h-3 w-3" />
                </button>
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-2">
              {contacts.slice(0, 4).map((c) => (
                <Link
                  key={c.id}
                  to={`/contacts/${c.id}`}
                  className="flex items-center gap-2 text-sm py-1 border-b last:border-0 hover:text-primary transition-colors"
                >
                  <div className="h-7 w-7 rounded-full bg-primary/10 flex items-center justify-center text-xs font-semibold text-primary shrink-0">
                    {(c.full_name ?? '').slice(0, 2).toUpperCase()}
                  </div>
                  <div className="min-w-0 flex-1">
                    <p className="font-medium truncate">
                      {c.full_name ?? c.email ?? 'Unnamed'}
                    </p>
                    {c.job_title && (
                      <p className="text-xs text-muted-foreground truncate">
                        {c.job_title}
                      </p>
                    )}
                  </div>
                </Link>
              ))}
            </CardContent>
          </Card>
        )}
      </div>

      {/* -- Right column (1/3) -- */}
      <div className="space-y-4">
        {/* Location + contact details */}
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Details</CardTitle>
          </CardHeader>
          <CardContent className="space-y-2.5 text-sm">
            {(company.address || company.city || company.headquarters) && (
              <div className="flex gap-2">
                <MapPin className="h-4 w-4 text-muted-foreground shrink-0 mt-0.5" />
                <div>
                  {company.address && <p>{company.address}</p>}
                  {(company.city || company.country) && (
                    <p className="text-muted-foreground">
                      {[company.city, company.country]
                        .filter(Boolean)
                        .join(', ')}
                    </p>
                  )}
                  {!company.address &&
                    !company.city &&
                    company.headquarters && (
                      <p className="text-muted-foreground">
                        {company.headquarters}
                      </p>
                    )}
                </div>
              </div>
            )}
            {company.phone && (
              <div className="flex items-center gap-2">
                <Phone className="h-4 w-4 text-muted-foreground shrink-0" />
                <a
                  href={`tel:${company.phone}`}
                  className="hover:text-primary transition-colors"
                >
                  {company.phone}
                </a>
              </div>
            )}
            {company.email && (
              <div className="flex items-center gap-2">
                <Mail className="h-4 w-4 text-muted-foreground shrink-0" />
                <a
                  href={`mailto:${company.email}`}
                  className="hover:text-primary transition-colors truncate"
                >
                  {company.email}
                </a>
              </div>
            )}
            {company.website && (
              <div className="flex items-center gap-2">
                <Globe className="h-4 w-4 text-muted-foreground shrink-0" />
                <a
                  href={company.website}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-blue-500 hover:underline truncate"
                >
                  {company.website.replace(/^https?:\/\/(www\.)?/, '')}
                </a>
              </div>
            )}
            {company.industry && (
              <div className="flex items-center gap-2">
                <Briefcase className="h-4 w-4 text-muted-foreground shrink-0" />
                <span>{company.industry}</span>
              </div>
            )}
            {company.founded_year && (
              <div className="flex items-center gap-2">
                <Calendar className="h-4 w-4 text-muted-foreground shrink-0" />
                <span>Founded {company.founded_year}</span>
              </div>
            )}
            {company.employee_count && (
              <div className="flex items-center gap-2">
                <Users className="h-4 w-4 text-muted-foreground shrink-0" />
                <span>{company.employee_count} employees</span>
              </div>
            )}
            {company.organization_id && (
              <div className="flex items-center gap-2">
                <CheckCircle2 className="h-4 w-4 text-green-600 shrink-0" />
                <span className="text-green-700 font-medium">
                  Platform Organization
                </span>
              </div>
            )}
          </CardContent>
        </Card>

        {/* Tags */}
        {tags.length > 0 && (
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium flex items-center gap-1.5">
                <Tag className="h-4 w-4 text-muted-foreground" />
                Tags
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="flex flex-wrap gap-1.5">
                {tags.map((t) => (
                  <Badge
                    key={t}
                    variant="secondary"
                    className="text-xs font-normal capitalize"
                  >
                    {t}
                  </Badge>
                ))}
              </div>
            </CardContent>
          </Card>
        )}

        {/* Business Hours */}
        <BusinessHoursCard hours={company.business_hours} />

        {/* Extra contact methods from junction table */}
        {contactMethods.length > 0 && (
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium">
                Additional Contacts
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-2">
              {contactMethods.map((m) => (
                <div
                  key={m.id}
                  className="flex items-center justify-between text-sm group"
                >
                  <div className="flex items-center gap-2 min-w-0">
                    <span className="capitalize text-muted-foreground text-xs w-16 shrink-0">
                      {m.method_type}
                    </span>
                    <span className="truncate">{m.value}</span>
                    {m.label && (
                      <span className="text-xs text-muted-foreground">
                        ({m.label})
                      </span>
                    )}
                  </div>
                  <button
                    className="opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-destructive transition-opacity shrink-0"
                    onClick={() => onRemoveMethod(m.id)}
                  >
                    <X className="h-3.5 w-3.5" />
                  </button>
                </div>
              ))}
            </CardContent>
          </Card>
        )}
      </div>
    </div>
  );
}
