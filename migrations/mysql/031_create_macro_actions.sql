-- Migration: Create macro_actions table
-- Feature: 010-macro-system
-- Description: Store actions associated with macros

CREATE TABLE IF NOT EXISTS macro_actions (
    id VARCHAR(36) PRIMARY KEY,
    macro_id VARCHAR(36) NOT NULL,
    action_type VARCHAR(50) NOT NULL CHECK(action_type IN ('set_status', 'assign_to_user', 'assign_to_team', 'add_tag', 'set_priority')),
    action_value TEXT NOT NULL,
    action_order INT NOT NULL,
    FOREIGN KEY (macro_id) REFERENCES macros(id) ON DELETE CASCADE
);

-- Indexes for performance
CREATE INDEX idx_macro_actions_macro_id ON macro_actions(macro_id);
CREATE INDEX idx_macro_actions_order ON macro_actions(macro_id, action_order);
