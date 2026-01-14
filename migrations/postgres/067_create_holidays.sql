-- Feature 029: SLA Business Hours - Holiday Calendar
-- Create holidays table for excluding specific dates from SLA calculations

CREATE TABLE IF NOT EXISTS holidays (
    id VARCHAR(36) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    date DATE NOT NULL, -- Date without time
    recurring BOOLEAN NOT NULL DEFAULT FALSE, -- Repeats annually on same month-day
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL
);

-- Index for fast date lookup during SLA calculation
CREATE INDEX idx_holidays_date ON holidays(date);
-- Index for recurring holidays
CREATE INDEX idx_holidays_recurring ON holidays(recurring) WHERE recurring = true;
