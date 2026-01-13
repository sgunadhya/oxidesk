-- Create rule_evaluation_logs table
CREATE TABLE rule_evaluation_logs (
    id TEXT PRIMARY KEY NOT NULL,
    rule_id TEXT NOT NULL,
    rule_name TEXT NOT NULL,
    event_type TEXT NOT NULL,
    conversation_id TEXT,
    matched BOOLEAN NOT NULL,
    condition_result TEXT CHECK(condition_result IN ('true', 'false', 'error')),
    action_executed BOOLEAN NOT NULL,
    action_result TEXT CHECK(action_result IN ('success', 'failure', 'error', 'skipped')),
    error_message TEXT,
    evaluation_time_ms INTEGER NOT NULL,
    evaluated_at TEXT NOT NULL,
    cascade_depth INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (rule_id) REFERENCES automation_rules(id) ON DELETE CASCADE,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE SET NULL
);

-- Create indexes for efficient queries
CREATE INDEX idx_rule_evaluation_logs_rule_id ON rule_evaluation_logs(rule_id);
CREATE INDEX idx_rule_evaluation_logs_conversation_id ON rule_evaluation_logs(conversation_id);
CREATE INDEX idx_rule_evaluation_logs_evaluated_at ON rule_evaluation_logs(evaluated_at);
CREATE INDEX idx_rule_evaluation_logs_event_type ON rule_evaluation_logs(event_type);

-- Seed automation management permission
INSERT INTO permissions (id, name, description, created_at, updated_at)
VALUES ('perm_automation_manage', 'automation:manage', 'Manage automation rules', datetime('now'), datetime('now'));

-- Grant to Admin role
INSERT INTO role_permissions (role_id, permission_id, created_at)
VALUES ('00000000-0000-0000-0000-000000000001', 'perm_automation_manage', datetime('now'));
