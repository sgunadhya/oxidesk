-- Create SLA policies table
CREATE TABLE sla_policies (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    first_response_time TEXT NOT NULL,  -- Format: "2h", "30m", "1d"
    resolution_time TEXT NOT NULL,      -- Format: "24h", "2d"
    next_response_time TEXT NOT NULL,   -- Format: "4h", "30m"
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Index for fast lookups
CREATE INDEX idx_sla_policies_name ON sla_policies(name);
