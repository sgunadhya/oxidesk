-- Migration 014: Create assignment_history table
-- Feature: 004-conversation-assignment
-- Description: Audit trail for all assignment changes

CREATE TABLE assignment_history (
    id TEXT PRIMARY KEY NOT NULL,
    conversation_id TEXT NOT NULL,
    assigned_user_id TEXT,
    assigned_team_id TEXT,
    assigned_by TEXT NOT NULL,
    assigned_at TEXT NOT NULL,
    unassigned_at TEXT,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
    FOREIGN KEY (assigned_by) REFERENCES users(id) ON DELETE SET NULL
);

CREATE INDEX idx_assignment_history_conversation ON assignment_history(conversation_id);
CREATE INDEX idx_assignment_history_user ON assignment_history(assigned_user_id);
