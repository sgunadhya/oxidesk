-- Create messages table for PostgreSQL
-- Messages are the communication units between agents and contacts
-- Supports incoming (from customers) and outgoing (from agents) messages
-- Includes delivery status tracking and retry logic

CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    type TEXT NOT NULL CHECK (type IN ('incoming', 'outgoing')),
    status TEXT NOT NULL CHECK (status IN ('received', 'pending', 'sent', 'failed')),
    content TEXT NOT NULL,
    author_id TEXT NOT NULL, -- User ID (agent or contact)
    is_immutable BOOLEAN NOT NULL DEFAULT FALSE,
    retry_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL,
    sent_at TIMESTAMPTZ, -- Timestamp when message was successfully sent (for outgoing messages)
    updated_at TIMESTAMPTZ NOT NULL,

    CONSTRAINT fk_message_conversation FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
    CONSTRAINT fk_message_author FOREIGN KEY (author_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Index for fetching messages by conversation (most common query)
CREATE INDEX IF NOT EXISTS idx_messages_conversation_id ON messages(conversation_id);

-- Index for fetching messages by status (for delivery queue processing)
CREATE INDEX IF NOT EXISTS idx_messages_status ON messages(status);

-- Index for fetching messages by type
CREATE INDEX IF NOT EXISTS idx_messages_type ON messages(type);

-- Composite index for delivery queue queries (pending/failed messages with retry logic)
CREATE INDEX IF NOT EXISTS idx_messages_status_retry ON messages(status, retry_count);

-- Index for timestamp-based queries
CREATE INDEX IF NOT EXISTS idx_messages_created_at ON messages(created_at DESC);
