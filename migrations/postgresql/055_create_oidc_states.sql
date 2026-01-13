-- OIDC temporary state storage for OAuth2 flow
-- Stores state, nonce, and PKCE verifier during authentication
CREATE TABLE oidc_states (
    state VARCHAR(64) PRIMARY KEY,
    provider_name VARCHAR(100) NOT NULL,
    nonce VARCHAR(64) NOT NULL,
    pkce_verifier VARCHAR(128) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,

    FOREIGN KEY (provider_name) REFERENCES oidc_providers(name) ON DELETE CASCADE
);

-- Index for cleanup of expired states
CREATE INDEX idx_oidc_states_expires_at ON oidc_states(expires_at);
