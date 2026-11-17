ALTER TABLE observations RENAME TO observations_v2;
DROP INDEX observations_report;
CREATE TABLE observations (
    observation_id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    report_id TEXT NOT NULL,
    current_revision INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status = 'pending'),
    payload_json TEXT NOT NULL DEFAULT '{}',
    updated_at INTEGER NOT NULL DEFAULT 0,
    candidate_key TEXT,
    ocr_revision INTEGER NOT NULL DEFAULT 1,
    UNIQUE (user_id, observation_id),
    UNIQUE (user_id, report_id, candidate_key)
);
CREATE INDEX observations_report ON observations (user_id, report_id);
INSERT INTO observations
SELECT observation_id,user_id,report_id,current_revision + CASE WHEN status='rejected' THEN 1 ELSE 0 END,'pending',
       CASE WHEN status='rejected'
            THEN json_set(payload_json,'$.ocr_mapping_note','Previously excluded. Review before submitting. ' || coalesce(json_extract(payload_json,'$.ocr_mapping_note'),''))
            ELSE payload_json END,
       updated_at,candidate_key,ocr_revision
FROM observations_v2;
DROP TABLE observations_v2;
PRAGMA user_version = 3;
