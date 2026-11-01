pub const SCHEMA_VERSION: &str = "PRAGMA user_version";
pub const BUMP_REPORT: &str =
    "UPDATE reports SET revision = revision + 1 WHERE user_id = ? AND report_id = ?";
pub const HEALTH_TOMBSTONE: &str = "SELECT EXISTS(SELECT 1 FROM health_records WHERE user_id = ? AND platform = ? AND record_id = ? AND deleted = 1 AND source_id IN (?, '*'))";
pub const DELETE_HEALTH_ORIGINS: &str = "UPDATE health_records SET deleted = 1, start_at = 0, end_at = 0, payload_json = '{}', raw_id = NULL WHERE user_id = ? AND platform = ? AND record_id = ?";
pub const PURGE_HEALTH_ORIGINS: &str =
    "DELETE FROM health_originals WHERE user_id = ? AND platform = ? AND record_id = ?";
pub const INSERT_ANALYSIS: &str = "INSERT INTO analysis_runs (run_id, user_id, status, input_digest, input_json, created_at, model) VALUES (?, ?, 'queued', ?, ?, ?, ?)";
pub const ANALYSIS_BY_DIGEST: &str = "SELECT run_id FROM analysis_runs WHERE user_id = ? AND input_digest = ? AND status != 'stale' ORDER BY created_at DESC LIMIT 1";
pub const ANALYSIS_LIST: &str = "SELECT run_id, status, output_json, error_code, created_at, model, json_extract(input_json,'$.prompt_version') AS prompt_version FROM analysis_runs WHERE user_id = ? ORDER BY created_at DESC LIMIT 100";
pub const ANALYSIS_GET: &str = "SELECT run_id, status, input_json, output_json, error_code, created_at, model FROM analysis_runs WHERE user_id = ? AND run_id = ?";
pub const CLAIM_ANALYSIS: &str = "UPDATE analysis_runs SET status = 'running' WHERE run_id = (SELECT run_id FROM analysis_runs WHERE status = 'queued' ORDER BY created_at LIMIT 1) RETURNING run_id, user_id, input_json";
pub const FINISH_ANALYSIS: &str = "UPDATE analysis_runs SET status = ?, output_json = ?, error_code = ? WHERE user_id = ? AND run_id = ? AND status = 'running'";
pub const RECOVER_ANALYSIS: &str =
    "UPDATE analysis_runs SET status = 'queued' WHERE status = 'running'";
pub const RETRY_ANALYSIS: &str = "UPDATE analysis_runs SET status = 'queued', error_code = NULL WHERE user_id = ? AND run_id = ? AND status = 'failed'";
pub const INSERT_FEEDBACK: &str = "INSERT INTO analysis_feedback (user_id, run_id, feedback_id, note, created_at) VALUES (?, ?, ?, ?, ?)";
pub const ANALYSIS_FEEDBACK: &str = "SELECT feedback_id, note, created_at FROM analysis_feedback WHERE user_id = ? AND run_id = ? ORDER BY created_at";
pub const EXPORT_TABLES: &[(&str, &str)] = &[
    ("daily_views", "SELECT * FROM daily_views WHERE user_id = ?"),
    (
        "derived_results",
        "SELECT * FROM derived_results WHERE user_id = ?",
    ),
    (
        "deleted_uploads",
        "SELECT * FROM deleted_uploads WHERE user_id = ?",
    ),
    (
        "sync_batches",
        "SELECT * FROM sync_batches WHERE user_id = ?",
    ),
    ("reports", "SELECT * FROM reports WHERE user_id = ?"),
    ("raw_files", "SELECT * FROM raw_files WHERE user_id = ?"),
    (
        "extraction_inputs",
        "SELECT * FROM extraction_inputs WHERE user_id = ?",
    ),
    (
        "extraction_outputs",
        "SELECT * FROM extraction_outputs WHERE user_id = ?",
    ),
    (
        "observations",
        "SELECT * FROM review_observations WHERE user_id = ?",
    ),
    (
        "report_relations",
        "SELECT * FROM report_relations WHERE user_id = ?",
    ),
    (
        "extraction_pages",
        "SELECT * FROM extraction_pages WHERE user_id = ?",
    ),
    (
        "health_connections",
        "SELECT * FROM health_connections WHERE user_id = ?",
    ),
    (
        "health_records",
        "SELECT * FROM health_records WHERE user_id = ?",
    ),
    (
        "health_originals",
        "SELECT * FROM health_originals WHERE user_id = ?",
    ),
    (
        "sync_coverage",
        "SELECT * FROM sync_coverage WHERE user_id = ?",
    ),
    (
        "analysis_runs",
        "SELECT * FROM analysis_runs WHERE user_id = ?",
    ),
    (
        "analysis_feedback",
        "SELECT * FROM analysis_feedback WHERE user_id = ?",
    ),
];
pub const DELETE_EXPORT: &str = "DELETE FROM exports WHERE user_id = ? AND export_id = ?";
pub const USER_DATA_REVISION: &str = "SELECT data_revision FROM users WHERE user_id = ?";
pub const TOUCH_USER_DATA: &str =
    "UPDATE users SET data_revision = data_revision + 1 WHERE user_id = ?";
pub const DELETE_FILE: &str = "DELETE FROM raw_files WHERE user_id = ? AND file_id = ?";
pub const DELETE_USER: &str = "DELETE FROM users WHERE user_id = ?";
pub const PURGE_FEEDBACK: &str = "DELETE FROM analysis_feedback WHERE user_id = ?";
pub const INSERT_CLEANUP: &str = "INSERT INTO cleanup_files (cleanup_id, relative_path, created_at) VALUES (?, ?, ?) ON CONFLICT (relative_path) DO NOTHING";
pub const CLEANUP_LIST: &str =
    "SELECT cleanup_id, relative_path FROM cleanup_files ORDER BY created_at LIMIT 100";
pub const CLEANUP_DONE: &str = "DELETE FROM cleanup_files WHERE cleanup_id = ?";
pub const INSERT_EXPORT: &str =
    "INSERT INTO exports (export_id, user_id, status, created_at) VALUES (?, ?, 'queued', ?)";
pub const EXPORT_LIST: &str = "SELECT export_id, status, error_code, created_at FROM exports WHERE user_id = ? ORDER BY created_at DESC LIMIT 100";
pub const EXPORT_GET: &str = "SELECT status FROM exports WHERE user_id = ? AND export_id = ?";
pub const CLAIM_EXPORT: &str = "UPDATE exports SET status = 'running' WHERE export_id = (SELECT export_id FROM exports WHERE status = 'queued' ORDER BY created_at LIMIT 1) RETURNING export_id, user_id";
pub const FINISH_EXPORT: &str = "UPDATE exports SET status = ?, error_code = ? WHERE user_id = ? AND export_id = ? AND status = 'running'";
pub const RECOVER_EXPORTS: &str = "UPDATE exports SET status = 'queued' WHERE status = 'running'";
pub const VERIFY_JOB_LEASE: &str = "SELECT COUNT(*) FROM jobs WHERE user_id = ? AND job_id = ? AND lease_token = ? AND status = 'running' AND lease_until > ?";
pub const CREATE_CONNECTION: &str = "INSERT INTO health_connections (connection_id, user_id, platform, installation_id, created_at) VALUES (?, ?, ?, ?, ?) ON CONFLICT (user_id, platform, installation_id) DO NOTHING";
pub const CONNECTION_BY_INSTALLATION: &str = "SELECT connection_id, platform, installation_id FROM health_connections WHERE user_id = ? AND platform = ? AND installation_id = ?";
pub const CONNECTION: &str =
    "SELECT platform FROM health_connections WHERE user_id = ? AND connection_id = ?";
