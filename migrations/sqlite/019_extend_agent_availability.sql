-- Migration 019: Extend Agent Availability States
-- Feature: 006-agent-availability
-- Description: Add offline and away_manual states, add timestamp tracking

-- SQLite doesn't support modifying CHECK constraints, so we need to recreate the column
-- First, we'll create a new table with the updated schema

CREATE TABLE agents_new (
    id TEXT PRIMARY KEY,
    user_id TEXT UNIQUE NOT NULL,
    first_name TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    availability_status TEXT NOT NULL DEFAULT 'offline'
        CHECK(availability_status IN ('offline', 'online', 'away', 'away_manual', 'away_and_reassigning')),
    last_login_at TEXT,
    last_activity_at TEXT,
    away_since TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Copy data from old table (map existing values)
INSERT INTO agents_new (id, user_id, first_name, password_hash, availability_status, last_login_at, last_activity_at, away_since, created_at, updated_at)
SELECT
    id,
    user_id,
    first_name,
    password_hash,
    CASE availability_status
        WHEN 'online' THEN 'online'
        WHEN 'away' THEN 'away'
        WHEN 'away_and_reassigning' THEN 'away_and_reassigning'
        ELSE 'offline'
    END,
    NULL, -- last_login_at
    NULL, -- last_activity_at
    NULL, -- away_since
    datetime('now'), -- created_at
    datetime('now')  -- updated_at
FROM agents;

-- Drop old table
DROP TABLE agents;

-- Rename new table
ALTER TABLE agents_new RENAME TO agents;

-- Recreate indexes
CREATE INDEX idx_agents_availability ON agents(availability_status);
CREATE INDEX idx_agents_last_activity ON agents(last_activity_at);
