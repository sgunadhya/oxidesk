-- PostgreSQL migration for conversations table
-- Feature: 002-conversation-lifecycle

CREATE SEQUENCE IF NOT EXISTS conversation_ref_seq START 100 INCREMENT 1;

CREATE TYPE conversation_status AS ENUM ('open', 'snoozed', 'resolved', 'closed');

CREATE TABLE IF NOT EXISTS conversations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    reference_number INTEGER NOT NULL UNIQUE DEFAULT nextval('conversation_ref_seq'),
    status conversation_status NOT NULL DEFAULT 'open',
    inbox_id UUID NOT NULL,
    contact_id UUID NOT NULL,
    subject TEXT,
    resolved_at TIMESTAMPTZ,
    snoozed_until TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    version INTEGER NOT NULL DEFAULT 1,

    CONSTRAINT conversation_inbox_fk
        FOREIGN KEY (inbox_id) REFERENCES inboxes(id) ON DELETE RESTRICT,
    CONSTRAINT conversation_contact_fk
        FOREIGN KEY (contact_id) REFERENCES contacts(id) ON DELETE RESTRICT,
    CONSTRAINT conversation_snooze_check
        CHECK (status != 'snoozed' OR snoozed_until IS NOT NULL),
    CONSTRAINT conversation_resolved_check
        CHECK (status != 'resolved' OR resolved_at IS NOT NULL)
);

-- Indexes
CREATE INDEX idx_conversations_status ON conversations(status);
CREATE INDEX idx_conversations_inbox ON conversations(inbox_id);
CREATE INDEX idx_conversations_contact ON conversations(contact_id);
CREATE INDEX idx_conversations_snooze ON conversations(snoozed_until) WHERE status = 'snoozed';
CREATE INDEX idx_conversations_created ON conversations(created_at DESC);
CREATE UNIQUE INDEX idx_conversations_reference ON conversations(reference_number);

-- Trigger for updated_at
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_conversations_updated_at
    BEFORE UPDATE ON conversations
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