pub const SYNC_BATCH: &str =
    "SELECT digest FROM sync_batches WHERE user_id = ? AND connection_id = ? AND batch_id = ?";
pub const INSERT_SYNC_BATCH: &str = "INSERT INTO sync_batches (user_id, connection_id, batch_id, digest, received_at) VALUES (?, ?, ?, ?, ?)";
pub const HEALTH_RECORD: &str = "SELECT version, deleted, payload_json, record_type FROM health_records WHERE user_id = ? AND platform = ? AND source_id = ? AND record_id = ?";
pub const UPSERT_HEALTH: &str = "INSERT INTO health_records (user_id, platform, source_id, record_id, record_type, start_at, end_at, version, deleted, payload_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT (user_id, platform, source_id, record_id) DO UPDATE SET record_type = excluded.record_type, start_at = excluded.start_at, end_at = excluded.end_at, version = excluded.version, deleted = excluded.deleted, payload_json = excluded.payload_json, raw_id = CASE WHEN excluded.deleted = 1 THEN NULL ELSE health_records.raw_id END";
pub const INSERT_HEALTH_ORIGINAL: &str = "INSERT INTO health_originals (user_id, platform, source_id, record_id, raw_id, raw_path, received_at, raw_offset, raw_length) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)";
pub const PURGE_HEALTH_ORIGINALS: &str = "DELETE FROM health_originals WHERE user_id = ? AND platform = ? AND source_id = ? AND record_id = ?";
pub const UPSERT_COVERAGE: &str = "INSERT INTO sync_coverage (user_id, connection_id, record_type, status, last_sync_at, first_success_at, last_success_at) VALUES (?, ?, ?, ?, ?, ?, ?) ON CONFLICT (user_id, connection_id, record_type) DO UPDATE SET status = excluded.status, last_sync_at = excluded.last_sync_at, first_success_at = COALESCE(sync_coverage.first_success_at, excluded.first_success_at), last_success_at = COALESCE(excluded.last_success_at, sync_coverage.last_success_at)";
pub const COVERAGE: &str = "SELECT c.connection_id, c.platform, c.installation_id, s.record_type, s.status, s.last_sync_at, s.first_success_at, s.last_success_at, (SELECT MIN(h.start_at) FROM health_records h WHERE h.user_id = c.user_id AND h.platform = c.platform AND h.record_type = s.record_type AND h.deleted = 0) AS visible_start_at, (SELECT MAX(h.end_at) FROM health_records h WHERE h.user_id = c.user_id AND h.platform = c.platform AND h.record_type = s.record_type AND h.deleted = 0) AS visible_end_at FROM health_connections c LEFT JOIN sync_coverage s ON s.user_id = c.user_id AND s.connection_id = c.connection_id WHERE c.user_id = ? ORDER BY c.connection_id, s.record_type";
pub const HEALTH_RANGE: &str = r#"
SELECT platform
     , source_id
     , record_id
     , record_type
     , start_at
     , end_at
     , version
     , json_object('payload', json_object(
           'source_name', json_extract(payload_json, '$.payload.source_name'),
           'device', json_extract(payload_json, '$.payload.device'),
           'value', CASE WHEN json_type(payload_json, '$.payload.value') IN ('integer', 'real')
                         THEN json_extract(payload_json, '$.payload.value') ELSE NULL END,
           'unit', CASE WHEN json_extract(payload_json, '$.payload.unit') IN ('count', 'ms', 'count/min', 'kcal')
                        THEN json_extract(payload_json, '$.payload.unit') ELSE NULL END,
           'category', CASE WHEN json_type(payload_json, '$.payload.category') = 'integer'
                            THEN json_extract(payload_json, '$.payload.category') ELSE NULL END,
           'stages', CASE WHEN platform = 'health_connect' AND record_type = 'sleep'
                         THEN json_extract(payload_json, '$.payload.stages') ELSE NULL END
       )) AS payload_json
  FROM health_records
 WHERE user_id = ?
   AND deleted = 0
   AND record_type = ?
   AND end_at >= ?
   AND start_at < ?
 ORDER BY start_at
        , source_id
        , record_id
 LIMIT 50001
"#;
pub const HEALTH_LIST: &str = "SELECT h.platform, r.raw_path, r.raw_offset, r.raw_length FROM health_records h JOIN health_originals r ON r.user_id=h.user_id AND r.raw_id=h.raw_id WHERE h.user_id = ? AND h.deleted = 0 ORDER BY h.start_at, h.source_id, h.record_id LIMIT ? OFFSET ?";
pub const SET_HEALTH_ORIGINAL: &str = "UPDATE health_records SET raw_id = ? WHERE user_id = ? AND platform = ? AND source_id = ? AND record_id = ?";
pub const CURRENT_HEALTH_ORIGINAL: &str = "SELECT r.raw_path, r.raw_offset, r.raw_length FROM health_records h JOIN health_originals r ON r.user_id=h.user_id AND r.raw_id=h.raw_id WHERE h.user_id=? AND h.platform=? AND h.source_id=? AND h.record_id=? AND h.deleted=0";
pub const INSERT_REPORT: &str =
    "INSERT INTO reports (report_id, user_id, file_id, created_at) VALUES (?, ?, ?, ?)";
pub const REPORT: &str = "SELECT r.report_id, r.revision, r.context_json, f.original_name, f.page_count, r.created_at FROM reports r JOIN raw_files f ON f.user_id = r.user_id AND f.file_id = r.file_id WHERE r.user_id = ? AND r.report_id = ?";
pub const REPORTS: &str = "SELECT r.report_id, r.revision, r.context_json, f.original_name, f.page_count, r.created_at FROM reports r JOIN raw_files f ON f.user_id = r.user_id AND f.file_id = r.file_id WHERE r.user_id = ? AND (? IS NULL OR r.report_id > ?) ORDER BY r.report_id LIMIT ?";
pub const OBSERVATIONS: &str = "SELECT o.observation_id, o.report_id, o.current_revision AS revision, o.status, o.payload_json FROM review_observations o WHERE o.user_id = ? AND o.report_id = ? ORDER BY o.observation_id";
pub const OBSERVATION: &str = "SELECT o.current_revision FROM review_observations o WHERE o.user_id = ? AND o.report_id = ? AND o.observation_id = ?";
pub const INSERT_OBSERVATION: &str = "INSERT INTO draft.observations (observation_id, user_id, report_id, current_revision, candidate_key) VALUES (?, ?, ?, 1, ?)";
pub const SAVE_OBSERVATION: &str = "INSERT INTO draft.observations (user_id,observation_id,current_revision,status,payload_json,updated_at,report_id,candidate_key) SELECT ?1,?2,?3,?4,?5,?6,report_id,candidate_key FROM review_observations WHERE user_id=?1 AND observation_id=?2 ON CONFLICT(observation_id) DO UPDATE SET current_revision=excluded.current_revision,status=excluded.status,payload_json=excluded.payload_json,updated_at=excluded.updated_at";
pub const CONFIRM_OBSERVATION: &str = "INSERT INTO main.observations (user_id,observation_id,current_revision,status,payload_json,updated_at,report_id,candidate_key) SELECT ?1,?2,?3,?4,?5,?6,report_id,candidate_key FROM review_observations WHERE user_id=?1 AND observation_id=?2 ON CONFLICT(observation_id) DO UPDATE SET current_revision=excluded.current_revision,status=excluded.status,payload_json=excluded.payload_json,updated_at=excluded.updated_at";
pub const UPDATE_REPORT: &str = "UPDATE reports SET revision = revision + 1, context_json = COALESCE(?, context_json) WHERE user_id = ? AND report_id = ? AND revision = ?";
pub const OBSERVATION_HISTORY: &str = "SELECT observation_id, current_revision AS revision, status, payload_json, updated_at AS created_at FROM review_observations WHERE user_id = ? AND observation_id = ?";
pub const CONFIRMED_OBSERVATIONS: &str = "SELECT o.observation_id, o.report_id, o.current_revision AS revision, o.status, o.payload_json FROM review_observations o WHERE o.user_id = ? AND o.status = 'confirmed' AND NOT EXISTS (SELECT 1 FROM report_relations r WHERE r.user_id = o.user_id AND r.report_id = o.report_id) ORDER BY o.observation_id LIMIT 10001";
pub const REPORT_RELATION: &str =
    "SELECT preferred_report_id, kind FROM report_relations WHERE user_id = ? AND report_id = ?";
