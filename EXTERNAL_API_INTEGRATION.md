# External API Integration Guide

This guide covers the implementation of real external API integrations for Nora's communication and scheduling tools, replacing placeholder implementations with production-ready functionality.

## Overview

Three external integrations have been implemented:

1. **SMTP Email** - Send emails via any SMTP server (Gmail, Office 365, SendGrid, etc.)
2. **Discord Webhooks** - Post messages to Discord channels
3. **Google Calendar API** - Create events, check availability, find meeting slots

## Architecture

### Code Structure

```
crates/nora/
├── src/
│   ├── integrations.rs          # New: External API clients
│   ├── tools.rs                 # Updated: Uses real integrations
│   └── lib.rs                   # Updated: Exports integrations module
├── Cargo.toml                   # Updated: Added lettre, google-calendar3, yup-oauth2
```

### Integration Flow

```
User Request (via API/Voice)
    ↓
NoraExecutiveTool enum variant (SendEmail/SendDiscordMessage/CreateCalendarEvent)
    ↓
ExecutiveTools.execute_tool()
    ↓
Try real service (EmailService/DiscordService/CalendarService)
    ↓
Success: Return real result | Failure: Fallback to mock/log
```

## Implementation Details

### 1. SMTP Email Integration

**Library**: `lettre` v0.11 (Rust SMTP client with TLS support)

**Configuration** (environment variables):
```bash
SMTP_HOST=smtp.gmail.com
SMTP_PORT=587
SMTP_USERNAME=your-email@example.com
SMTP_PASSWORD=your-app-password
SMTP_FROM_EMAIL=your-email@example.com
SMTP_FROM_NAME=Nora AI Assistant
SMTP_USE_TLS=true
```

