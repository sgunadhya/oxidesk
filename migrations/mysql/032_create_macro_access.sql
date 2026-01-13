-- Migration: Create macro_access table
-- Feature: 010-macro-system
-- Description: Store access control for restricted macros

CREATE TABLE IF NOT EXISTS macro_access (
    id VARCHAR(36) PRIMARY KEY,
    macro_id VARCHAR(36) NOT NULL,
    entity_type VARCHAR(20) NOT NULL CHECK(entity_type IN ('user', 'team')),
    entity_id VARCHAR(36) NOT NULL,
    granted_at DATETIME NOT NULL,
    granted_by VARCHAR(36) NOT NULL,
    FOREIGN KEY (macro_id) REFERENCES macros(id) ON DELETE CASCADE,
    FOREIGN KEY (granted_by) REFERENCES users(id),
    UNIQUE KEY unique_macro_access (macro_id, entity_type, entity_id)
);

-- Indexes for performance
CREATE INDEX idx_macro_access_macro_id ON macro_access(macro_id);
CREATE INDEX idx_macro_access_entity ON macro_access(entity_type, entity_id);
CREATE INDEX idx_macro_access_composite ON macro_access(macro_id, entity_type, entity_id);
