CREATE TABLE observations (
    observation_id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    report_id TEXT NOT NULL,
    current_revision INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'rejected')),
    payload_json TEXT NOT NULL DEFAULT '{}',
    updated_at INTEGER NOT NULL DEFAULT 0,
    candidate_key TEXT,
    UNIQUE (user_id, observation_id),
    UNIQUE (user_id, report_id, candidate_key)
);
CREATE INDEX observations_report ON observations (user_id, report_id);
PRAGMA user_version = 1;
