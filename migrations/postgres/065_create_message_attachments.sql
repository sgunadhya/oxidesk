-- Migration 065: Create message_attachments table
-- PostgreSQL version
-- Feature: 021-email-integration

CREATE TABLE message_attachments (
    id TEXT PRIMARY KEY NOT NULL,
    message_id TEXT NOT NULL,

    -- File metadata
    filename TEXT NOT NULL,
    content_type TEXT,  -- MIME type (e.g., "image/png", "application/pdf")
    file_size BIGINT NOT NULL,  -- Size in bytes
    file_path TEXT NOT NULL,  -- Absolute path on disk

    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,

    FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE
);

-- Index for querying attachments by message
CREATE INDEX idx_message_attachments_message_id ON message_attachments(message_id);
