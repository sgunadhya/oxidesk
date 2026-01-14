-- Migration 066: Create email_processing_log table
-- MySQL version
-- Feature: 021-email-integration

CREATE TABLE email_processing_log (
    id VARCHAR(255) PRIMARY KEY NOT NULL,
    inbox_id VARCHAR(255) NOT NULL,

    -- Email identifiers
    email_message_id TEXT NOT NULL,  -- Email Message-ID header (RFC 5322)
    email_uid VARCHAR(255),  -- IMAP UID for tracking processed emails

    -- Email metadata
    from_address VARCHAR(255) NOT NULL,
    subject TEXT,

    -- Processing status
    processing_status ENUM('success', 'failed', 'duplicate') NOT NULL,
    error_message TEXT,

    -- Created entities (if successful)
    conversation_id VARCHAR(255),
    message_id VARCHAR(255),

    -- Timestamps
    processed_at DATETIME NOT NULL,

    -- Unique constraint to prevent duplicate processing
    UNIQUE KEY unique_inbox_email (inbox_id, email_message_id(255)),

    FOREIGN KEY (inbox_id) REFERENCES inboxes(id) ON DELETE CASCADE,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE SET NULL,
    FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE SET NULL
);

-- Indexes for performance and auditing
CREATE INDEX idx_email_processing_log_inbox_id ON email_processing_log(inbox_id);
CREATE INDEX idx_email_processing_log_processed_at ON email_processing_log(processed_at);
CREATE INDEX idx_email_processing_log_status ON email_processing_log(processing_status);
CREATE INDEX idx_email_processing_log_conversation ON email_processing_log(conversation_id);
