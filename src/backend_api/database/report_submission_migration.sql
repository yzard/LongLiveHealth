DROP VIEW temp.review_observations;

CREATE TABLE migrated_observations (
    observation_id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    report_id TEXT NOT NULL,
    current_revision INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'submitted')),
    payload_json TEXT NOT NULL DEFAULT '{}',
    updated_at INTEGER NOT NULL DEFAULT 0,
    candidate_key TEXT,
    UNIQUE (user_id, observation_id),
    UNIQUE (user_id, report_id, candidate_key),
    FOREIGN KEY (user_id, report_id) REFERENCES reports (user_id, report_id) ON DELETE CASCADE
);
INSERT INTO migrated_observations (observation_id, user_id, report_id, current_revision, status, payload_json, updated_at, candidate_key)
SELECT observation_id, user_id, report_id,
       current_revision + CASE WHEN status = 'rejected' THEN 1 ELSE 0 END,
       CASE status WHEN 'confirmed' THEN 'submitted' WHEN 'rejected' THEN 'pending' ELSE status END,
       CASE WHEN status = 'rejected' THEN
           json_set(payload_json, '$.ocr_mapping_note', 'Previously excluded. Review before submitting. ' || coalesce(json_extract(payload_json, '$.ocr_mapping_note'), ''))
           ELSE payload_json END,
       updated_at, candidate_key
  FROM observations;

DROP TABLE observations;
ALTER TABLE migrated_observations RENAME TO observations;
CREATE INDEX observations_report ON observations (user_id, report_id);
