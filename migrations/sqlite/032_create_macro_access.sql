-- Migration: Create macro_access table
-- Feature: 010-macro-system
-- Description: Store access control for restricted macros

CREATE TABLE IF NOT EXISTS macro_access (
    id TEXT PRIMARY KEY,
    macro_id TEXT NOT NULL,
    entity_type TEXT NOT NULL CHECK(entity_type IN ('user', 'team')),
    entity_id TEXT NOT NULL,
    granted_at TEXT NOT NULL,
    granted_by TEXT NOT NULL,
    FOREIGN KEY (macro_id) REFERENCES macros(id) ON DELETE CASCADE,
    FOREIGN KEY (granted_by) REFERENCES users(id),
    UNIQUE(macro_id, entity_type, entity_id)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_macro_access_macro_id ON macro_access(macro_id);
CREATE INDEX IF NOT EXISTS idx_macro_access_entity ON macro_access(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_macro_access_composite ON macro_access(macro_id, entity_type, entity_id);
