-- Migration 011: Add assignment fields to conversations
-- Feature: 004-conversation-assignment
-- Description: Add fields to track user and team assignment

ALTER TABLE conversations ADD COLUMN assigned_user_id VARCHAR(255);
ALTER TABLE conversations ADD COLUMN assigned_team_id VARCHAR(255);
ALTER TABLE conversations ADD COLUMN assigned_at DATETIME;
ALTER TABLE conversations ADD COLUMN assigned_by VARCHAR(255);

ALTER TABLE conversations ADD CONSTRAINT fk_assigned_user
    FOREIGN KEY (assigned_user_id) REFERENCES users(id) ON DELETE SET NULL;

ALTER TABLE conversations ADD CONSTRAINT fk_assigned_team
    FOREIGN KEY (assigned_team_id) REFERENCES teams(id) ON DELETE SET NULL;

CREATE INDEX idx_conversations_assigned_user ON conversations(assigned_user_id);
CREATE INDEX idx_conversations_assigned_team ON conversations(assigned_team_id);
