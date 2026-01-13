-- Migration 015: Seed assignment permissions
-- Feature: 004-conversation-assignment
-- Description: Add new permissions for conversation assignment

-- Insert new permissions
INSERT INTO permissions (id, name, description, resource, action)
VALUES
    ('perm_conversations_update_user_assignee', 'conversations:update_user_assignee',
     'Assign conversations to users', 'conversations', 'update_user_assignee'),
    ('perm_conversations_update_team_assignee', 'conversations:update_team_assignee',
     'Assign conversations to teams', 'conversations', 'update_team_assignee'),
    ('perm_conversations_read_unassigned', 'conversations:read_unassigned',
     'View unassigned conversations', 'conversations', 'read_unassigned');

-- Grant to Admin role
INSERT INTO role_permissions (role_id, permission_id)
SELECT 'role_admin', id FROM permissions
WHERE name IN (
    'conversations:update_user_assignee',
    'conversations:update_team_assignee',
    'conversations:read_unassigned'
);

-- Grant to Agent role (they can self-assign and view unassigned)
INSERT INTO role_permissions (role_id, permission_id)
SELECT 'role_agent', id FROM permissions
WHERE name IN (
    'conversations:update_user_assignee',
    'conversations:read_unassigned'
);
