-- Migration: Create webhooks table
-- Database: SQLite
-- Feature: 012-webhook-system

CREATE TABLE IF NOT EXISTS webhooks (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    url TEXT NOT NULL,
    subscribed_events TEXT NOT NULL,  -- JSON array of event types
    secret TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,  -- SQLite uses INTEGER for boolean
    created_at TEXT NOT NULL,  -- ISO 8601 format
    updated_at TEXT NOT NULL,  -- ISO 8601 format
    created_by TEXT NOT NULL,
    FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE CASCADE
);

-- Index for filtering active webhooks during event delivery
CREATE INDEX idx_webhooks_active ON webhooks(is_active);

-- Index for querying webhooks by creator
CREATE INDEX idx_webhooks_created_by ON webhooks(created_by);
