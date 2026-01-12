-- Migration 002: Seed roles and permissions
-- SQLite version

-- Insert default roles
INSERT INTO roles (id, name, description, created_at, updated_at) VALUES
('00000000-0000-0000-0000-000000000001', 'Admin', 'Full system access', datetime('now'), datetime('now')),
('00000000-0000-0000-0000-000000000002', 'Agent', 'Standard support agent', datetime('now'), datetime('now'));

-- Insert permissions
INSERT INTO permissions (id, name, description, created_at, updated_at) VALUES
-- User management
('10000000-0000-0000-0000-000000000001', 'users:read', 'View users', datetime('now'), datetime('now')),
('10000000-0000-0000-0000-000000000002', 'users:create', 'Create users', datetime('now'), datetime('now')),
('10000000-0000-0000-0000-000000000003', 'users:update', 'Update users', datetime('now'), datetime('now')),
('10000000-0000-0000-0000-000000000004', 'users:delete', 'Delete users', datetime('now'), datetime('now')),

-- Agent management
('11000000-0000-0000-0000-000000000001', 'agents:read', 'View agents', datetime('now'), datetime('now')),
('11000000-0000-0000-0000-000000000002', 'agents:create', 'Create agents', datetime('now'), datetime('now')),
('11000000-0000-0000-0000-000000000003', 'agents:update', 'Update agents', datetime('now'), datetime('now')),
('11000000-0000-0000-0000-000000000004', 'agents:delete', 'Delete agents', datetime('now'), datetime('now')),

-- Contact management
('12000000-0000-0000-0000-000000000001', 'contacts:read', 'View contacts', datetime('now'), datetime('now')),
('12000000-0000-0000-0000-000000000002', 'contacts:create', 'Create contacts', datetime('now'), datetime('now')),
('12000000-0000-0000-0000-000000000003', 'contacts:update', 'Update contacts', datetime('now'), datetime('now')),
('12000000-0000-0000-0000-000000000004', 'contacts:delete', 'Delete contacts', datetime('now'), datetime('now')),

-- Role management
('13000000-0000-0000-0000-000000000001', 'roles:read', 'View roles', datetime('now'), datetime('now')),
('13000000-0000-0000-0000-000000000002', 'roles:manage', 'Create/update/delete roles', datetime('now'), datetime('now'));

-- Assign all permissions to Admin role
INSERT INTO role_permissions (role_id, permission_id, created_at)
SELECT '00000000-0000-0000-0000-000000000001', id, datetime('now')
FROM permissions;

-- Assign read-only permissions to Agent role
INSERT INTO role_permissions (role_id, permission_id, created_at) VALUES
('00000000-0000-0000-0000-000000000002', '10000000-0000-0000-0000-000000000001', datetime('now')),
('00000000-0000-0000-0000-000000000002', '11000000-0000-0000-0000-000000000001', datetime('now')),
('00000000-0000-0000-0000-000000000002', '12000000-0000-0000-0000-000000000001', datetime('now')),
('00000000-0000-0000-0000-000000000002', '13000000-0000-0000-0000-000000000001', datetime('now'));
