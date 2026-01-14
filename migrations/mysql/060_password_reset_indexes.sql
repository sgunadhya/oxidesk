-- Migration 060: Add composite index for password reset rate limiting
-- Feature: 017-password-reset
-- Purpose: Optimize rate limiting queries (count recent reset requests per user)

-- Add composite index on (user_id, created_at) for efficient rate limiting queries
CREATE INDEX idx_password_reset_tokens_user_created
ON password_reset_tokens(user_id, created_at);

-- Note: The password_reset_tokens table was created in migration 001_create_users.sql
-- This migration only adds the composite index for performance optimization
