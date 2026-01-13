-- Migration 053: Create OIDC providers table for SSO configuration
-- Stores configuration for external identity providers (Google, Microsoft, etc.)

CREATE TABLE oidc_providers (
    id VARCHAR(36) PRIMARY KEY,
    name VARCHAR(100) NOT NULL UNIQUE,
    issuer_url TEXT NOT NULL,
    client_id VARCHAR(255) NOT NULL,
    client_secret TEXT NOT NULL,  -- Should be encrypted at application level
    redirect_uri TEXT NOT NULL,
    scopes TEXT NOT NULL,  -- JSON array of OIDC scopes
    enabled BOOLEAN NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for efficient lookups
CREATE INDEX idx_oidc_providers_name ON oidc_providers(name);
CREATE INDEX idx_oidc_providers_enabled ON oidc_providers(enabled);
