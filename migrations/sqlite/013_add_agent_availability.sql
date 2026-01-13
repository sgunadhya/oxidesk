-- Migration 013: Add availability_status to agents
-- Feature: 004-conversation-assignment
-- Description: Track agent availability for auto-unassignment feature

ALTER TABLE agents ADD COLUMN availability_status TEXT NOT NULL DEFAULT 'online'
    CHECK(availability_status IN ('online', 'away', 'away_and_reassigning'));

CREATE INDEX idx_agents_availability ON agents(availability_status);
