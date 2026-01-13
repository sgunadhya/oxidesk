-- Migration 009: Create teams table
-- Feature: 004-conversation-assignment
-- Description: Teams are groups of agents that can be assigned conversations

CREATE TABLE teams (
    id VARCHAR(255) PRIMARY KEY NOT NULL,
    name VARCHAR(255) NOT NULL UNIQUE,
    description TEXT,
    sla_policy_id VARCHAR(255), -- Future: FK to sla_policies (Feature 008)
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_teams_name (name)
);