pub const RELATION_DEPENDENTS: &str =
    "SELECT COUNT(*) FROM report_relations WHERE user_id = ? AND preferred_report_id = ?";
pub const SET_RELATION: &str = "INSERT INTO report_relations (user_id, report_id, preferred_report_id, kind) VALUES (?, ?, ?, ?) ON CONFLICT (user_id, report_id) DO UPDATE SET preferred_report_id = excluded.preferred_report_id, kind = excluded.kind";
pub const CLEAR_RELATION: &str = "DELETE FROM report_relations WHERE user_id = ? AND report_id = ?";
pub const INVALIDATE_ANALYSES: &str = "UPDATE analysis_runs SET status = 'stale', input_json = '{}', output_json = NULL, error_code = 'data_changed' WHERE user_id = ? AND status != 'stale'";
pub const INVALIDATE_EXPORTS: &str =
    "UPDATE exports SET status = 'stale' WHERE user_id = ? AND status != 'stale'";
pub const CANDIDATE_EXISTS: &str = "SELECT COUNT(*) FROM review_observations WHERE user_id = ? AND report_id = ? AND candidate_key = ?";
pub const INSERT_PAGE: &str = "INSERT INTO extraction_pages (user_id, report_id, run_id, page_number, status, content, model, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)";
pub const EXTRACTION_PAGES: &str = "SELECT run_id, page_number, status, content, model, created_at FROM extraction_pages WHERE user_id = ? AND report_id = ? ORDER BY created_at DESC, run_id, page_number";

pub const SET_SCHEMA_VERSION: &str = "PRAGMA user_version = 8";

pub const CREATE_USER: &str = r#"
INSERT INTO users (user_id
     , username
     , password_hash
     , created_at)
VALUES (?
     , ?
     , ?
     , ?)
"#;

pub const CREDENTIALS: &str = r#"
SELECT user_id
     , username
     , password_hash
     , credential_version
     , must_change_password
  FROM users
 WHERE username = ?
   AND is_active = 1
"#;

pub const DISABLE_USER: &str = r#"
UPDATE users
   SET is_active = 0
     , credential_version = credential_version + 1
 WHERE username = ?
RETURNING user_id
"#;

pub const RESET_PASSWORD: &str = r#"
UPDATE users
   SET password_hash = ?
     , must_change_password = 0
     , credential_version = credential_version + 1
 WHERE username = ?
RETURNING user_id
"#;

pub const REVOKE_USER_SESSIONS: &str = r#"
DELETE
  FROM sessions
 WHERE user_id = ?
"#;

pub const EXPIRE_SESSIONS: &str = r#"
DELETE
  FROM sessions
 WHERE expires_at <= ?1 AND (refresh_expires_at IS NULL OR refresh_expires_at <= ?1)
"#;

pub const CREATE_SESSION: &str = r#"
INSERT INTO sessions (token_hash
     , user_id
     , expires_at
     , refresh_hash
     , refresh_expires_at)
SELECT ?
     , user_id
     , ?
     , ?
     , ?
  FROM users
 WHERE user_id = ?
   AND credential_version = ?
   AND is_active = 1
"#;

pub const SESSION_USER: &str = r#"
SELECT users.user_id 
     , users.username
     , users.username = 'admin' AS is_admin
     , users.must_change_password
  FROM sessions
  JOIN users ON users.user_id = sessions.user_id
 WHERE ((sessions.token_hash = ?1 AND sessions.expires_at > ?2)
        OR (sessions.previous_token_hash = ?1 AND sessions.previous_expires_at > ?2))
   AND users.is_active = 1
"#;

pub const REVOKE_SESSION: &str = r#"
DELETE
  FROM sessions
 WHERE token_hash = ?1 OR previous_token_hash = ?1
"#;

pub const EXPIRE_LOGIN_WINDOWS: &str = r#"
DELETE
  FROM login_windows
 WHERE starts_at <= ?
"#;

pub const RESERVE_LOGIN: &str = r#"
INSERT INTO login_windows (username_hash
     , starts_at
     , attempt_count)
VALUES (?
     , ?
     , 1) ON CONFLICT (username_hash) DO UPDATE
   SET attempt_count = attempt_count + 1
RETURNING attempt_count
"#;

pub const INSERT_FILE: &str = r#"
INSERT INTO raw_files (file_id
     , user_id
     , upload_id
     , relative_path
     , processing_path
     , processing_sha256
     , original_name
     , content_type
     , sha256
     , byte_count
     , page_count
     , created_at)
VALUES (?
     , ?
     , ?
     , ?
     , ?
     , ?
     , ?
     , ?
     , ?
     , ?
     , ?
     , ?)
"#;

pub const FILE_BY_UPLOAD: &str = r#"
SELECT file_id
     , upload_id
     , relative_path
     , processing_path
     , processing_sha256
     , original_name
     , content_type
     , sha256
     , byte_count
     , page_count
     , created_at
  FROM raw_files
 WHERE user_id = ?
   AND upload_id = ?
"#;

pub const FILE_BY_ID: &str = r#"
SELECT file_id
     , upload_id
     , relative_path
     , processing_path
     , processing_sha256
     , original_name
     , content_type
     , sha256
     , byte_count
     , page_count
     , created_at
  FROM raw_files
 WHERE user_id = ?
   AND file_id = ?
"#;

pub const LIST_FILES: &str = r#"
SELECT file_id
     , upload_id
     , relative_path
     , processing_path
     , processing_sha256
     , original_name
     , content_type
     , sha256
     , byte_count
     , page_count
     , created_at
  FROM raw_files
 WHERE user_id = ?
   AND (? IS NULL OR file_id > ?)
ORDER BY file_id
 LIMIT ?
"#;

pub const INSERT_JOB: &str = r#"
INSERT INTO jobs (job_id
     , user_id
     , file_id
     , kind
     , status
     , error_code
     , created_at)
VALUES (?
     , ?
     , ?
     , 'document_extract'
     , 'queued'
     , NULL
     , ?)
"#;

pub const JOB_BY_ID: &str = r#"
SELECT job_id
     , file_id
     , kind
     , status
     , attempt_count
     , error_code
     , created_at
  FROM jobs
 WHERE user_id = ?
   AND job_id = ?
"#;

pub const JOB_BY_FILE: &str = r#"
SELECT job_id
     , file_id
     , kind
     , status
     , attempt_count
     , error_code
     , created_at
  FROM jobs
 WHERE user_id = ?
   AND file_id = ?
"#;

pub const LIST_JOBS: &str = r#"
SELECT job_id
     , file_id
     , kind
     , status
     , attempt_count
     , error_code
     , created_at
  FROM jobs
 WHERE user_id = ?
   AND (? IS NULL OR job_id > ?)
ORDER BY job_id
 LIMIT ?
"#;

pub const RETRY_JOB: &str = r#"
UPDATE jobs
   SET status = 'queued'
     , error_code = NULL
     , attempt_count = 0
 WHERE user_id = ?
   AND job_id = ?
   AND status = 'failed'
"#;

