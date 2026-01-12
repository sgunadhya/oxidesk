-- Migration 001: Create users tables with per-type email uniqueness
-- SQLite version

-- Enable foreign key constraints
PRAGMA foreign_keys = ON;

-- Users table (base table for all users)
CREATE TABLE users (
    id TEXT(36) PRIMARY KEY,
    email TEXT(255) NOT NULL,
    user_type TEXT(20) NOT NULL CHECK(user_type IN ('agent', 'contact')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(email, user_type)  -- Per-type email uniqueness
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_type ON users(user_type);

-- Agents table (support staff)
CREATE TABLE agents (
    id TEXT(36) PRIMARY KEY,
    user_id TEXT(36) UNIQUE NOT NULL,
    first_name TEXT(100) NOT NULL,
    password_hash TEXT(255) NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_agents_user_id ON agents(user_id);

-- Contacts table (customers)
CREATE TABLE contacts (
    id TEXT(36) PRIMARY KEY,
    user_id TEXT(36) UNIQUE NOT NULL,
    first_name TEXT(100),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_contacts_user_id ON contacts(user_id);

-- Roles table
CREATE TABLE roles (
    id TEXT(36) PRIMARY KEY,
    name TEXT(50) UNIQUE NOT NULL,
    description TEXT(255),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_roles_name ON roles(name);

-- Permissions table
CREATE TABLE permissions (
    id TEXT(36) PRIMARY KEY,
    name TEXT(100) UNIQUE NOT NULL,
    description TEXT(255),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_permissions_name ON permissions(name);

-- User roles junction table (many-to-many)
CREATE TABLE user_roles (
    user_id TEXT(36) NOT NULL,
    role_id TEXT(36) NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (user_id, role_id),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE
);

CREATE INDEX idx_user_roles_role_id ON user_roles(role_id);

-- Role permissions junction table (many-to-many)
CREATE TABLE role_permissions (
    role_id TEXT(36) NOT NULL,
    permission_id TEXT(36) NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (role_id, permission_id),
    FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE,
    FOREIGN KEY (permission_id) REFERENCES permissions(id) ON DELETE CASCADE
);

CREATE INDEX idx_role_permissions_permission_id ON role_permissions(permission_id);

-- Contact channels table (links contacts to inboxes)
CREATE TABLE contact_channels (
    id TEXT(36) PRIMARY KEY,
    contact_id TEXT(36) NOT NULL,
    inbox_id TEXT(36) NOT NULL,
    email TEXT(255) NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (contact_id) REFERENCES contacts(id) ON DELETE CASCADE,
    UNIQUE(contact_id, inbox_id)
);

CREATE INDEX idx_contact_channels_contact_id ON contact_channels(contact_id);
CREATE INDEX idx_contact_channels_inbox_id ON contact_channels(inbox_id);

-- Sessions table
CREATE TABLE sessions (
    id TEXT(36) PRIMARY KEY,
    user_id TEXT(36) NOT NULL,
    token TEXT(64) UNIQUE NOT NULL,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_sessions_token ON sessions(token);
CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_expires_at ON sessions(expires_at);

-- Password reset tokens table
CREATE TABLE password_reset_tokens (
    id TEXT(36) PRIMARY KEY,
    user_id TEXT(36) NOT NULL,
    token TEXT(64) UNIQUE NOT NULL,
    expires_at TEXT NOT NULL,
    used INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_password_reset_tokens_token ON password_reset_tokens(token);
CREATE INDEX idx_password_reset_tokens_user_id ON password_reset_tokens(user_id);
CREATE INDEX idx_password_reset_tokens_expires_at ON password_reset_tokens(expires_at);
