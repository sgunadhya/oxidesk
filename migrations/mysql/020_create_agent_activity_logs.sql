-- Migration 020: Agent Activity Logs
-- Feature: 006-agent-availability
-- Description: Create activity log table for agent authentication and availability events

CREATE TABLE agent_activity_logs (
    id VARCHAR(36) PRIMARY KEY NOT NULL,
    agent_id VARCHAR(36) NOT NULL,
    event_type VARCHAR(50) NOT NULL CHECK(event_type IN ('agent_login', 'agent_logout', 'availability_changed')),
    old_status VARCHAR(50),
    new_status VARCHAR(50),
    metadata TEXT, -- JSON for extensibility
    created_at DATETIME NOT NULL,
    FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
);

-- Create indexes for efficient queries
CREATE INDEX idx_activity_logs_agent ON agent_activity_logs(agent_id);
CREATE INDEX idx_activity_logs_event_type ON agent_activity_logs(event_type);
CREATE INDEX idx_activity_logs_created_at ON agent_activity_logs(created_at DESC);
