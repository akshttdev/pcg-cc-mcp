import { useCallback, useEffect, useState } from 'react';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { FormField } from '@/components/ui/form-field';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import type {
  CreateCrmContactRequest,
  CrmContactRecord,
  UpdateCrmContactRequest,
} from '@/lib/api';
import { CONTACT_SOURCE_INFO, LIFECYCLE_STAGE_INFO } from '@/types/crm';

interface ContactFormDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  projectId: string;
  contact?: CrmContactRecord;
  onSubmit: (data: CreateCrmContactRequest | UpdateCrmContactRequest) => void;
  isLoading: boolean;
}

export function ContactFormDialog({
  open,
  onOpenChange,
  // eslint-disable-next-line @typescript-eslint/no-unused-vars, unused-imports/no-unused-vars
  projectId: _projectId,
  contact,
  onSubmit,
  isLoading,
}: ContactFormDialogProps) {
  const emptyForm = {
    first_name: '',
    last_name: '',
    email: '',
    phone: '',
    mobile: '',
    company_name: '',
    job_title: '',
    department: '',
    linkedin_url: '',
    twitter_handle: '',
    website: '',
    lifecycle_stage: 'lead',
    source: '' as string,
    tags: '',
  };

  const contactToForm = (c: CrmContactRecord) => ({
    first_name: c.first_name || '',
    last_name: c.last_name || '',
    email: c.email || '',
    phone: c.phone || '',
    mobile: c.mobile || '',
    company_name: c.company_name || '',
    job_title: c.job_title || '',
    department: c.department || '',
    linkedin_url: c.linkedin_url || '',
    twitter_handle: c.twitter_handle || '',
    website: c.website || '',
    lifecycle_stage: c.lifecycle_stage || 'lead',
    source: c.source || '',
    tags: Array.isArray(c.tags) ? c.tags.join(', ') : (c.tags || ''),
  });

  const [formData, setFormData] = useState(contact ? contactToForm(contact) : emptyForm);

  useEffect(() => {
    setFormData(contact ? contactToForm(contact) : emptyForm);
  }, [contact, open]);

  const handleFieldChange = useCallback(
    (field: string) => (e: React.ChangeEvent<HTMLInputElement>) => {
      setFormData((prev) => ({ ...prev, [field]: e.target.value }));
    },
    [],
  );

  const handleSelectChange = useCallback(
    (field: string) => (value: string) => {
      setFormData((prev) => ({ ...prev, [field]: value === '__none__' ? '' : value }));
    },
    [],
  );

  const handleCancel = useCallback(() => {
    onOpenChange(false);
  }, [onOpenChange]);

  const handleSubmit = useCallback((e: React.FormEvent) => {
    e.preventDefault();
    const parsedTags = formData.tags
      ? formData.tags.split(',').map((t) => t.trim()).filter(Boolean)
      : undefined;
    onSubmit({
      ...formData,
      first_name: formData.first_name || undefined,
      last_name: formData.last_name || undefined,
      email: formData.email || undefined,
      phone: formData.phone || undefined,
      mobile: formData.mobile || undefined,
      company_name: formData.company_name || undefined,
      job_title: formData.job_title || undefined,
      department: formData.department || undefined,
      linkedin_url: formData.linkedin_url || undefined,
      twitter_handle: formData.twitter_handle || undefined,
      website: formData.website || undefined,
      source: formData.source || undefined,
      tags: parsedTags,
    });
  }, [formData, onSubmit]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>{contact ? 'Edit Contact' : 'Add Contact'}</DialogTitle>
          <DialogDescription>
            {contact
              ? 'Update contact information'
              : 'Add a new contact to your CRM'}
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="space-y-4 max-h-[60vh] overflow-y-auto pr-1">
          <div className="grid gap-4 md:grid-cols-2">
            <FormField label="First Name" htmlFor="first_name">
              <Input
                id="first_name"
                value={formData.first_name}
                onChange={handleFieldChange('first_name')}
              />
            </FormField>
            <FormField label="Last Name" htmlFor="last_name">
              <Input
                id="last_name"
                value={formData.last_name}
                onChange={handleFieldChange('last_name')}
              />
            </FormField>
          </div>
          <FormField label="Email" htmlFor="email">
            <Input
              id="email"
              type="email"
              value={formData.email}
              onChange={handleFieldChange('email')}
            />
          </FormField>
          <div className="grid gap-4 md:grid-cols-2">
            <FormField label="Phone" htmlFor="phone">
              <Input
                id="phone"
                value={formData.phone}
                onChange={handleFieldChange('phone')}
                placeholder="+1 555-0100"
              />
            </FormField>
            <FormField label="Mobile" htmlFor="mobile">
              <Input
                id="mobile"
                value={formData.mobile}
                onChange={handleFieldChange('mobile')}
                placeholder="+1 555-0101"
              />
            </FormField>
          </div>
          <div className="grid gap-4 md:grid-cols-2">
            <FormField label="Company" htmlFor="company_name">
              <Input
                id="company_name"
                value={formData.company_name}
                onChange={handleFieldChange('company_name')}
              />
            </FormField>
            <FormField label="Job Title" htmlFor="job_title">
              <Input
                id="job_title"
                value={formData.job_title}
                onChange={handleFieldChange('job_title')}
              />
            </FormField>
          </div>
          <FormField label="Department" htmlFor="department">
            <Input
              id="department"
              value={formData.department}
              onChange={handleFieldChange('department')}
              placeholder="Engineering, Sales, Marketing..."
            />
          </FormField>
          <div className="grid gap-4 md:grid-cols-2">
            <FormField label="Lifecycle Stage" htmlFor="lifecycle_stage">
              <Select
                value={formData.lifecycle_stage}
                onValueChange={handleSelectChange('lifecycle_stage')}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {Object.entries(LIFECYCLE_STAGE_INFO).map(([key, info]) => (
                    <SelectItem key={key} value={key}>
                      {info.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </FormField>
            <FormField label="Source" htmlFor="source">
              <Select
                value={formData.source || '__none__'}
                onValueChange={handleSelectChange('source')}
              >
                <SelectTrigger>
                  <SelectValue placeholder="Select source" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="__none__">Not specified</SelectItem>
                  {Object.entries(CONTACT_SOURCE_INFO).map(([key, info]) => (
                    <SelectItem key={key} value={key}>
                      {info.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </FormField>
          </div>
          <FormField label="LinkedIn URL" htmlFor="linkedin_url">
            <Input
              id="linkedin_url"
              value={formData.linkedin_url}
              onChange={handleFieldChange('linkedin_url')}
              placeholder="https://linkedin.com/in/..."
            />
          </FormField>
          <div className="grid gap-4 md:grid-cols-2">
            <FormField label="Twitter Handle" htmlFor="twitter_handle">
              <Input
                id="twitter_handle"
                value={formData.twitter_handle}
                onChange={handleFieldChange('twitter_handle')}
                placeholder="@handle"
              />
            </FormField>
            <FormField label="Website" htmlFor="website">
              <Input
                id="website"
                value={formData.website}
                onChange={handleFieldChange('website')}
                placeholder="https://..."
              />
            </FormField>
          </div>
          <FormField label="Tags" htmlFor="tags">
            <Input
              id="tags"
              value={formData.tags}
              onChange={handleFieldChange('tags')}
              placeholder="Comma-separated tags, e.g. vip, conference-2026"
            />
          </FormField>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={handleCancel}>
              Cancel
            </Button>
            <Button type="submit" disabled={isLoading}>
              {isLoading ? 'Saving...' : contact ? 'Save Changes' : 'Add Contact'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