**Features**:
- ✅ STARTTLS encryption support
- ✅ Multiple recipients
- ✅ HTML/Plain text emails
- ✅ Connection pooling (via lettre's pool feature)
- ✅ Graceful fallback to logging if not configured
- ✅ Detailed error messages

**Code Location**: `crates/nora/src/integrations.rs:26-137`

**Example Usage**:
```rust
let email_service = EmailService::from_env()?;
let message_id = email_service.send_email(
    &["recipient@example.com"],
    "Subject",
    "Body text",
    false  // is_html
).await?;
```

### 2. Discord Webhook Integration

**Library**: `reqwest` v0.12 (HTTP client)

**Configuration** (environment variables):
```bash
DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/123456789/AbCdEfGhIjKlMnOpQrStUvWxYz
```

**Features**:
- ✅ Simple message posting
- ✅ User mentions (<@user_id> format)
- ✅ Rich embeds (title, description, color, timestamp)
- ✅ Custom username/avatar (via webhook payload)
- ✅ Graceful fallback to logging if not configured
- ✅ HTTP error handling

**Code Location**: `crates/nora/src/integrations.rs:139-278`

**Example Usage**:
```rust
let discord_service = DiscordService::from_env()?;
discord_service.send_message(
    "Hello from Nora!",
    &["123456789"]  // User IDs to mention
).await?;

// Or send rich embed
discord_service.send_embed(
    "Status Update",
    "All systems operational",
    Some(0x00FF00)  // Green color
).await?;
```

### 3. Google Calendar API Integration

**Libraries**:
- `google-calendar3` v5.0 (Google Calendar API client)
- `yup-oauth2` v10.0 (OAuth 2.0 authentication)

**Configuration** (environment variables):
```bash
GOOGLE_CALENDAR_CREDENTIALS=./credentials/google-service-account.json
GOOGLE_CALENDAR_ID=primary
```

**Features**:
- ✅ Create calendar events with attendees
- ✅ Check user availability (freebusy query)
- ✅ Find available meeting slots
- ✅ Service account authentication (server-to-server)
- ✅ Timezone-aware (uses DateTime<Utc>)
- ✅ Graceful fallback to mock data if not configured

**Code Location**: `crates/nora/src/integrations.rs:280-530`

**Example Usage**:
```rust
let calendar_service = CalendarService::from_env()?;

// Create event
let event_id = calendar_service.create_event(
    "Team Meeting",
    start_time,
    end_time,
    &["colleague@example.com"],
    Some("Conference Room A")
).await?;

// Check availability
let is_available = calendar_service.check_availability(
    "user@example.com",
    start_time,
    end_time
).await?;

// Find slots
let slots = calendar_service.find_available_slots(
    &["user1@example.com", "user2@example.com"],
    60,  // duration in minutes
    7    // days ahead
).await?;
```

## Setup Instructions

### Prerequisites

1. **Rust Dependencies**: Already added to `Cargo.toml`
2. **Environment Variables**: Copy `.env.integration.example` to `.env`

### SMTP Email Setup

#### Gmail

1. Enable 2-Factor Authentication on your Google account
2. Generate App Password:
   - Go to https://myaccount.google.com/apppasswords
   - Select "Mail" and "Other (Custom name)"
   - Copy the 16-character password
3. Configure `.env`:
   ```bash
   SMTP_HOST=smtp.gmail.com
   SMTP_PORT=587
   SMTP_USERNAME=your-email@gmail.com
   SMTP_PASSWORD=your-16-char-app-password
   SMTP_FROM_EMAIL=your-email@gmail.com
   SMTP_USE_TLS=true
   ```

#### Office 365 / Outlook.com

1. Go to https://account.microsoft.com/security
2. Enable "App passwords" under Advanced security options
3. Create app password
4. Configure `.env`:
   ```bash
   SMTP_HOST=smtp.office365.com
   SMTP_PORT=587
   SMTP_USERNAME=your-email@outlook.com
   SMTP_PASSWORD=your-app-password
   SMTP_FROM_EMAIL=your-email@outlook.com
   SMTP_USE_TLS=true
   ```

#### SendGrid / Other SMTP Services

1. Get API key from your provider
2. Configure `.env`:
   ```bash
   SMTP_HOST=smtp.sendgrid.net
   SMTP_PORT=587
   SMTP_USERNAME=apikey
   SMTP_PASSWORD=your-sendgrid-api-key
   SMTP_FROM_EMAIL=verified-sender@yourdomain.com
   SMTP_USE_TLS=true
   ```

### Discord Webhook Setup

1. Open Discord server settings
2. Navigate to **Integrations** → **Webhooks**
3. Click **New Webhook**
4. Configure:
   - **Name**: Nora AI Assistant
   - **Channel**: Select target channel
   - **Avatar**: (optional) Upload Nora's icon
5. Click **Copy Webhook URL**
6. Add to `.env`:
   ```bash
   DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/YOUR_WEBHOOK_ID/YOUR_WEBHOOK_TOKEN
   ```

### Google Calendar Setup

#### 1. Create Google Cloud Project

1. Go to https://console.cloud.google.com/
2. Create new project or select existing
3. Enable Google Calendar API:
   - Navigate to **APIs & Services** → **Library**
   - Search "Google Calendar API"
   - Click **Enable**

#### 2. Create Service Account

1. Navigate to **IAM & Admin** → **Service Accounts**
2. Click **Create Service Account**
3. Fill in:
   - **Name**: nora-calendar-service
   - **Description**: Nora AI Calendar Integration
4. Click **Create and Continue**
5. Skip role assignment (Step 2)
6. Click **Done**

#### 3. Generate Credentials

1. Click on the service account you just created
2. Go to **Keys** tab
3. Click **Add Key** → **Create new key**
4. Select **JSON** format
5. Click **Create** (file downloads automatically)
6. Move file to secure location: `./credentials/google-service-account.json`
7. Set permissions: `chmod 600 ./credentials/google-service-account.json`

#### 4. Share Calendar with Service Account

1. Open Google Calendar (https://calendar.google.com)
2. Settings → Settings for my calendars → Select calendar
3. Scroll to "Share with specific people"
4. Click **Add people**
5. Enter the service account email (from JSON file: `client_email` field)
6. Set permission to **Make changes to events**
7. Click **Send**

#### 5. Configure Environment

Add to `.env`:
```bash
GOOGLE_CALENDAR_CREDENTIALS=./credentials/google-service-account.json
GOOGLE_CALENDAR_ID=primary  # or specific calendar ID from settings
```

## Testing

### Test Email

```bash
curl -X POST http://localhost:8080/api/nora/tools/execute \
  -H "Content-Type: application/json" \
  -d '{
    "tool": {
      "SendEmail": {
        "recipients": ["test@example.com"],
        "subject": "Test Email from Nora",
        "body": "This is a test email sent via SMTP",
        "priority": "Normal"
      }
    }
  }'
```

**Expected Response** (configured):
```json
{
  "success": true,
  "recipients": ["test@example.com"],
  "subject": "Test Email from Nora",
  "priority": "Normal",
  "message_id": "550e8400-e29b-41d4-a716-446655440000",
  "sent_via": "SMTP"
}
```

**Expected Response** (not configured):
```json
{
  "success": true,
  "recipients": ["test@example.com"],
  "subject": "Test Email from Nora",
  "priority": "Normal",
  "message_id": "550e8400-e29b-41d4-a716-446655440000",
  "note": "SMTP not configured - email logged only. Set SMTP_USERNAME, SMTP_PASSWORD, SMTP_FROM_EMAIL env vars to enable."
}
```

### Test Discord

```bash
curl -X POST http://localhost:8080/api/nora/tools/execute \
  -H "Content-Type: application/json" \
  -d '{
    "tool": {
      "SendDiscordMessage": {
        "channel": "#general",
        "message": "Hello from Nora AI! 👋",
        "mention_users": []
      }
    }
  }'
```

**Expected Response** (configured):
```json
{
  "success": true,
  "channel": "#general",
  "message": "Hello from Nora AI! 👋",
  "mentioned_users": [],
  "timestamp": "2025-11-20T12:00:00Z",
  "sent_via": "Discord Webhook"
}
```

### Test Calendar

```bash
curl -X POST http://localhost:8080/api/nora/tools/execute \
  -H "Content-Type: application/json" \
  -d '{
    "tool": {
      "CreateCalendarEvent": {
        "title": "Team Standup",
        "start_time": "2025-11-21T10:00:00Z",
        "end_time": "2025-11-21T10:30:00Z",
        "attendees": ["colleague@example.com"],
        "location": "Zoom"
      }
    }
  }'
```

**Expected Response** (configured):
```json
{
  "success": true,
  "event_id": "abc123xyz",
  "title": "Team Standup",
  "start_time": "2025-11-21T10:00:00Z",
  "end_time": "2025-11-21T10:30:00Z",
  "attendees": ["colleague@example.com"],
  "location": "Zoom",
  "calendar_provider": "Google Calendar"
}
```

## Error Handling

All integrations follow a **graceful degradation** pattern:

1. **Try real service** - Attempt to use configured API
2. **Log warning** - If service fails, log error details
3. **Fallback** - Return mock/logged response with helpful note
4. **Never crash** - Always return valid JSON response

Example error flow:
```
User calls SendEmail tool
    ↓
ExecutiveTools checks if email_service is Some
    ↓
email_service.send_email() → Err("SMTP authentication failed")
    ↓
Log warning: "SMTP send failed, logging only: SMTP authentication failed"
    ↓
Return success response with note: "SMTP not configured - email logged only"
```

This ensures Nora continues to function even when external services are unavailable.

## Security Considerations

### Credentials Storage

✅ **Do**:
- Store credentials in `.env` file (gitignored)
- Use environment variables in production
- Use app-specific passwords (not main account passwords)
- Set file permissions: `chmod 600 .env credentials/*.json`
- Rotate credentials regularly

❌ **Don't**:
- Commit credentials to version control
- Share `.env` files
- Use plaintext passwords in code
- Store credentials in frontend/client code

### API Access Controls

- **SMTP**: Use app passwords, enable 2FA on email accounts
- **Discord**: Limit webhook to specific channel, regenerate if leaked
- **Google Calendar**: Service account with minimal permissions (calendar access only)

### Data Privacy

- **Email content**: May contain sensitive information, ensure SMTP uses TLS
- **Calendar events**: May reveal schedules, restrict service account access
- **Discord messages**: Visible to all channel members, avoid PII

## Troubleshooting

### SMTP Errors

**"Authentication failed"**
- Check `SMTP_USERNAME` and `SMTP_PASSWORD` are correct
- For Gmail: Ensure using App Password, not regular password
- Verify 2FA is enabled for Gmail

**"Connection timeout"**
- Check firewall allows outbound connections on port 587
- Try alternative port (465 for SSL, 25 for non-encrypted)
- Verify `SMTP_HOST` is correct

**"Relay access denied"**
- Ensure `SMTP_FROM_EMAIL` matches authenticated user
- For corporate SMTP: Check if IP/user is whitelisted

### Discord Webhook Errors

**"Invalid Webhook Token"**
- Regenerate webhook URL from Discord server settings
- Ensure no spaces/newlines in `DISCORD_WEBHOOK_URL`

**"Unknown Webhook"**
- Webhook may have been deleted, create new one
- Check webhook URL is complete (includes token)

**Rate limiting**
- Discord limits: 5 requests per 2 seconds per webhook
- Add delays between rapid messages
- Use embed batching for multiple updates

### Google Calendar Errors

**"Invalid credentials"**
- Verify JSON file path is correct
- Check JSON format is valid (not corrupted)
- Ensure service account email is shared on calendar

**"Insufficient permissions"**
- Grant service account "Make changes to events" permission
- Share calendar with exact email from JSON (`client_email`)

**"Calendar not found"**
- Check `GOOGLE_CALENDAR_ID` (use "primary" for main calendar)
- Verify calendar exists and is accessible

## Performance Considerations

### Connection Pooling

- **SMTP**: `lettre` supports connection pooling (enabled via `pool` feature)
- **Discord**: Single webhook URL, no connection overhead
- **Google Calendar**: OAuth token cached by `yup-oauth2`

### Rate Limits

| Service | Limit | Mitigation |
|---------|-------|------------|
| Gmail SMTP | 500 emails/day (free), 2000/day (Workspace) | Use SendGrid for high volume |
| Discord Webhooks | 5 requests/2 sec, 30/min | Queue messages, batch embeds |
| Google Calendar API | 1,000,000 queries/day | Cache availability checks |

### Async Execution

All integrations use `async/await`:
```rust
pub async fn send_email(...) -> Result<String>
pub async fn send_message(...) -> Result<()>
pub async fn create_event(...) -> Result<String>
```

This prevents blocking the main Nora thread during network I/O.

## Future Enhancements

### Planned Features

- [ ] **Email templates**: HTML templates with variables
- [ ] **Email attachments**: Support sending files
- [ ] **Discord slash commands**: Two-way Discord integration
- [ ] **Calendar recurring events**: Support RRULE
- [ ] **Multi-calendar support**: Query multiple calendars simultaneously
- [ ] **Notification webhooks**: Generic webhook system for any service
- [ ] **Retry logic**: Automatic retries with exponential backoff
- [ ] **Circuit breaker**: Fail fast when service is down

### Alternative Services

Consider adding support for:
- **Email**: AWS SES, Mailgun, Postmark
- **Chat**: Slack, Microsoft Teams, Telegram
- **Calendar**: Microsoft Outlook Calendar, Calendly

## Changelog

### v0.0.97 (2025-11-20)

**Added**:
- SMTP email integration using `lettre`
- Discord webhook integration using `reqwest`
- Google Calendar API integration using `google-calendar3` and `yup-oauth2`
- Configuration via environment variables
- Graceful fallback to mock/log responses
- Comprehensive error handling
- `.env.integration.example` template
- This integration guide

**Changed**:
- `SendSlackMessage` renamed to `SendDiscordMessage`
- `tools.rs`: Uses real services instead of placeholders
- `ExecutiveTools::new()`: Initializes service clients from env

**Dependencies Added**:
- `lettre = "0.11"` (SMTP)
- `google-calendar3 = "5.0"` (Calendar API)
- `yup-oauth2 = "10.0"` (OAuth 2.0)

## References

- [lettre documentation](https://docs.rs/lettre/)
- [Discord Webhooks Guide](https://discord.com/developers/docs/resources/webhook)
- [Google Calendar API Reference](https://developers.google.com/calendar/api/v3/reference)
- [OAuth 2.0 Service Accounts](https://developers.google.com/identity/protocols/oauth2/service-account)
