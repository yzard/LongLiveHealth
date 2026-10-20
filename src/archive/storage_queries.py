"""SQLite statements for the offline storage compactor."""
COPY_CACHE = "PRAGMA cache_size = -131072"
SCHEMA = "SELECT type, name, sql FROM sqlite_master WHERE sql IS NOT NULL AND name NOT LIKE 'sqlite_%'"
TABLES = "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'"
REBUILD_TOTALS = "INSERT INTO health_totals (user_id, record_count) SELECT user_id, COUNT(*) FROM health_records WHERE platform IN ('apple_health', 'health_connect') AND deleted = 0 GROUP BY user_id"
CHECK_ORIGINALS = "SELECT COUNT(*) FROM health_records h WHERE h.deleted=0 AND NOT EXISTS(SELECT 1 FROM health_originals r WHERE r.user_id=h.user_id AND r.raw_id=h.raw_id AND r.platform=h.platform AND r.source_id=h.source_id AND r.record_id=h.record_id)"
def select_rows(table):
    # Names are exclusively from the local canonical schema.
    return f'SELECT * FROM "{table}"'


def count_rows(table):
    # Same closed canonical/legacy table mapping as select_rows.
    return f'SELECT COUNT(*) FROM "{table}"'
