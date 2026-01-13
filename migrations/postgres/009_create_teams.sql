-- Migration 009: Create teams table
-- Feature: 004-conversation-assignment
-- Description: Teams are groups of agents that can be assigned conversations

CREATE TABLE teams (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    sla_policy_id TEXT, -- Future: FK to sla_policies (Feature 008)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_teams_name ON teams(name);
