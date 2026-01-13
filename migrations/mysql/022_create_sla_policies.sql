-- Create SLA policies table
CREATE TABLE sla_policies (
    id VARCHAR(255) PRIMARY KEY NOT NULL,
    name VARCHAR(255) NOT NULL UNIQUE,
    description TEXT,
    first_response_time VARCHAR(50) NOT NULL,  -- Format: "2h", "30m", "1d"
    resolution_time VARCHAR(50) NOT NULL,      -- Format: "24h", "2d"
    next_response_time VARCHAR(50) NOT NULL,   -- Format: "4h", "30m"
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);

-- Index for fast lookups
CREATE INDEX idx_sla_policies_name ON sla_policies(name);
