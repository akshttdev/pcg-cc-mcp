# Testing Nora Project Management Features

## Quick Test Guide

### Prerequisites
```bash
# Start the development server
npm run dev
```

The server will start on the configured port (check terminal output).

### Test 1: Create a Project

```bash
curl -X POST http://localhost:3000/api/nora/project/create \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test WebApp",
    "gitRepoPath": "/Users/yourname/projects/webapp"
  }'
```

**Expected Response:**
```json
{
  "projectId": "uuid-here",
  "name": "Test WebApp",
  "gitRepoPath": "/Users/yourname/projects/webapp",
  "createdAt": "2024-01-15T10:30:00Z"
}
```

Save the `projectId` for the next test.

### Test 2: Create a Kanban Board

```bash
curl -X POST http://localhost:3000/api/nora/board/create \
  -H "Content-Type: application/json" \
  -d '{
    "projectId": "paste-project-id-here",
    "name": "Sprint 1 - Q1 2024",
    "description": "First sprint tasks",
    "boardType": "custom"
  }'
```

**Expected Response:**
```json
{
  "boardId": "uuid-here",
  "projectId": "project-uuid",
  "name": "Sprint 1 - Q1 2024",
  "boardType": "Custom",
  "createdAt": "2024-01-15T10:31:00Z"
}
```

Save the `boardId` for the next test.

### Test 3: Create a Task on the Board

```bash
curl -X POST http://localhost:3000/api/nora/task/create \
  -H "Content-Type: application/json" \
  -d '{
    "projectId": "paste-project-id-here",
    "boardId": "paste-board-id-here",
    "title": "Implement user authentication",
    "description": "Add JWT-based authentication with OAuth support",
    "priority": "high",
    "tags": ["backend", "security", "auth"]
  }'
```

**Expected Response:**
```json
{
  "taskId": "uuid-here",
  "projectId": "project-uuid",
  "boardId": "board-uuid",
  "title": "Implement user authentication",
  "status": "Pending",
  "priority": "High",
  "createdAt": "2024-01-15T10:32:00Z"
}
```

## Test via Nora Chat

If Nora is running, you can test via natural language:

1. **Create a project:**
   ```
   "Nora, create a new project called 'Mobile App' with a git repo at /projects/mobile"
   ```

2. **Create a board:**
   ```
   "Create a dev board called 'Sprint 1' in the Mobile App project"
   ```

3. **Create tasks:**
   ```
   "Add a high priority task to Sprint 1: 'Setup React Native project'"
   ```

## Verification

Check the database to verify records were created:

```bash
# If using SQLite
sqlite3 dev_assets/data.db "SELECT * FROM projects WHERE name LIKE '%Test%';"
sqlite3 dev_assets/data.db "SELECT * FROM project_boards WHERE name LIKE '%Sprint%';"
sqlite3 dev_assets/data.db "SELECT * FROM tasks WHERE title LIKE '%authentication%';"
```

## Board Types

Valid `boardType` values:
- `"custom"` - General purpose kanban board
- `"executive_assets"` - Executive planning board
- `"brand_assets"` - Brand assets management
- `"dev_assets"` - Development tasks board
- `"social_assets"` - Social media content board

## Priority Levels

Valid `priority` values:
- `"critical"` - Urgent, blocking issues
- `"high"` - Important tasks
- `"medium"` - Standard priority
- `"low"` - Nice to have

## Common Issues

### "Nora not initialized"
Make sure Nora instance is running. Check server logs for initialization errors.

### Invalid UUID format
Ensure you're using valid UUIDs from previous API responses. UUIDs must be in format: `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`

### Board type not recognized
Use exact strings from the Board Types list above. Case-insensitive matching is supported.

## Success Criteria

✅ Project created with valid UUID
✅ Board created and linked to project
✅ Tasks created and appear on the board
✅ All timestamps in ISO 8601 format
✅ Enums properly formatted (e.g., "High" not "high")
✅ Optional fields handled correctly (null/undefined)

## Next Steps

After successful API testing:
1. Test Nora tool execution (requires frontend integration)
2. Verify board visualization in UI
3. Test multi-step workflows (create project → board → tasks in one conversation)
4. Add drag-and-drop task management
