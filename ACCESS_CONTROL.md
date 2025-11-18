# Access Control & Permissions Summary

## Overview

The PCG-CC-MCP application now has comprehensive Role-Based Access Control (RBAC) with project-level permissions.

## User Roles

### System Roles

#### Admin (Global)
- **Full system access** - Can manage everything
- Can create/update/delete projects
- Can manage all users
- Can assign users to projects
- Bypasses all project-level permission checks
- Can access all projects regardless of membership

#### Regular User (Non-Admin)
- **Limited access** - Only to assigned projects
- Cannot create projects
- Cannot manage other users
- Cannot see projects they're not assigned to
- Needs explicit project membership to access any project

### Project-Level Roles

When a regular user is granted access to a project, they get one of these roles:

#### Owner
- **Full project control**
- Can read, write, and delete the project
- Can manage project members (add/remove users, change roles)
- Can update project settings
- Can delete the project

#### Admin
- **Management access**
- Can read and write to the project
- Can manage project members
- Cannot delete the project

#### Editor
- **Write access**
- Can read and write to the project
- Cannot manage members
- Cannot delete the project

#### Viewer
- **Read-only access**
- Can only view the project
- Cannot make any changes
- Cannot manage members

## Protected Endpoints

### User Management (Admin Only)
All `/api/users/*` endpoints require admin role:
- ✅ `GET /api/users` - List users
- ✅ `GET /api/users/{id}` - Get user details
- ✅ `POST /api/users/create` - Create new user
- ✅ `PATCH /api/users/{id}` - Update user
- ✅ `PATCH /api/users/{id}/role` - Change user role
- ✅ `PATCH /api/users/{id}/suspend` - Suspend user
- ✅ `PATCH /api/users/{id}/activate` - Activate user
- ✅ `DELETE /api/users/{id}` - Deactivate user

### Project Creation (Admin Only)
- ✅ `POST /api/projects` - Create project (Admin only)

### Project Management (Admin or Owner)
- ✅ `PUT /api/projects/{id}` - Update project (Admin or Owner)
- ✅ `DELETE /api/projects/{id}` - Delete project (Admin or Owner)

### Project Access (Based on Membership)
- ✅ `GET /api/projects` - List projects
  - **Admin**: Sees all projects
  - **Regular user**: Only sees assigned projects
- ✅ `GET /api/projects/{id}/*` - All project routes
  - **Admin**: Full access to all projects
  - **Regular user**: Must have at least Viewer role

### Project Member Management (Admin Only)
All `/api/permissions/projects/*` endpoints require authentication:
- ✅ `GET /api/permissions/projects/{id}/members` - List members (Authenticated)
- ✅ `POST /api/permissions/projects/{id}/members` - Add member (Admin only)
- ✅ `DELETE /api/permissions/members/{user_id}` - Remove member (Admin only)
- ✅ `POST /api/permissions/members/{user_id}/role` - Update role (Admin only)
- ✅ `GET /api/permissions/projects/{id}/access` - Check access (Authenticated)
- ✅ `GET /api/permissions/my-projects` - List my projects (Authenticated)

## Permission Checks

### How It Works

1. **Authentication Required**
   - All project routes require authentication via cookie or Bearer token
   - Unauthenticated requests get `401 Unauthorized`

2. **Project List Filtering**
   - When a user requests `/api/projects`, the backend filters results:
     - Admins get all projects
     - Regular users get only projects where they're in `project_members` table

3. **Project Access Checks**
   - When accessing `/api/projects/{id}/*`:
     - Admins bypass all checks
     - Regular users must have a record in `project_members` for that project
     - Minimum required role is checked (Viewer for read, Editor for write, etc.)
     - Missing access returns `403 Forbidden`

4. **Admin-Only Operations**
   - Creating projects: Checked via `require_admin()` method
   - Managing users: Protected by `require_admin` middleware
   - All admin operations return `403 Forbidden` for non-admins

## Database Schema

### Users Table
```sql
users
  - id (BLOB)
  - username (TEXT)
  - email (TEXT)
  - is_admin (INTEGER) -- 1 = admin, 0 = regular user
  - is_active (INTEGER) -- 1 = active, 0 = inactive
  ...
```

### Project Members Table
```sql
project_members
  - id (BLOB)
  - project_id (TEXT)
  - user_id (BLOB)
  - role (TEXT) -- 'owner', 'admin', 'editor', 'viewer'
  - granted_by (BLOB) -- who granted access
  - granted_at (TEXT)
```

### Permission Audit Log
```sql
permission_audit_log
  - id (BLOB)
  - user_id (BLOB)
  - action (TEXT) -- 'grant', 'revoke', 'update_role'
  - resource_type (TEXT) -- 'project'
  - resource_id (TEXT) -- project_id
  - details (TEXT) -- JSON with additional info
  - performed_by (BLOB)
  - performed_at (TEXT)
```

