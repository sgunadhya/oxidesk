-- Add message-related fields to conversations table for PostgreSQL
-- Tracks the last message and timestamps for conversation activity

ALTER TABLE conversations ADD COLUMN IF NOT EXISTS last_message_id TEXT;
ALTER TABLE conversations ADD COLUMN IF NOT EXISTS last_message_at TIMESTAMPTZ;
ALTER TABLE conversations ADD COLUMN IF NOT EXISTS last_reply_at TIMESTAMPTZ;

-- Add foreign key constraint to link last_message_id to messages table
ALTER TABLE conversations ADD CONSTRAINT fk_conversation_last_message
    FOREIGN KEY (last_message_id) REFERENCES messages(id) ON DELETE SET NULL;
