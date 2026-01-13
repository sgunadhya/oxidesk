-- Migration 026: Seed SLA management permissions
-- Feature: 008-sla-policy-application
-- Description: Grant sla:manage permission to Admin and Agent roles
-- Note: The sla:manage permission already exists from migration 002

-- Grant sla:manage to admin role (Admin role already has all permissions from migration 002)
-- This is a no-op, but included for clarity

-- Grant sla:manage to agent role
-- Note: Admin role already has this permission via "all permissions" grant in migration 002
-- Agent role does not have it yet, so we add it here
INSERT IGNORE INTO role_permissions (role_id, permission_id, created_at)
SELECT
    r.id as role_id,
    p.id as permission_id,
    NOW() as created_at
FROM roles r, permissions p
WHERE r.name = 'Agent' AND p.name = 'sla:manage';
