-- Migration 014: Create assignment_history table
-- Feature: 004-conversation-assignment
-- Description: Audit trail for all assignment changes

CREATE TABLE assignment_history (
    id VARCHAR(255) PRIMARY KEY NOT NULL,
    conversation_id VARCHAR(255) NOT NULL,
    assigned_user_id VARCHAR(255),
    assigned_team_id VARCHAR(255),
    assigned_by VARCHAR(255) NOT NULL,
    assigned_at DATETIME NOT NULL,
    unassigned_at DATETIME,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
    FOREIGN KEY (assigned_by) REFERENCES users(id) ON DELETE SET NULL,
    INDEX idx_assignment_history_conversation (conversation_id),
    INDEX idx_assignment_history_user (assigned_user_id)
);
