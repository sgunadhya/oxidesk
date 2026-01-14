-- Migration 064: Create inbox_email_configs table
-- MySQL version
-- Feature: 021-email-integration

CREATE TABLE inbox_email_configs (
    id VARCHAR(255) PRIMARY KEY NOT NULL,
    inbox_id VARCHAR(255) UNIQUE NOT NULL,

    -- IMAP configuration for receiving emails
    imap_host VARCHAR(255) NOT NULL,
    imap_port INT NOT NULL DEFAULT 993,
    imap_username VARCHAR(255) NOT NULL,
    imap_password VARCHAR(255) NOT NULL,  -- Should be encrypted at rest in production
    imap_use_tls TINYINT(1) NOT NULL DEFAULT 1,
    imap_folder VARCHAR(255) NOT NULL DEFAULT 'INBOX',

    -- SMTP configuration for sending emails
    smtp_host VARCHAR(255) NOT NULL,
    smtp_port INT NOT NULL DEFAULT 587,
    smtp_username VARCHAR(255) NOT NULL,
    smtp_password VARCHAR(255) NOT NULL,  -- Should be encrypted at rest in production
    smtp_use_tls TINYINT(1) NOT NULL DEFAULT 1,

    -- Email identity
    email_address VARCHAR(255) NOT NULL,
    display_name VARCHAR(255) NOT NULL,

    -- Polling configuration
    poll_interval_seconds INT NOT NULL DEFAULT 30,
    enabled TINYINT(1) NOT NULL DEFAULT 1,
    last_poll_at DATETIME,

    -- Timestamps
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL,

    FOREIGN KEY (inbox_id) REFERENCES inboxes(id) ON DELETE CASCADE
);

-- Indexes for performance
CREATE INDEX idx_inbox_email_configs_inbox_id ON inbox_email_configs(inbox_id);
CREATE INDEX idx_inbox_email_configs_enabled ON inbox_email_configs(enabled);
CREATE INDEX idx_inbox_email_configs_last_poll ON inbox_email_configs(last_poll_at);