pub const EXHAUST_JOBS: &str = r#"
UPDATE jobs
   SET status = 'failed'
     , error_code = 'attempts_exhausted'
     , lease_token = NULL
     , lease_until = NULL
 WHERE status = 'running'
   AND lease_until <= ?
   AND attempt_count >= ?
"#;

pub const CLAIM_JOB: &str = r#"
UPDATE jobs
   SET status = 'running' 
     , attempt_count = attempt_count + 1 
     , lease_token = ? 
     , lease_until = ?
 WHERE job_id = (
SELECT job_id
  FROM jobs
 WHERE (status = 'queued' OR (status = 'running'
   AND lease_until <= ?))
   AND attempt_count < ?
ORDER BY created_at
     , job_id
 LIMIT 1 )
RETURNING job_id
     , user_id
     , file_id
     , lease_token
     , lease_until
"#;

pub const FINISH_JOB: &str = r#"
UPDATE jobs
   SET status = ?
     , error_code = ?
     , lease_token = NULL
     , lease_until = NULL
 WHERE job_id = ?
   AND lease_token = ?
   AND status = 'running'
   AND lease_until > ?
"#;

pub const RENEW_JOB: &str = r#"
UPDATE jobs
   SET lease_until = ?
 WHERE job_id = ?
   AND lease_token = ?
   AND status = 'running'
   AND lease_until > ?
"#;

pub const ALL_EXPORT_IDS: &str = "SELECT export_id FROM exports WHERE user_id = ?";

pub const UPLOAD_DELETED: &str =
    "SELECT COUNT(*) FROM deleted_uploads WHERE user_id = ? AND upload_id = ?";
pub const TOMBSTONE_UPLOAD: &str = "INSERT INTO deleted_uploads (user_id, upload_id) SELECT user_id, upload_id FROM raw_files WHERE user_id = ? AND file_id = ? ON CONFLICT DO NOTHING";
pub const DUPLICATE_FILES: &str = "SELECT file_id, original_name FROM raw_files WHERE user_id = ? AND sha256 = (SELECT sha256 FROM raw_files WHERE user_id = ? AND file_id = ?) AND file_id != ? ORDER BY file_id LIMIT 100";

pub const RAW_PATHS: &str = "SELECT relative_path FROM raw_files UNION SELECT raw_path AS relative_path FROM health_originals UNION SELECT relative_path FROM cleanup_files";
pub const HEALTH_RAW_TO_DELETE: &str = "SELECT raw_path FROM health_originals WHERE user_id = ? AND platform = ? AND record_id = ? AND (source_id = ? OR ? = '*')";
pub const HEALTH_RAW_LIST: &str = "SELECT raw_id AS revision_id, platform, source_id, record_id, raw_path, received_at, raw_offset, raw_length FROM health_originals WHERE user_id = ? AND (? IS NULL OR raw_id > ?) ORDER BY raw_id LIMIT ?";
pub const HEALTH_RAW_GET: &str = "SELECT raw_path, raw_offset, raw_length FROM health_originals WHERE user_id = ? AND raw_id = ?";
pub const INSERT_EXTRACTION_OUTPUT: &str = "INSERT INTO extraction_outputs (user_id, report_id, run_id, page_number, stage, response_body, status_code, content, model, adapter, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";
pub const EXTRACTION_OUTPUTS: &str = "SELECT run_id, page_number, stage, model, adapter, created_at FROM extraction_outputs WHERE user_id = ? AND report_id = ? ORDER BY created_at DESC, run_id, page_number, stage";

pub const EXTRACTION_OUTPUT: &str = "SELECT response_body, status_code, content FROM extraction_outputs WHERE user_id = ? AND report_id = ? AND run_id = ? AND page_number = ? AND stage = ?";

pub const INSERT_EXTRACTION_INPUT: &str = "INSERT INTO extraction_inputs (user_id, report_id, run_id, page_number, input_json, created_at) VALUES (?, ?, ?, ?, ?, ?)";
pub const EXTRACTION_INPUTS: &str = "SELECT run_id, page_number, created_at, json_extract(input_json, '$.text_layer.status') AS text_status, json_extract(input_json, '$.error_code') AS error_code FROM extraction_inputs WHERE user_id = ? AND report_id = ? ORDER BY created_at DESC, run_id, page_number";

pub const EXTRACTION_INPUT: &str = "SELECT input_json FROM extraction_inputs WHERE user_id = ? AND report_id = ? AND run_id = ? AND page_number = ?";

pub const WELLNESS_SOURCES: &str = "SELECT platform, source_id, record_type, COUNT(*) AS record_count, MIN(start_at) AS first_at, MAX(end_at) AS last_at FROM health_records WHERE user_id = ? AND deleted = 0 GROUP BY platform, source_id, record_type ORDER BY platform, source_id, record_type";

// Domain reads are bounded independently from the complete raw archive.
pub const WELLNESS_ENTRIES: &str = "SELECT platform, source_id, record_id, record_type, start_at, end_at, version, payload_json FROM health_records WHERE user_id = ? AND platform = 'manual' AND source_id = 'longlivehealth' AND deleted = 0 AND start_at >= ? AND start_at < ? AND (? IS NULL OR record_type = ?) ORDER BY start_at DESC, record_id LIMIT 1001";
pub const TRAINING_IMPORTS: &str = "SELECT source_id, record_id, start_at, end_at, version, json_extract(payload_json, '$.payload.filename') AS filename, json_extract(payload_json, '$.payload.distance_m') AS distance_m FROM health_records WHERE user_id = ? AND platform = 'file_import' AND source_id IN ('gpx-1.1','tcx-2','fit') AND deleted = 0 AND (? IS NULL OR record_id > ?) ORDER BY record_id LIMIT 101";

pub const WELLNESS_SERIES: &str = r#"
WITH records AS (
 SELECT * FROM health_records
 WHERE user_id=? AND record_type=? AND deleted=0 AND start_at>=? AND start_at<?
), samples AS (
 SELECT platform,source_id,record_id,version,CAST(start_at AS REAL) AS start_at,
        CASE WHEN json_type(payload_json,'$.payload.value') IN ('integer','real')
             THEN CAST(json_extract(payload_json,'$.payload.value') AS REAL) END AS value,
        substr(json_extract(payload_json,'$.payload.unit'),1,64) AS unit
 FROM records WHERE json_array_length(payload_json,'$.payload.quantity_samples') IS NULL
 UNION ALL
 SELECT r.platform,r.source_id,r.record_id || ':' || j.key,r.version,
        CAST(json_extract(j.value,'$.at') AS REAL),CAST(json_extract(j.value,'$.value') AS REAL),COALESCE(json_extract(j.value,'$.unit'),json_extract(r.payload_json,'$.payload.unit'),'count/min')
 FROM records r,json_each(r.payload_json,'$.payload.quantity_samples') j
)
SELECT * FROM samples ORDER BY platform,source_id,start_at,record_id LIMIT 200001
"#;

pub const WELLNESS_TIMELINE: &str = r#"
WITH events AS (
    SELECT start_at AS event_at, json_array('health',platform,source_id,record_id) AS event_key,
           record_id, record_type, platform || ':' || source_id AS source, version,
           'record_start' AS time_semantics
      FROM health_records
     WHERE user_id=? AND start_at>=? AND start_at<? AND deleted=0
       AND record_type IN ('sleep','workout','training','training_day','sleep_correction','journal','nutrition','cycle','body','blood_pressure')
    UNION ALL
    SELECT created_at, json_array('report',report_id), report_id, 'report', 'report_upload', revision, 'upload'
      FROM reports WHERE user_id=? AND created_at>=? AND created_at<?
)
SELECT * FROM events
 WHERE ? IS NULL OR event_at < ? OR (event_at = ? AND event_key > ?)
 ORDER BY event_at DESC,event_key ASC
 LIMIT 201
