-- Create automation_rules table
CREATE TABLE automation_rules (
    id CHAR(36) PRIMARY KEY,
    name VARCHAR(200) NOT NULL UNIQUE,
    description TEXT,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    rule_type VARCHAR(50) NOT NULL CHECK(rule_type IN ('conversation_update', 'message_received', 'assignment_changed')),
    event_subscription JSON NOT NULL,
    `condition` JSON NOT NULL,
    `action` JSON NOT NULL,
    priority INT NOT NULL DEFAULT 100 CHECK(priority >= 1 AND priority <= 1000),
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_automation_rules_enabled (enabled),
    INDEX idx_automation_rules_priority (priority),
    INDEX idx_automation_rules_name (name)
);
