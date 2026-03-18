# Quick Start: Testing Multi-User Authentication

## Prerequisites
- PostgreSQL running locally or accessible
- Rust toolchain installed
- Node.js and pnpm installed

## Step 1: Set Up PostgreSQL Database

```bash
# Create database
createdb pcg_dashboard

# Set environment variable
export DATABASE_URL="postgresql://localhost/pcg_dashboard?sslmode=disable"
```

Or add to your `.env` file:
```
DATABASE_URL=postgresql://localhost/pcg_dashboard?sslmode=disable
```

## Step 2: Run Migrations

```bash
# Run the PostgreSQL migrations to create tables
sqlx migrate run --database-url "${DATABASE_URL}" --source crates/db/migrations_pg
```

## Step 3: Create Initial Admin User

You need to create at least one admin user to login. You can do this with a simple script:

### Option A: Using `psql`

```bash
# Generate a bcrypt hash for password "admin"
# You can use https://bcrypt-generator.com/ or the command below if you have bcrypt-tool installed
# Password: admin
# Hash: $2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5GyYzS.CwA0Su

psql $DATABASE_URL <<EOF
INSERT INTO users (
  id, username, email, full_name, password_hash, is_admin, is_active
) VALUES (
  gen_random_uuid(),
  'admin',
  'admin@pcg.com',
  'Admin User',
  '\$2b\$12\$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5GyYzS.CwA0Su',
  true,
  true
);
EOF
```

### Option B: Create a Rust CLI tool (recommended for production)

Create `crates/server/src/bin/create_admin.rs`:

```rust
use db::{repositories::UserRepository, services::AuthService};
use sqlx::PgPool;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    
    let pool = PgPool::connect(&database_url).await?;
    let user_repo = UserRepository::new(pool.clone());
    let auth_service = AuthService::new(pool.clone());
    
    // Create admin user
    let username = std::env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".to_string());
    let email = std::env::var("ADMIN_EMAIL").unwrap_or_else(|_| "admin@pcg.com".to_string());
    let password = std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "admin".to_string());
    let full_name = std::env::var("ADMIN_FULL_NAME").unwrap_or_else(|_| "Admin User".to_string());
    
    let password_hash = auth_service.hash_password(&password)?;
    
    let user = user_repo.create(
        username.clone(),
        email.clone(),
        full_name,
        password_hash,
        true, // is_admin
    ).await?;
    
    println!("✅ Admin user created successfully!");
    println!("   Username: {}", username);
    println!("   Email: {}", email);
    println!("   ID: {}", user.id);
    println!("\nYou can now login at http://localhost:5173/login");
    
    Ok(())
}
```

Then run:
```bash
cargo run --bin create_admin --features postgres
```

## Step 4: Start the Backend

```bash
# Start backend with PostgreSQL feature enabled
pnpm run backend:dev

# Or manually:
cd crates/server
cargo run --features postgres
```

The backend should start on port 3000 (or your configured port).

## Step 5: Start the Frontend

```bash
# In a separate terminal
pnpm run frontend:dev
```

The frontend should start on port 5173 (or your configured port).

## Step 6: Test the Authentication Flow

1. **Navigate to the app**: Open http://localhost:5173
   - You should be automatically redirected to http://localhost:5173/login

2. **Login with admin credentials**:
   - Username: `admin`
   - Password: `admin` (or whatever you set)
   - Click "Sign in"

3. **Verify successful login**:
   - You should be redirected to the dashboard
   - Profile section in navbar should show "Admin User"
   - Status indicator should show "Online"

