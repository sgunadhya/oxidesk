-- Migration 054: Create auth_events table for authentication audit logging
-- Tracks all authentication attempts (success and failure) for security monitoring

CREATE TABLE auth_events (
    id VARCHAR(36) PRIMARY KEY,
    event_type VARCHAR(50) NOT NULL,  -- login_success, login_failure, logout, session_expired, rate_limit_exceeded
    user_id VARCHAR(36),  -- NULL if user not identified (failed login to non-existent email)
    email VARCHAR(255) NOT NULL,
    auth_method VARCHAR(20) NOT NULL,  -- password, oidc
    provider_name VARCHAR(100),  -- OIDC provider name if applicable
    ip_address VARCHAR(45) NOT NULL,  -- IPv4 or IPv6
    user_agent TEXT,
    error_reason TEXT,  -- Details if event_type is failure
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL
);

-- Indexes for efficient audit queries
CREATE INDEX idx_auth_events_timestamp ON auth_events(timestamp);
CREATE INDEX idx_auth_events_email ON auth_events(email);
CREATE INDEX idx_auth_events_user_id ON auth_events(user_id);
CREATE INDEX idx_auth_events_type ON auth_events(event_type);
CREATE INDEX idx_auth_events_ip ON auth_events(ip_address);
