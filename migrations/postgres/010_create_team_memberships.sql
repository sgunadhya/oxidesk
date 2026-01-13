-- Migration 010: Create team_memberships table
-- Feature: 004-conversation-assignment
-- Description: Many-to-many relationship between teams and users (agents)

CREATE TABLE team_memberships (
    id TEXT PRIMARY KEY NOT NULL,
    team_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('member', 'lead')),
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE(team_id, user_id)
);

CREATE INDEX idx_team_memberships_team ON team_memberships(team_id);
CREATE INDEX idx_team_memberships_user ON team_memberships(user_id);
