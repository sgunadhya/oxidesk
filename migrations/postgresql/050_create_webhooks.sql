-- Migration: Create webhooks table
-- Database: PostgreSQL
-- Feature: 012-webhook-system

CREATE TABLE IF NOT EXISTS webhooks (
    id VARCHAR(255) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    url VARCHAR(2048) NOT NULL,
    subscribed_events TEXT NOT NULL,  -- JSON array of event types
    secret VARCHAR(255) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    created_by VARCHAR(255) NOT NULL,
    FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE CASCADE
);

-- Index for filtering active webhooks during event delivery
CREATE INDEX idx_webhooks_active ON webhooks(is_active);

-- Index for querying webhooks by creator
CREATE INDEX idx_webhooks_created_by ON webhooks(created_by);
