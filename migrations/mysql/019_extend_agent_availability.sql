-- Migration 019: Extend Agent Availability States
-- Feature: 006-agent-availability
-- Description: Add offline and away_manual states, add timestamp tracking

-- Drop existing check constraint (MySQL 8.0.16+)
ALTER TABLE agents DROP CHECK agents_chk_1;

-- Modify column to support new states
ALTER TABLE agents MODIFY COLUMN availability_status VARCHAR(50) NOT NULL DEFAULT 'offline'
    CHECK (availability_status IN ('offline', 'online', 'away', 'away_manual', 'away_and_reassigning'));

-- Add timestamp tracking columns
ALTER TABLE agents ADD COLUMN last_login_at DATETIME NULL;
ALTER TABLE agents ADD COLUMN last_activity_at DATETIME NULL;
ALTER TABLE agents ADD COLUMN away_since DATETIME NULL;

-- Recreate/update indexes
DROP INDEX IF EXISTS idx_agents_availability ON agents;
CREATE INDEX idx_agents_availability ON agents(availability_status);
CREATE INDEX idx_agents_last_activity ON agents(last_activity_at);
