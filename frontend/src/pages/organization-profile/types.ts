export interface OrgMember {
  id: string;
  user_id: string;
  role: string;
  joined_at: string;
  user?: { username: string; full_name: string; email: string; avatar_url?: string };
}

export interface OrganizationProfilePageProps {
  defaultTab?: string;
  defaultPipeline?: string;
}
