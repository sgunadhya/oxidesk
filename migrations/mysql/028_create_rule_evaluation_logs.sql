-- Create rule_evaluation_logs table
CREATE TABLE rule_evaluation_logs (
    id CHAR(36) PRIMARY KEY,
    rule_id CHAR(36) NOT NULL,
    rule_name VARCHAR(200) NOT NULL,
    event_type VARCHAR(100) NOT NULL,
    conversation_id CHAR(36),
    matched BOOLEAN NOT NULL,
    condition_result VARCHAR(10) CHECK(condition_result IN ('true', 'false', 'error')),
    action_executed BOOLEAN NOT NULL,
    action_result VARCHAR(10) CHECK(action_result IN ('success', 'failure', 'error', 'skipped')),
    error_message TEXT,
    evaluation_time_ms BIGINT NOT NULL,
    evaluated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    cascade_depth INT NOT NULL DEFAULT 0,
    FOREIGN KEY (rule_id) REFERENCES automation_rules(id) ON DELETE CASCADE,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE SET NULL,
    INDEX idx_rule_evaluation_logs_rule_id (rule_id),
    INDEX idx_rule_evaluation_logs_conversation_id (conversation_id),
    INDEX idx_rule_evaluation_logs_evaluated_at (evaluated_at),
    INDEX idx_rule_evaluation_logs_event_type (event_type)
);

-- Seed automation management permission
INSERT INTO permissions (id, name, description, created_at, updated_at)
VALUES (UUID(), 'automation:manage', 'Manage automation rules', NOW(), NOW());

-- Grant to Admin role
INSERT INTO role_permissions (role_id, permission_id, created_at)
SELECT
    (SELECT id FROM roles WHERE name = 'Admin'),
    (SELECT id FROM permissions WHERE name = 'automation:manage'),
    NOW();
