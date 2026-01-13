-- Create conversation_tags join table for many-to-many relationship
-- Feature: 005-conversation-tagging

CREATE TABLE conversation_tags (
    conversation_id TEXT NOT NULL,
    tag_id TEXT NOT NULL,
    added_by TEXT NOT NULL,
    added_at TIMESTAMP NOT NULL,
    PRIMARY KEY (conversation_id, tag_id),
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE,
    FOREIGN KEY (added_by) REFERENCES users(id) ON DELETE SET NULL
);

-- Index for finding all tags for a conversation
CREATE INDEX idx_conversation_tags_conversation ON conversation_tags(conversation_id);

-- Index for finding all conversations with a specific tag
CREATE INDEX idx_conversation_tags_tag ON conversation_tags(tag_id);
