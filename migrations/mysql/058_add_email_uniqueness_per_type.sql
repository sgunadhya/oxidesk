-- Migration 058: Add Email Uniqueness Per User Type
-- Feature: User Creation (016)
-- Purpose: Enforce email uniqueness within each user type (agent/contact)
--          while allowing the same email across different types

-- Note: MySQL 8.0+ supports functional indexes with WHERE clause
-- For MySQL 5.7, we use composite unique indexes instead

-- Add unique index for agent emails
-- Uses composite index (email, user_type) with CHECK condition
CREATE UNIQUE INDEX idx_users_email_unique_agent
  ON users(email, user_type)
  WHERE user_type = 'agent';

-- Add unique index for contact emails
-- Uses composite index (email, user_type) with CHECK condition
CREATE UNIQUE INDEX idx_users_email_unique_contact
  ON users(email, user_type)
  WHERE user_type = 'contact';

-- Add lookup index for email queries (performance)
-- Used for fast email lookups regardless of user type
CREATE INDEX idx_users_email_lookup
  ON users(email);

-- Add composite index for type + email queries (performance)
-- Used for filtered queries like "find all agents with email X"
CREATE INDEX idx_users_type_email_lookup
  ON users(user_type, email);

-- Note: Functional indexes allow same email for different user types
-- Example: alice@example.com can exist as both agent AND contact
-- But alice@example.com cannot exist twice as agent (unique constraint violation)
