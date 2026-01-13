-- Migration 025: Add business hours support and fix CASCADE deletion
-- Feature: 008-sla-policy-application
-- Description: Adds business_hours to teams table and changes applied_slas FK to CASCADE

-- Step 1: Add business_hours column to teams table
ALTER TABLE teams ADD COLUMN business_hours TEXT; -- JSON format: {"timezone": "America/New_York", "schedule": [...]}

-- Step 2: Fix CASCADE deletion for applied_slas
-- Drop existing foreign key constraint
ALTER TABLE applied_slas DROP CONSTRAINT applied_slas_sla_policy_id_fkey;

-- Add new foreign key constraint with CASCADE
ALTER TABLE applied_slas
    ADD CONSTRAINT applied_slas_sla_policy_id_fkey
    FOREIGN KEY (sla_policy_id) REFERENCES sla_policies(id) ON DELETE CASCADE;
