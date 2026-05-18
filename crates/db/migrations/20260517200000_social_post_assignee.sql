-- Add assignee to social_posts so a team member can be responsible for verifying
-- each post before it publishes (even in automated workflows).
ALTER TABLE social_posts ADD COLUMN assignee_id BLOB REFERENCES users(id);
