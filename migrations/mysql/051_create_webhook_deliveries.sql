-- Migration: Create webhook_deliveries table
-- Database: MySQL
-- Feature: 012-webhook-system

CREATE TABLE IF NOT EXISTS webhook_deliveries (
    id VARCHAR(255) PRIMARY KEY,
    webhook_id VARCHAR(255) NOT NULL,
    event_type VARCHAR(100) NOT NULL,
    payload TEXT NOT NULL,  -- JSON payload sent to webhook
    signature VARCHAR(100) NOT NULL,  -- HMAC-SHA256 signature (format: "sha256=...")
    status VARCHAR(20) NOT NULL CHECK(status IN ('queued', 'success', 'failed')),
    http_status_code INTEGER,  -- HTTP response status code (200, 500, etc.)
    retry_count INTEGER NOT NULL DEFAULT 0,
    next_retry_at DATETIME,  -- Timestamp for next retry
    attempted_at DATETIME,  -- Timestamp of most recent attempt
    completed_at DATETIME,  -- Timestamp of final completion
    error_message TEXT,  -- Error details if delivery failed
    FOREIGN KEY (webhook_id) REFERENCES webhooks(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- Index for querying deliveries by webhook
CREATE INDEX idx_webhook_deliveries_webhook_id ON webhook_deliveries(webhook_id);

-- Index for background worker to find pending retries
CREATE INDEX idx_webhook_deliveries_status_retry ON webhook_deliveries(status, next_retry_at);

-- Index for querying delivery logs chronologically
CREATE INDEX idx_webhook_deliveries_attempted_at ON webhook_deliveries(attempted_at);
