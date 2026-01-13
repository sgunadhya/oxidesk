-- Migration 025: Add business hours support and fix CASCADE deletion
-- Feature: 008-sla-policy-application
-- Description: Adds business_hours to teams table and changes applied_slas FK to CASCADE

-- Step 1: Add business_hours column to teams table
ALTER TABLE teams ADD COLUMN business_hours TEXT; -- JSON format: {"timezone": "America/New_York", "schedule": [...]}

-- Step 2: Fix CASCADE deletion for applied_slas
-- SQLite doesn't support ALTER COLUMN for foreign keys, so we need to recreate the table

-- Create new applied_slas table with CASCADE
CREATE TABLE applied_slas_new (
    id TEXT PRIMARY KEY NOT NULL,
    conversation_id TEXT UNIQUE NOT NULL,
    sla_policy_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'met', 'breached')),
    first_response_deadline_at TEXT NOT NULL,
    resolution_deadline_at TEXT NOT NULL,
    applied_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
    FOREIGN KEY (sla_policy_id) REFERENCES sla_policies(id) ON DELETE CASCADE  -- Changed from RESTRICT to CASCADE
);

-- Copy data from old table
INSERT INTO applied_slas_new SELECT * FROM applied_slas;

-- Drop old table
DROP TABLE applied_slas;

-- Rename new table
ALTER TABLE applied_slas_new RENAME TO applied_slas;

-- Recreate indexes
CREATE UNIQUE INDEX idx_applied_slas_conversation ON applied_slas(conversation_id);
CREATE INDEX idx_applied_slas_policy ON applied_slas(sla_policy_id);
CREATE INDEX idx_applied_slas_status ON applied_slas(status);