"#;

pub const EXPORT_COLUMNS: &str =
    "SELECT name,type,\"notnull\",pk FROM pragma_table_info(?) ORDER BY cid";

pub const ENABLE_USER: &str = r#"
UPDATE users
   SET password_hash = ?
     , is_active = 1
     , credential_version = credential_version + 1
 WHERE username = ?
RETURNING user_id
"#;

pub const INSERT_DERIVED_RESULT: &str = "INSERT INTO derived_results (result_id,user_id,kind,algorithm_version,input_revision,parameters_json,input_json,output_json,created_at) VALUES (?,?,'behavior_association',?,?,?,?,?,?)";
pub const LIST_DERIVED_RESULTS: &str = "SELECT result_id,kind,algorithm_version,input_revision,parameters_json,created_at FROM derived_results WHERE user_id=? ORDER BY created_at DESC,result_id LIMIT 100";
pub const GET_DERIVED_RESULT: &str =
    "SELECT output_json FROM derived_results WHERE user_id=? AND result_id=?";
pub const LIMIT_DERIVED_RESULTS: &str = "DELETE FROM derived_results WHERE user_id=? AND result_id NOT IN (SELECT result_id FROM derived_results WHERE user_id=? ORDER BY created_at DESC,result_id LIMIT 100)";

pub const HRV_WINDOWS: &str = "SELECT platform, source_id, record_id, record_type, start_at, end_at, version, payload_json FROM health_records WHERE user_id = ? AND deleted = 0 AND record_type IN ('nn_intervals', 'heartbeat_series') AND start_at >= ? AND start_at < ? ORDER BY start_at, record_id LIMIT 1001";
pub const WELLNESS_LIBRARY: &str = "SELECT platform, source_id, record_id, record_type, start_at, end_at, version, payload_json FROM health_records WHERE user_id = ? AND platform = 'manual' AND source_id = 'longlivehealth' AND record_type = ? AND deleted = 0 ORDER BY start_at DESC, record_id LIMIT 1001";
pub const FOOD_PORTION: &str = "SELECT platform, source_id, record_id, record_type, start_at, end_at, version, payload_json FROM health_records WHERE user_id = ? AND platform = 'manual' AND source_id = 'longlivehealth' AND record_id = ? AND record_type IN ('food', 'recipe') AND deleted = 0";

pub const SPECIALTY_LIST: &str = "SELECT platform,source_id,record_id,record_type,start_at,version,payload_json, (SELECT json_group_array(json_object('id',c.record_type,'value',json_extract(c.payload_json,'$.payload.value'),'unit',json_extract(c.payload_json,'$.payload.unit'))) FROM json_each(h.payload_json,'$.payload.related_sample_ids') ids CROSS JOIN health_records c ON c.user_id=h.user_id AND c.platform=h.platform AND c.source_id=h.source_id AND c.record_id=ids.value AND c.deleted=0) AS related_values FROM health_records h WHERE user_id=?1 AND deleted=0 AND record_type=?2 AND (?3 IS NULL OR (start_at,platform,source_id,record_id)<(?4,?5,?6,?7)) ORDER BY start_at DESC,platform DESC,source_id DESC,record_id DESC LIMIT 51";
pub const SPECIALTY_CLINICAL_LIST: &str = "SELECT platform,source_id,record_id,record_type,start_at,version,payload_json, (SELECT json_group_array(json_object('id',c.record_type,'value',json_extract(c.payload_json,'$.payload.value'),'unit',json_extract(c.payload_json,'$.payload.unit'))) FROM json_each(h.payload_json,'$.payload.related_sample_ids') ids CROSS JOIN health_records c ON c.user_id=h.user_id AND c.platform=h.platform AND c.source_id=h.source_id AND c.record_id=ids.value AND c.deleted=0) AS related_values FROM health_records h WHERE user_id=?1 AND deleted=0 AND ?2 IS NOT NULL AND json_type(payload_json,'$.payload.fhir')='object' AND (?3 IS NULL OR (start_at,platform,source_id,record_id)<(?4,?5,?6,?7)) ORDER BY start_at DESC,platform DESC,source_id DESC,record_id DESC LIMIT 51";
pub const SPECIALTY_BLOOD_PRESSURE_LIST: &str = "SELECT platform,source_id,record_id,record_type,start_at,version,payload_json, (SELECT json_group_array(json_object('id',c.record_type,'value',json_extract(c.payload_json,'$.payload.value'),'unit',json_extract(c.payload_json,'$.payload.unit'))) FROM json_each(h.payload_json,'$.payload.related_sample_ids') ids CROSS JOIN health_records c ON c.user_id=h.user_id AND c.platform=h.platform AND c.source_id=h.source_id AND c.record_id=ids.value AND c.deleted=0) AS related_values FROM health_records h WHERE user_id=?1 AND deleted=0 AND ?2 IS NOT NULL AND record_type IN ('HKCorrelationTypeIdentifierBloodPressure','blood_pressure','blood_pressure_systolic','blood_pressure_diastolic') AND (record_type IN ('HKCorrelationTypeIdentifierBloodPressure','blood_pressure') OR NOT EXISTS (SELECT 1 FROM health_records p,json_each(p.payload_json,'$.payload.related_sample_ids') ids WHERE p.user_id=h.user_id AND p.platform=h.platform AND p.source_id=h.source_id AND p.record_type='HKCorrelationTypeIdentifierBloodPressure' AND p.deleted=0 AND ids.value=h.record_id)) AND (?3 IS NULL OR (start_at,platform,source_id,record_id)<(?4,?5,?6,?7)) ORDER BY start_at DESC,platform DESC,source_id DESC,record_id DESC LIMIT 51";
pub const SPECIALTY_GET: &str = "SELECT platform,source_id,record_id,record_type,start_at,end_at,version,payload_json FROM health_records WHERE user_id=? AND platform=? AND source_id=? AND record_id=? AND deleted=0";
pub const RELATED_RECORDS: &str = "SELECT record_type,version,start_at,CASE WHEN json_type(payload_json,'$.payload.value') IN ('real','integer') THEN CAST(json_extract(payload_json,'$.payload.value') AS REAL) END AS value,json_extract(payload_json,'$.payload.unit') AS unit FROM health_records WHERE user_id=? AND platform=? AND source_id=? AND record_id=? AND deleted=0 LIMIT 1";

pub const DAILY_VIEW_GET: &str = "SELECT result_json FROM daily_views WHERE user_id=? AND request_key=? AND status='ready' AND input_revision=(SELECT data_revision FROM users WHERE user_id=?)";
pub const DAILY_VIEW_SAVE: &str = "INSERT INTO daily_views(user_id,request_key,parameters_json,input_revision,status,result_json,requested_at) VALUES(?,?,?,?,'ready',?,?) ON CONFLICT(user_id,request_key) DO UPDATE SET input_revision=excluded.input_revision,status='ready',result_json=excluded.result_json,claim_token=NULL,lease_until=NULL,attempts=0,error_code=NULL,requested_at=excluded.requested_at";
pub const DAILY_VIEW_LIMIT: &str = "DELETE FROM daily_views WHERE user_id=? AND request_key NOT IN (SELECT request_key FROM daily_views WHERE user_id=? ORDER BY requested_at DESC,request_key LIMIT 64)";
pub const DAILY_VIEW_CLAIM: &str = "UPDATE daily_views SET status='running',claim_token=?,lease_until=?,attempts=attempts+1 WHERE (user_id,request_key)=(SELECT user_id,request_key FROM daily_views WHERE status='queued' OR (status='running' AND lease_until<=?) ORDER BY requested_at,user_id,request_key LIMIT 1) RETURNING user_id,request_key,parameters_json,input_revision,attempts";
pub const DAILY_VIEW_FINISH: &str = "UPDATE daily_views SET status='ready',result_json=?,claim_token=NULL,lease_until=NULL,error_code=NULL WHERE user_id=? AND request_key=? AND claim_token=? AND input_revision=? AND input_revision=(SELECT data_revision FROM users WHERE user_id=?)";
pub const DAILY_VIEW_FAIL: &str = "UPDATE daily_views SET status=CASE WHEN attempts>=3 THEN 'failed' ELSE 'queued' END,claim_token=NULL,lease_until=NULL,error_code='calculation_failed' WHERE user_id=? AND request_key=? AND claim_token=?";
pub const DAILY_VIEW_EXHAUSTED: &str = "UPDATE daily_views SET status='failed',claim_token=NULL,lease_until=NULL,error_code='attempts_exhausted' WHERE status='running' AND attempts>=3 AND lease_until<=?";

