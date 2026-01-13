-- Migration: Add role protection support and permissions array
-- Feature: 014-rbac-system
-- Purpose: Add is_protected flag and permissions JSON array to roles table

-- Add is_protected column to roles table
ALTER TABLE roles ADD COLUMN is_protected BOOLEAN NOT NULL DEFAULT 0;

-- Add permissions column to store permissions as JSON array
ALTER TABLE roles ADD COLUMN permissions TEXT NOT NULL DEFAULT '[]';

-- Migrate existing role-permission relationships to JSON array
-- For each role, collect all its permissions into a JSON array
UPDATE roles SET permissions = (
    SELECT COALESCE(
        json_group_array(p.name),
        '[]'
    )
    FROM role_permissions rp
    INNER JOIN permissions p ON rp.permission_id = p.id
    WHERE rp.role_id = roles.id
)
WHERE EXISTS (
    SELECT 1 FROM role_permissions WHERE role_id = roles.id
);

-- Mark Admin role as protected
-- Use name-based lookup since Admin role should already exist from seed data
UPDATE roles SET is_protected = 1 WHERE name = 'Admin';

-- Create index for protection queries
CREATE INDEX IF NOT EXISTS idx_roles_is_protected ON roles(is_protected);

-- Create index for name lookups (if not exists from previous migrations)
CREATE INDEX IF NOT EXISTS idx_roles_name ON roles(name);
