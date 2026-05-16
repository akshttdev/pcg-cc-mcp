-- Email sync cursor + provider-side metadata
--
-- Adds incremental-sync cursors so the EmailSyncWorker can pull only new
-- messages on each tick (Gmail history API; Zoho modseq / last poll time;
-- IMAP UIDVALIDITY+UIDNEXT).
--
-- All columns are nullable: existing accounts have no cursor and will
-- bootstrap from a full inbox scan on the first sync pass.

ALTER TABLE email_accounts ADD COLUMN gmail_history_id TEXT;
ALTER TABLE email_accounts ADD COLUMN sync_cursor TEXT;        -- generic provider cursor (Zoho modseq, IMAP uidnext, etc.)
ALTER TABLE email_accounts ADD COLUMN sync_in_progress INTEGER NOT NULL DEFAULT 0; -- 1 while a sync pass holds the row

CREATE INDEX IF NOT EXISTS idx_email_accounts_sync_in_progress
    ON email_accounts(sync_in_progress) WHERE sync_in_progress = 1;
