-- Seed tag management permissions
-- Feature: 005-conversation-tagging

-- Insert tag permissions
INSERT INTO permissions (id, name, description, created_at, updated_at) VALUES
    ('tag-perm-001', 'tags:create', 'Create new tags', datetime('now'), datetime('now')),
    ('tag-perm-002', 'tags:read', 'View tags', datetime('now'), datetime('now')),
    ('tag-perm-003', 'tags:update', 'Update tag properties', datetime('now'), datetime('now')),
    ('tag-perm-004', 'tags:delete', 'Delete tags', datetime('now'), datetime('now')),
    ('tag-perm-005', 'conversations:update_tags', 'Modify conversation tags', datetime('now'), datetime('now'));

-- Assign all tag permissions to Admin role (00000000-0000-0000-0000-000000000001)
INSERT INTO role_permissions (role_id, permission_id, created_at)
SELECT '00000000-0000-0000-0000-000000000001', id, datetime('now')
FROM permissions
WHERE name IN ('tags:create', 'tags:read', 'tags:update', 'tags:delete', 'conversations:update_tags');

-- Assign read and update_tags permissions to Agent role (00000000-0000-0000-0000-000000000002)
INSERT INTO role_permissions (role_id, permission_id, created_at) VALUES
    ('00000000-0000-0000-0000-000000000002', 'tag-perm-002', datetime('now')),
    ('00000000-0000-0000-0000-000000000002', 'tag-perm-005', datetime('now'));
