-- Migration: Create webhook_deliveries table
-- Database: SQLite
-- Feature: 012-webhook-system

CREATE TABLE IF NOT EXISTS webhook_deliveries (
    id TEXT PRIMARY KEY,
    webhook_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    payload TEXT NOT NULL,  -- JSON payload sent to webhook
    signature TEXT NOT NULL,  -- HMAC-SHA256 signature (format: "sha256=...")
    status TEXT NOT NULL CHECK(status IN ('queued', 'success', 'failed')),
    http_status_code INTEGER,  -- HTTP response status code (200, 500, etc.)
    retry_count INTEGER NOT NULL DEFAULT 0,
    next_retry_at TEXT,  -- ISO 8601 timestamp for next retry
    attempted_at TEXT,  -- ISO 8601 timestamp of most recent attempt
    completed_at TEXT,  -- ISO 8601 timestamp of final completion
    error_message TEXT,  -- Error details if delivery failed
    FOREIGN KEY (webhook_id) REFERENCES webhooks(id) ON DELETE CASCADE
);

-- Index for querying deliveries by webhook
CREATE INDEX idx_webhook_deliveries_webhook_id ON webhook_deliveries(webhook_id);

-- Index for background worker to find pending retries
CREATE INDEX idx_webhook_deliveries_status_retry ON webhook_deliveries(status, next_retry_at);

-- Index for querying delivery logs chronologically
CREATE INDEX idx_webhook_deliveries_attempted_at ON webhook_deliveries(attempted_at);
