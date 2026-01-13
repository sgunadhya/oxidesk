-- Create SLA events table (individual trackable targets)
CREATE TABLE sla_events (
    id TEXT PRIMARY KEY NOT NULL,
    applied_sla_id TEXT NOT NULL,
    event_type TEXT NOT NULL CHECK(event_type IN ('first_response', 'resolution', 'next_response')),
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'met', 'breached')),
    deadline_at TEXT NOT NULL,
    met_at TEXT,
    breached_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (applied_sla_id) REFERENCES applied_slas(id) ON DELETE CASCADE
);

-- Indexes for fast lookups and breach detection
CREATE INDEX idx_sla_events_applied_sla ON sla_events(applied_sla_id);
CREATE INDEX idx_sla_events_status_deadline ON sla_events(status, deadline_at);

-- Unique constraint: only one pending event per (applied_sla_id, event_type)
CREATE UNIQUE INDEX idx_sla_events_pending_unique
ON sla_events(applied_sla_id, event_type, status)
WHERE status = 'pending';