pub const USERNAME_BY_ID: &str = "SELECT username FROM users WHERE user_id = ?";
pub const INITIALIZE_ADMIN: &str = "INSERT INTO users (user_id, username, password_hash, created_at, must_change_password) VALUES (?, 'admin', ?, ?, 1) ON CONFLICT(username) DO NOTHING";
pub const RESET_ADMIN: &str = "UPDATE users SET password_hash = ?, must_change_password = 1, is_active = 1, credential_version = credential_version + 1 WHERE username = 'admin' RETURNING user_id";
pub const LIST_USERS: &str = "SELECT user_id, username, username = 'admin' AS is_admin, must_change_password FROM users WHERE (? IS NULL OR user_id > ?) ORDER BY user_id LIMIT ?";
pub const CHANGE_OWN_PASSWORD: &str = "UPDATE users SET password_hash = ?, must_change_password = 0, credential_version = credential_version + 1 WHERE user_id = ? AND credential_version = ? AND is_active = 1 AND EXISTS (SELECT 1 FROM sessions WHERE user_id = users.user_id AND ((token_hash = ?4 AND expires_at > ?5) OR (previous_token_hash = ?4 AND previous_expires_at > ?5))) RETURNING user_id";
pub const CLEAR_ADMIN_LOGIN_WINDOW: &str = "DELETE FROM login_windows WHERE username_hash = ?";

pub const WELLNESS_ENTRY: &str = "SELECT platform, source_id, record_id, record_type, start_at, end_at, version, payload_json FROM health_records WHERE user_id = ? AND platform = 'manual' AND source_id = 'longlivehealth' AND record_id = ? AND deleted = 0";

pub const HEALTH_RECORD_TOTAL: &str =
    "SELECT COALESCE((SELECT record_count FROM health_totals WHERE user_id = ?), 0)";

// Only project the chart payload; HealthKit raw archives can be several MiB each.
// Avoid the platform primary-key scan across all health types on large archives.
pub const METRIC_HEALTH_HISTORY: &str = "SELECT source_id, record_id, record_type, start_at, json_object('value', json_extract(payload_json, '$.payload.value'), 'unit', json_extract(payload_json, '$.payload.unit'), 'source_name', json_extract(payload_json, '$.payload.source_name'), 'fhir', json_extract(payload_json, '$.payload.fhir')) AS chart_payload FROM health_records INDEXED BY health_records_range WHERE user_id = ? AND platform = 'apple_health' AND deleted = 0 AND (record_type = ? OR record_type = 'HKClinicalTypeIdentifierLabResultRecord') ORDER BY start_at, source_id, record_id LIMIT 10001";

// Input contains only bounded record/source identifiers and payload lengths, never raw health payloads.
pub const HEALTH_BATCH_CURRENT: &str = "SELECT json_extract(item.value, '$[0]') AS input_source, json_extract(item.value, '$[1]') AS input_id, current.version, current.deleted, CASE WHEN LENGTH(CAST(current.payload_json AS BLOB)) = json_extract(item.value, '$[2]') THEN current.payload_json ELSE NULL END AS payload_json, current.record_type, EXISTS(SELECT 1 FROM health_records AS tombstone WHERE tombstone.user_id = ?1 AND tombstone.platform = ?2 AND tombstone.source_id IN (json_extract(item.value, '$[0]'), '*') AND tombstone.record_id = json_extract(item.value, '$[1]') AND tombstone.deleted = 1) AS tombstoned FROM json_each(?3) AS item LEFT JOIN health_records AS current ON current.user_id = ?1 AND current.platform = ?2 AND current.source_id = json_extract(item.value, '$[0]') AND current.record_id = json_extract(item.value, '$[1]')";

pub const REFRESH_SESSION: &str = "SELECT token_hash, refresh_hash FROM sessions WHERE (refresh_hash = ?1 OR previous_refresh_hash = ?1) AND refresh_expires_at > ?2 AND EXISTS (SELECT 1 FROM users WHERE users.user_id = sessions.user_id AND is_active = 1)";
pub const ROTATE_SESSION: &str = "UPDATE sessions SET previous_token_hash = token_hash, previous_expires_at = MIN(expires_at, ?1 + 300), previous_refresh_hash = refresh_hash, token_hash = ?2, expires_at = ?3, refresh_hash = ?4, refresh_expires_at = ?5 WHERE token_hash = ?6";

pub const RENEW_SESSION_EXPIRY: &str =
    "UPDATE sessions SET expires_at = ?1, refresh_expires_at = ?2 WHERE token_hash = ?3";
pub const REVOKE_REFRESH_SESSION: &str =
    "DELETE FROM sessions WHERE refresh_hash = ?1 OR previous_refresh_hash = ?1";

pub const REPORT_ORIGINAL: &str = "SELECT file_id, relative_path, content_type FROM raw_files WHERE user_id = ? AND file_id = (SELECT file_id FROM reports WHERE user_id = ? AND report_id = ?)";
pub const RELOCATE_ORIGINAL: &str =
    "UPDATE raw_files SET relative_path = ? WHERE user_id = ? AND file_id = ?";

pub const CANCEL_PATH_CLEANUP: &str = "DELETE FROM cleanup_files WHERE relative_path = ?";

pub const SOURCE_DISPLAY_NAME: &str = "SELECT json_extract(payload_json, '$.payload.source_name') FROM health_records WHERE user_id = ? AND platform = ? AND source_id = ? AND deleted = 0 LIMIT 1";
pub const REPORT_SOURCE_CATALOG: &str = "SELECT r.report_id, r.context_json, f.original_name FROM reports r JOIN raw_files f ON f.user_id = r.user_id AND f.file_id = r.file_id WHERE r.user_id = ? ORDER BY r.report_id";

pub const HEART_SAMPLE_REINDEX: &str = "SELECT h.user_id,h.platform,h.source_id,h.record_id,h.raw_id,h.payload_json,o.raw_path,o.raw_offset,o.raw_length FROM health_records h JOIN health_originals o ON o.user_id=h.user_id AND o.raw_id=h.raw_id WHERE h.record_type='heart_rate' AND h.deleted=0 AND json_extract(h.payload_json,'$.payload.quantity_count')>1 AND json_type(h.payload_json,'$.payload.quantity_samples') IS NULL";
pub const RECOVERY_HRV_REINDEX: &str = "SELECT h.user_id,h.platform,h.source_id,h.record_id,h.raw_id,h.payload_json,o.raw_path,o.raw_offset,o.raw_length FROM health_records h JOIN health_originals o ON o.user_id=h.user_id AND o.raw_id=h.raw_id WHERE h.platform='apple_health' AND h.record_type='HKQuantityTypeIdentifierHeartRateVariabilityRMSSD' AND h.deleted=0 AND json_type(h.payload_json,'$.payload.value') IS NULL";
pub const UPDATE_HEALTH_SAMPLE_PROJECTION: &str = "UPDATE health_records SET payload_json=? WHERE user_id=? AND platform=? AND source_id=? AND record_id=? AND raw_id=? AND payload_json=?";

