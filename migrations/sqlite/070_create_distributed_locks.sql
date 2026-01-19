CREATE TABLE distributed_locks (
    key TEXT PRIMARY KEY NOT NULL,
    owner TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL
);
