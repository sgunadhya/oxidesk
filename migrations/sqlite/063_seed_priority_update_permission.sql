-- Migration 063: Seed priority update permission
-- SQLite version
-- Feature: 020-priority-management

-- Insert priority update permission
INSERT INTO permissions (id, name, description, created_at, updated_at) VALUES
    ('priority-perm-001', 'conversations:update_priority', 'Update conversation priority level', datetime('now'), datetime('now'));

-- Assign permission to Admin role
INSERT INTO role_permissions (role_id, permission_id, created_at)
SELECT '00000000-0000-0000-0000-000000000001', id, datetime('now')
FROM permissions
WHERE name = 'conversations:update_priority';

-- Assign permission to Agent role
INSERT INTO role_permissions (role_id, permission_id, created_at)
SELECT '00000000-0000-0000-0000-000000000002', id, datetime('now')
FROM permissions
WHERE name = 'conversations:update_priority';