pub const DAILY_HISTORY_BOUNDS: &str = r#"
SELECT MIN(start_at)
     , MAX(end_at)
  FROM health_records
 WHERE user_id = ?
   AND record_type = ?
   AND deleted = 0
"#;

pub const MEASUREMENT_SAMPLES: &str = r#"
WITH records AS (
 SELECT * FROM health_records
 WHERE user_id=?1 AND record_type=?2 AND deleted=0 AND end_at>=?3 AND start_at<?4
), samples AS (
 SELECT platform,source_id,record_id,CAST(start_at AS REAL) AS at,
        CASE WHEN json_type(payload_json,'$.payload.value') IN ('integer','real')
             THEN CAST(json_extract(payload_json,'$.payload.value') AS REAL) END AS value,
        substr(json_extract(payload_json,'$.payload.unit'),1,64) AS unit
 FROM records WHERE json_array_length(payload_json,'$.payload.quantity_samples') IS NULL
 UNION ALL
 SELECT r.platform,r.source_id,r.record_id || ':' || j.key,
        CAST(json_extract(j.value,'$.at') AS REAL),CAST(json_extract(j.value,'$.value') AS REAL),COALESCE(json_extract(j.value,'$.unit'),json_extract(r.payload_json,'$.payload.unit'),'count/min')
 FROM records r,json_each(r.payload_json,'$.payload.quantity_samples') j
 UNION ALL
 SELECT platform,source_id,record_id,CAST(start_at AS REAL),
        CAST(json_extract(payload_json,'$.payload.entry.content.weight_kg') AS REAL),'kg'
 FROM health_records
 WHERE ?2='body_mass' AND user_id=?1 AND platform='manual' AND record_type='body' AND deleted=0
   AND start_at>=?3 AND start_at<?4 AND json_type(payload_json,'$.payload.entry.content.weight_kg') IN ('integer','real')
)
SELECT * FROM samples WHERE at>=?3 AND at<?4
"#;
// Bounds use plotted sample timestamps, not the enclosing record's end date.
pub const MEASUREMENT_BOUNDS: &str = r#"
WITH latest_start AS (
    SELECT start_at FROM health_records INDEXED BY health_records_range
     WHERE user_id=?1 AND record_type=?2 AND deleted=0 ORDER BY start_at DESC LIMIT 1
)
SELECT MIN(first_at),CAST(MAX(last_at) AS INTEGER) FROM (
    SELECT
      (SELECT start_at FROM health_records INDEXED BY health_records_range
        WHERE user_id=?1 AND record_type=?2 AND deleted=0 ORDER BY start_at LIMIT 1) AS first_at,
      (SELECT MAX(CASE WHEN json_array_length(payload_json,'$.payload.quantity_samples') IS NULL THEN start_at
                  ELSE (SELECT MAX(CAST(json_extract(j.value,'$.at') AS REAL))
                          FROM json_each(payload_json,'$.payload.quantity_samples') j) END)
         FROM health_records INDEXED BY health_records_end
        WHERE user_id=?1 AND record_type=?2 AND deleted=0
          AND end_at>=(SELECT start_at FROM latest_start)) AS last_at
    UNION ALL
    SELECT
      (SELECT start_at FROM health_records INDEXED BY health_records_range
        WHERE ?2='body_mass' AND user_id=?1 AND record_type='body' AND platform='manual' AND deleted=0
          AND json_type(payload_json,'$.payload.entry.content.weight_kg') IN ('integer','real')
        ORDER BY start_at LIMIT 1) AS first_at,
      (SELECT start_at FROM health_records INDEXED BY health_records_range
        WHERE ?2='body_mass' AND user_id=?1 AND record_type='body' AND platform='manual' AND deleted=0
          AND json_type(payload_json,'$.payload.entry.content.weight_kg') IN ('integer','real')
        ORDER BY start_at DESC LIMIT 1) AS last_at
)
"#;

pub fn measurement_history_query() -> String {
    format!(
        "{MEASUREMENT_SAMPLES} ORDER BY at DESC,platform,source_id,record_id LIMIT 501 OFFSET ?5"
    )
}

pub const ATTACH_DRAFTS: &str = "ATTACH DATABASE ? AS draft";
pub const REVIEW_OBSERVATIONS_VIEW: &str = r#"
CREATE TEMP VIEW review_observations AS
SELECT o.* FROM main.observations o WHERE NOT EXISTS (
 SELECT 1 FROM draft.observations d WHERE d.observation_id=o.observation_id AND d.current_revision>o.current_revision)
UNION ALL
SELECT d.* FROM draft.observations d WHERE EXISTS (SELECT 1 FROM main.reports r WHERE r.user_id=d.user_id AND r.report_id=d.report_id)
AND NOT EXISTS (SELECT 1 FROM main.observations o WHERE o.observation_id=d.observation_id AND o.current_revision>=d.current_revision);
"#;
pub const MIGRATE_REPORT_DRAFTS: &str = "INSERT INTO draft.observations SELECT * FROM main.observations WHERE status!='confirmed' ON CONFLICT(observation_id) DO UPDATE SET current_revision=excluded.current_revision,status=excluded.status,payload_json=excluded.payload_json,updated_at=excluded.updated_at WHERE excluded.current_revision>observations.current_revision";
pub const REMOVE_SUPERSEDED_OFFICIAL: &str = "DELETE FROM main.observations WHERE EXISTS (SELECT 1 FROM draft.observations d WHERE d.observation_id=observations.observation_id AND (d.current_revision>observations.current_revision OR (d.current_revision=observations.current_revision AND observations.status!='confirmed')))";
pub const REMOVE_SUPERSEDED_DRAFTS: &str = "DELETE FROM draft.observations WHERE NOT EXISTS (SELECT 1 FROM main.reports r WHERE r.user_id=observations.user_id AND r.report_id=observations.report_id) OR EXISTS (SELECT 1 FROM main.observations o WHERE o.observation_id=observations.observation_id AND o.current_revision>=observations.current_revision)";

pub const HEALTH_ARCHIVE_END: &str = "SELECT raw_offset + 8 + raw_length FROM health_originals WHERE raw_path = ? AND raw_offset IS NOT NULL ORDER BY raw_offset DESC LIMIT 1";

// Normal Apple batches have unique identities and no deletions. Their conflict and
// tombstone checks use HEALTH_BATCH_CURRENT; write the validated rows in two calls.
pub const UPSERT_APPLE_BATCH: &str = r#"
INSERT INTO health_records (user_id, platform, source_id, record_id, record_type, start_at, end_at, version, deleted, payload_json, raw_id)
SELECT ?1, 'apple_health'
     , json_extract(value, '$[0]'), json_extract(value, '$[1]'), json_extract(value, '$[2]')
     , json_extract(value, '$[3]'), json_extract(value, '$[4]'), json_extract(value, '$[5]')
     , 0, json_extract(value, '$[6]'), json_extract(value, '$[7]')
  FROM json_each(?2)
 WHERE TRUE
ON CONFLICT(user_id, platform, record_id, source_id) DO UPDATE SET
    record_type=excluded.record_type, start_at=excluded.start_at, end_at=excluded.end_at,
    version=excluded.version, deleted=0, payload_json=excluded.payload_json, raw_id=excluded.raw_id
"#;
pub const INSERT_APPLE_ORIGINALS: &str = r#"
INSERT INTO health_originals (user_id, platform, source_id, record_id, raw_id, raw_path, raw_offset, raw_length, received_at)
SELECT ?1, 'apple_health'
     , json_extract(value, '$[0]'), json_extract(value, '$[1]'), json_extract(value, '$[7]')
     , json_extract(value, '$[8]'), json_extract(value, '$[9]'), json_extract(value, '$[10]'), ?3
  FROM json_each(?2)