4. **Test profile menu**:
   - Click on the avatar/name in the navbar
   - Should see user details, email, "Admin" badge
   - Should see "Admin Panel" link (because you're an admin)
   - Should see settings links and logout button

5. **Test protected routes**:
   - Navigate to different pages (Projects, Nora, Settings)
   - All should work normally

6. **Test session persistence**:
   - Refresh the page (Cmd/Ctrl + R)
   - You should stay logged in
   - Close the browser and reopen
   - Navigate to http://localhost:5173
   - You should still be logged in (session cookie persists)

7. **Test logout**:
   - Click avatar → "Sign out"
   - Should be redirected to /login
   - Session should be cleared
   - Try accessing http://localhost:5173
   - Should be redirected to /login

8. **Test protection**:
   - While logged out, try accessing http://localhost:5173/projects
   - Should be redirected to /login
   - After login, should be redirected back to /projects

## Troubleshooting

### Backend won't start
- Check DATABASE_URL is set correctly
- Verify PostgreSQL is running: `psql $DATABASE_URL -c "SELECT 1"`
- Check migrations ran successfully: `sqlx migrate info --database-url "${DATABASE_URL}"`

### Login fails with "Invalid credentials"
- Verify the admin user was created: `psql $DATABASE_URL -c "SELECT username, email, is_admin FROM users"`
- Check the password hash is correct
- Look at backend logs for detailed error messages

### Frontend can't connect to backend
- Check backend is running on the correct port
- Verify VITE_API_URL in frontend/.env points to backend
- Check browser console for CORS or network errors

### Session not persisting
- Check browser allows cookies
- Verify session was created: `psql $DATABASE_URL -c "SELECT * FROM sessions"`
- Check session hasn't expired
- Look for cookie in browser DevTools → Application → Cookies

### TypeScript errors
- Run `pnpm run frontend:check` to see all errors
- Check AuthContext is properly wrapped around routes in App.tsx
- Verify shared types are up to date: `pnpm run generate-types`

## Next Development Steps

Once authentication is working:

1. **Create more users**: Either manually in PostgreSQL or build the admin panel
2. **Build admin panel**: Create UI for user management at /admin
3. **Add organizations**: Create organizations and assign users
4. **Multi-tenancy**: Update projects/assets to belong to organizations
5. **Test team features**: Test multiple users accessing shared resources

## Default Admin Credentials

**⚠️ SECURITY WARNING**: Change these credentials immediately in production!

- Username: `admin`
- Password: `admin`
- Email: `admin@pcg.com`

## Environment Variables Reference

### Backend (.env in project root)
```bash
DATABASE_URL=postgresql://localhost/pcg_dashboard?sslmode=disable
BACKEND_PORT=3000
HOST=localhost
RUST_LOG=info
```

### Frontend (frontend/.env)
```bash
VITE_API_URL=http://localhost:3000
```

## Database Connection String Format

```
postgresql://[user]:[password]@[host]:[port]/[database]?sslmode=[mode]
```

Examples:
```bash
# Local development
postgresql://localhost/pcg_dashboard?sslmode=disable

# With credentials
postgresql://pcguser:password@localhost:5432/pcg_dashboard?sslmode=disable

# Remote server with SSL
postgresql://user:pass@db.example.com:5432/pcg_prod?sslmode=require
```

## Useful SQL Queries

### List all users
```sql
SELECT id, username, email, full_name, is_admin, is_active, created_at 
FROM users 
ORDER BY created_at DESC;
```

### List active sessions
```sql
SELECT s.id, u.username, u.email, s.created_at, s.last_accessed_at, s.expires_at
FROM sessions s
JOIN users u ON s.user_id = u.id
WHERE s.expires_at > NOW()
ORDER BY s.last_accessed_at DESC;
```

### Create additional user
```sql
INSERT INTO users (id, username, email, full_name, password_hash, is_admin, is_active)
VALUES (
  gen_random_uuid(),
  'testuser',
  'test@pcg.com',
  'Test User',
  '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5GyYzS.CwA0Su', -- password: admin
  false, -- not admin
  true
);
```

### Manually expire all sessions (force logout all users)
```sql
UPDATE sessions SET expires_at = NOW() - INTERVAL '1 day';
```

### Delete a session (logout specific user)
```sql
DELETE FROM sessions WHERE user_id = (SELECT id FROM users WHERE username = 'admin');
```

## Success Criteria

✅ Backend starts without errors
✅ Frontend starts without errors  
✅ Can access login page
✅ Can login with admin credentials
✅ Redirected to dashboard after login
✅ Profile shows correct user info
✅ Session persists across page refreshes
✅ Can logout successfully
✅ Protected routes redirect to login when not authenticated
✅ Can access protected routes when authenticated

Once all these work, the authentication system is ready for development of the admin panel!
