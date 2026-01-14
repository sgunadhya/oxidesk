-- Migration 062: Seed core conversation permissions
-- PostgreSQL version

-- Insert conversation permissions
INSERT INTO permissions (id, name, description, created_at, updated_at) VALUES
    ('conv-perm-001', 'conversations:create', 'Create new conversations', NOW(), NOW()),
    ('conv-perm-002', 'conversations:read_all', 'Read all conversations', NOW(), NOW()),
    ('conv-perm-003', 'conversations:read_assigned', 'Read assigned conversations only', NOW(), NOW()),
    ('conv-perm-004', 'conversations:update_all', 'Update any conversation', NOW(), NOW()),
    ('conv-perm-005', 'conversations:update_assigned', 'Update assigned conversations only', NOW(), NOW()),
    ('conv-perm-006', 'conversations:delete', 'Delete conversations', NOW(), NOW());

-- Assign all conversation permissions to Admin role
INSERT INTO role_permissions (role_id, permission_id, created_at)
SELECT '00000000-0000-0000-0000-000000000001', id, NOW()
FROM permissions
WHERE name IN (
    'conversations:create',
    'conversations:read_all',
    'conversations:read_assigned',
    'conversations:update_all',
    'conversations:update_assigned',
    'conversations:delete'
);

-- Assign standard conversation permissions to Agent role
INSERT INTO role_permissions (role_id, permission_id, created_at)
SELECT '00000000-0000-0000-0000-000000000002', id, NOW()
FROM permissions
WHERE name IN (
    'conversations:create',
    'conversations:read_assigned',
    'conversations:update_assigned'
);
