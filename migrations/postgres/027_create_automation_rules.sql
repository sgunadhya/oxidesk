-- Create automation_rules table
CREATE TABLE automation_rules (
    id UUID PRIMARY KEY,
    name VARCHAR(200) NOT NULL UNIQUE,
    description TEXT,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    rule_type VARCHAR(50) NOT NULL CHECK(rule_type IN ('conversation_update', 'message_received', 'assignment_changed')),
    event_subscription JSONB NOT NULL,
    condition JSONB NOT NULL,
    action JSONB NOT NULL,
    priority INTEGER NOT NULL DEFAULT 100 CHECK(priority >= 1 AND priority <= 1000),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes for efficient queries
CREATE INDEX idx_automation_rules_enabled ON automation_rules(enabled);
CREATE INDEX idx_automation_rules_priority ON automation_rules(priority);
CREATE INDEX idx_automation_rules_name ON automation_rules(name);

-- GIN index for JSONB event_subscription for efficient filtering
CREATE INDEX idx_automation_rules_event_subscription ON automation_rules USING GIN(event_subscription);
