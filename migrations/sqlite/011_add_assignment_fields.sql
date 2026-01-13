-- Migration 011: Add assignment fields to conversations
-- Feature: 004-conversation-assignment
-- Description: Add fields to track user and team assignment

ALTER TABLE conversations ADD COLUMN assigned_user_id TEXT;
ALTER TABLE conversations ADD COLUMN assigned_team_id TEXT;
ALTER TABLE conversations ADD COLUMN assigned_at TEXT;
ALTER TABLE conversations ADD COLUMN assigned_by TEXT;

-- Note: SQLite doesn't support ADD CONSTRAINT after table creation
-- Foreign keys will be enforced if PRAGMA foreign_keys=ON is set

CREATE INDEX idx_conversations_assigned_user ON conversations(assigned_user_id);
CREATE INDEX idx_conversations_assigned_team ON conversations(assigned_team_id);
