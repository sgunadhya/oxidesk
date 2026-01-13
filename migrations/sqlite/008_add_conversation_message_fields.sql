-- Add message-related fields to conversations table for SQLite
-- Tracks the last message and timestamps for conversation activity

ALTER TABLE conversations ADD COLUMN last_message_id TEXT;
ALTER TABLE conversations ADD COLUMN last_message_at TEXT;
ALTER TABLE conversations ADD COLUMN last_reply_at TEXT;

-- Note: Foreign key constraint cannot be added via ALTER TABLE in SQLite
-- The application layer will enforce the relationship