## Example Scenarios

### Scenario 1: Regular User Tries to Create Project
```
Request: POST /api/projects
User: john (is_admin = 0)
Result: 403 Forbidden - "Admin access required"
```

### Scenario 2: Regular User Accesses Assigned Project
```
Request: GET /api/projects/abc-123
User: john (is_admin = 0)
Project: abc-123 has john as "editor" in project_members
Result: 200 OK - Returns project data
```

### Scenario 3: Regular User Accesses Unassigned Project
```
Request: GET /api/projects/xyz-789
User: john (is_admin = 0)
Project: xyz-789 has NO entry for john in project_members
Result: 403 Forbidden - "You do not have access to this project"
```

### Scenario 4: Admin Accesses Any Project
```
Request: GET /api/projects/any-project
User: admin (is_admin = 1)
Result: 200 OK - Admins bypass all project checks
```

### Scenario 5: Regular User Lists Projects
```
Request: GET /api/projects
User: john (is_admin = 0)
Projects in DB: project-1, project-2, project-3, project-4
John's memberships: project-1 (editor), project-3 (viewer)
Result: 200 OK - Returns [project-1, project-3]
```

## Frontend Integration

### Settings Pages

#### Users Settings (`/settings/users`)
- **Visibility**: Admin only
- **Features**: Create users, manage roles, suspend accounts

#### Projects Settings (`/settings/projects`)
- **Visibility**: Admin only
- **Features**: Manage project access, assign users with roles

### Project List
- **All users**: See "My Projects" (filtered by access)
- **Admins**: See all projects

### Project Detail Pages
- **Editors/Owners**: Can edit project content
- **Viewers**: Read-only access
- **No access**: Cannot view page (403)

## Security Best Practices

✅ **Implemented:**
- Cookie-based session authentication
- Password hashing with bcrypt
- Role-based access control
- Project-level permissions
- Audit logging for permission changes
- Non-admin users cannot escalate privileges
- Session expiration (30 days)

🔒 **Recommended:**
- Enable Cloudflare Access for production deployments
- Rotate admin passwords regularly
- Regular database backups
- Monitor audit logs for suspicious activity
- Use strong passwords (enforced in UI)

## Testing Permissions

### Create Test Users

```sql
-- Create an admin user
INSERT INTO users (id, username, email, password_hash, is_admin, is_active, created_at)
VALUES (
  randomblob(16),
  'admin',
  'admin@example.com',
  '$2b$12$...',  -- hashed password
  1,  -- is_admin
  1,  -- is_active
  datetime('now')
);

-- Create a regular user
INSERT INTO users (id, username, email, password_hash, is_admin, is_active, created_at)
VALUES (
  randomblob(16),
  'john',
  'john@example.com',
  '$2b$12$...',  -- hashed password
  0,  -- not admin
  1,  -- is_active
  datetime('now')
);
```

### Grant Project Access

```sql
-- Give john editor access to project-1
INSERT INTO project_members (id, project_id, user_id, role, granted_by, granted_at)
VALUES (
  randomblob(16),
  'project-uuid-here',
  (SELECT id FROM users WHERE username = 'john'),
  'editor',
  (SELECT id FROM users WHERE username = 'admin'),
  datetime('now')
);
```

### Test Access

1. Login as admin → Should see all projects, all settings
2. Login as john → Should only see assigned projects, no settings
3. Try to create project as john → Should fail with 403
4. Try to access unassigned project as john → Should fail with 403

## Troubleshooting

### "403 Forbidden" when accessing project
- Check if user has entry in `project_members` table for that project
- Verify user account is active (`is_active = 1`)
- Check if user has sufficient role for the operation

### User can't see any projects
- Check `project_members` table has entries for that user
- Verify project IDs are correct (UUID format)
- Check if user is active

### Regular user sees admin features
- Verify `is_admin = 0` in users table
- Check frontend is correctly reading user role from `/api/auth/me`
- Clear browser cookies and re-login

## Migration Notes

If you have existing projects and users, you need to:

1. **Decide on access strategy**:
   - Option A: Give all existing users owner access to all projects
   - Option B: Manually assign users to their relevant projects

2. **Run migration SQL** (Option A - give everyone access):
```sql
-- Give all non-admin users owner access to all projects
INSERT INTO project_members (id, project_id, user_id, role, granted_by, granted_at)
SELECT 
  randomblob(16),
  p.id,
  u.id,
  'owner',
  NULL,
  datetime('now')
FROM projects p
CROSS JOIN users u
WHERE u.is_admin = 0;
```

3. **Or manually assign** through the UI:
   - Login as admin
   - Go to Settings → Projects
   - Click "Manage Access" on each project
   - Add users with appropriate roles