"#;

pub const EXTRACTION_VALIDATION_CONTENT: &str = "SELECT page_number, content FROM extraction_outputs WHERE user_id = ?1 AND report_id = ?2 AND run_id = (SELECT run_id FROM extraction_outputs WHERE user_id = ?1 AND report_id = ?2 ORDER BY created_at DESC, run_id LIMIT 1) ORDER BY page_number";

pub const APPLE_QUANTITY_REINDEX: &str = "SELECT h.rowid AS archive_row,h.user_id,h.platform,h.source_id,h.record_id,h.raw_id,h.record_type,h.payload_json,o.raw_path,o.raw_offset,o.raw_length FROM health_records h JOIN health_originals o ON o.user_id=h.user_id AND o.raw_id=h.raw_id WHERE h.rowid>? AND h.platform='apple_health' AND h.deleted=0 AND ((h.record_type GLOB 'HKQuantityTypeIdentifier*' AND json_type(h.payload_json,'$.payload.value') IS NULL) OR (json_extract(h.payload_json,'$.payload.quantity_count')>1 AND json_type(h.payload_json,'$.payload.quantity_samples') IS NULL)) ORDER BY h.rowid LIMIT 500";

// A rescan replaces report results, never the original file or user-entered report info.
pub const REPORT_ACTIVE_JOB: &str =
    "SELECT COUNT(*) FROM jobs WHERE user_id=? AND file_id=? AND status IN ('queued','running')";
pub const RESCAN_CLEAR: &[&str] = &[
    "DELETE FROM main.observations WHERE user_id=? AND report_id=?",
    "DELETE FROM draft.observations WHERE user_id=? AND report_id=?",
    "DELETE FROM extraction_pages WHERE user_id=? AND report_id=?",
    "DELETE FROM extraction_inputs WHERE user_id=? AND report_id=?",
    "DELETE FROM extraction_outputs WHERE user_id=? AND report_id=?",
    "DELETE FROM jobs WHERE user_id=? AND file_id=?",
];

/// Ordered complete scalar history used to resolve measurement-time reference context.
pub fn reference_context_history_query() -> String {
    format!("{MEASUREMENT_SAMPLES} ORDER BY at,platform,source_id,record_id")
}

// The explicit relationship, account, platform and source form a measurement pair.
pub const COMBINED_SAMPLES: &str = r#"
SELECT p.platform,p.source_id,p.record_id,COALESCE(json_extract(p.payload_json,'$.payload.chart_at'),p.start_at) AS at,c.record_type AS component,c.record_id AS component_record,NULL AS title,
       CASE WHEN json_type(c.payload_json,'$.payload.value') IN ('integer','real')
            THEN CAST(json_extract(c.payload_json,'$.payload.value') AS REAL) END AS value,
       json_extract(c.payload_json,'$.payload.unit') AS unit
  FROM health_records p
  LEFT JOIN json_each(p.payload_json,'$.payload.related_sample_ids') ids ON TRUE
  LEFT JOIN health_records c ON c.user_id=p.user_id AND c.platform=p.platform
       AND c.source_id=p.source_id AND c.record_id=ids.value AND c.deleted=0
 WHERE p.user_id=?1 AND p.record_type=?2 AND p.deleted=0 AND COALESCE(json_extract(p.payload_json,'$.payload.chart_at'),p.start_at)>=?3 AND COALESCE(json_extract(p.payload_json,'$.payload.chart_at'),p.start_at)<?4
   AND json_type(p.payload_json,'$.payload.related_sample_ids')='array'
 UNION ALL
SELECT p.platform,p.source_id,p.record_id,COALESCE(json_extract(p.payload_json,'$.payload.chart_at'),p.start_at),json_extract(j.value,'$.id'),json_extract(j.value,'$.id'),
       json_extract(j.value,'$.title'),CASE WHEN json_type(j.value,'$.value') IN ('integer','real') THEN CAST(json_extract(j.value,'$.value') AS REAL) END,json_extract(j.value,'$.unit')
  FROM health_records p LEFT JOIN json_each(p.payload_json,'$.payload.chart_components') j ON TRUE
 WHERE p.user_id=?1 AND p.record_type=?2 AND p.deleted=0 AND COALESCE(json_extract(p.payload_json,'$.payload.chart_at'),p.start_at)>=?3 AND COALESCE(json_extract(p.payload_json,'$.payload.chart_at'),p.start_at)<?4
   AND json_type(p.payload_json,'$.payload.related_sample_ids') IS NULL
 UNION ALL
SELECT s.platform,s.source_id,s.record_id,s.start_at,s.record_type,s.record_id,NULL,
       CASE WHEN json_type(s.payload_json,'$.payload.value') IN ('integer','real') THEN CAST(json_extract(s.payload_json,'$.payload.value') AS REAL) END,
       json_extract(s.payload_json,'$.payload.unit')
 FROM health_records s
 WHERE ?2='HKCorrelationTypeIdentifierBloodPressure' AND s.user_id=?1 AND s.deleted=0
   AND s.record_type IN ('blood_pressure_systolic','blood_pressure_diastolic') AND s.start_at>=?3 AND s.start_at<?4
   AND NOT EXISTS (SELECT 1 FROM health_records p,json_each(p.payload_json,'$.payload.related_sample_ids') ids
       WHERE p.user_id=s.user_id AND p.platform=s.platform AND p.source_id=s.source_id AND p.deleted=0
         AND p.record_type=?2 AND ids.value=s.record_id)
 UNION ALL
SELECT p.platform,p.source_id,p.record_id,p.start_at,
       CASE f.value WHEN 'systolic_mmhg' THEN 'blood_pressure_systolic' ELSE 'blood_pressure_diastolic' END,f.value,NULL,
       CAST(json_extract(p.payload_json,'$.payload.entry.content.'||f.value) AS REAL),'mmHg'
 FROM health_records p,json_each('["systolic_mmhg","diastolic_mmhg"]') f
 WHERE ?2='HKCorrelationTypeIdentifierBloodPressure' AND p.user_id=?1 AND p.deleted=0 AND p.platform='manual' AND p.record_type='blood_pressure'
   AND p.start_at>=?3 AND p.start_at<?4
"#;

pub const COMPONENT_REINDEX: &str = r#"
SELECT h.rowid AS archive_row,h.user_id,h.platform,h.source_id,h.record_id,h.record_type,
       h.start_at,h.end_at,h.raw_id,h.payload_json,o.raw_path,o.raw_offset,o.raw_length
  FROM health_records h JOIN health_originals o ON o.user_id=h.user_id AND o.raw_id=h.raw_id
 WHERE h.rowid>? AND h.deleted=0 AND json_type(h.payload_json,'$.payload.chart_components') IS NULL
   AND (h.record_type LIKE 'HKCategoryTypeIdentifier%' OR h.record_type IN
       ('mindful_session','activity_summary','state_of_mind','vision_prescription','electrocardiogram','heartbeat_series','workout','medication_dose_event'))
 ORDER BY h.rowid LIMIT 100
"#;

pub const COMBINED_BOUNDS: &str = "SELECT MIN(COALESCE(json_extract(payload_json,'$.payload.chart_at'),start_at)), MAX(COALESCE(json_extract(payload_json,'$.payload.chart_at'),start_at)) FROM health_records WHERE user_id=?1 AND (record_type=?2 OR (?2='HKCorrelationTypeIdentifierBloodPressure' AND record_type IN ('blood_pressure_systolic','blood_pressure_diastolic','blood_pressure'))) AND deleted=0";
