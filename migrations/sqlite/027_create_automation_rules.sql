-- Create automation_rules table
CREATE TABLE automation_rules (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    rule_type TEXT NOT NULL CHECK(rule_type IN ('conversation_update', 'message_received', 'assignment_changed')),
    event_subscription TEXT NOT NULL, -- JSON array of event names
    condition TEXT NOT NULL,          -- JSON condition expression
    action TEXT NOT NULL,             -- JSON action definition
    priority INTEGER NOT NULL DEFAULT 100 CHECK(priority >= 1 AND priority <= 1000),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Create indexes for efficient queries
CREATE INDEX idx_automation_rules_enabled ON automation_rules(enabled);
CREATE INDEX idx_automation_rules_priority ON automation_rules(priority);
CREATE INDEX idx_automation_rules_name ON automation_rules(name);
