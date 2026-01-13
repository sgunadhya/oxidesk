-- Migration 012: Create conversation_participants table
-- Feature: 004-conversation-assignment
-- Description: Track which users (agents) are participants in conversations

CREATE TABLE conversation_participants (
    id VARCHAR(255) PRIMARY KEY NOT NULL,
    conversation_id VARCHAR(255) NOT NULL,
    user_id VARCHAR(255) NOT NULL,
    added_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    added_by VARCHAR(255), -- Can be null if system-added
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE KEY unique_conversation_user (conversation_id, user_id),
    INDEX idx_participants_conversation (conversation_id),
    INDEX idx_participants_user (user_id)
);
