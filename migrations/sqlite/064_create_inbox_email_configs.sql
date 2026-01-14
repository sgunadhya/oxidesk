-- Migration 064: Create inbox_email_configs table
-- SQLite version
-- Feature: 021-email-integration

CREATE TABLE inbox_email_configs (
    id TEXT PRIMARY KEY NOT NULL,
    inbox_id TEXT UNIQUE NOT NULL,

    -- IMAP configuration for receiving emails
    imap_host TEXT NOT NULL,
    imap_port INTEGER NOT NULL DEFAULT 993,
    imap_username TEXT NOT NULL,
    imap_password TEXT NOT NULL,  -- Should be encrypted at rest in production
    imap_use_tls INTEGER NOT NULL DEFAULT 1,  -- Boolean: 0=false, 1=true
    imap_folder TEXT NOT NULL DEFAULT 'INBOX',

    -- SMTP configuration for sending emails
    smtp_host TEXT NOT NULL,
    smtp_port INTEGER NOT NULL DEFAULT 587,
    smtp_username TEXT NOT NULL,
    smtp_password TEXT NOT NULL,  -- Should be encrypted at rest in production
    smtp_use_tls INTEGER NOT NULL DEFAULT 1,  -- Boolean: 0=false, 1=true

    -- Email identity
    email_address TEXT NOT NULL,
    display_name TEXT NOT NULL,

    -- Polling configuration
    poll_interval_seconds INTEGER NOT NULL DEFAULT 30,
    enabled INTEGER NOT NULL DEFAULT 1,  -- Boolean: 0=disabled, 1=enabled
    last_poll_at TEXT,  -- ISO8601 timestamp of last successful poll

    -- Timestamps
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,

    FOREIGN KEY (inbox_id) REFERENCES inboxes(id) ON DELETE CASCADE
);

-- Indexes for performance
CREATE INDEX idx_inbox_email_configs_inbox_id ON inbox_email_configs(inbox_id);
CREATE INDEX idx_inbox_email_configs_enabled ON inbox_email_configs(enabled);
CREATE INDEX idx_inbox_email_configs_last_poll ON inbox_email_configs(last_poll_at);
