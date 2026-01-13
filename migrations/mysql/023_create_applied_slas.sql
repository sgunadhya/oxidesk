-- Create applied SLAs table (SLA instances applied to conversations)
CREATE TABLE applied_slas (
    id VARCHAR(255) PRIMARY KEY NOT NULL,
    conversation_id VARCHAR(255) UNIQUE NOT NULL,
    sla_policy_id VARCHAR(255) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'met', 'breached')),
    first_response_deadline_at TIMESTAMP NOT NULL,
    resolution_deadline_at TIMESTAMP NOT NULL,
    applied_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
    FOREIGN KEY (sla_policy_id) REFERENCES sla_policies(id) ON DELETE RESTRICT
);

-- Indexes for fast lookups
CREATE UNIQUE INDEX idx_applied_slas_conversation ON applied_slas(conversation_id);
CREATE INDEX idx_applied_slas_policy ON applied_slas(sla_policy_id);
CREATE INDEX idx_applied_slas_status ON applied_slas(status);
