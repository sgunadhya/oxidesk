-- Migration 052: Enhance sessions table with CSRF token, last accessed timestamp, and auth method
-- Add CSRF token for cross-site request forgery protection
-- Add last_accessed_at for sliding window expiration
-- Add auth_method to track password vs OIDC login
-- Add provider_name for OIDC sessions

-- Add new columns to sessions table
ALTER TABLE sessions
ADD COLUMN csrf_token VARCHAR(64) NOT NULL DEFAULT '',
ADD COLUMN last_accessed_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
ADD COLUMN auth_method VARCHAR(20) NOT NULL DEFAULT 'password',
ADD COLUMN provider_name VARCHAR(100);

-- Create index for efficient session expiration queries
CREATE INDEX idx_sessions_last_accessed ON sessions(last_accessed_at);

-- Create index for auth method queries (analytics)
CREATE INDEX idx_sessions_auth_method ON sessions(auth_method);
