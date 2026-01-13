-- Create messages table for MySQL
-- Messages are the communication units between agents and contacts
-- Supports incoming (from customers) and outgoing (from agents) messages
-- Includes delivery status tracking and retry logic

CREATE TABLE IF NOT EXISTS messages (
    id VARCHAR(255) PRIMARY KEY,
    conversation_id VARCHAR(255) NOT NULL,
    type VARCHAR(50) NOT NULL CHECK (type IN ('incoming', 'outgoing')),
    status VARCHAR(50) NOT NULL CHECK (status IN ('received', 'pending', 'sent', 'failed')),
    content TEXT NOT NULL,
    author_id VARCHAR(255) NOT NULL,
    is_immutable BOOLEAN NOT NULL DEFAULT FALSE,
    retry_count INT NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL,
    sent_at DATETIME NULL,
    updated_at DATETIME NOT NULL,

    CONSTRAINT fk_message_conversation FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
    CONSTRAINT fk_message_author FOREIGN KEY (author_id) REFERENCES users(id) ON DELETE CASCADE,

    INDEX idx_messages_conversation_id (conversation_id),
    INDEX idx_messages_status (status),
    INDEX idx_messages_type (type),
    INDEX idx_messages_status_retry (status, retry_count),
    INDEX idx_messages_created_at (created_at DESC)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
