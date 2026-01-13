-- Migration 015: Seed assignment permissions
-- Feature: 004-conversation-assignment
-- Description: Add new permissions for conversation assignment

-- Insert new permissions
INSERT INTO permissions (id, name, description, created_at, updated_at)
VALUES
    ('perm_conversations_update_user_assignee', 'conversations:update_user_assignee',
     'Assign conversations to users', NOW(), NOW()),
    ('perm_conversations_update_team_assignee', 'conversations:update_team_assignee',
     'Assign conversations to teams', NOW(), NOW()),
    ('perm_conversations_read_unassigned', 'conversations:read_unassigned',
     'View unassigned conversations', NOW(), NOW());

-- Grant to Admin role
INSERT INTO role_permissions (role_id, permission_id, created_at)
SELECT '00000000-0000-0000-0000-000000000001', id, NOW() FROM permissions
WHERE name IN (
    'conversations:update_user_assignee',
    'conversations:update_team_assignee',
    'conversations:read_unassigned'
);

-- Grant to Agent role (they can self-assign and view unassigned)
INSERT INTO role_permissions (role_id, permission_id, created_at)
SELECT '00000000-0000-0000-0000-000000000002', id, NOW() FROM permissions
WHERE name IN (
    'conversations:update_user_assignee',
    'conversations:read_unassigned'
);
