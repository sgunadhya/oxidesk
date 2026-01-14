-- Migration: Add API key authentication fields to agents table
-- Feature: 015-api-key-auth
-- Purpose: Enable API key-based authentication for external systems

-- Add API key credential fields
ALTER TABLE agents ADD COLUMN api_key TEXT;
ALTER TABLE agents ADD COLUMN api_secret_hash TEXT;
ALTER TABLE agents ADD COLUMN api_key_description TEXT;
ALTER TABLE agents ADD COLUMN api_key_created_at TEXT;
ALTER TABLE agents ADD COLUMN api_key_last_used_at TEXT;
ALTER TABLE agents ADD COLUMN api_key_revoked_at TEXT;

-- Create unique index on api_key to enforce one key per value
CREATE UNIQUE INDEX idx_agents_api_key_unique ON agents(api_key) WHERE api_key IS NOT NULL;

-- Create partial index on api_key for fast lookups of active keys
CREATE INDEX idx_agents_api_key_lookup ON agents(api_key) WHERE api_key IS NOT NULL;

-- Create index on created_at for listing/sorting operations
CREATE INDEX idx_agents_api_key_created_at ON agents(api_key_created_at) WHERE api_key IS NOT NULL;
