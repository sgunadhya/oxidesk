-- Migration 019: Extend Agent Availability States
-- Feature: 006-agent-availability
-- Description: Add offline and away_manual states, add timestamp tracking

-- Drop existing constraint
ALTER TABLE agents DROP CONSTRAINT IF EXISTS agents_availability_status_check;

-- Modify column to support new states
ALTER TABLE agents ALTER COLUMN availability_status SET DEFAULT 'offline';

-- Add new constraint with all 5 states
ALTER TABLE agents ADD CONSTRAINT agents_availability_status_check
    CHECK (availability_status IN ('offline', 'online', 'away', 'away_manual', 'away_and_reassigning'));

-- Add timestamp tracking columns
ALTER TABLE agents ADD COLUMN IF NOT EXISTS last_login_at TIMESTAMP;
ALTER TABLE agents ADD COLUMN IF NOT EXISTS last_activity_at TIMESTAMP;
ALTER TABLE agents ADD COLUMN IF NOT EXISTS away_since TIMESTAMP;

-- Recreate/update indexes
DROP INDEX IF EXISTS idx_agents_availability;
CREATE INDEX idx_agents_availability ON agents(availability_status);
CREATE INDEX idx_agents_last_activity ON agents(last_activity_at);
