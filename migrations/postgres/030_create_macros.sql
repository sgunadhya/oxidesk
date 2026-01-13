-- Migration: Create macros table
-- Feature: 010-macro-system
-- Description: Store reusable message templates with actions

CREATE TABLE IF NOT EXISTS macros (
    id VARCHAR(36) PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    message_content TEXT NOT NULL,
    created_by VARCHAR(36) NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    usage_count INTEGER NOT NULL DEFAULT 0,
    access_control VARCHAR(10) NOT NULL DEFAULT 'all' CHECK(access_control IN ('all', 'restricted')),
    FOREIGN KEY (created_by) REFERENCES users(id)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_macros_name ON macros(name);
CREATE INDEX IF NOT EXISTS idx_macros_created_by ON macros(created_by);
CREATE INDEX IF NOT EXISTS idx_macros_access_control ON macros(access_control);
CREATE INDEX IF NOT EXISTS idx_macros_usage_count ON macros(usage_count DESC);
