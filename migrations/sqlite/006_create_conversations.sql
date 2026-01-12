-- Create conversations table
CREATE TABLE conversations (
    id TEXT PRIMARY KEY NOT NULL,
    reference_number INTEGER NOT NULL UNIQUE,
    status TEXT NOT NULL CHECK(status IN ('open', 'snoozed', 'resolved', 'closed')) DEFAULT 'open',
    inbox_id TEXT NOT NULL,
    contact_id TEXT NOT NULL,
    subject TEXT,
    resolved_at TEXT, -- ISO 8601 timestamp
    snoozed_until TEXT, -- ISO 8601 timestamp
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,

    FOREIGN KEY (inbox_id) REFERENCES inboxes(id) ON DELETE RESTRICT,
    FOREIGN KEY (contact_id) REFERENCES contacts(id) ON DELETE RESTRICT,
    CHECK (status != 'snoozed' OR snoozed_until IS NOT NULL),
    CHECK (status != 'resolved' OR resolved_at IS NOT NULL)
);

-- Indices
CREATE INDEX idx_conversations_status ON conversations(status);
CREATE INDEX idx_conversations_inbox ON conversations(inbox_id);
CREATE INDEX idx_conversations_contact ON conversations(contact_id);
CREATE INDEX idx_conversations_snooze ON conversations(snoozed_until) WHERE status = 'snoozed';
-- UNIQUE index on reference_number is implicit from UNIQUE constraint

-- Trigger to auto-generate reference number starting at 100
CREATE TRIGGER conversations_ref_number_insert
AFTER INSERT ON conversations
WHEN NEW.reference_number IS NULL
BEGIN
    UPDATE conversations
    SET reference_number = (SELECT COALESCE(MAX(reference_number), 99) + 1 FROM conversations)
    WHERE rowid = NEW.rowid;
END;

-- Trigger to auto-update updated_at timestamp
CREATE TRIGGER conversations_updated_at_timestamp
AFTER UPDATE ON conversations
FOR EACH ROW
BEGIN
    UPDATE conversations SET updated_at = datetime('now') WHERE id = OLD.id;
END;
