-- Seed tag management permissions
-- Feature: 005-conversation-tagging

-- Insert tag permissions
INSERT INTO permissions (id, name, description, created_at, updated_at) VALUES
    ('tag-perm-001', 'tags:create', 'Create new tags', NOW(), NOW()),
    ('tag-perm-002', 'tags:read', 'View tags', NOW(), NOW()),
    ('tag-perm-003', 'tags:update', 'Update tag properties', NOW(), NOW()),
    ('tag-perm-004', 'tags:delete', 'Delete tags', NOW(), NOW()),
    ('tag-perm-005', 'conversations:update_tags', 'Modify conversation tags', NOW(), NOW());

-- Assign all tag permissions to Admin role (00000000-0000-0000-0000-000000000001)
INSERT INTO role_permissions (role_id, permission_id, created_at)
SELECT '00000000-0000-0000-0000-000000000001', id, NOW()
FROM permissions
WHERE name IN ('tags:create', 'tags:read', 'tags:update', 'tags:delete', 'conversations:update_tags');

-- Assign read and update_tags permissions to Agent role (00000000-0000-0000-0000-000000000002)
INSERT INTO role_permissions (role_id, permission_id, created_at) VALUES
    ('00000000-0000-0000-0000-000000000002', 'tag-perm-002', NOW()),
    ('00000000-0000-0000-0000-000000000002', 'tag-perm-005', NOW());
