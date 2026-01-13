-- Migration: Add macro_actions field to messages table
-- Feature: 010-macro-system
-- Description: Store queued actions from macro application

ALTER TABLE messages ADD COLUMN macro_actions TEXT NULL;
