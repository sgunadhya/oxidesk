-- Create tags table for conversation classification
-- Feature: 005-conversation-tagging

CREATE TABLE tags (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT UNIQUE NOT NULL,
    description TEXT,
    color TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Index for fast tag name lookups
CREATE INDEX idx_tags_name ON tags(name);
