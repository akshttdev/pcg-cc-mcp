# Dropbox Integration Setup Guide

This guide walks through configuring Nora/Editron with Dropbox so that shared folders trigger automatic ingests.

## 1. Create a Dropbox App & Webhook
1. Visit <https://www.dropbox.com/developers/apps> and create a Scoped App with `files.metadata.read`, `files.content.read`, and `sharing.read` scopes.
2. Under **Webhook**, register Nora's endpoint:
   - URL: `https://<your-domain>/api/webhooks/dropbox`
   - Verify that Dropbox can reach it (you should see the challenge logged in the server output).
3. Generate an **App secret** and copy it into your environment as `DROPBOX_WEBHOOK_SECRET` (same value used for signing challenges).

## 2. Provision Access Tokens
Each Dropbox account that should trigger Editron must provide a long-lived access token.

1. From the Dropbox App console, click **Generated access token** under the appropriate user.
2. Store the token securely (1Password / Vault). You'll paste it when creating a source record.

## 3. Register Sources via API
Use the new management endpoints to map Dropbox accounts → project metadata.

```bash
# List sources
curl -H "Authorization: Bearer <your-session>" \
     https://localhost:3001/api/dropbox/sources

# Create a source
curl -X POST -H "Content-Type: application/json" \
     -H "Authorization: Bearer <token>" \
     -d '{
           "account_id": "dbid:AA...",
           "label": "ACME Event Footage",
           "source_url": "https://www.dropbox.com/scl/fo/...",
           "project_id": "<uuid>",
           "storage_tier": "hot",
           "access_token": "sl.BC...",
           "reference_name_template": "{label} – {date}",
           "ingest_strategy": "shared_link",
           "auto_ingest": true
         }'
     https://localhost:3001/api/dropbox/sources
```

## 4. Webhook Flow
1. Dropbox notifies `/api/webhooks/dropbox` with affected account IDs.
2. Nora verifies the signature, looks up matching `dropbox_sources`, and immediately queues media ingests via `MediaPipelineService`.
3. `dropbox_sources.last_processed_at` is updated so operators can see freshness.

## 5. Polling Fallback
A background monitor (`DropboxMonitor`) runs every 5 minutes. If a source hasn't ingested within 12 hours (or has never run), it automatically queues a refresh. This prevents missed footage even if Dropbox temporarily drops webhook events.

## 6. Required Environment Variables
Add the following to your `.env`:

```bash
DROPBOX_WEBHOOK_SECRET=generated_from_app_console
MEDIA_STORAGE_PATH=/absolute/path/to/media_pipeline
```

## 7. Testing Checklist
- Send a manual webhook: `curl -X POST https://localhost:3001/api/webhooks/dropbox -d '{"list_folder": {"accounts": ["dbid:AA..."]}}'`
- Confirm logs show queued batches and `dropbox_sources.last_processed_at` updates.
- Verify `/api/dropbox/sources` reflects the new timestamps.

With this setup, Dropbox uploads → webhook → auto-ingest → Editron pipeline happens without manual intervention.
