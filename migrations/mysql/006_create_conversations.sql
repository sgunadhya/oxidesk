-- MySQL migration for conversations table
-- Feature: 002-conversation-lifecycle

CREATE TABLE IF NOT EXISTS conversations (
    id CHAR(36) PRIMARY KEY DEFAULT (UUID()),
    reference_number INT NOT NULL AUTO_INCREMENT UNIQUE,
    status ENUM('open', 'snoozed', 'resolved', 'closed') NOT NULL DEFAULT 'open',
    inbox_id CHAR(36) NOT NULL,
    contact_id CHAR(36) NOT NULL,
    subject TEXT,
    resolved_at TIMESTAMP(6) NULL,
    snoozed_until TIMESTAMP(6) NULL,
    created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    version INT NOT NULL DEFAULT 1,

    CONSTRAINT conversation_inbox_fk
        FOREIGN KEY (inbox_id) REFERENCES inboxes(id) ON DELETE RESTRICT,
    CONSTRAINT conversation_contact_fk
        FOREIGN KEY (contact_id) REFERENCES contacts(id) ON DELETE RESTRICT,
    INDEX idx_conversations_status (status),
    INDEX idx_conversations_inbox (inbox_id),
    INDEX idx_conversations_contact (contact_id),
    INDEX idx_conversations_snooze (snoozed_until),
    INDEX idx_conversations_created (created_at DESC),
    UNIQUE KEY idx_conversations_reference (reference_number)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- Start auto_increment at 100
ALTER TABLE conversations AUTO_INCREMENT = 100;
