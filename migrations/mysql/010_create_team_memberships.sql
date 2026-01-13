-- Migration 010: Create team_memberships table
-- Feature: 004-conversation-assignment
-- Description: Many-to-many relationship between teams and users (agents)

CREATE TABLE team_memberships (
    id VARCHAR(255) PRIMARY KEY NOT NULL,
    team_id VARCHAR(255) NOT NULL,
    user_id VARCHAR(255) NOT NULL,
    role VARCHAR(50) NOT NULL CHECK(role IN ('member', 'lead')),
    joined_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE KEY unique_team_user (team_id, user_id),
    INDEX idx_team_memberships_team (team_id),
    INDEX idx_team_memberships_user (user_id)
);
