-- Add priority column to conversations table
-- Feature: 009-automation-rule-engine

ALTER TABLE conversations ADD COLUMN priority TEXT;

-- Index for priority-based queries
CREATE INDEX idx_conversations_priority ON conversations(priority);
