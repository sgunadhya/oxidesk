-- Migration: Create macro_application_logs table
-- Feature: 010-macro-system
-- Description: Audit trail for macro applications

CREATE TABLE IF NOT EXISTS macro_application_logs (
    id VARCHAR(36) PRIMARY KEY,
    macro_id VARCHAR(36) NOT NULL,
    agent_id VARCHAR(36) NOT NULL,
    conversation_id VARCHAR(36) NOT NULL,
    applied_at DATETIME NOT NULL,
    actions_queued TEXT NOT NULL,  -- JSON array of action types
    variables_replaced INT NOT NULL DEFAULT 0,
    FOREIGN KEY (macro_id) REFERENCES macros(id),
    FOREIGN KEY (agent_id) REFERENCES users(id),
    FOREIGN KEY (conversation_id) REFERENCES conversations(id)
);

-- Indexes for performance
CREATE INDEX idx_macro_logs_macro_id ON macro_application_logs(macro_id);
CREATE INDEX idx_macro_logs_agent_id ON macro_application_logs(agent_id);
CREATE INDEX idx_macro_logs_conversation_id ON macro_application_logs(conversation_id);
CREATE INDEX idx_macro_logs_applied_at ON macro_application_logs(applied_at DESC);
