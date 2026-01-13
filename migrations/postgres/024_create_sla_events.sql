-- Create SLA events table (individual trackable targets)
CREATE TABLE sla_events (
    id VARCHAR(255) PRIMARY KEY NOT NULL,
    applied_sla_id VARCHAR(255) NOT NULL,
    event_type VARCHAR(50) NOT NULL CHECK(event_type IN ('first_response', 'resolution', 'next_response')),
    status VARCHAR(50) NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'met', 'breached')),
    deadline_at TIMESTAMP NOT NULL,
    met_at TIMESTAMP,
    breached_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (applied_sla_id) REFERENCES applied_slas(id) ON DELETE CASCADE
);

-- Indexes for fast lookups and breach detection
CREATE INDEX idx_sla_events_applied_sla ON sla_events(applied_sla_id);
CREATE INDEX idx_sla_events_status_deadline ON sla_events(status, deadline_at);

-- Unique constraint: only one pending event per (applied_sla_id, event_type)
CREATE UNIQUE INDEX idx_sla_events_pending_unique
ON sla_events(applied_sla_id, event_type)
WHERE status = 'pending';
