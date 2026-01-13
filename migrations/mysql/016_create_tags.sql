-- Create tags table for conversation classification
-- Feature: 005-conversation-tagging

CREATE TABLE tags (
    id VARCHAR(255) PRIMARY KEY NOT NULL,
    name VARCHAR(255) UNIQUE NOT NULL,
    description TEXT,
    color VARCHAR(7),
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL
);

-- Index for fast tag name lookups
CREATE INDEX idx_tags_name ON tags(name);
