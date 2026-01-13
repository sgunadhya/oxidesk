-- Migration 003: Create inboxes table
-- Description: Inboxes represent channels for receiving customer communications

CREATE TABLE inboxes (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT(100) NOT NULL,
    channel_type TEXT(50) NOT NULL CHECK(channel_type IN ('email', 'chat', 'api')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_inboxes_name ON inboxes(name);
CREATE INDEX idx_inboxes_channel_type ON inboxes(channel_type);

-- Insert a default test inbox for backwards compatibility
INSERT INTO inboxes (id, name, channel_type, created_at, updated_at)
VALUES ('inbox-001', 'Default Inbox', 'email', datetime('now'), datetime('now'));
