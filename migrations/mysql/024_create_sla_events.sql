-- Create SLA events table (individual trackable targets)
CREATE TABLE sla_events (
    id VARCHAR(255) PRIMARY KEY NOT NULL,
    applied_sla_id VARCHAR(255) NOT NULL,
    event_type VARCHAR(50) NOT NULL CHECK(event_type IN ('first_response', 'resolution', 'next_response')),
    status VARCHAR(50) NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'met', 'breached')),
    deadline_at TIMESTAMP NOT NULL,
    met_at TIMESTAMP NULL,
    breached_at TIMESTAMP NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (applied_sla_id) REFERENCES applied_slas(id) ON DELETE CASCADE
);

-- Indexes for fast lookups and breach detection
CREATE INDEX idx_sla_events_applied_sla ON sla_events(applied_sla_id);
CREATE INDEX idx_sla_events_status_deadline ON sla_events(status, deadline_at);

-- Note: MySQL doesn't support partial indexes like PostgreSQL/SQLite
-- Uniqueness will be enforced at application level for pending events
