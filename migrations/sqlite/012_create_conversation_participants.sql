-- Migration 012: Create conversation_participants table
-- Feature: 004-conversation-assignment
-- Description: Track which users (agents) are participants in conversations

CREATE TABLE conversation_participants (
    id TEXT PRIMARY KEY NOT NULL,
    conversation_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    added_at TEXT NOT NULL,
    added_by TEXT, -- Can be null if system-added
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE(conversation_id, user_id)
);

CREATE INDEX idx_participants_conversation ON conversation_participants(conversation_id);
CREATE INDEX idx_participants_user ON conversation_participants(user_id);
