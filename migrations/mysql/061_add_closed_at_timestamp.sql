-- Migration 061: Add closed_at timestamp to conversations table
-- Feature 019: Status Transitions - Allow Resolved → Closed with timestamp tracking

ALTER TABLE conversations ADD COLUMN closed_at TIMESTAMP NULL;

-- Create index for efficient queries filtering by closed conversations
CREATE INDEX idx_conversations_closed_at ON conversations(closed_at);
