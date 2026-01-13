-- Create rule_evaluation_logs table
CREATE TABLE rule_evaluation_logs (
    id UUID PRIMARY KEY,
    rule_id UUID NOT NULL,
    rule_name VARCHAR(200) NOT NULL,
    event_type VARCHAR(100) NOT NULL,
    conversation_id UUID,
    matched BOOLEAN NOT NULL,
    condition_result VARCHAR(10) CHECK(condition_result IN ('true', 'false', 'error')),
    action_executed BOOLEAN NOT NULL,
    action_result VARCHAR(10) CHECK(action_result IN ('success', 'failure', 'error', 'skipped')),
    error_message TEXT,
    evaluation_time_ms BIGINT NOT NULL,
    evaluated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
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
VALUES (gen_random_uuid(), 'automation:manage', 'Manage automation rules', NOW(), NOW());

-- Grant to Admin role
INSERT INTO role_permissions (role_id, permission_id, created_at)
SELECT
    (SELECT id FROM roles WHERE name = 'Admin'),
    (SELECT id FROM permissions WHERE name = 'automation:manage'),
    NOW();
