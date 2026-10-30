"""Statements for the offline schema 7 to 8 raw archive conversion."""
VERSION = 'PRAGMA user_version'
STAGING_EXISTS = "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='packed_health_originals'"
MISSING = """
SELECT o.rowid AS migration_rowid, o.*
  FROM health_originals o
  LEFT JOIN packed_health_originals p ON p.user_id=o.user_id AND p.raw_id=o.raw_id
 WHERE o.rowid > ? AND p.raw_id IS NULL
 ORDER BY o.rowid
 LIMIT 5000
"""
INSERT = "INSERT INTO packed_health_originals (user_id,platform,source_id,record_id,raw_id,raw_path,received_at,raw_offset,raw_length) VALUES (?,?,?,?,?,?,?,?,?)"
END = "SELECT raw_offset + 8 + raw_length FROM packed_health_originals WHERE raw_path=? AND raw_offset IS NOT NULL ORDER BY raw_offset DESC LIMIT 1"
CREATE_END_INDEX = 'CREATE INDEX IF NOT EXISTS packed_health_end ON packed_health_originals(raw_path,raw_offset)'
COUNTS = 'SELECT (SELECT COUNT(*) FROM health_originals), (SELECT COUNT(*) FROM packed_health_originals)'
OLD_APPLE_PATHS = "SELECT raw_path FROM health_originals WHERE platform='apple_health'"
DROP_OLD = 'DROP TABLE health_originals'
RENAME = 'ALTER TABLE packed_health_originals RENAME TO health_originals'
DROP_END_INDEX = 'DROP INDEX packed_health_end'
SET_VERSION = 'PRAGMA user_version=8'
CHECK_KEYS = 'PRAGMA foreign_key_check'
CHECK_LINKS = 'SELECT COUNT(*) FROM health_records h WHERE h.deleted=0 AND NOT EXISTS (SELECT 1 FROM health_originals o WHERE o.user_id=h.user_id AND o.raw_id=h.raw_id)'

SCHEMA_TABLE = "SELECT sql FROM sqlite_master WHERE type='table' AND name='health_originals'"
SCHEMA_INDEXES = "SELECT sql FROM sqlite_master WHERE type='index' AND tbl_name='health_originals' AND sql IS NOT NULL"
FULL_SYNC = 'PRAGMA synchronous=FULL'
FOREIGN_KEYS = 'PRAGMA foreign_keys=ON'
BEGIN_WRITE = 'BEGIN IMMEDIATE'

COPY_CACHE = 'PRAGMA cache_size=-1048576'

INVALIDATE_EXPORTS = "UPDATE exports SET status='stale' WHERE status!='stale'"

DEFER_CHECKPOINT = 'PRAGMA wal_autocheckpoint=0'
