-- Migration 068: Add soft delete fields to users and inboxes
-- Feature: 027-soft-delete
-- Description: Add deleted_at and deleted_by fields for soft delete functionality

-- Add soft delete fields to users table
ALTER TABLE users
    ADD COLUMN deleted_at TIMESTAMP NULL DEFAULT NULL,
    ADD COLUMN deleted_by CHAR(36) NULL DEFAULT NULL;

CREATE INDEX idx_users_deleted_at ON users(deleted_at);

-- Add soft delete fields to inboxes table
ALTER TABLE inboxes
    ADD COLUMN deleted_at TIMESTAMP NULL DEFAULT NULL,
    ADD COLUMN deleted_by VARCHAR(36) NULL DEFAULT NULL;

CREATE INDEX idx_inboxes_deleted_at ON inboxes(deleted_at);
