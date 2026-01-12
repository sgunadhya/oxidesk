-- Migration 002: Seed roles and permissions
-- MySQL version

-- Insert default roles
INSERT INTO roles (id, name, description, created_at, updated_at) VALUES
('00000000-0000-0000-0000-000000000001', 'Admin', 'Full system access', NOW(), NOW()),
('00000000-0000-0000-0000-000000000002', 'Agent', 'Standard support agent', NOW(), NOW());

-- Insert permissions
INSERT INTO permissions (id, name, description, created_at, updated_at) VALUES
-- User management
('10000000-0000-0000-0000-000000000001', 'users:read', 'View users', NOW(), NOW()),
('10000000-0000-0000-0000-000000000002', 'users:create', 'Create users', NOW(), NOW()),
('10000000-0000-0000-0000-000000000003', 'users:update', 'Update users', NOW(), NOW()),
('10000000-0000-0000-0000-000000000004', 'users:delete', 'Delete users', NOW(), NOW()),

-- Agent management
('11000000-0000-0000-0000-000000000001', 'agents:read', 'View agents', NOW(), NOW()),
('11000000-0000-0000-0000-000000000002', 'agents:create', 'Create agents', NOW(), NOW()),
('11000000-0000-0000-0000-000000000003', 'agents:update', 'Update agents', NOW(), NOW()),
('11000000-0000-0000-0000-000000000004', 'agents:delete', 'Delete agents', NOW(), NOW()),

-- Contact management
('12000000-0000-0000-0000-000000000001', 'contacts:read', 'View contacts', NOW(), NOW()),
('12000000-0000-0000-0000-000000000002', 'contacts:create', 'Create contacts', NOW(), NOW()),
('12000000-0000-0000-0000-000000000003', 'contacts:update', 'Update contacts', NOW(), NOW()),
('12000000-0000-0000-0000-000000000004', 'contacts:delete', 'Delete contacts', NOW(), NOW()),

-- Role management
('13000000-0000-0000-0000-000000000001', 'roles:read', 'View roles', NOW(), NOW()),
('13000000-0000-0000-0000-000000000002', 'roles:manage', 'Create/update/delete roles', NOW(), NOW());

-- Assign all permissions to Admin role
INSERT INTO role_permissions (role_id, permission_id, created_at)
SELECT '00000000-0000-0000-0000-000000000001', id, NOW()
FROM permissions;

-- Assign read-only permissions to Agent role
INSERT INTO role_permissions (role_id, permission_id, created_at) VALUES
('00000000-0000-0000-0000-000000000002', '10000000-0000-0000-0000-000000000001', NOW()),
('00000000-0000-0000-0000-000000000002', '11000000-0000-0000-0000-000000000001', NOW()),
('00000000-0000-0000-0000-000000000002', '12000000-0000-0000-0000-000000000001', NOW()),
('00000000-0000-0000-0000-000000000002', '13000000-0000-0000-0000-000000000001', NOW());
