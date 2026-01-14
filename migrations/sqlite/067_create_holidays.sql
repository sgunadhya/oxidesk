-- Feature 029: SLA Business Hours - Holiday Calendar
-- Create holidays table for excluding specific dates from SLA calculations

CREATE TABLE IF NOT EXISTS holidays (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    date TEXT NOT NULL, -- YYYY-MM-DD format
    recurring INTEGER NOT NULL DEFAULT 0, -- 0 = false, 1 = true (repeats annually)
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Index for fast date lookup during SLA calculation
CREATE INDEX IF NOT EXISTS idx_holidays_date ON holidays(date);
-- Index for recurring holidays (to find all recurring holidays efficiently)
CREATE INDEX IF NOT EXISTS idx_holidays_recurring ON holidays(recurring) WHERE recurring = 1;
