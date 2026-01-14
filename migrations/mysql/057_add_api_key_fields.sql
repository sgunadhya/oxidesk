-- Migration: Add API key authentication fields to agents table
-- Feature: 015-api-key-auth
-- Purpose: Enable API key-based authentication for external systems

-- Add API key credential fields
ALTER TABLE agents ADD COLUMN api_key TEXT;
ALTER TABLE agents ADD COLUMN api_secret_hash TEXT;
ALTER TABLE agents ADD COLUMN api_key_description TEXT;
ALTER TABLE agents ADD COLUMN api_key_created_at DATETIME;
ALTER TABLE agents ADD COLUMN api_key_last_used_at DATETIME;
ALTER TABLE agents ADD COLUMN api_key_revoked_at DATETIME;

-- Create unique index on api_key to enforce one key per value
-- MySQL doesn't support partial indexes, so we'll use a functional index with a prefix
CREATE UNIQUE INDEX idx_agents_api_key_unique ON agents(api_key(32));

-- Create index on api_key for fast lookups
CREATE INDEX idx_agents_api_key_lookup ON agents(api_key(32));

-- Create index on created_at for listing/sorting operations
CREATE INDEX idx_agents_api_key_created_at ON agents(api_key_created_at);
