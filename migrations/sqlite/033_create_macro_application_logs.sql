-- Migration: Create macro_application_logs table
-- Feature: 010-macro-system
-- Description: Audit trail for macro applications

CREATE TABLE IF NOT EXISTS macro_application_logs (
    id TEXT PRIMARY KEY,
    macro_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    conversation_id TEXT NOT NULL,
    applied_at TEXT NOT NULL,
    actions_queued TEXT NOT NULL,  -- JSON array of action types
    variables_replaced INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (macro_id) REFERENCES macros(id),
    FOREIGN KEY (agent_id) REFERENCES users(id),
    FOREIGN KEY (conversation_id) REFERENCES conversations(id)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_macro_logs_macro_id ON macro_application_logs(macro_id);
CREATE INDEX IF NOT EXISTS idx_macro_logs_agent_id ON macro_application_logs(agent_id);
CREATE INDEX IF NOT EXISTS idx_macro_logs_conversation_id ON macro_application_logs(conversation_id);
CREATE INDEX IF NOT EXISTS idx_macro_logs_applied_at ON macro_application_logs(applied_at DESC);
