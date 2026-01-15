-- Create jobs table for generic background task processing
CREATE TABLE jobs (
    id TEXT PRIMARY KEY NOT NULL,
    job_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending', -- pending, processing, completed, failed
    run_at DATETIME NOT NULL,
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 5,
    last_error TEXT,
    locked_until DATETIME -- For concurrency control
);

-- Index for efficient polling of pending jobs
CREATE INDEX idx_jobs_poll ON jobs(status, run_at);

-- Index for cleaning up old completed/failed jobs
CREATE INDEX idx_jobs_cleanup ON jobs(status, created_at);
