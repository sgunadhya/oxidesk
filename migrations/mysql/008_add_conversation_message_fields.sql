-- Add message-related fields to conversations table for MySQL
-- Tracks the last message and timestamps for conversation activity

ALTER TABLE conversations ADD COLUMN last_message_id VARCHAR(255) NULL;
ALTER TABLE conversations ADD COLUMN last_message_at DATETIME NULL;
ALTER TABLE conversations ADD COLUMN last_reply_at DATETIME NULL;

-- Add foreign key constraint to link last_message_id to messages table
ALTER TABLE conversations ADD CONSTRAINT fk_conversation_last_message
    FOREIGN KEY (last_message_id) REFERENCES messages(id) ON DELETE SET NULL;

-- Add index for timestamp queries
CREATE INDEX idx_conversations_last_message_at ON conversations(last_message_at);
CREATE INDEX idx_conversations_last_reply_at ON conversations(last_reply_at);
