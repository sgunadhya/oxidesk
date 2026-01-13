-- Migration 021: Availability Configuration
-- Feature: 006-agent-availability
-- Description: Store availability thresholds as configuration

CREATE TABLE IF NOT EXISTS system_config (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,
    description TEXT,
    updated_at TEXT NOT NULL
);

-- Seed availability configuration values
INSERT INTO system_config (key, value, description, updated_at) VALUES
    ('availability.inactivity_timeout_seconds', '300', 'Time in seconds before online agent goes away (default 5 min)', datetime('now')),
    ('availability.max_idle_threshold_seconds', '1800', 'Time in seconds before away agent is reassigned (default 30 min)', datetime('now'));
