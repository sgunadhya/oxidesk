-- Migration 059: Add last_name column to agents table
-- Feature: 016-user-creation
-- Description: Support optional last name for agents

ALTER TABLE agents ADD COLUMN last_name VARCHAR(255);
